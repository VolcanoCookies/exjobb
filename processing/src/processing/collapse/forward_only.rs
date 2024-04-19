use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    stable_graph::StableDiGraph,
    visit::EdgeRef,
    Direction::{Incoming, Outgoing},
};

use crate::processing::{merge_edge_data, EdgeData, NodeData};

pub fn forward_only(graph: &mut StableDiGraph<NodeData, EdgeData>) {
    let mut nucleation_points = Vec::new();
    for node in graph.node_indices() {
        if is_nucleation_point(graph, node) {
            nucleation_points.push(node);
        }
    }
    for node in nucleation_points {
        start_nucleation(graph, node);
    }
}

fn start_nucleation(graph: &mut StableDiGraph<NodeData, EdgeData>, node: NodeIndex) {
    let paths = graph
        .edges_directed(node, Outgoing)
        .filter(|edge| !edge.weight().is_connector)
        .map(|edge| edge.id())
        .collect::<Vec<_>>();
    for edge in paths {
        collapse_node(graph, edge);
    }
}

fn collapse_node(graph: &mut StableDiGraph<NodeData, EdgeData>, edge: EdgeIndex) {
    let data = graph.edge_weight(edge).unwrap();
    let endpoints = graph.edge_endpoints(edge).unwrap();
    let start = endpoints.0;
    let end = endpoints.1;

    let mut edges_data = vec![data.clone()];
    let mut nodes = Vec::new();

    let mut head = end;
    loop {
        let out_edges = graph.edges_directed(head, Outgoing);
        // Do not try to collapse if we have multiple paths
        if out_edges.count() != 1 {
            break;
        }

        // Do not collapse a node that has more than one incoming path
        let in_edges = graph.edges_directed(head, Incoming);
        if in_edges.count() > 1 {
            break;
        }

        let mut out_edges = graph.edges_directed(head, Outgoing);
        let edge = out_edges.next().unwrap();
        let data = graph.edge_weight(edge.id()).unwrap();

        // Do not try to collapse a connector
        if data.is_connector {
            break;
        }

        // Add prev head to the list of nodes to remove
        nodes.push(head);

        edges_data.push(data.clone());
        head = edge.target();

        let head_data = graph.node_weight(head).unwrap();
        // Do not push past a sensor
        if head_data.sensor.is_some() {
            break;
        }
    }

    // Do not collapse if we only have at most one edge
    if head == end || edges_data.len() < 2 {
        return;
    }

    let start_data = graph.node_weight(start).unwrap();
    let end_data = graph.node_weight(head).unwrap();

    let edge_data = merge_edge_data(*start_data, *end_data, edges_data);

    for node in nodes {
        graph.remove_node(node);
    }

    graph.add_edge(start, head, edge_data);
}

fn is_nucleation_point(graph: &StableDiGraph<NodeData, EdgeData>, node: NodeIndex) -> bool {
    let data = graph.node_weight(node).unwrap();

    let has_sensor = data.sensor.is_some();
    let non_connectors_in = graph
        .edges_directed(node, Incoming)
        .filter(|edge| !edge.weight().is_connector)
        .count();
    let non_connectors_out = graph
        .edges_directed(node, Outgoing)
        .filter(|edge| !edge.weight().is_connector)
        .count();

    // Only start nucleation if we have at least one non-connector out, and are either a sensor or do not have exactly one non-connector in
    non_connectors_out > 0 && (has_sensor || non_connectors_in != 1)
}
