use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
    Direction::{Incoming, Outgoing},
    Graph,
};

use crate::{
    math::dist,
    parse::{Point, RoadData, RoadDirection, SensorData},
};

#[derive(Debug, Clone, Copy)]
pub struct NodeData {
    pub point: Point,
    pub direction: RoadDirection,
    pub sensor: Option<SensorData>,
}

#[derive(Debug, Clone)]
pub struct EdgeData {
    pub distance: f32,
    pub main_number: i32,
    pub sub_number: i32,
    pub polyline: Vec<Point>,
}

pub fn parse_data(
    road_data: Vec<RoadData>,
    sensor_data: Vec<SensorData>,
) -> Graph<NodeData, EdgeData> {
    let mut graph = DiGraph::new();

    // Merge sensors next to each other that are in the same direction
    /*
    let mut chunks = vec![vec![Vec::new(); 256]; 256];
    let lat_extent = extent(sensor_data, |s| s.point.latitude);
    let lon_extent = extent(sensor_data, |s| s.point.longitude);

    for sensor in sensor_data {
        let lat = ((sensor.point.latitude - lat_extent.0) / (lat_extent.1 - lat_extent.0) * 256.0)
            .round() as usize;
        let lon = ((sensor.point.longitude - lon_extent.0) / (lon_extent.1 - lon_extent.0) * 256.0)
            .round() as usize;
        chunks[lat][lon].push(sensor);
    }
     */

    println!("Clustering sensors");
    let mut clusters: Vec<Vec<SensorData>> = Vec::new();
    'outer: for sensor in sensor_data.iter() {
        for cluster in clusters.iter_mut() {
            let first = &cluster[0];
            let cluster_lanes = cluster.iter().map(|c| c.lane).collect::<Vec<_>>();
            if dist(sensor.point, first.point) < 10.0
                && sensor.side == first.side
                && !cluster_lanes.contains(&sensor.lane)
            {
                cluster.push(*sensor);
                continue 'outer;
            }
        }

        let new_cluster = vec![*sensor];
        clusters.push(new_cluster);
    }
    println!("Number of clusters: {}", clusters.len());

    println!("Merging sensors");
    let mut merged_sensors = Vec::new();
    for cluster in clusters {
        let mut flow_rate = 0.0;
        let mut average_speed = 0.0;
        let mut point = Point {
            latitude: 0.0,
            longitude: 0.0,
        };

        for sensor in cluster.iter() {
            flow_rate += sensor.flow_rate;
            average_speed += sensor.average_speed;
            point.latitude += sensor.point.latitude;
            point.longitude += sensor.point.longitude;
        }

        let len = cluster.len() as f32;
        let sensor = SensorData {
            site_id: cluster[0].site_id,
            flow_rate: flow_rate / len,
            average_speed: average_speed / len,
            point: Point {
                latitude: point.latitude / len,
                longitude: point.longitude / len,
            },
            lane: 1,
            side: cluster[0].side,
        };
        merged_sensors.push(sensor);
    }
    println!("Number of merged sensors: {}", merged_sensors.len());

    for road in road_data {
        let mut prev_node: Option<(NodeIndex, NodeData)> = None;

        for (idx, point) in road.coordinates.iter().enumerate() {
            let node_data = NodeData {
                point: *point,
                direction: road.direction,
                sensor: None,
            };
            let node = graph.add_node(node_data);

            if let Some((prev_idx, prev_data)) = prev_node {
                let dist = dist(prev_data.point, node_data.point);
                let edge_data = EdgeData {
                    distance: dist,
                    main_number: road.main_number,
                    sub_number: road.sub_number,
                    polyline: vec![prev_data.point, node_data.point],
                };
                graph.add_edge(prev_idx, node, edge_data);
            }

            if idx == 0 {
                // Connect to other roads
            }

            if idx == road.coordinates.len() - 1 {
                // Connect to other roads
            }

            prev_node = Some((node, node_data));
        }
    }

    println!("Assigning sensors to nodes");
    for sensor in merged_sensors {
        let mut closest_node = None;
        let mut closest_dist = f32::INFINITY;

        for node in graph.node_indices() {
            let data = graph.node_weight(node).unwrap();
            let dist = dist(sensor.point, data.point);
            if dist < closest_dist {
                closest_dist = dist;
                closest_node = Some(node);
            }
        }

        if let Some(node) = closest_node {
            let data = graph.node_weight_mut(node).unwrap();
            data.sensor = Some(sensor);
        }
    }

    println!("Finding loops");
    for node in graph.node_indices() {
        let from = graph.edges_directed(node, Outgoing);

        for edge in from {
            let to = graph.edges_directed(node, Incoming);
            for other in to {
                if edge.source() == other.target() && edge.target() == other.source() {
                    println!("Found a loop");
                }
            }
        }
    }

    println!("Connecting individual roads");

    println!("Collapsing nodes");
    let mut to_collapse = find_node_to_collapse(&graph);
    while let Some(node) = to_collapse {
        collapse_node(&mut graph, node);
        to_collapse = find_node_to_collapse(&graph);
    }

    graph
}

fn collapse_node(graph: &mut Graph<NodeData, EdgeData>, node: NodeIndex) {
    let mut forwards = vec![graph.node_weight(node).unwrap().point];
    let mut backwards = Vec::new();

    let start;
    let end;

    let mut to_remove = vec![node];

    // Walk forwards until we hit a node we cannot collapse
    let mut distance_forwards = 0.0;
    let mut current = node;
    let mut prev_edge;
    loop {
        let edge = graph.edges_directed(current, Outgoing).next().unwrap();
        let next = edge.target();

        prev_edge = edge.id();

        if can_collapse_node(graph, next) {
            let data = graph.edge_weight(edge.id()).unwrap();
            distance_forwards += data.distance;
            forwards.extend(data.polyline.iter().skip(1));
            to_remove.push(current);
            current = next;
        } else {
            end = next;
            break;
        }
    }

    // Walk backwards until we hit a node we cannot collapse
    let mut distance_backwards = 0.0;
    let mut current = node;
    loop {
        let edge = graph.edges_directed(current, Incoming).next().unwrap();
        let next = edge.source();

        if can_collapse_node(graph, next) {
            let data = graph.edge_weight(edge.id()).unwrap();
            distance_backwards += data.distance;
            backwards.extend(data.polyline.iter().rev().skip(1));
            to_remove.push(current);
            current = next;
        } else {
            start = next;
            break;
        }
    }

    let prev_edge_data = graph.edge_weight(prev_edge).unwrap();

    let edge_data = EdgeData {
        distance: distance_forwards + distance_backwards,
        main_number: prev_edge_data.main_number,
        sub_number: prev_edge_data.sub_number,
        polyline: backwards.into_iter().rev().chain(forwards).collect(),
    };

    graph.add_edge(start, end, edge_data);

    for node in to_remove {
        graph.remove_node(node);
    }
}

fn find_node_to_collapse(graph: &Graph<NodeData, EdgeData>) -> Option<NodeIndex> {
    for node in graph.node_indices() {
        if can_collapse_node(graph, node) {
            return Some(node);
        }
    }

    None
}

fn can_collapse_node(graph: &Graph<NodeData, EdgeData>, node: NodeIndex) -> bool {
    let data = graph.node_weight(node).unwrap();
    if data.sensor.is_some() {
        return false;
    }

    let out_edges = graph.edges_directed(node, Outgoing);
    let in_edges = graph.edges_directed(node, Incoming);

    if out_edges.count() != 1 || in_edges.count() != 1 {
        return false;
    }

    let mut out_edges = graph.edges_directed(node, Outgoing);
    let mut in_edges = graph.edges_directed(node, Incoming);

    let out_data = out_edges.next().unwrap().weight();
    let in_data = in_edges.next().unwrap().weight();

    out_data.main_number == in_data.main_number && out_data.sub_number == in_data.sub_number
}
