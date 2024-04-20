use fixedbitset::FixedBitSet;
use petgraph::{
    graph::NodeIndex,
    stable_graph::StableDiGraph,
    visit::{VisitMap, Visitable},
};

use crate::{
    math::geo_distance,
    output::{Canvas, DrawOptions},
    parse::Point,
    processing::{build_node_acceleration_structure, EdgeData, NodeData},
};

pub fn draw_reachable(
    graph: StableDiGraph<NodeData, EdgeData>,
    point: Point,
    range: f32,
    inverse: bool,
) -> Canvas {
    let mut canvas = Canvas::from_graph(4000, &graph);

    let node_tree = build_node_acceleration_structure(&graph);
    let borrow = [point.latitude, point.longitude];
    let mut close_iter = node_tree.iter_nearest(&borrow, &geo_distance).unwrap();

    let mut visited = graph.visit_map();

    while let Some((dist, (idx, _))) = close_iter.next() {
        if dist > range {
            break;
        }
        visit(&graph, *idx, &mut visited);
    }

    let edge_color = if inverse { "red" } else { "green" };
    let edge_connector_color = if inverse { "crimson" } else { "teal" };

    for edge in graph.edge_indices() {
        let start = graph.edge_endpoints(edge).unwrap().0;
        let end = graph.edge_endpoints(edge).unwrap().1;
        let is_visited = visited.is_visited(&start) && visited.is_visited(&end);
        if is_visited == inverse {
            continue;
        }
        let data = graph.edge_weight(edge).unwrap();
        if data.is_connector {
            let start = graph.node_weight(start).unwrap();
            let end = graph.node_weight(end).unwrap();
            canvas.draw_line(
                start.point,
                end.point,
                DrawOptions {
                    color: edge_connector_color,
                    stroke: 0.25,
                    ..Default::default()
                },
            );
        } else {
            canvas.draw_polyline(
                data.polyline.clone(),
                DrawOptions {
                    color: edge_color,
                    stroke: 0.25,
                    ..Default::default()
                },
            );
        }
    }

    let triangle_color = if inverse { "orange" } else { "lime" };

    for node in graph.node_indices() {
        let data = graph.node_weight(node).unwrap();
        let is_visited = visited.is_visited(&node);
        if is_visited == inverse {
            continue;
        }
        canvas.draw_triangle(data.point, triangle_color, 0.75, data.heading);
    }

    let reachable = visited.count_ones(..);
    if inverse {
        println!("Unreachable nodes: {}", graph.node_count() - reachable);
    } else {
        println!("Reachable nodes: {}", reachable);
    }

    return canvas;
}

fn visit(graph: &StableDiGraph<NodeData, EdgeData>, node: NodeIndex, visited: &mut FixedBitSet) {
    let mut to_visit = vec![node];
    while let Some(node) = to_visit.pop() {
        if visited.visit(node) {
            let neighbours = graph.neighbors(node);
            to_visit.extend(neighbours);
        }
    }
}
