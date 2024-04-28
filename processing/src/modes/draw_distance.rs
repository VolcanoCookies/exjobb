use std::{mem::swap, time::Instant};

use console::style;
use petgraph::{
    stable_graph::StableDiGraph,
    visit::{EdgeRef, IntoEdgeReferences, VisitMap},
};

use crate::{
    custom_bfs::CustomBfs,
    math::{geo_distance, lerp},
    output::{Canvas, DrawOptions},
    processing::{build_node_acceleration_structure, EdgeData, NodeData},
    progress::eta_bar,
    visitor::{self},
    PointQuery,
};

pub fn draw_distance(
    mut graph: StableDiGraph<NodeData, EdgeData>,
    query: PointQuery,
    max_distance: f64,
    distance_metric: visitor::DistanceMetric,
    forward_only: bool,
) -> Canvas {
    let mut step = 1;
    let steps = 3;

    fn fsteps(step: i32, steps: i32) -> console::StyledObject<String> {
        style(format!("[{}/{}]", step, steps)).bold().dim()
    }

    let tree = build_node_acceleration_structure(&graph);
    let p = [query.point.latitude, query.point.longitude];
    let (_, (node, _)) = tree
        .iter_nearest(&p, &geo_distance)
        .unwrap()
        .filter(|(dist, (_, data))| query.heading.contains(&data.heading) && *dist <= query.radius)
        .next()
        .expect("No node found for query");

    println!(
        "{} Filtering {} nodes at a distance of {}",
        fsteps(step, steps),
        style(graph.node_count()).bold().dim(),
        style(max_distance).bold().dim()
    );
    let start = Instant::now();
    let pb = eta_bar(graph.node_count() as usize);
    let mut bfs = CustomBfs::new(&graph, *node, distance_metric.to_function());
    let next_func = if forward_only {
        CustomBfs::next
    } else {
        CustomBfs::next_undirected
    };
    while let Some((idx, dist, _)) = next_func(&mut bfs, &graph) {
        if dist > max_distance {
            bfs.discovered.set(idx.index(), false);
            break;
        }
        pb.inc(1);
    }
    pb.finish_and_clear();
    println!(
        "{:?} Found {} nodes in range",
        style(start.elapsed()).bold().dim().yellow(),
        style(bfs.distances.len()).bold().dim()
    );
    step += 1;

    println!("{} Culling nodes not on path", fsteps(step, steps));
    let start = Instant::now();
    let pb = eta_bar(graph.node_count() as usize);

    let mut to_remove = Vec::new();
    for node in graph.node_indices() {
        if bfs.discovered.is_visited(&node) {
            pb.inc(1);
            continue;
        }

        to_remove.push(node);
        pb.inc(1);
    }
    let to_remove_len = to_remove.len();
    for node in to_remove {
        graph.remove_node(node);
    }
    pb.finish_and_clear();
    println!(
        "{:?} Removed {} nodes",
        style(start.elapsed()).bold().dim().yellow(),
        style(to_remove_len).bold().dim()
    );
    step += 1;

    let mut canvas = Canvas::from_graph(4000, &graph);
    canvas.draw_circle(query.point, "red", 10.0);

    let grad = colorgrad::CustomGradient::new()
        .html_colors(&["gold", "hotpink", "darkturquoise"])
        .domain(&[0.0, max_distance])
        .build()
        .unwrap();

    println!("{} Drawing graph", fsteps(step, steps));
    let start = Instant::now();
    let pb = eta_bar(graph.node_count() as usize);
    for edge in graph.edge_references() {
        let data = edge.weight();

        let mut source = graph.node_weight(edge.source()).unwrap();
        let mut target = graph.node_weight(edge.target()).unwrap();

        let mut source_distance = bfs.distances.get(&edge.source()).unwrap();
        let mut target_distance = bfs.distances.get(&edge.target()).unwrap();

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
            canvas.draw_line(
                source.point,
                target.point,
                DrawOptions {
                    color,
                    stroke: 1.0,
                    ..Default::default()
                },
            );
        }

        pb.inc(1);
    }
    pb.finish_and_clear();
    println!(
        "{:?} Drew {} edges",
        style(start.elapsed()).bold().dim().yellow(),
        style(graph.edge_count()).bold().dim()
    );

    canvas
}
