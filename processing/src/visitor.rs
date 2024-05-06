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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TravelTime {
    pub time: f64,
    pub speeds: Vec<(NodeIndex, f64)>,
}

pub fn convert_ms_to_kmh(speed: f64) -> f64 {
    speed / 1000.0 * 3600.0
}

pub fn convert_kmh_to_ms(speed: f64) -> f64 {
    speed * 1000.0 / 3600.0
}
