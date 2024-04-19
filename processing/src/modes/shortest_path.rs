use geo::point;
use svg::Document;

use crate::{
    output,
    parse::{RoadData, SensorData},
    processing::{
        build_node_acceleration_structure, closest_node, parse_data, GraphProcessingOptions,
    },
    visitor::{self, Path},
    PointQuery,
};

pub fn shortest_path(
    road_data: Vec<RoadData>,
    sensor_data: Vec<SensorData>,
    opts: GraphProcessingOptions,
    print_path_roads: bool,
) -> Document {
    let graph = parse_data(road_data, sensor_data, opts);

    let desired_path = vec![
        PointQuery::new(59.241836, 17.837475, f32::INFINITY, 0.0..180.0),
        //PointQuery::new(59.325564, 18.003914, f32::INFINITY, -20.0..120.0),
        PointQuery::new(59.296046, 18.060011, f32::INFINITY, 45.0..135.0),
        PointQuery::new(59.316857, 18.070623, f32::INFINITY, -45.0..45.0),
        PointQuery::new(59.367279, 18.020929, f32::INFINITY, -80.0..80.0),
    ];

    let tree = build_node_acceleration_structure(&graph);
    let points = desired_path
        .iter()
        .map(|query| closest_node(&graph, &tree, query.clone()).unwrap())
        .collect::<Vec<_>>();

    println!("Finding shortest path");
    let path = visitor::shortest_path(&graph, points);
    if let Some(path) = path.as_ref() {
        println!("Shortest path length: {}", path.length);
        if print_path_roads {
            let mut prev = -2;
            for node in path.nodes.iter() {
                let data = graph.node_weight(*node).unwrap();
                if prev != data.original_road_id {
                    println!("Road: {}", data.original_road_id);
                }
                prev = data.original_road_id;
            }
        }
    } else {
        println!("No path found");
    }

    output::render(
        4000,
        &graph,
        path,
        output::RenderOptions {
            show_sensors: false,
            show_sensor_connections: false,
            show_road_caps: true,
            show_road_connections: true,
            show_graph_edges: false,
            show_graph_nodes: false,
            show_original_edges: true,
            show_path: true,
        },
    )
}
