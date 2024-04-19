use petgraph::stable_graph::StableDiGraph;

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

pub fn draw_roads(graph: StableDiGraph<NodeData, EdgeData>, unique_ids: Vec<i32>) -> Canvas {
    if unique_ids.is_empty() {
        panic!("No unique ids provided");
    } else if unique_ids.len() > COLORS.len() {
        panic!("Too many unique ids provided");
    }

    let mut canvas = Canvas::from_graph(4000, &graph);

    for edge in graph.edge_indices() {
        let data = graph.edge_weight(edge).unwrap();
        if data.polyline.is_empty() {
            continue;
        }

        let mut color = None;
        for (idx, id) in unique_ids.iter().enumerate() {
            if data.original_road_id == *id {
                color = Some(COLORS[idx]);
                break;
            }
        }
        let color = color.unwrap();
        canvas.draw_polyline(
            data.polyline.clone(),
            DrawOptions {
                color,
                stroke: 1.0,
                ..Default::default()
            },
        );
    }

    return canvas;
}
