use petgraph::{
    graph::NodeIndex,
    stable_graph::StableDiGraph,
    visit::EdgeRef,
    Direction::{Incoming, Outgoing},
};

use crate::{
    math::midpoint,
    processing::{direction_from_data, EdgeData, NodeData},
};

pub fn naive(graph: &mut StableDiGraph<NodeData, EdgeData>) {
    while let Some(node) = find_node_to_collapse(graph) {
        collapse_node(graph, node);
    }
}

fn collapse_node(graph: &mut StableDiGraph<NodeData, EdgeData>, node: NodeIndex) {
    let data = graph.node_weight(node).unwrap();
    let mut forwards = vec![data.point];
    let mut backwards = Vec::new();

    let start;
    let end;

    let mut to_remove = vec![node];

    // Walk forwards until we hit a node we cannot collapse
    let mut distance_forwards = 0.0;
    let mut speed_limit_forwards = 0.0;
    let mut current = node;
    let mut prev_edge;
    loop {
        let edge = graph.edges_directed(current, Outgoing).next().unwrap();
        let edge_data = graph.edge_weight(edge.id()).unwrap();
        let next = edge.target();

        prev_edge = edge.id();

        if edge_data.is_connector {
            end = current;
            break;
        }

        distance_forwards += edge_data.distance;
        speed_limit_forwards += edge_data.speed_limit.unwrap_or(0.0) * edge_data.distance;
        forwards.extend(edge_data.polyline.iter().skip(1));
        to_remove.push(current);
        current = next;

        if !can_collapse_node(graph, next) {
            end = next;
            break;
        }
    }

    // Walk backwards until we hit a node we cannot collapse
    let mut distance_backwards = 0.0;
    let mut speed_limit_backwards = 0.0;
    let mut current = node;
    loop {
        let edge = graph.edges_directed(current, Incoming).next().unwrap();
        let edge_data = graph.edge_weight(edge.id()).unwrap();
        let next = edge.source();

        if edge_data.is_connector {
            start = current;
            break;
        }

        distance_backwards += edge_data.distance;
        speed_limit_backwards += edge_data.speed_limit.unwrap_or(0.0) * edge_data.distance;
        backwards.extend(edge_data.polyline.iter().rev().skip(1));
        to_remove.push(current);
        current = next;

        if !can_collapse_node(graph, next) {
            start = next;
            break;
        }
    }

    let prev_edge_data = graph.edge_weight(prev_edge).unwrap();

    let start_data = graph.node_weight(start).unwrap();
    let end_data = graph.node_weight(end).unwrap();

    let speed_limit_forwards = speed_limit_forwards / distance_forwards;
    let speed_limit_backwards = speed_limit_backwards / distance_backwards;
    let speed_limit = (speed_limit_forwards + speed_limit_backwards) / 2.0;

    let edge_data = EdgeData {
        distance: distance_forwards + distance_backwards,
        main_number: prev_edge_data.main_number,
        sub_number: prev_edge_data.sub_number,
        polyline: backwards.into_iter().rev().chain(forwards).collect(),
        is_connector: false,
        midpoint: midpoint(start_data.point, end_data.point),
        direction: direction_from_data(*start_data, *end_data),
        original_road_id: -1,
        speed_limit: Some(speed_limit),
        metadata: Default::default(),
    };

    graph.add_edge(start, end, edge_data);

    for node in to_remove {
        graph.remove_node(node);
    }
}

fn find_node_to_collapse(graph: &StableDiGraph<NodeData, EdgeData>) -> Option<NodeIndex> {
    for node in graph.node_indices() {
        if can_collapse_node(graph, node) {
            return Some(node);
        }
    }

    None
}

fn can_collapse_node(graph: &StableDiGraph<NodeData, EdgeData>, node: NodeIndex) -> bool {
    let data = graph.node_weight(node).unwrap();
    if data.sensor.is_some() || data.original_road_id == -1 {
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

    if out_data.is_connector && in_data.is_connector {
        return false;
    }

    out_data.main_number == in_data.main_number && out_data.sub_number == in_data.sub_number
}
