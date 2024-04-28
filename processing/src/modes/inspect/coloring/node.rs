use clap::ValueEnum;
use console::style;
use petgraph::stable_graph::StableDiGraph;

use crate::{
    modes::inspect::InspectOptions,
    output::Canvas,
    processing::{EdgeData, NodeData},
    progress::Progress,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NodeColor {
    None,
    Simple,
    Junctions,
}

impl NodeColor {
    pub fn coloring_function(
        &self,
    ) -> fn(&mut Progress, &mut Canvas, &StableDiGraph<NodeData, EdgeData>, &InspectOptions) {
        match self {
            NodeColor::None => noop,
            NodeColor::Simple => simple_coloring,
            NodeColor::Junctions => coloring_junctions,
        }
    }
}

fn noop(
    _progress: &mut Progress,
    _canvas: &mut Canvas,
    _graph: &StableDiGraph<NodeData, EdgeData>,
    _options: &InspectOptions,
) {
}

fn simple_coloring(
    progress: &mut Progress,
    canvas: &mut Canvas,
    graph: &StableDiGraph<NodeData, EdgeData>,
    _options: &InspectOptions,
) {
    progress.step_sized(
        graph.node_count(),
        format!("Drawing {} nodes", style(graph.node_count()).bold()),
    );

    for node in graph.node_indices() {
        let data = graph.node_weight(node).unwrap();

        canvas.draw_triangle(data.point, "green", 1.5, data.heading);

        progress.tick();
    }
}

fn coloring_junctions(
    progress: &mut Progress,
    canvas: &mut Canvas,
    graph: &StableDiGraph<NodeData, EdgeData>,
    _options: &InspectOptions,
) {
    progress.step_sized(
        graph.node_count(),
        format!("Drawing {} nodes", style(graph.node_count()).bold()),
    );

    for node in graph.node_indices() {
        let data = graph.node_weight(node).unwrap();

        let edges_in = graph.edges_directed(node, petgraph::Direction::Incoming);
        let edges_out = graph.edges_directed(node, petgraph::Direction::Outgoing);
        let edge_count = edges_in.count() + edges_out.count();

        let color = match edge_count {
            0 => "blue",
            1 => "yellow",
            2 => "green",
            _ => "red",
        };

        canvas.draw_triangle(data.point, color, 1.5, data.heading);

        progress.tick();
    }
}
