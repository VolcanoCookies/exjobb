use petgraph::{
    stable_graph::StableGraph,
    visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
};
use svg::{node::element::path::Data, Document, Node};

use crate::{
    math::lerp,
    parse::Point,
    processing::{EdgeData, NodeData},
    visitor::Path,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderOptions {
    pub show_sensors: bool,
    pub show_sensor_connections: bool,
    pub show_road_caps: bool,
    pub show_road_connections: bool,
    pub show_graph_edges: bool,
    pub show_graph_nodes: bool,
    pub show_original_edges: bool,
    pub show_path: bool,
}

pub fn render(
    width: u32,
    graph: &StableGraph<NodeData, EdgeData>,
    path: Option<Path>,
    opts: RenderOptions,
) -> Document {
    let size = calc_canvas_size(width, &graph);
    let mut canvas = Canvas::new(size);
    println!("Number of edges: {}", graph.edge_count());

    if opts.show_sensors {
        for (_, data) in graph.node_references() {
            if let Some(sensor) = data.sensor {
                canvas.draw_circle(sensor.point, "green", 4.0);
            }
        }
    }

    if opts.show_original_edges {
        for edge in graph.edge_references() {
            let data = edge.weight();
            canvas.draw_polyline(
                data.polyline.clone(),
                DrawOptions {
                    color: "purple",
                    stroke: 0.75,
                    ..Default::default()
                },
            );
        }
    }

    if opts.show_graph_edges {
        for edge in graph.edge_references() {
            let data = edge.weight();
            if data.is_connector {
                continue;
            }

            let endpoints = graph.edge_endpoints(edge.id()).unwrap();
            let source = graph.node_weight(endpoints.0).unwrap().point;
            let target = graph.node_weight(endpoints.1).unwrap().point;

            canvas.draw_line(
                source,
                target,
                DrawOptions {
                    color: "blue",
                    stroke: 0.5,
                    ..Default::default()
                },
            );
        }
    }

    if opts.show_road_connections {
        for edge in graph.edge_references() {
            let data = edge.weight();
            if !data.is_connector {
                continue;
            }

            let endpoints = graph.edge_endpoints(edge.id()).unwrap();
            let source = graph.node_weight(endpoints.0).unwrap().point;
            let target = graph.node_weight(endpoints.1).unwrap().point;

            canvas.draw_line(
                source,
                target,
                DrawOptions {
                    color: "yellow",
                    stroke: 0.55,
                    ..Default::default()
                },
            );
        }
    }

    println!("Number of nodes: {}", graph.node_count());
    for (_, data) in graph.node_references() {
        if let Some(sensor) = data.sensor {
            if opts.show_sensor_connections {
                canvas.draw_line(
                    sensor.point,
                    data.point,
                    DrawOptions {
                        color: "teal",
                        stroke: 1.0,
                        ..Default::default()
                    },
                )
            }
        }

        if opts.show_graph_nodes {
            canvas.draw_circle(data.point, "red", 0.5);
        }
    }

    for (node, data) in graph.node_references() {
        let edge_count = graph.neighbors_undirected(node).count();
        canvas.text(data.point, &format!("{}", edge_count));
    }

    if opts.show_path {
        if let Some(path) = path {
            let len = path.nodes.len();
            let mut iter = path.nodes.iter();
            let prev = iter.next().unwrap();
            let mut prev_data = graph.node_weight(*prev).unwrap();

            let from_color = (255, 0, 0);
            let to_color = (0, 255, 0);

            for (i, node) in iter.enumerate() {
                let prog = i as f32 / len as f32;
                let color = (
                    lerp(from_color.0 as f32, to_color.0 as f32, prog).round() as u8,
                    lerp(from_color.1 as f32, to_color.1 as f32, prog).round() as u8,
                    lerp(from_color.2 as f32, to_color.2 as f32, prog).round() as u8,
                );

                let data = graph.node_weight(*node).unwrap();
                canvas.draw_line(
                    prev_data.point,
                    data.point,
                    DrawOptions {
                        color: &format!("rgb({}, {}, {})", color.0, color.1, color.2),
                        stroke: 0.3,
                        ..Default::default()
                    },
                );

                prev_data = data;
            }
        }
    }

    canvas.document
}

pub fn render_polyline(width: u32, polyline: Vec<Point>) -> Document {
    let lon_extent = polyline
        .iter()
        .map(|p| p.longitude)
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), lon| {
            (min.min(lon), max.max(lon))
        });
    let lat_extent = polyline
        .iter()
        .map(|p| p.latitude)
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), lat| {
            (min.min(lat), max.max(lat))
        });

    let size = CanvasSize {
        width,
        height: (width as f32 * (lat_extent.1 - lat_extent.0) / (lon_extent.1 - lon_extent.0))
            as u32,
        min_lat: lat_extent.0,
        max_lat: lat_extent.1,
        min_lon: lon_extent.0,
        max_lon: lon_extent.1,
    };

    let mut canvas = Canvas::new(size);

    canvas.draw_polyline(
        polyline,
        DrawOptions {
            color: "red",
            stroke: 0.5,
            ..Default::default()
        },
    );

    canvas.document
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

pub fn calc_canvas_size(width: u32, graph: &StableGraph<NodeData, EdgeData>) -> CanvasSize {
    let points = graph.node_weights().collect::<Vec<_>>();

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

pub struct Canvas {
    pub size: CanvasSize,
    pub document: Document,
}

pub struct DrawOptions<'a> {
    pub color: &'a str,
    pub stroke: f32,
    pub stroke_linecap: &'a str,
    pub stroke_linejoin: &'a str,
    pub stroke_dasharray: &'a str,
}

impl<'a> Default for DrawOptions<'a> {
    fn default() -> Self {
        DrawOptions {
            color: "black",
            stroke: 1.0,
            stroke_linecap: "butt",
            stroke_linejoin: "miter",
            stroke_dasharray: "",
        }
    }
}

impl Canvas {
    pub fn new(size: CanvasSize) -> Self {
        let document = Document::new()
            .set("viewBox", (0, 0, size.width, size.height))
            .add(
                svg::node::element::Rectangle::new()
                    .set("width", size.width)
                    .set("height", size.height)
                    .set("fill", "#1f1f1f"),
            );

        Canvas { size, document }
    }

    pub fn from_graph(width: u32, graph: &StableGraph<NodeData, EdgeData>) -> Self {
        let size = calc_canvas_size(width, graph);
        Canvas::new(size)
    }

    pub fn from_extents(width: usize, extents: ((f32, f32), (f32, f32))) -> Self {
        let lat_extent = extents.0;
        let lon_extent = extents.1;
        let height =
            (width as f32 * (lat_extent.1 - lat_extent.0) / (lon_extent.1 - lon_extent.0)) as u32;
        let size = CanvasSize {
            width: width as u32,
            height,
            min_lat: lat_extent.0,
            max_lat: lat_extent.1,
            min_lon: lon_extent.0,
            max_lon: lon_extent.1,
        };
        Canvas::new(size)
    }

    pub fn draw_circle(&mut self, point: Point, color: &str, size: f32) {
        let (x, y) = convert_point(point, self.size);
        self.document.append(
            svg::node::element::Circle::new()
                .set("cx", x)
                .set("cy", y)
                .set("r", size)
                .set("fill", color),
        );
    }
    pub fn draw_circles(&mut self, points: Vec<Point>, color: &str, size: f32) {
        for point in points.iter() {
            self.draw_circle(*point, color, size);
        }
    }

    pub fn draw_line(&mut self, start: Point, end: Point, opts: DrawOptions) {
        self.draw_polyline(vec![start, end], opts);
    }

    pub fn draw_polyline(&mut self, points: Vec<Point>, opts: DrawOptions) {
        if points.len() < 2 {
            return;
        }
        let mut path = Data::new();
        let mut iter = points.iter();
        let point = iter.next().unwrap();
        path = path.move_to(convert_point(*point, self.size));
        for point in iter {
            path = path.line_to(convert_point(*point, self.size));
        }
        self.document.append(
            svg::node::element::Path::new()
                .set("fill", "none")
                .set("stroke", opts.color)
                .set("stroke-width", opts.stroke)
                .set("stroke-linecap", opts.stroke_linecap)
                .set("stroke-linejoin", opts.stroke_linejoin)
                .set("stroke-dasharray", opts.stroke_dasharray)
                .set("d", path),
        );
    }

    pub fn text(&mut self, point: Point, text: &str) {
        let (x, y) = convert_point(point, self.size);

        self.document.append(
            svg::node::element::Text::new(text)
                .set("x", x)
                .set("y", y)
                .set("font-size", 1)
                .set("fill", "white"),
        );
    }

    pub fn save(&self, path: &str) {
        svg::save(path, &self.document).unwrap();
    }
}
