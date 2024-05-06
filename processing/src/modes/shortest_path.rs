use petgraph::{stable_graph::StableDiGraph, visit::IntoNodeReferences};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{
    custom_bfs::Positionable,
    math::geo_distance,
    output::{Canvas, DrawOptions},
    processing::{build_node_acceleration_structure, EdgeData, NodeData, ProcessedGraph},
    visitor::{self, convert_kmh_to_ms},
    PointQuery,
};

pub fn shortest_path(
    progessed_graph: ProcessedGraph,
    desired_path: Vec<PointQuery>,
    cull_to_path_distance: f64,
    distance_metric: visitor::DistanceMetric,
) -> Canvas {
    let ProcessedGraph {
        mut graph,
        sensor_store,
    } = progessed_graph;

    let tree = build_node_acceleration_structure(&graph);
    let points = desired_path
        .iter()
        .map(|query| {
            let p = [query.point.latitude, query.point.longitude];
            let mut iter = tree.iter_nearest(&p, &geo_distance).unwrap();
            while let Some((dist, (idx, data))) = iter.next() {
                if query.heading.contains(&data.heading) && dist <= query.radius {
                    return *idx;
                }
            }

            panic!("No node found for query {:?}", query);
        })
        .collect::<Vec<_>>();

    println!("Finding shortest path");
    let path = visitor::shortest_path(&graph, points, distance_metric).expect("No path found");
    match distance_metric {
        visitor::DistanceMetric::Space => println!("Shortest path distance: {}m", path.length),
        visitor::DistanceMetric::Time => {
            let distance = path.nodes.windows(2).fold(0.0, |acc, nodes| {
                let edge = graph.edges_connecting(nodes[0], nodes[1]).next().unwrap();
                acc + edge.weight().distance
            });
            let average_speed = distance / path.length;
            println!(
                "Shortest path time: {}s, distance: {}, average speed: {}m/s",
                path.length, distance, average_speed
            )
        }
    }

    println!("Shortest path length: {}", path.length);

    let start = path.nodes[0];
    let start = graph.node_weight(start).unwrap().point;
    let points = vec![start]
        .into_iter()
        .chain(
            path.nodes
                .windows(2)
                .flat_map(|pair| {
                    let from = pair[0];
                    let to = pair[1];

                    let edge = graph.edges_connecting(from, to).next().unwrap();

                    edge.weight().polyline.iter().skip(1).cloned()
                })
                .collect::<Vec<_>>()
                .into_iter(),
        )
        .collect::<Vec<_>>();

    if !cull_to_path_distance.is_nan() {
        let mut path_tree = kdtree::KdTree::new(2);
        for point in points.iter() {
            path_tree
                .add([point.latitude, point.longitude], ())
                .unwrap();
        }
        for missed in path.missed.iter() {
            let data = graph.node_weight(*missed).unwrap();
            path_tree
                .add([data.point.latitude, data.point.longitude], ())
                .unwrap();
        }

        let par_iter = graph.node_indices().par_bridge();
        let to_remove = par_iter
            .filter(|node| {
                let data = graph.node_weight(*node).unwrap();
                let point = data.point;
                let (dist, _) = path_tree
                    .nearest(&[point.latitude, point.longitude], 1, &geo_distance)
                    .unwrap()[0];

                dist > cull_to_path_distance
            })
            .collect::<Vec<_>>();

        for node in to_remove {
            graph.remove_node(node);
        }
    }

    let mut canvas = Canvas::from_graph(4000, &graph);

    for query in desired_path {
        canvas.draw_circle(query.point, "magenta", 10.0);
    }

    for edge in graph.edge_weights() {
        let color = if edge.is_connector {
            "lime"
        } else if edge.speed_limit.is_none() {
            "aqua"
        } else {
            "green"
        };
        canvas.draw_polyline(
            edge.polyline.clone(),
            DrawOptions {
                color: color.into(),
                stroke: 1.0,
                ..Default::default()
            },
        )
    }

    canvas.draw_polyline(
        points,
        DrawOptions {
            color: "red".into(),
            stroke: 0.75,
            ..Default::default()
        },
    );

    for (idx, data) in graph.node_references() {
        if data.has_sensor {
            let sensors = sensor_store.get(&idx).unwrap();
            for sensor in sensors {
                canvas.draw_line(
                    sensor.point(),
                    data.point,
                    DrawOptions {
                        stroke: 1.0,
                        color: "aqua".into(),
                        ..Default::default()
                    },
                );
            }
            canvas.draw_circle(data.point, "yellow", 2.5);
        }
    }

    for node in path.nodes.iter() {
        let data = graph.node_weight(*node).unwrap();
        if data.has_sensor {
            let sensors = sensor_store.get(node).unwrap();
            for sensor in sensors {
                canvas.draw_line(
                    sensor.point(),
                    data.point,
                    DrawOptions {
                        stroke: 1.0,
                        color: "aqua".into(),
                        ..Default::default()
                    },
                );
                canvas.text(sensor.point(), format!("{}", sensor.site_id).as_str());
            }

            canvas.draw_circle(data.point, "orange", 2.5);
        }
    }

    for missed in path.missed.iter() {
        println!("Missed node: {:?}", missed);
        let data = graph.node_weight(*missed).unwrap();
        canvas.draw_circle(data.point, "red", 5.0);
    }

    let travel_time = calculate_travel_time(&graph, &path);

    println!("Travel time: {}s", travel_time);

    canvas
}

fn calculate_travel_time(graph: &StableDiGraph<NodeData, EdgeData>, path: &visitor::Path) -> f64 {
    let mut travel_time = 0.0;
    let mut previous_speed_limit = convert_kmh_to_ms(50.0);

    for nodes in path.nodes.windows(2) {
        let edge = graph.edges_connecting(nodes[0], nodes[1]).next().unwrap();
        let data = edge.weight();
        let speed_limit = if let Some(speed_limit) = data.speed_limit {
            convert_kmh_to_ms(speed_limit)
        } else {
            previous_speed_limit
        };
        let distance = data.distance;

        let time = distance / speed_limit;
        travel_time += time;
        previous_speed_limit = speed_limit;
    }

    travel_time
}
