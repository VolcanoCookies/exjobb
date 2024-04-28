use clap::ValueEnum;
use petgraph::{graph::NodeIndex, stable_graph::StableDiGraph};
use serde::{Deserialize, Serialize};

use crate::{
    custom_bfs::CustomBfs,
    math::geo_distance,
    processing::{EdgeData, NodeData},
};

pub struct Path {
    pub nodes: Vec<NodeIndex>,
    pub length: f64,
    pub complete: bool,
    pub missed: Vec<NodeIndex>,
}

struct SubPath {
    nodes: Vec<NodeIndex>,
    length: f64,
}

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceMetric {
    Space,
    Time,
}

impl DistanceMetric {
    pub fn unit(self) -> &'static str {
        match self {
            DistanceMetric::Space => "m",
            DistanceMetric::Time => "s",
        }
    }
}

impl DistanceMetric {
    pub fn to_function(self) -> fn(&NodeData, &NodeData, &EdgeData) -> f64 {
        match self {
            DistanceMetric::Space => distance_space,
            DistanceMetric::Time => distance_time,
        }
    }
}

pub fn shortest_path(
    graph: &StableDiGraph<NodeData, EdgeData>,
    points: Vec<NodeIndex>,
    metric: DistanceMetric,
) -> Option<Path> {
    let mut path = Vec::new();
    let mut length = 0.0;

    let distance_fn = match metric {
        DistanceMetric::Space => distance_space,
        DistanceMetric::Time => distance_time,
    };

    let mut iter = points.iter();
    let mut start = iter.next()?;

    let mut complete = true;
    let mut missed = Vec::new();
    for end in iter {
        let p = shortest_path_singular(graph, *start, *end, distance_fn);

        let p = if let Some(p) = p {
            p
        } else {
            missed.push(*end);
            complete = false;
            continue;
        };

        path.extend(p.nodes.iter());
        length += p.length;
        start = end;
    }

    path.push(*start);

    Some(Path {
        nodes: path,
        length,
        complete,
        missed,
    })
}

fn shortest_path_singular(
    graph: &StableDiGraph<NodeData, EdgeData>,
    start: NodeIndex,
    end: NodeIndex,
    distance_fn: fn(&NodeData, &NodeData, &EdgeData) -> f64,
) -> Option<SubPath> {
    let mut search = CustomBfs::new(graph, start, distance_fn);
    while let Some((idx, dist, path)) = search.next(&graph) {
        if idx == end {
            return Some(SubPath {
                nodes: path,
                length: dist,
            });
        }
    }

    None
}

fn distance_space(from: &NodeData, to: &NodeData, _edge: &EdgeData) -> f64 {
    let from = [from.point.latitude, from.point.longitude];
    let to = [to.point.latitude, to.point.longitude];
    geo_distance(&from, &to)
}

fn distance_time(_from: &NodeData, _to: &NodeData, edge: &EdgeData) -> f64 {
    let speed_kmh = edge.speed_limit.unwrap_or(0.0);
    let speed = speed_kmh * 1000.0 / 3600.0;
    let distance = edge.distance;
    distance / speed
}

pub fn calculate_travel_time(graph: &StableDiGraph<NodeData, EdgeData>, path: &Path) -> f64 {
    let mut travel_time = 0.0;
    let mut previous_speed_limit = convert_speed(50.0);

    for nodes in path.nodes.windows(2) {
        let edge = graph.edges_connecting(nodes[0], nodes[1]).next().unwrap();
        let data = edge.weight();
        let speed_limit = if let Some(speed_limit) = data.speed_limit {
            convert_speed(speed_limit)
        } else {
            previous_speed_limit
        };
        let distance = data.distance;

        let time = distance / speed_limit;
        travel_time += time;
        previous_speed_limit = speed_limit;
    }

    travel_time
}

pub fn calculate_travel_time_sensors(
    graph: &StableDiGraph<NodeData, EdgeData>,
    path: &Path,
) -> f64 {
    let mut distance = 0.0;
    let mut sensors = Vec::new();

    path.nodes.windows(2).for_each(|nodes| {
        let edge = graph.edges_connecting(nodes[0], nodes[1]).next().unwrap();
        let data = edge.weight();

        let end_data = graph.node_weight(nodes[1]).unwrap();

        distance += data.distance;
        if let Some(sensor) = &end_data.sensor {
            sensors.push((*sensor, distance));
        }
    });
    let total_distance = distance;

    let mut iter = sensors.iter();
    let mut prev = iter.next().unwrap();

    let (sensor, distance) = prev;
    let mut travel_time = distance / convert_speed(sensor.average_speed);

    while let Some(curr) = iter.next() {
        let (sensor, distance) = curr;
        let distance = distance - prev.1;
        let time = 2.0 * distance
            / (convert_speed(prev.0.average_speed) + convert_speed(sensor.average_speed));
        travel_time += time;
        prev = curr;
    }

    let distance = total_distance - prev.1;
    travel_time += distance / convert_speed(prev.0.average_speed);

    travel_time
}

pub fn convert_speed(speed: f64) -> f64 {
    speed * 1000.0 / 3600.0
}

pub fn calculate_travel_distance(
    graph: &StableDiGraph<NodeData, EdgeData>,
    path: &Vec<NodeIndex>,
) -> f64 {
    path.windows(2).fold(0.0, |acc, nodes| {
        let (edge, _) = graph.find_edge_undirected(nodes[0], nodes[1]).unwrap();
        let data = graph.edge_weight(edge).unwrap();
        acc + data.distance
    })
}
