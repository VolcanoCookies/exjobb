use petgraph::stable_graph::StableGraph;
use svg::{node::element::path::Data, Document, Node};

use crate::{
    parse::Point,
    processing::{EdgeData, NodeData},
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

fn convert_point(point: Point, canvas_size: CanvasSize) -> (f64, f64) {
    let lat_extent = canvas_size.max_lat - canvas_size.min_lat;
    let lon_extent = canvas_size.max_lon - canvas_size.min_lon;

    let x = ((point.longitude - canvas_size.min_lon) / lon_extent) * canvas_size.width as f64;
    //let y = ((point.latitude - canvas_size.min_lat) / lat_extent) * canvas_size.height as f32;

    let y = canvas_size.height as f64
        - ((point.latitude - canvas_size.min_lat) / lat_extent) * canvas_size.height as f64;

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
    let height = (width as f64 * (max_lat - min_lat) / (max_lon - min_lon)) as u32;
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
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
}

pub struct Canvas {
    pub size: CanvasSize,
    pub document: Document,
}

#[derive(Debug, Clone)]
pub struct DrawOptions {
    pub color: String,
    pub stroke: f32,
    pub stroke_linecap: String,
    pub stroke_linejoin: String,
    pub stroke_dasharray: String,
}

impl<'a> Default for DrawOptions {
    fn default() -> Self {
        DrawOptions {
            color: "black".into(),
            stroke: 1.0,
            stroke_linecap: "butt".into(),
            stroke_linejoin: "miter".into(),
            stroke_dasharray: "".into(),
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

    pub fn from_extents(width: usize, extents: ((f64, f64), (f64, f64))) -> Self {
        let lat_extent = extents.0;
        let lon_extent = extents.1;
        let height =
            (width as f64 * (lat_extent.1 - lat_extent.0) / (lon_extent.1 - lon_extent.0)) as u32;
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

    pub fn draw_triangle(&mut self, center: Point, color: &str, size: f64, angle: f64) {
        let mut path = Data::new();
        let (x, y) = convert_point(center, self.size);
        let angle = (angle + 150.0).to_radians();
        let (x1, y1) = (x + angle.cos() * size, y + angle.sin() * size);
        let (x2, y2) = (
            x + (angle + 2.0 * std::f64::consts::PI / 3.0).cos() * size,
            y + (angle + 2.0 * std::f64::consts::PI / 3.0).sin() * size,
        );
        let (x3, y3) = (
            x + (angle + 4.0 * std::f64::consts::PI / 3.0).cos() * size,
            y + (angle + 4.0 * std::f64::consts::PI / 3.0).sin() * size,
        );

        path = path
            .move_to((x, y))
            .line_to((x1, y1))
            .line_to((x2, y2))
            .line_to((x3, y3))
            .close();
        self.document.append(
            svg::node::element::Path::new()
                .set("fill", color)
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

    pub fn draw_cross(&mut self, point: Point, color: &str, size: f64) {
        let (x, y) = convert_point(point, self.size);
        let path = Data::new()
            .move_to((x - size, y - size))
            .line_to((x + size, y + size))
            .move_to((x - size, y + size))
            .line_to((x + size, y - size));
        self.document.append(
            svg::node::element::Path::new()
                .set("fill", "none")
                .set("stroke", color)
                .set("stroke-width", size)
                .set("d", path),
        );
    }

    pub fn save(&self, path: &str) {
        svg::save(path, &self.document).unwrap();
    }
}
