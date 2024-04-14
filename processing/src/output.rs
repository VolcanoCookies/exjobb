use petgraph::{
    visit::{EdgeRef, IntoNodeReferences},
    Direction::{Incoming, Outgoing},
    Graph,
};
use svg::{node::element::path::Data, Document};

use crate::{
    graph::{EdgeData, NodeData},
    parse::Point,
};

pub fn render(width: u32, graph: &Graph<NodeData, EdgeData>) -> Document {
    let size = calc_canvas_size(width, &graph);

    let mut document = Document::new()
        .set("viewBox", (0, 0, size.width, size.height))
        .add(
            svg::node::element::Rectangle::new()
                .set("width", size.width)
                .set("height", size.height)
                .set("fill", "#1f1f1f"),
        );

    let gradient_id = "gradient";

    let gradient = svg::node::element::LinearGradient::new()
        .set("id", gradient_id)
        .set("gradientUnits", "userSpaceOnUse")
        .add(
            svg::node::element::Stop::new()
                .set("offset", "0%")
                .set("stop-color", "red"),
        )
        .add(
            svg::node::element::Stop::new()
                .set("offset", "100%")
                .set("stop-color", "blue"),
        );

    document = document.add(gradient);

    println!("Number of edges: {}", graph.edge_count());
    for edge in graph.edge_references() {
        let idx = edge.id();
        let weight = edge.weight();
        let endpoints = graph.edge_endpoints(idx).unwrap();
        let source = endpoints.0;
        let target = endpoints.1;
        let source_pos = graph.node_weight(source).unwrap().point;
        let target_pos = graph.node_weight(target).unwrap().point;

        let path = Data::new()
            .move_to(convert_point(source_pos, size))
            .line_to(convert_point(target_pos, size));

        document = document.add(
            svg::node::element::Path::new()
                .set("fill", "none")
                .set("stroke", "blue")
                .set("stroke-width", 2)
                .set("d", path),
        );

        let mut path = Data::new();
        path = path.move_to(convert_point(weight.polyline[0], size));
        for point in weight.polyline.iter() {
            path = path.line_to(convert_point(*point, size));
        }
        document = document.add(
            svg::node::element::Path::new()
                .set("fill", "none")
                .set("stroke", "purple")
                .set("stroke-width", 1)
                .set("d", path),
        );
    }

    println!("Number of nodes: {}", graph.node_count());
    let mut count = 0;
    for (idx, data) in graph.node_references() {
        if let Some(sensor) = data.sensor {
            let (x, y) = convert_point(sensor.point, size);
            document = document.add(
                svg::node::element::Circle::new()
                    .set("cx", x)
                    .set("cy", y)
                    .set("r", 4)
                    .set("fill", "green"),
            );

            let path = Data::new()
                .move_to((x, y))
                .line_to(convert_point(data.point, size));
            document = document.add(
                svg::node::element::Path::new()
                    .set("fill", "none")
                    .set("stroke", "yellow")
                    .set("stroke-width", 2)
                    .set("d", path),
            );
        }

        let out_edges = graph.edges_directed(idx, Outgoing);
        let in_edges = graph.edges_directed(idx, Incoming);
        if out_edges.count() + in_edges.count() > 1 {
            continue;
        }

        count += 1;
        let (x, y) = convert_point(data.point, size);

        document = document.add(
            svg::node::element::Circle::new()
                .set("cx", x)
                .set("cy", y)
                .set("r", 2)
                .set("fill", "red"),
        );
    }

    println!("Number of dead ends: {}", count);

    document
}

fn convert_point(point: Point, canvas_size: CanvasSize) -> (f32, f32) {
    let lat_extent = canvas_size.max_lat - canvas_size.min_lat;
    let lon_extent = canvas_size.max_lon - canvas_size.min_lon;

    let x = ((point.longitude - canvas_size.min_lon) / lon_extent) * canvas_size.width as f32;
    let y = ((point.latitude - canvas_size.min_lat) / lat_extent) * canvas_size.height as f32;

    let y = canvas_size.height as f32
        - ((point.latitude - canvas_size.min_lat) / lat_extent) * canvas_size.height as f32;

    (x, y)
}

pub fn calc_canvas_size(width: u32, points: &Graph<NodeData, EdgeData>) -> CanvasSize {
    let points = points.node_weights().collect::<Vec<_>>();

    let min_lat = points
        .iter()
        .map(|d| d.point.latitude)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_lat = points
        .iter()
        .map(|d| d.point.latitude)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let min_lon = points
        .iter()
        .map(|d| d.point.longitude)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_lon = points
        .iter()
        .map(|d| d.point.longitude)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let height = (width as f32 * (max_lat - min_lat) / (max_lon - min_lon)) as u32;
    CanvasSize {
        width,
        height,
        min_lat,
        max_lat,
        min_lon,
        max_lon,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanvasSize {
    width: u32,
    height: u32,
    min_lat: f32,
    max_lat: f32,
    min_lon: f32,
    max_lon: f32,
}
