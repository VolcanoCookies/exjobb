mod coloring;

use clap::{Args, ValueEnum};
use console::style;
use petgraph::{graph::NodeIndex, stable_graph::StableDiGraph, visit::VisitMap};

use crate::{
    custom_bfs::CustomBfs,
    math::geo_distance,
    output::Canvas,
    parse::Point,
    processing::{build_node_acceleration_structure, EdgeData, NodeData},
    progress::Progress,
    visitor::DistanceMetric,
};

use self::coloring::{EdgeColor, LineStyle, NodeColor};

#[derive(Debug, Clone, Args)]
pub struct InspectOptions {
    #[clap(flatten)]
    point: Point,
    #[clap(short, long)]
    range: f64,
    #[clap(
        short = 'n',
        long,
        default_value = "none",
        default_missing_value = "simple"
    )]
    node_color: NodeColor,
    #[clap(short = 'e', long, default_value = "none")]
    edge_color: EdgeColor,
    #[clap(short, long, default_value = "space")]
    metric: DistanceMetric,
    #[clap(
        short = 'd',
        long,
        default_value = "false",
        default_missing_value = "true"
    )]
    directed: bool,
    #[clap(short, long, default_value = "air")]
    filter: FilterMode,
    #[clap(flatten)]
    line_style: LineStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FilterMode {
    #[clap(name = "road", help = "Distance along the road network")]
    RoadDistance,
    #[clap(name = "air", help = "Distance as the crow flies")]
    AirDistance,
}

pub fn inspect(mut graph: StableDiGraph<NodeData, EdgeData>, options: InspectOptions) -> Canvas {
    let mut progress = Progress::new();

    // Remove nodes outside of range
    filter_distance(&mut progress, &mut graph, &options);

    let mut canvas = Canvas::from_graph(4000, &graph);

    canvas.draw_cross(options.point, "red", 5.0);

    let color_func = options.edge_color.coloring_function();
    color_func(&mut progress, &mut canvas, &graph, &options);

    let color_func = options.node_color.coloring_function();
    color_func(&mut progress, &mut canvas, &graph, &options);

    canvas
}

/// Find the closest node to a point
fn find_closest_node_to(graph: &StableDiGraph<NodeData, EdgeData>, point: Point) -> NodeIndex {
    let tree = build_node_acceleration_structure(&graph);
    let p = [point.latitude, point.longitude];
    let (_, (center_node, _)) = tree
        .iter_nearest(&p, &geo_distance)
        .unwrap()
        .next()
        .expect("No node found for query");
    *center_node
}

fn filter_distance(
    progress: &mut Progress,
    graph: &mut StableDiGraph<NodeData, EdgeData>,
    opts: &InspectOptions,
) {
    if opts.filter == FilterMode::AirDistance && opts.metric == DistanceMetric::Time {
        panic!("Cannot filter by air distance using time metric");
    }

    if opts.filter == FilterMode::AirDistance {
        progress.step_sized(graph.node_count(), "Filtering nodes by air distance");
        let tree = build_node_acceleration_structure(&graph);
        let p = [opts.point.latitude, opts.point.longitude];
        let to_remove = tree
            .iter_nearest(&p, &geo_distance)
            .unwrap()
            .filter(|(dist, _)| {
                progress.tick();
                *dist > opts.range
            })
            .map(|(_, (node, _))| node)
            .collect::<Vec<_>>();
        let len = to_remove.len();
        for node in to_remove {
            graph.remove_node(*node);
        }
        progress.finish(format!("Removed {} nodes", style(len).bold()));
    } else {
        progress.step_sized(
            graph.node_count(),
            format!(
                "Filtering nodes at a distance of {}{}",
                style(opts.range).bold(),
                style(opts.metric.unit()).bold()
            ),
        );
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
        let to_remove = graph
            .node_indices()
            .filter(|node| !bfs.discovered.is_visited(node))
            .collect::<Vec<_>>();
        let len = to_remove.len();
        for node in to_remove {
            graph.remove_node(node);
        }
        progress.finish(format!("Removed {} nodes", style(len).bold()));
    }
}
