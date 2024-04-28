use std::{collections::VecDeque, mem::swap};

use clap::ValueEnum;
use console::style;
use fixedbitset::FixedBitSet;
use petgraph::{
    graph::NodeIndex,
    stable_graph::StableDiGraph,
    visit::{EdgeRef, IntoEdgeReferences, VisitMap, Visitable},
};

use crate::{
    custom_bfs::CustomBfs,
    math::{geo_distance, lerp},
    modes::{
        draw_disjoint::COLORS,
        inspect::{find_closest_node_to, InspectOptions},
    },
    output::Canvas,
    processing::{EdgeData, Metadata, NodeData},
    progress::Progress,
    visitor::DistanceMetric,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EdgeColor {
    None,
    Disjoint,
    Distance,
    DistanceDirected,
    ForbiddenDirection,
}

impl EdgeColor {
    pub fn coloring_function(
        &self,
    ) -> fn(&mut Progress, &mut Canvas, &StableDiGraph<NodeData, EdgeData>, &InspectOptions) {
        match self {
            EdgeColor::None => noop,
            EdgeColor::Disjoint => disjoint,
            EdgeColor::Distance => distance,
            EdgeColor::DistanceDirected => distance,
            EdgeColor::ForbiddenDirection => forbidden_direction,
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

fn disjoint(
    progress: &mut Progress,
    canvas: &mut Canvas,
    graph: &StableDiGraph<NodeData, EdgeData>,
    opts: &InspectOptions,
) {
    progress.step_sized(
        graph.edge_count(),
        format!("Drawing {} edges", style(graph.edge_count()).bold()),
    );

    fn visit(
        graph: &StableDiGraph<NodeData, EdgeData>,
        node: NodeIndex,
        directed: bool,
    ) -> FixedBitSet {
        let mut to_visit = VecDeque::new();
        to_visit.push_back(node);
        let mut visited = graph.visit_map();
        while let Some(node) = to_visit.pop_front() {
            if visited.visit(node) {
                let edges_out = graph
                    .edges_directed(node, petgraph::Direction::Outgoing)
                    .map(|edge| edge.target());
                to_visit.extend(edges_out);
                if !directed {
                    let edges_in = graph
                        .edges_directed(node, petgraph::Direction::Incoming)
                        .map(|edge| edge.source());
                    to_visit.extend(edges_in);
                }
            }
        }

        visited
    }

    let mut sets: Vec<FixedBitSet> = Vec::new();
    'outer: for node in graph.node_indices() {
        for set in sets.iter() {
            if set.is_visited(&node) {
                continue 'outer;
            }
        }
        let set = visit(&graph, node, opts.directed);
        sets.push(set);
    }

    let sets = sets.iter().fold(Vec::new(), |mut acc, set| {
        for idx in 0..acc.len() {
            let other = &acc[idx];
            if set.intersection(other).next().is_some() {
                acc[idx] = set.union(other).collect();
                return acc;
            }
        }
        acc.push(set.clone());
        acc
    });

    for edge in graph.edge_references() {
        let mut color = "gray";
        for (idx, set) in sets.iter().enumerate() {
            if set.is_visited(&edge.source()) && set.is_visited(&edge.target()) {
                color = COLORS[idx % COLORS.len()];
                break;
            }
        }

        let opts = opts.line_style.to_draw(color);
        canvas.draw_polyline(edge.weight().polyline.clone(), opts);

        progress.tick();
    }

    progress.finish(format!(
        "Drew {} edges in {} different sets",
        style(graph.edge_count()).bold(),
        style(sets.len()).bold(),
    ));
}

fn distance(
    progress: &mut Progress,
    canvas: &mut Canvas,
    graph: &StableDiGraph<NodeData, EdgeData>,
    opts: &InspectOptions,
) {
    progress.step_sized(graph.node_count(), "Calculating distances");
    let center_node = find_closest_node_to(&graph, opts.point);
    let mut bfs = CustomBfs::new(&graph, center_node, opts.metric.to_function());
    let next_func = if opts.directed {
        CustomBfs::next
    } else {
        CustomBfs::next_undirected
    };
    while let Some((idx, dist, _)) = next_func(&mut bfs, &graph) {
        if dist > opts.range {
            bfs.discovered.set(idx.index(), false);
            break;
        }
        progress.tick();
    }
    progress.finish(format!(
        "Found {} nodes in range",
        style(bfs.distances.len()).bold()
    ));

    let direction = if graph.is_directed() {
        "directed"
    } else {
        "undirected"
    };
    let name = match opts.metric {
        DistanceMetric::Space => "Space distance",
        DistanceMetric::Time => "Time distance",
    };
    progress.step_sized(
        graph.node_count(),
        format!(
            "Drawing {} edges: {} {}",
            style(graph.edge_count()).bold(),
            name,
            direction
        ),
    );

    let grad = colorgrad::CustomGradient::new()
        .html_colors(&["gold", "hotpink", "darkturquoise"])
        .domain(&[0.0, opts.range])
        .build()
        .unwrap();

    for edge in graph.edge_references() {
        let data = edge.weight();

        let mut source = graph.node_weight(edge.source()).unwrap();
        let mut target = graph.node_weight(edge.target()).unwrap();

        let source_distance = bfs.distances.get(&edge.source());
        let target_distance = bfs.distances.get(&edge.target());

        if source_distance.is_some() && target_distance.is_some() {
            let mut source_distance = source_distance.unwrap();
            let mut target_distance = target_distance.unwrap();

            let mut polyline = data.polyline.clone();
            if source_distance > target_distance {
                swap(&mut source, &mut target);
                swap(&mut source_distance, &mut target_distance);
                polyline.reverse();
            }

            let polyline_len = polyline.windows(2).fold(0.0, |acc, pair| {
                acc + geo_distance(
                    &[pair[0].latitude, pair[0].longitude],
                    &[pair[1].latitude, pair[1].longitude],
                )
            });

            let mut distance = 0.0;
            for pair in data.polyline.windows(2) {
                let a = pair[0];
                let b = pair[1];
                let a = [a.latitude, a.longitude];
                let b = [b.latitude, b.longitude];

                let dist = geo_distance(&a, &b);
                distance += dist;

                let traversed_perc = distance / polyline_len;
                let diff = target_distance - source_distance;
                let dist = lerp(0.0, diff, traversed_perc) + source_distance;

                let color = grad.at(dist);

                let color = format!(
                    "rgb({}, {}, {})",
                    color.r * 255.0,
                    color.g * 255.0,
                    color.b * 255.0
                );
                let opts = opts.line_style.to_draw(color.as_str());
                canvas.draw_line(source.point, target.point, opts);
            }
        } else {
            let opts = opts.line_style.to_draw("gray");
            canvas.draw_polyline(edge.weight().polyline.clone(), opts);
        }

        progress.tick();
    }
    progress.finish(format!("Drew {} edges", style(graph.edge_count()).bold()));
}

fn forbidden_direction(
    progress: &mut Progress,
    canvas: &mut Canvas,
    graph: &StableDiGraph<NodeData, EdgeData>,
    opts: &InspectOptions,
) {
    progress.step_sized(graph.node_count(), "Drawing edges with forbidden direction");

    for edge in graph.edge_references() {
        let data = edge.weight();

        let color = match data.direction {
            crate::parse::RoadDirection::Forward => "blue",
            crate::parse::RoadDirection::Backward => "red",
            crate::parse::RoadDirection::Both => "green",
            crate::parse::RoadDirection::None => "magenta",
        };
        let opts = opts.line_style.to_draw(color);
        canvas.draw_polyline(data.polyline.clone(), opts);

        progress.tick();
    }
    progress.finish(format!("Drew {} edges", style(graph.edge_count()).bold()));
}
