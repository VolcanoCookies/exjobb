use crate::{
    custom_bfs::Positionable,
    math::{geo_distance, midpoint},
    mongo::model::SensorMetadata,
    parse::{Point, RoadDirection},
    progress::Progress,
};

use std::collections::{HashMap, HashSet};

use clap::{Args, ValueEnum};
use console::style;
use kdtree::KdTree;
use petgraph::{
    graph::NodeIndex,
    prelude::EdgeIndex,
    stable_graph::{StableDiGraph, StableGraph},
    visit::{Bfs, EdgeRef, IntoEdgeReferences, IntoNodeReferences, VisitMap},
    Direction::{Incoming, Outgoing},
};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{
    math::{angle_average, angle_diff, dist, line_heading, point_line_dist_approx},
    parse::RoadData,
};

pub mod collapse;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NodeData {
    pub point: Point,
    pub direction: RoadDirection,
    pub main_number: i32,
    pub sub_number: i32,
    pub original_road_id: i32,
    pub heading: f64,
    pub is_road_cap: bool,
    pub has_sensor: bool,
}

impl Positionable for NodeData {
    fn point(&self) -> Point {
        self.point
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeData {
    pub distance: f64,
    pub main_number: i32,
    pub sub_number: i32,
    pub polyline: Vec<Point>,
    pub is_connector: bool,
    pub midpoint: Point,
    pub direction: RoadDirection,
    pub original_road_id: i32,
    pub speed_limit: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DriveDirection {
    Forward,
    Backward,
    Both,
    None,
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
    let mut speed_limit = first.speed_limit.unwrap_or(0.0) * first.distance;

    for edge_data in edge_iter {
        distance += edge_data.distance;
        polyline.extend(edge_data.polyline.iter().skip(1));
        speed_limit += edge_data.speed_limit.unwrap_or(0.0) * edge_data.distance;
    }

    let speed_limit = speed_limit / distance;

    EdgeData {
        distance,
        main_number: first.main_number,
        sub_number: first.sub_number,
        polyline,
        is_connector: first.is_connector,
        midpoint: midpoint(start.point, end.point),
        direction: first.direction,
        original_road_id: first.original_road_id,
        speed_limit: Some(speed_limit),
    }
}

#[derive(Debug, Args)]
pub struct GraphProcessingOptions {
    #[clap(short, long, default_value = "false", default_missing_value = "true")]
    dedup_road_data: bool,
    #[clap(
        short,
        long,
        value_parser = crate::args::parse_f64_nan_inf,
        default_value = "inf"
    )]
    max_distance_from_sensors: f64,
    #[clap(
        short = 'M',
        long,
        value_parser = crate::args::parse_f64_nan_inf,
        default_missing_value = "0"
    )]
    merge_overlap_distance: f64,
    #[clap(short, long, default_value = "none")]
    collapse_nodes: NodeCollapse,
    #[clap(
        short = 'R',
        long,
        default_value = "false",
        default_missing_value = "true"
    )]
    remove_disjoint_nodes: bool,
    #[clap(
        short = 'D',
        long,
        default_value = "false",
        default_missing_value = "true"
    )]
    dedup_edges: bool,
    #[clap(
        short = 'v',
        long,
        default_value = "-1",
        default_missing_value = "20.0"
    )]
    connect_distance: f64,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum NodeCollapse {
    Naive,
    ForwardOnly,
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedGraph {
    pub graph: StableDiGraph<NodeData, EdgeData>,
    pub sensor_store: HashMap<NodeIndex, Vec<SensorMetadata>>,
}

pub fn process_graph(
    options: GraphProcessingOptions,
    mut road_data: Vec<RoadData>,
    sensor_data: Vec<SensorMetadata>,
) -> ProcessedGraph {
    let mut progress = Progress::new();
    let process_start = std::time::Instant::now();

    let mut graph = StableDiGraph::new();
    let mut sensor_store = HashMap::<NodeIndex, Vec<SensorMetadata>>::new();

    progress.step_unsized("Calculating middle and range of sensors");
    let sensor_middle = sensor_data.iter().map(|s| s.point()).fold(
        Point {
            latitude: 0.0,
            longitude: 0.0,
        },
        |acc, p| Point {
            latitude: acc.latitude + p.latitude,
            longitude: acc.longitude + p.longitude,
        },
    );
    let sensor_middle = Point {
        latitude: sensor_middle.latitude / sensor_data.len() as f64,
        longitude: sensor_middle.longitude / sensor_data.len() as f64,
    };
    let range = sensor_data
        .iter()
        .map(|s| dist(sensor_middle, s.point()))
        .fold(0.0, f64::max);
    let range = range + options.max_distance_from_sensors;
    progress.finish(format!(
        "Middle: {:?}, Range: {}",
        style(sensor_middle).bold(),
        style(range).bold()
    ));

    if options.dedup_road_data {
        progress.step_sized(road_data.len(), "Deduplicating road data");

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
            progress.tick();
        }
        road_data = unique_roads;
        progress.finish(format!(
            "Removed {} duplicate roads",
            style(len - road_data.len()).bold()
        ));
    } else {
        progress.step_single("Skipping deduplication of road data");
    }

    progress.step_unsized("Adding nodes and edges");
    let mut skipped = 0;
    for road in road_data.iter_mut() {
        let mut prev_node: Option<(NodeIndex, NodeData)> = None;

        if road.direction == RoadDirection::None {
            continue;
        }

        if road.direction == RoadDirection::Backward {
            road.coordinates.reverse()
        }

        for (idx, point) in road.coordinates.iter().enumerate() {
            if options.max_distance_from_sensors < f64::INFINITY {
                let d = dist(sensor_middle, *point);
                if d > range {
                    prev_node = None;
                    skipped += 1;
                    continue;
                }
            }

            let node_data = NodeData {
                point: *point,
                direction: road.direction,
                main_number: road.main_number,
                sub_number: road.sub_number,
                original_road_id: road.unique_id,
                heading: 0.0,
                is_road_cap: idx == 0 || idx == road.coordinates.len() - 1,
                has_sensor: false,
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
                    speed_limit: Some(road.speed_limit),
                };
                if road.direction == RoadDirection::Both {
                    let mut rev_edge_data = edge_data.clone();
                    rev_edge_data.polyline.reverse();
                    graph.add_edge(node, prev_idx, rev_edge_data);
                }

                graph.add_edge(prev_idx, node, edge_data);
            }

            prev_node = Some((node, node_data));
        }
        progress.tick();
    }
    progress.finish(format!(
        "Added {} nodes and {} edges, skipping {} nodes out of range",
        style(graph.node_count()).bold(),
        style(graph.edge_count()).bold(),
        style(skipped).bold()
    ));

    progress.step_sized(graph.node_count(), "Calculating node headings");
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
        progress.tick();
    }
    progress.finish("Calculated node headings");

    if options.max_distance_from_sensors < f64::INFINITY {
        progress.step_sized(
            graph.node_count(),
            format!(
                "Removing nodes not within {}m of any sensors",
                style(options.max_distance_from_sensors).bold()
            ),
        );
        let pb = progress.get_pb();
        let sensor_tree = build_sensor_acceleration_structure(sensor_data.iter());
        let to_remove = graph
            .node_indices()
            .par_bridge()
            .filter(|node| {
                let data = graph.node_weight(*node).unwrap();
                let middle_dist = dist(sensor_middle, data.point);
                if middle_dist > range {
                    pb.inc(1);
                    return true;
                }

                let (dist, _) = find_closest_sensor(&sensor_tree, data.point);
                if dist > options.max_distance_from_sensors {
                    pb.inc(1);
                    return true;
                }
                pb.inc(1);
                false
            })
            .collect::<Vec<_>>();

        let len = to_remove.len();
        for node in to_remove {
            graph.remove_node(node);
        }

        progress.finish(format!("Removed {} nodes", style(len).bold()));
    } else {
        progress.step_single("Skipping removal of nodes not close to any sensors");
    }

    if !options.merge_overlap_distance.is_nan() {
        progress.step_sized(
            graph.node_count(),
            format!(
                "Merging nodes with overlap distance {}",
                style(options.merge_overlap_distance).bold()
            ),
        );

        let node_tree = build_node_acceleration_structure(&graph);
        let mut removed = HashSet::new();
        let indices = graph.node_indices().collect::<Vec<_>>();
        for node in indices {
            if removed.contains(&node) {
                progress.tick();
                continue;
            }
            let data = graph.node_weight(node).unwrap().clone();

            if !data.is_road_cap {
                progress.tick();
                continue;
            }

            let borrowed = [data.point.latitude, data.point.longitude];
            let mut close_iter = node_tree.iter_nearest(&borrowed, &geo_distance).unwrap();

            while let Some((_, (other, other_data))) = close_iter.next() {
                if node == *other {
                    continue;
                }

                let d = dist(data.point, other_data.point);
                if d <= options.merge_overlap_distance {
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
                } else {
                    break;
                }
            }
            progress.tick();
        }
        progress.finish(format!(
            "Merged {} overlapping nodes",
            style(removed.len()).bold()
        ));
    }

    progress.step_sized(sensor_data.len(), "Assigning sensors to nodes");
    let node_tree = build_node_acceleration_structure(&graph);
    for sensor in sensor_data {
        let (_, closest_idx) = find_closest_node(&node_tree, sensor.point());
        sensor_store
            .entry(closest_idx)
            .or_insert(Vec::new())
            .push(sensor);
        let data = graph.node_weight_mut(closest_idx).unwrap();
        data.has_sensor = true;
        progress.tick();
    }
    progress.finish(format!(
        "Assigned sensors to {} nodes",
        style(sensor_store.len()).bold()
    ));

    progress.step_sized(graph.edge_count(), "Finding longest road segment");
    let mut longest_road_segment = f64::NEG_INFINITY;
    for edge in graph.edge_indices() {
        let data = graph.edge_weight(edge).unwrap();
        if data.distance > longest_road_segment {
            longest_road_segment = data.distance;
        }
        progress.tick();
    }
    progress.finish(format!(
        "Longest road segment: {}",
        style(longest_road_segment).bold()
    ));

    if options.connect_distance >= 0.0 {
        progress.step_sized(graph.node_count(), "Connecting individual roads");
        let pb = progress.get_pb();
        let edge_tree = build_edge_acceleration_structure(&graph, None);
        let par_iter = graph.node_indices().par_bridge();
        let to_connect = par_iter
            .filter_map(|node| {
                let data = graph.node_weight(node).unwrap();

                let in_edges = graph.edges_directed(node, Incoming);
                let out_edges = graph.edges_directed(node, Outgoing);
                let is_cap = in_edges.count() + out_edges.count() == 1;

                let unique_edges = unique_edges_in_range(
                    &graph,
                    &edge_tree,
                    data.point,
                    options.connect_distance,
                    longest_road_segment,
                    |(_, data)| data.original_road_id,
                );
                pb.inc(1);

                for (_, edge) in unique_edges {
                    let edge_data = graph.edge_weight(edge).unwrap();

                    if data.main_number == edge_data.main_number
                        && data.sub_number == edge_data.sub_number
                    {
                        continue;
                    }
                    if edge_data.original_road_id == data.original_road_id || edge_data.is_connector
                    {
                        continue;
                    }

                    let endpoints = graph.edge_endpoints(edge).unwrap();
                    let start = graph.node_weight(endpoints.0).unwrap();
                    let end = graph.node_weight(endpoints.1).unwrap();

                    if !is_cap {
                        // Only allow to connect to roads with the same heading if its not a road cap
                        let heading = line_heading(start.point, end.point);
                        if angle_diff(heading, data.heading).abs() > 15.0 {
                            //continue;
                        }

                        let s_head = line_heading(data.point, start.point);
                        let e_head = line_heading(data.point, end.point);

                        let s_head_diff = angle_diff(data.heading, s_head).abs();
                        let e_head_diff = angle_diff(data.heading, e_head).abs();

                        if s_head_diff > 15.0 && e_head_diff > 15.0 {
                            continue;
                        } else if s_head_diff > 15.0 {
                            return Some((node, endpoints.1));
                        } else if e_head_diff > 15.0 {
                            return Some((node, endpoints.0));
                        } else {
                            if dist(data.point, start.point) > dist(data.point, end.point) {
                                return Some((node, endpoints.1));
                            } else {
                                return Some((node, endpoints.0));
                            }
                        }
                    } else {
                        if dist(data.point, start.point) > dist(data.point, end.point) {
                            return Some((node, endpoints.1));
                        } else {
                            return Some((node, endpoints.0));
                        }
                    }
                }
                return None;
            })
            .collect::<Vec<_>>();

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
                speed_limit: None,
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
                speed_limit: None,
            };
            graph.add_edge(to, from, edge_data);
        }
        progress.finish(format!(
            "Connected {} roads and skipped {}",
            style(connected).bold(),
            style(skipped).bold()
        ));
    } else {
        progress.step_single("Skipping connection of individual roads");
    }

    if options.remove_disjoint_nodes {
        progress.step_unsized("Removing disjointed nodes");
        let (some_sensor_idx, _) = graph
            .node_references()
            .into_iter()
            .find(|(idx, _)| sensor_store.contains_key(idx))
            .unwrap();
        let mut seen = Vec::new();
        let mut bfs = Bfs::new(&graph, some_sensor_idx);

        for node in graph.node_indices() {
            if sensor_store.contains_key(&node) {
                bfs.stack.push_front(node);
                bfs.discovered.visit(node);
                seen.push(node);
            }
            progress.tick();
        }

        while let Some(node) = bfs.next(&graph) {
            seen.push(node);
            progress.tick();
        }

        let mut to_remove = Vec::new();
        for node in graph.node_indices() {
            if !seen.contains(&node) {
                to_remove.push(node);
            }
            progress.tick();
        }
        let len = to_remove.len();
        for node in to_remove {
            graph.remove_node(node);
        }
        progress.finish(format!("Removed {} disjointed nodes", style(len).bold()));
    } else {
        progress.step_single("Skipping removal of disjointed nodes");
    }

    if options.dedup_edges {
        progress.step_sized(graph.edge_count(), "Removing duplicate edges");

        let edge_tree = build_edge_acceleration_structure(&graph, None);
        let mut edges_to_remove = Vec::new();
        for edge in graph.edge_references() {
            let data = edge.weight();
            let idx = edge.id();
            if data.is_connector || edges_to_remove.contains(&idx) {
                progress.tick();
                continue;
            }

            let borrowed = [data.midpoint.latitude, data.midpoint.longitude];
            let (_, (closest_idx, _)) = *edge_tree
                .nearest(&borrowed, 2, &geo_distance)
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
            progress.tick();
        }

        let len = edges_to_remove.len();
        for edge in edges_to_remove {
            graph.remove_edge(edge);
        }

        progress.finish(format!("Removed {} duplicate edges", style(len).bold()));
    } else {
        progress.step_single("Skipping removal of duplicate edges");
    }

    match options.collapse_nodes {
        NodeCollapse::Naive => {
            progress.step_unsized(format!("Collapsing nodes: {}", style("naive").bold()));

            let nodes = graph.node_count();
            collapse::naive(&mut graph);

            progress.finish(format!(
                "Collapsed {} nodes",
                style(nodes - graph.node_count()).bold()
            ));
        }
        NodeCollapse::ForwardOnly => {
            progress.step_unsized(format!(
                "Collapsing nodes: {}",
                style("forward only").bold()
            ));

            let nodes = graph.node_count();
            collapse::forward_only(&mut graph);

            progress.finish(format!(
                "Collapsed {} nodes",
                style(nodes - graph.node_count()).bold()
            ));
        }
        NodeCollapse::None => {
            progress.step_single("Skipping node collapse");
        }
    }

    println!(
        "{:?} Completed processing graph with {} nodes and {} edges remaining",
        style(process_start.elapsed()).bold().dim().yellow(),
        style(graph.node_count()).bold(),
        style(graph.edge_count()).bold()
    );

    ProcessedGraph {
        graph,
        sensor_store,
    }
}

fn are_neighbours(graph: &StableDiGraph<NodeData, EdgeData>, a: NodeIndex, b: NodeIndex) -> bool {
    graph.edges_connecting(a, b).count() > 0
}

fn build_sensor_acceleration_structure<'a, I: Iterator<Item = &'a SensorMetadata>>(
    sensors: I,
) -> KdTree<f64, SensorMetadata, [f64; 2]> {
    let mut kdtree = KdTree::new(2);

    sensors.for_each(|data| {
        let point = data.point();
        kdtree
            .add([point.latitude, point.longitude], data.clone())
            .unwrap();
    });

    kdtree
}

fn find_closest_sensor(
    kdtree: &KdTree<f64, SensorMetadata, [f64; 2]>,
    point: Point,
) -> (f64, SensorMetadata) {
    let (_, data) = *kdtree
        .nearest(&[point.latitude, point.longitude], 1, &geo_distance)
        .unwrap()
        .first()
        .unwrap();

    let dist = dist(data.point(), point);

    (dist, data.clone())
}

pub fn build_node_acceleration_structure(
    graph: &StableGraph<NodeData, EdgeData>,
) -> KdTree<f64, (NodeIndex, NodeData), [f64; 2]> {
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
) -> KdTree<f64, (EdgeIndex, EdgeData), [f64; 2]> {
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
    kdtree: &KdTree<f64, (NodeIndex, NodeData), [f64; 2]>,
    point: Point,
) -> (f64, NodeIndex) {
    let (_, idx_data) = *kdtree
        .nearest(&[point.latitude, point.longitude], 1, &geo_distance)
        .unwrap()
        .first()
        .unwrap();

    let dist = dist(idx_data.1.point, point);

    (dist, idx_data.0)
}

fn unique_edges_in_range<G>(
    graph: &StableDiGraph<NodeData, EdgeData>,
    kdtree: &KdTree<f64, (EdgeIndex, EdgeData), [f64; 2]>,
    point: Point,
    max_dist: f64,
    longest_road: f64,
    group_by: fn(&(EdgeIndex, EdgeData)) -> G,
) -> Vec<(f64, EdgeIndex)>
where
    G: PartialEq + Eq + std::hash::Hash + Clone,
{
    let binding = [point.latitude, point.longitude];
    let iter = kdtree.iter_nearest(&binding, &geo_distance).unwrap();

    let mut edges: HashMap<G, (f64, EdgeIndex)> = HashMap::new();
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
