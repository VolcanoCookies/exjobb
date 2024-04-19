use fixedbitset::FixedBitSet;
use petgraph::{graph::NodeIndex, stable_graph::StableDiGraph, visit::VisitMap};

use crate::{
    output::{Canvas, DrawOptions},
    processing::{EdgeData, NodeData},
};

const COLORS: [&str; 25] = [
    "#006400", "#808000", "#483d8b", "#b22222", "#008080", "#000080", "#9acd32", "#8fbc8f",
    "#8b008b", "#ff0000", "#ff8c00", "#ffff00", "#00ff00", "#00fa9a", "#8a2be2", "#00ffff",
    "#0000ff", "#ff00ff", "#1e90ff", "#db7093", "#f0e68c", "#87ceeb", "#ff1493", "#ffa07a",
    "#ee82ee",
];

pub fn draw_disjoint(graph: StableDiGraph<NodeData, EdgeData>) -> Canvas {
    let mut canvas = Canvas::from_graph(4000, &graph);

    let mut sets: Vec<FixedBitSet> = Vec::new();
    for node in graph.node_indices() {
        for set in sets.iter() {
            if set.is_visited(&node) {
                continue;
            }
        }
        let mut visited = FixedBitSet::with_capacity(graph.node_count());
        visit(&graph, node, &mut visited);
        sets.push(visited);
    }

    for edge in graph.edge_indices() {
        let data = graph.edge_weight(edge).unwrap();
        let start = graph.edge_endpoints(edge).unwrap().0;
        let end = graph.edge_endpoints(edge).unwrap().1;
        let mut color = None;
        for (idx, set) in sets.iter().enumerate() {
            if set.is_visited(&start) && set.is_visited(&end) {
                color = Some(COLORS[idx]);
                break;
            }
        }
        let color = color.unwrap();
        canvas.draw_line(
            graph.node_weight(start).unwrap().point,
            graph.node_weight(end).unwrap().point,
            DrawOptions {
                color,
                stroke: 1.0,
                ..Default::default()
            },
        );
    }

    for node in graph.node_indices() {
        let data = graph.node_weight(node).unwrap();
        let mut color = None;
        for (idx, set) in sets.iter().enumerate() {
            if set.is_visited(&node) {
                color = Some(COLORS[idx]);
                break;
            }
        }
        let color = color.unwrap();
        canvas.draw_circle(data.point, color, 1.0);
    }

    return canvas;
}

fn visit(graph: &StableDiGraph<NodeData, EdgeData>, node: NodeIndex, visited: &mut FixedBitSet) {
    let mut to_visit = vec![node];
    while let Some(node) = to_visit.pop() {
        if visited.visit(node) {
            let neighbours = graph.neighbors_undirected(node);
            to_visit.extend(neighbours);
        }
    }
}
