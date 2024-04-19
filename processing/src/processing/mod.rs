use crate::{
    custom_bfs::Positionable,
    math::midpoint,
    parse::{Point, RoadDirection, SensorData},
};

use std::collections::{HashMap, HashSet};

use clap::ValueEnum;
use kdtree::{distance::squared_euclidean, KdTree};
use petgraph::{
    graph::NodeIndex,
    prelude::EdgeIndex,
    stable_graph::{StableDiGraph, StableGraph},
    visit::{Bfs, EdgeRef, IntoEdgeReferences, IntoNodeReferences, VisitMap},
    Direction::{Incoming, Outgoing},
};
use serde::{Deserialize, Serialize};

use crate::{
    math::{angle_average, angle_diff, dist, line_heading, point_line_dist_approx},
    parse::RoadData,
    PointQuery,
};

pub mod collapse;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NodeData {
    pub point: Point,
    pub direction: RoadDirection,
    pub sensor: Option<SensorData>,
    pub main_number: i32,
    pub sub_number: i32,
    pub original_road_id: i32,
    pub heading: f32,
}

impl Positionable for NodeData {
    fn point(&self) -> Point {
        self.point
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeData {
    pub distance: f32,
    pub main_number: i32,
    pub sub_number: i32,
    pub polyline: Vec<Point>,
    pub is_connector: bool,
    pub midpoint: Point,
    pub direction: RoadDirection,
    pub original_road_id: i32,
}

fn merge_edge_data(start: NodeData, end: NodeData, data: Vec<EdgeData>) -> EdgeData {
    if data.is_empty() {
        panic!("No data to merge");
    } else if data.len() == 1 {
        return data[0].clone();
    }

    let mut edge_iter = data.into_iter();
    let first = edge_iter.next().unwrap();

    let mut distance = first.distance;
    let mut polyline = first.polyline;

    for edge_data in edge_iter {
        distance += edge_data.distance;
        polyline.extend(edge_data.polyline.iter().skip(1));
    }

    EdgeData {
        distance,
        main_number: first.main_number,
        sub_number: first.sub_number,
        polyline,
        is_connector: first.is_connector,
        midpoint: midpoint(start.point, end.point),
        direction: first.direction,
        original_road_id: first.original_road_id,
    }
}

pub struct GraphProcessingOptions {
    pub dedup_road_data: bool,
    pub max_distance_from_sensors: f32,
    pub merge_overlap_distance: f32,
    pub collapse_nodes: NodeCollapse,
    pub remove_disjoint_nodes: bool,
    pub dedup_edges: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum NodeCollapse {
    Naive,
    ForwardOnly,
    None,
}

impl Default for GraphProcessingOptions {
    fn default() -> Self {
        GraphProcessingOptions {
            dedup_road_data: false,
            max_distance_from_sensors: f32::INFINITY,
            merge_overlap_distance: f32::NAN,
            collapse_nodes: NodeCollapse::None,
            remove_disjoint_nodes: false,
            dedup_edges: false,
        }
    }
}

pub fn parse_data(
    mut road_data: Vec<RoadData>,
    sensor_data: Vec<SensorData>,
    opts: GraphProcessingOptions,
) -> StableGraph<NodeData, EdgeData> {
    let mut graph = StableDiGraph::new();

    if opts.dedup_road_data {
        println!("Deduplicating road data");
        let mut unique_roads = Vec::new();
        let len = road_data.len();
        'outer: for i in 0..len {
            for j in i..len {
                if i == j {
                    continue;
                }

                let road = &road_data[i];
                let other_road = &road_data[j];

                if road.coordinates.len() == other_road.coordinates.len() {
                    let identical = road
                        .coordinates
                        .iter()
                        .zip(other_road.coordinates.iter())
                        .all(|(a, b)| a == b);
                    if identical {
                        if road.length != other_road.length {
                            let diff = (road.length - other_road.length).abs();
                            panic!(
                                "Different lengths {} {} {}",
                                road.unique_id, other_road.unique_id, diff
                            );
                        }

                        if road.direction == other_road.direction {
                            continue 'outer;
                        }
                    }
                }
            }
            unique_roads.push(road_data[i].clone());
        }
        road_data = unique_roads;
        println!("Removed {} duplicate roads", len - road_data.len());
    }

    println!("Adding nodes and edges");
    for road in road_data {
        let mut prev_node: Option<(NodeIndex, NodeData)> = None;

        for (_, point) in road.coordinates.iter().enumerate() {
            let node_data = NodeData {
                point: *point,
                direction: road.direction,
                sensor: None,
                main_number: road.main_number,
                sub_number: road.sub_number,
                original_road_id: road.unique_id,
                heading: 0.0,
            };
            let node = graph.add_node(node_data);

            if let Some((prev_idx, prev_data)) = prev_node {
                let dist = dist(prev_data.point, node_data.point);
                let edge_data = EdgeData {
                    distance: dist,
                    main_number: road.main_number,
                    sub_number: road.sub_number,
                    polyline: vec![prev_data.point, node_data.point],
                    is_connector: false,
                    midpoint: midpoint(prev_data.point, node_data.point),
                    direction: direction_from_data(prev_data, node_data),
                    original_road_id: road.unique_id,
                };
                graph.add_edge(prev_idx, node, edge_data);
            }

            prev_node = Some((node, node_data));
        }
    }

    println!("Calculating node headings");
    for node in graph.clone().node_indices() {
        let in_edges = graph.edges_directed(node, Incoming);
        let out_edges = graph.edges_directed(node, Outgoing);

        let mut headings = Vec::new();

        for edge in in_edges.clone() {
            let endpoints = graph.edge_endpoints(edge.id()).unwrap();
            let start = graph.node_weight(endpoints.0).unwrap();
            let end = graph.node_weight(endpoints.1).unwrap();
            headings.push(line_heading(start.point, end.point));
        }

        for edge in out_edges.clone() {
            let endpoints = graph.edge_endpoints(edge.id()).unwrap();
            let start = graph.node_weight(endpoints.0).unwrap();
            let end = graph.node_weight(endpoints.1).unwrap();
            headings.push(line_heading(start.point, end.point));
        }

        let data = graph.node_weight_mut(node).unwrap();
        data.heading = angle_average(&headings);
    }

    if opts.max_distance_from_sensors < f32::INFINITY {
        println!("Removing nodes not close to any sensors");
        let sensor_tree = build_sensor_acceleration_structure(&sensor_data);
        let mut to_remove = Vec::new();
        for node in graph.node_indices() {
            let data = graph.node_weight(node).unwrap();
            let (dist, _) = find_closest_sensor(&sensor_tree, data.point);
            if dist > opts.max_distance_from_sensors {
                to_remove.push(node);
            }
        }
        println!("Removed {} nodes", to_remove.len());
        for node in to_remove {
            graph.remove_node(node);
        }
    }

    if !opts.merge_overlap_distance.is_nan() {
        println!(
            "Merging nodes with overlap distance {}",
            opts.merge_overlap_distance
        );
        let node_tree = build_node_acceleration_structure(&graph);
        let mut removed = HashSet::new();
        let indices = graph.node_indices().collect::<Vec<_>>();
        for node in indices {
            if removed.contains(&node) {
                continue;
            }
            let data = graph.node_weight(node).unwrap().clone();

            let borrowed = [data.point.latitude, data.point.longitude];
            let mut close_iter = node_tree
                .iter_nearest(&borrowed, &squared_euclidean)
                .unwrap();

            while let Some((_, (other, other_data))) = close_iter.next() {
                if node == *other {
                    continue;
                }

                let d = dist(data.point, other_data.point);
                if d <= opts.merge_overlap_distance {
                    let mut edges = Vec::new();
                    let in_edges = graph.edges_directed(*other, Incoming);
                    for edge in in_edges {
                        if !are_neighbours(&graph, edge.source(), node) {
                            edges.push((edge.source(), node, edge.weight().clone()));
                        }
                    }
                    let out_edges = graph.edges_directed(*other, Outgoing);
                    for edge in out_edges {
                        if !are_neighbours(&graph, node, edge.target()) {
                            edges.push((node, edge.target(), edge.weight().clone()));
                        }
                    }

                    graph.remove_node(*other);
                    removed.insert(*other);
                    for (from, to, data) in edges {
                        graph.add_edge(from, to, data);
                    }
                }

                if d > f32::max(5.0, opts.merge_overlap_distance * 2.0) {
                    break;
                }
            }
        }
        println!("Removed {} nodes", removed.len());
    }

    println!("Assigning sensors to nodes");
    let node_tree = build_node_acceleration_structure(&graph);
    let mut sensor_assignments = HashMap::<NodeIndex, Vec<SensorData>>::new();
    for sensor in sensor_data.iter() {
        let (_, closest_idx) = find_closest_node(&node_tree, sensor.point);
        let mut sensors = sensor_assignments
            .entry(closest_idx)
            .or_insert(Vec::new())
            .clone();
        sensors.push(*sensor);
        sensor_assignments.insert(closest_idx, sensors.clone());
    }

    println!("Merging sensors");
    let mut merged_sensors = Vec::new();
    for (idx, sensors) in sensor_assignments.iter() {
        let mut flow_rate = 0.0;
        let mut average_speed = 0.0;
        let mut point = Point {
            latitude: 0.0,
            longitude: 0.0,
        };

        for sensor in sensors.iter() {
            flow_rate += sensor.flow_rate;
            average_speed += sensor.average_speed;
            point.latitude += sensor.point.latitude;
            point.longitude += sensor.point.longitude;
        }

        let len = sensors.len() as f32;
        let sensor = SensorData {
            site_id: sensors[0].site_id,
            flow_rate: flow_rate / len,
            average_speed: average_speed / len,
            point: Point {
                latitude: point.latitude / len,
                longitude: point.longitude / len,
            },
            lane: 1,
            side: sensors[0].side,
        };

        merged_sensors.push(sensor);

        let data = graph.node_weight_mut(*idx).unwrap();
        data.sensor = Some(sensor);
    }
    println!(
        "Reduced {} sensors to {}",
        sensor_data.len(),
        merged_sensors.len()
    );

    println!("Finding longest road segment");
    let mut longest_road_segment = f32::NEG_INFINITY;
    for edge in graph.edge_indices() {
        let data = graph.edge_weight(edge).unwrap();
        if data.distance > longest_road_segment {
            longest_road_segment = data.distance;
        }
    }
    println!("Longest road segment: {}", longest_road_segment);

    println!("Connecting individual roads");
    let edge_tree = build_edge_acceleration_structure(&graph, None);
    let mut to_connect = Vec::new();
    for node in graph.node_indices() {
        let data = graph.node_weight(node).unwrap();

        let in_edges = graph.edges_directed(node, Incoming);
        let out_edges = graph.edges_directed(node, Outgoing);
        let is_cap = in_edges.count() + out_edges.count() == 1;

        let unique_edges = unique_edges_in_range(
            &graph,
            &edge_tree,
            data.point,
            20.0,
            longest_road_segment,
            |(_, data)| data.original_road_id,
        );

        for (_, edge) in unique_edges {
            let edge_data = graph.edge_weight(edge).unwrap();

            if data.main_number == edge_data.main_number && data.sub_number == edge_data.sub_number
            {
                continue;
            }
            if edge_data.original_road_id == data.original_road_id || edge_data.is_connector {
                continue;
            }

            let endpoints = graph.edge_endpoints(edge).unwrap();
            let start = graph.node_weight(endpoints.0).unwrap();
            let end = graph.node_weight(endpoints.1).unwrap();

            if !is_cap {
                // Only allow to connect to roads with the same heading if its not a road cap
                let heading = line_heading(start.point, end.point);
                if angle_diff(heading, data.heading).abs() > 10.0 {
                    continue;
                }

                let s_head = line_heading(data.point, start.point);
                let e_head = line_heading(data.point, end.point);

                let s_head_diff = angle_diff(data.heading, s_head).abs();
                let e_head_diff = angle_diff(data.heading, e_head).abs();

                if s_head_diff > 15.0 && e_head_diff > 15.0 {
                    continue;
                } else if s_head_diff > 15.0 {
                    to_connect.push((node, endpoints.1));
                    continue;
                } else if e_head_diff > 15.0 {
                    to_connect.push((node, endpoints.0));
                    continue;
                } else {
                    if dist(data.point, start.point) > dist(data.point, end.point) {
                        to_connect.push((node, endpoints.1));
                    } else {
                        to_connect.push((node, endpoints.0));
                    }
                }
            } else {
                if dist(data.point, start.point) > dist(data.point, end.point) {
                    to_connect.push((node, endpoints.1));
                } else {
                    to_connect.push((node, endpoints.0));
                }
            }
        }
    }

    let mut skipped = 0;
    let mut connected = 0;
    for (from, to) in to_connect {
        let from_data = graph.node_weight(from).unwrap().clone();
        let to_data = graph.node_weight(to).unwrap().clone();

        if are_neighbours(&graph, from, to) {
            skipped += 1;
            continue;
        }
        connected += 1;

        let d = dist(from_data.point, to_data.point);

        let edge_data = EdgeData {
            distance: d,
            main_number: 0,
            sub_number: 0,
            polyline: vec![],
            is_connector: true,
            midpoint: midpoint(from_data.point, to_data.point),
            direction: direction_from_data(from_data, to_data),
            original_road_id: -1,
        };
        graph.add_edge(from, to, edge_data);

        let edge_data = EdgeData {
            distance: d,
            main_number: 0,
            sub_number: 0,
            polyline: vec![],
            is_connector: true,
            midpoint: midpoint(to_data.point, from_data.point),
            direction: direction_from_data(to_data, from_data),
            original_road_id: -1,
        };
        graph.add_edge(to, from, edge_data);
    }
    println!("Connected {} roads and skipped {}", connected, skipped);

    if opts.remove_disjoint_nodes {
        println!("Removing disjointed nodes");
        let (some_sensor_idx, _) = graph
            .node_references()
            .into_iter()
            .find(|(_, data)| data.sensor.is_some())
            .unwrap();
        let mut seen = Vec::new();
        let mut bfs = Bfs::new(&graph, some_sensor_idx);

        for node in graph.node_indices() {
            let data = graph.node_weight(node).unwrap();
            if data.sensor.is_some() {
                bfs.stack.push_front(node);
                bfs.discovered.visit(node);
                seen.push(node);
            }
        }

        while let Some(node) = bfs.next(&graph) {
            seen.push(node);
        }

        let mut to_remove = Vec::new();
        for node in graph.node_indices() {
            if !seen.contains(&node) {
                to_remove.push(node);
            }
        }
        println!("Removed {} disjoint nodes", to_remove.len());
        for node in to_remove {
            graph.remove_node(node);
        }
    }

    if opts.dedup_edges {
        println!("Removing duplicate edges");
        let edge_tree = build_edge_acceleration_structure(&graph, None);
        let mut edges_to_remove = Vec::new();
        for edge in graph.edge_references() {
            let data = edge.weight();
            let idx = edge.id();
            if data.is_connector || edges_to_remove.contains(&idx) {
                continue;
            }

            let borrowed = [data.midpoint.latitude, data.midpoint.longitude];
            let (_, (closest_idx, _)) = *edge_tree
                .nearest(&borrowed, 2, &squared_euclidean)
                .unwrap()
                .iter()
                .filter(|e| e.1 .0 != edge.id())
                .next()
                .unwrap();

            // Check if edges have the same endpoints
            let endpoints = graph.edge_endpoints(edge.id()).unwrap();
            let closest_endpoints = graph.edge_endpoints(*closest_idx).unwrap();
            let start = graph.node_weight(endpoints.0).unwrap();
            let end = graph.node_weight(endpoints.1).unwrap();
            let closest_start = graph.node_weight(closest_endpoints.0).unwrap();
            let closest_end = graph.node_weight(closest_endpoints.1).unwrap();

            if (start.point == closest_start.point && end.point == closest_end.point)
                || (start.point == closest_end.point && end.point == closest_start.point)
            {
                edges_to_remove.push(*closest_idx);
            }
        }
        println!("Found {} duplicate edges", edges_to_remove.len());
        for edge in edges_to_remove {
            graph.remove_edge(edge);
        }
    }

    match opts.collapse_nodes {
        NodeCollapse::Naive => {
            println!("Collapsing nodes: naive");
            let nodes = graph.node_count();
            collapse::naive(&mut graph);
            println!("Collapsed {} nodes", nodes - graph.node_count());
        }
        NodeCollapse::ForwardOnly => {
            println!("Collapsing nodes: forward only");
            let nodes = graph.node_count();
            collapse::forward_only(&mut graph);
            println!("Collapsed {} nodes", nodes - graph.node_count());
        }
        NodeCollapse::None => {}
    }

    println!(
        "Graph has {} nodes and {} edges",
        graph.node_count(),
        graph.edge_count()
    );

    graph
}

fn are_neighbours(graph: &StableDiGraph<NodeData, EdgeData>, a: NodeIndex, b: NodeIndex) -> bool {
    graph.edges_connecting(a, b).count() > 0
}

fn build_sensor_acceleration_structure(
    sensors: &Vec<SensorData>,
) -> KdTree<f32, SensorData, [f32; 2]> {
    let mut kdtree = KdTree::new(2);

    sensors.iter().for_each(|data| {
        kdtree
            .add([data.point.latitude, data.point.longitude], *data)
            .unwrap();
    });

    kdtree
}

fn find_closest_sensor(
    kdtree: &KdTree<f32, SensorData, [f32; 2]>,
    point: Point,
) -> (f32, SensorData) {
    let (_, data) = *kdtree
        .nearest(&[point.latitude, point.longitude], 1, &squared_euclidean)
        .unwrap()
        .first()
        .unwrap();

    let dist = dist(data.point, point);

    (dist, *data)
}

pub fn build_node_acceleration_structure(
    graph: &StableGraph<NodeData, EdgeData>,
) -> KdTree<f32, (NodeIndex, NodeData), [f32; 2]> {
    let mut kdtree = KdTree::new(2);

    graph.node_indices().for_each(|idx| {
        let data = graph.node_weight(idx).unwrap();
        kdtree
            .add([data.point.latitude, data.point.longitude], (idx, *data))
            .unwrap();
    });

    kdtree
}

fn build_edge_acceleration_structure(
    graph: &StableGraph<NodeData, EdgeData>,
    filter: Option<fn((EdgeIndex, &EdgeData)) -> bool>,
) -> KdTree<f32, (EdgeIndex, EdgeData), [f32; 2]> {
    let mut kdtree = KdTree::new(2);

    graph
        .edge_indices()
        .filter(|idx| {
            if let Some(filter) = filter {
                let data = graph.edge_weight(*idx).unwrap();
                filter((*idx, data))
            } else {
                true
            }
        })
        .for_each(|idx| {
            let data = graph.edge_weight(idx).unwrap();
            let endpoints = graph.edge_endpoints(idx).unwrap();
            let start = graph.node_weight(endpoints.0).unwrap();
            let end = graph.node_weight(endpoints.1).unwrap();

            let midpoint = [
                (start.point.latitude + end.point.latitude) / 2.0,
                (start.point.longitude + end.point.longitude) / 2.0,
            ];

            kdtree.add(midpoint, (idx, data.clone())).unwrap();
        });

    kdtree
}

pub fn find_closest_node(
    kdtree: &KdTree<f32, (NodeIndex, NodeData), [f32; 2]>,
    point: Point,
) -> (f32, NodeIndex) {
    let (_, idx_data) = *kdtree
        .nearest(&[point.latitude, point.longitude], 1, &squared_euclidean)
        .unwrap()
        .first()
        .unwrap();

    let dist = dist(idx_data.1.point, point);

    (dist, idx_data.0)
}

fn unique_nodes_in_range<G>(
    kdtree: &KdTree<f32, (NodeIndex, NodeData), [f32; 2]>,
    point: Point,
    max_dist: f32,
    group_by: fn(&(NodeIndex, NodeData)) -> G,
) -> Vec<(f32, NodeIndex)>
where
    G: PartialEq + Eq + std::hash::Hash + Clone,
{
    let binding = [point.latitude, point.longitude];
    let iter = kdtree.iter_nearest(&binding, &squared_euclidean).unwrap();

    let mut nodes: HashMap<G, (f32, NodeIndex)> = HashMap::new();

    for (_, idx_data) in iter {
        let dist = dist(idx_data.1.point, point);
        if dist < max_dist {
            let group = group_by(&idx_data);
            if !nodes.contains_key(&group) {
                nodes.insert(group, (dist, idx_data.0));
            }
        } else {
            break;
        }
    }

    nodes.values().cloned().collect::<Vec<_>>()
}

fn unique_edges_in_range<G>(
    graph: &StableDiGraph<NodeData, EdgeData>,
    kdtree: &KdTree<f32, (EdgeIndex, EdgeData), [f32; 2]>,
    point: Point,
    max_dist: f32,
    longest_road: f32,
    group_by: fn(&(EdgeIndex, EdgeData)) -> G,
) -> Vec<(f32, EdgeIndex)>
where
    G: PartialEq + Eq + std::hash::Hash + Clone,
{
    let binding = [point.latitude, point.longitude];
    let iter = kdtree.iter_nearest(&binding, &squared_euclidean).unwrap();

    let mut edges: HashMap<G, (f32, EdgeIndex)> = HashMap::new();
    let limit = max_dist + longest_road / 2.0;

    for (_, (idx, data)) in iter {
        let dist_to_mid = dist(data.midpoint, point);
        if dist_to_mid > limit {
            break;
        }

        let endpoints = graph.edge_endpoints(*idx).unwrap();
        let start = graph.node_weight(endpoints.0).unwrap();
        let end = graph.node_weight(endpoints.1).unwrap();

        let actual_dist = point_line_dist_approx(point, start.point, end.point);
        if actual_dist < max_dist {
            let tuple = (*idx, data.clone());
            let group = group_by(&tuple);
            if !edges.contains_key(&group) {
                edges.insert(group, (actual_dist, *idx));
            }
        }
    }

    edges.values().cloned().collect::<Vec<_>>()
}

pub fn direction_from_data(a: NodeData, b: NodeData) -> RoadDirection {
    if a.direction == b.direction {
        a.direction
    } else {
        RoadDirection::Both
    }
}

pub fn closest_node(
    graph: &StableDiGraph<NodeData, EdgeData>,
    tree: &KdTree<f32, (NodeIndex, NodeData), [f32; 2]>,
    query: PointQuery,
) -> Option<NodeIndex> {
    let point_arr = [query.point.latitude, query.point.longitude];
    let closest_iter = tree.iter_nearest(&point_arr, &squared_euclidean).unwrap();

    for (_, (idx, data)) in closest_iter {
        if dist(data.point, query.point) > query.radius {
            return None;
        }

        let outgoing = graph.neighbors_directed(*idx, Outgoing);
        for neighbour in outgoing {
            let neighbour_data = graph.node_weight(neighbour).unwrap();
            let heading = line_heading(data.point, neighbour_data.point);
            if query.heading.contains(&heading) {
                return Some(*idx);
            }
        }
    }

    None
}
