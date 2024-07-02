mod args;
mod custom_bfs;
mod gpkg;
mod math;
mod modes;
mod mongo;
mod output;
mod parse;
mod processing;
mod progress;
mod travel_time;
mod util;
mod visitor;

use clap::{Parser, Subcommand};
use console::style;
use human_bytes::human_bytes;
use math::geo_distance;
use modes::{AggregateOptions, InspectOptions, TestPeriodDivisionOptions};
use mongo::client::MongoOptions;
use output::{calc_canvas_size_from_extents, Canvas, DrawOptions};
use parse::{parse_road_data, parse_sensor_data, Point};
use petgraph::visit::IntoEdgeReferences;
use processing::build_node_acceleration_structure;
use tokio::runtime::Runtime;
use visitor::DistanceMetric;

use crate::{
    modes::test_period_division, mongo::client::async_client::AsyncMongoClient, parse::read_roads,
    processing::ProcessedGraph, util::PointQuery,
};

#[derive(Debug, Parser)]
#[command(
    name = "traffic-simulator",
    version = "0.1.0",
    author = "Francis Gniady",
    about = "A traffic simulator"
)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    ParseRawData {
        #[clap(short = 'r', long)]
        raw_road_data: Option<String>,
        #[clap(short = 's', long)]
        raw_sensor_data: Option<String>,
        #[clap(short = 'R', long, default_value = "../roadData.json")]
        road_data: String,
        #[clap(short = 'S', long, default_value = "../sensorData.json")]
        sensor_data: String,
    },
    DrawRoad {
        #[clap(long, default_value = "./out/./out/graph.bin")]
        input: String,
        #[clap(long, default_value = "./out/graph.svg")]
        output: String,
        #[clap(short, long)]
        unique_ids: Vec<i32>,
    },
    ShortestPath {
        #[clap(long, default_value = "./out/graph.json")]
        input: String,
        #[clap(long, default_value = "./out/graph.svg")]
        output: String,
        #[clap(short, long, default_value = "./queries/query1.json")]
        query_file: String,
        #[clap(short, long, default_value = "nan")]
        cull_to_path_distance: f64,
        #[clap(short, long, default_value = "space")]
        metric: DistanceMetric,
    },
    DrawDisjoint {
        #[clap(long, default_value = "./out/graph.json")]
        input: String,
        #[clap(long, default_value = "./out/graph.svg")]
        output: String,
    },
    DrawReachable {
        #[clap(long, default_value = "./out/graph.bin")]
        input: String,
        #[clap(long, default_value = "./out/graph.svg")]
        output: String,
        #[clap(short = 'a', long = "lat")]
        latitude: f64,
        #[clap(short = 'o', long = "lon")]
        longitude: f64,
        #[clap(short, long)]
        range: f64,
        #[clap(short, long, default_value = "false", default_missing_value = "true")]
        inverse: bool,
    },
    DrawDistance {
        #[clap(long, default_value = "./out/graph.bin")]
        input: String,
        #[clap(long, default_value = "./out/graph.svg")]
        output: String,
        #[clap(short = 'a', long = "lat")]
        latitude: f64,
        #[clap(short = 'o', long = "lon")]
        longitude: f64,
        #[clap(short = 'c', long, default_value = "nan")]
        max_distance: f64,
        #[clap(short, long, default_value = "space")]
        metric: DistanceMetric,
        #[clap(short, long, default_value = "false", default_missing_value = "true")]
        forward_only: bool,
    },
    Process {
        #[clap(short, long, default_value = "./out/gpkgData.json")]
        road_data: String,
        #[clap(short, long, default_value = "./out/graph.json")]
        output: String,
        #[clap(flatten)]
        mongo_options: MongoOptions,
        #[clap(flatten)]
        processing_options: processing::GraphProcessingOptions,
    },
    ExtractGpkgData {
        #[clap(short, long, default_value = "SverigepaketTP.gpkg")]
        sqlite_file: String,
        #[clap(short, long, default_value = "./out/gpkgData.json")]
        output: String,
        #[clap(short, long)]
        query: Option<String>,
    },
    Inspect {
        #[clap(long, default_value = "./out/graph.json")]
        input: String,
        #[clap(long, default_value = "./out/graph.svg")]
        output: String,
        #[clap(flatten)]
        options: InspectOptions,
    },
    /*
    Simulate {
        #[clap(long, default_value = "./out/graph.bin")]
        input: String,
        #[clap(long, default_value = "./out/data.csv")]
        output: String,
        #[clap(short, long, default_value = "./simulations/sim1.json")]
        setup: String,
        #[clap(flatten)]
        options: SimulationOptions,
    }, */
    AggregateSensorData {
        #[clap(flatten)]
        options: AggregateOptions,
    },
    TestPeriodDivision {
        #[clap(flatten)]
        options: TestPeriodDivisionOptions,
    },
    LiveRoute {
        #[clap(flatten)]
        options: modes::LiveRouteOptions,
    },
    FindGaps {
        #[clap(flatten)]
        options: modes::FindGapsOptions,
    },
    Custom {},
    Custom2 {},
    Custom3 {},
}

fn main() {
    let start = std::time::Instant::now();

    let args: Cli = Cli::parse();

    match args.commands {
        Commands::ParseRawData {
            raw_road_data,
            raw_sensor_data,
            road_data,
            sensor_data,
        } => {
            if let Some(raw_road_data) = raw_road_data {
                println!("{} Parsing raw road data", style("[1/3]").bold().dim());
                let raw = std::fs::read_to_string(&raw_road_data).unwrap();
                let bytes = raw.len();
                println!(
                    "{} Raw road data size: {}",
                    style("[2/3]").bold().dim(),
                    style(human_bytes(bytes as f64)).red()
                );
                let raw_road_data: Vec<parse::RawRoadData> = serde_json::from_str(&raw).unwrap();
                let data = parse_road_data(raw_road_data);
                std::fs::write(&road_data, serde_json::to_string(&data).unwrap()).unwrap();
                let bytes = std::fs::metadata(&road_data).unwrap().len();
                println!(
                    "{} Parsed road data size: {}",
                    style("[3/3]").bold().dim(),
                    style(human_bytes(bytes as f64)).green()
                );
            }

            if let Some(raw_sensor_data) = raw_sensor_data {
                println!("{} Parsing raw sensor data", style("[1/3]").bold().dim());
                let raw = std::fs::read_to_string(&raw_sensor_data).unwrap();
                let bytes = raw.len();
                println!(
                    "{} Raw sensor data size: {}",
                    style("[2/3]").bold().dim(),
                    style(human_bytes(bytes as f64)).red()
                );
                let raw_sensor_data: Vec<parse::RawSensorData> =
                    serde_json::from_str(&raw).unwrap();
                let data = parse_sensor_data(raw_sensor_data);
                std::fs::write(&sensor_data, serde_json::to_string(&data).unwrap()).unwrap();
                let bytes = std::fs::metadata(&sensor_data).unwrap().len();
                println!(
                    "{} Parsed sensor data size: {}",
                    style("[3/3]").bold().dim(),
                    style(human_bytes(bytes as f64)).green()
                );
            }
        }
        Commands::DrawRoad {
            input,
            output,
            unique_ids,
        } => {
            let graph = bitcode::deserialize(&std::fs::read(&input).unwrap()).unwrap();
            let canvas = modes::draw_roads(graph, unique_ids);
            canvas.save(&output);
        }
        Commands::ShortestPath {
            input,
            output,
            query_file,
            cull_to_path_distance,
            metric,
        } => {
            let desired_path =
                serde_json::from_str(&std::fs::read_to_string(&query_file).unwrap()).unwrap();
            let processed_graph: ProcessedGraph =
                serde_json::from_str(&std::fs::read_to_string(&input).unwrap()).unwrap();
            let canvas =
                modes::shortest_path(processed_graph, desired_path, cull_to_path_distance, metric);
            canvas.save(&output);
        }
        Commands::DrawDisjoint { input, output } => {
            println!("Reading graph from {}", input);
            let processed_graph: ProcessedGraph =
                serde_json::from_str(&std::fs::read_to_string(&input).unwrap()).unwrap();
            let canvas = modes::draw_disjoint(processed_graph.graph);
            canvas.save(&output);
        }
        Commands::DrawReachable {
            input,
            output,
            latitude,
            longitude,
            range,
            inverse,
        } => {
            let point = parse::Point {
                latitude,
                longitude,
            };
            let graph = bitcode::deserialize(&std::fs::read(&input).unwrap()).unwrap();
            let canvas = modes::draw_reachable(graph, point, range, inverse);
            canvas.save(&output);
        }
        Commands::DrawDistance {
            input,
            output,
            latitude,
            longitude,
            max_distance,
            metric,
            forward_only,
        } => {
            let graph = bitcode::deserialize(&std::fs::read(&input).unwrap()).unwrap();
            let query = PointQuery::new(latitude, longitude, max_distance, -180.0..180.0);
            let canvas = modes::draw_distance(graph, query, max_distance, metric, forward_only);
            canvas.save(&output);
        }
        Commands::Inspect {
            input,
            output,
            options,
        } => {
            let processed_graph: ProcessedGraph =
                serde_json::from_str(&std::fs::read_to_string(&input).unwrap()).unwrap();
            let canvas = modes::inspect(processed_graph.graph, options);
            canvas.save(&output);
        }
        /*
        Commands::Simulate {
            input,
            output,
            setup,
            options,
        } => {
            let mut graph = bitcode::deserialize(&std::fs::read(&input).unwrap()).unwrap();
            let setup: SimulationSetup =
                serde_json::from_str(&std::fs::read_to_string(&setup).unwrap()).unwrap();
            modes::simulate(&mut graph, setup, options, &output);
        } */
        Commands::Process {
            road_data,
            output,
            mongo_options,
            processing_options,
        } => {
            let runtime = Runtime::new().unwrap();

            runtime.block_on(async {
                let road_data = read_roads(&road_data);

                let client = AsyncMongoClient::new(mongo_options).await.unwrap();

                let sensor_data = client
                    .get_all_sensors()
                    .await
                    .expect("Failed to get sensor data");

                let graph = processing::process_graph(processing_options, road_data, sensor_data);
                let data = serde_json::to_string(&graph).unwrap();
                std::fs::write(output.clone(), data).unwrap();
                let size = std::fs::metadata(output.clone()).unwrap().len();
                println!("Graph size: {} bytes", human_bytes(size as f64));
                println!("Wrote graph to {}", output);

                //let data = bitcode::serialize(&graph.graph).unwrap();
                //let _: ProcessedGraph = bitcode::deserialize(&data).unwrap();
            });
        }
        Commands::ExtractGpkgData {
            sqlite_file,
            output,
            query,
        } => {
            let road_data = gpkg::read_database(&sqlite_file, query);
            std::fs::write(&output, serde_json::to_string(&road_data).unwrap()).unwrap();
            let bytes = std::fs::metadata(&output).unwrap().len();
            println!(
                "Wrote {} to {}",
                style(human_bytes(bytes as f64)).green(),
                output
            );
        }
        Commands::AggregateSensorData { options } => {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                modes::aggregate(options).await;
            });
        }
        Commands::TestPeriodDivision { options } => {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                test_period_division(options).await;
            });
        }
        Commands::LiveRoute { options } => {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                modes::live_route(options).await;
            });
        }
        Commands::FindGaps { options } => {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                modes::find_gaps(options).await;
            });
        }
        Commands::Custom {} => {
            let processed_graph: ProcessedGraph =
                serde_json::from_str(&std::fs::read_to_string("./out/graph.json").unwrap())
                    .unwrap();

            const COLORS: [&str; 4] = ["#FFF275", "#07BEB8", "#FF3C38", "#A4A8D1"];

            fn get_polyline_from_query(graph: &ProcessedGraph, path: &str) -> Vec<Point> {
                let query: Vec<PointQuery> =
                    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

                let graph = &graph.graph;

                let tree = build_node_acceleration_structure(graph);
                let points = query
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

                println!("Finding shortest path for points {:?}", points);
                let path = visitor::shortest_path(&graph, points, DistanceMetric::Space)
                    .expect("No path found");

                println!("Path complete: {:?}", path.complete);

                path.nodes
                    .windows(2)
                    .flat_map(|edge| {
                        let edge = graph.edges_connecting(edge[0], edge[1]).next().unwrap();
                        edge.weight().polyline.clone()
                    })
                    .collect::<Vec<_>>()
            }

            let extent = [59.293914, 59.370097, 17.974399, 18.138043];

            let mut canvas =
                Canvas::new_with_background(calc_canvas_size_from_extents(4000, extent), "#100e16");

            for edge in processed_graph.graph.edge_references() {
                let data = edge.weight();
                canvas.draw_polyline(
                    data.polyline.clone(),
                    DrawOptions {
                        color: "#433E45".into(),
                        stroke: 4.0,
                        ..Default::default()
                    },
                );
            }

            let polyline_query_1 =
                get_polyline_from_query(&processed_graph, "./queries/query1km.json");
            let polyline_query_2 =
                get_polyline_from_query(&processed_graph, "./queries/query2km.json");
            let polyline_query_3 =
                get_polyline_from_query(&processed_graph, "./queries/query4km.json");
            let polyline_query_4 =
                get_polyline_from_query(&processed_graph, "./queries/query8km.json");

            let polylines = vec![
                polyline_query_1,
                polyline_query_2,
                polyline_query_3,
                polyline_query_4,
            ];

            for (i, polyline) in polylines.iter().enumerate().rev() {
                println!("Drawing polyline {}", i);
                println!("Length: {}", polyline.len());
                canvas.draw_polyline(
                    polyline.clone(),
                    DrawOptions {
                        color: COLORS[i].into(),
                        stroke: 10.0,
                        ..Default::default()
                    },
                );
            }

            canvas.save("./out/graphpathsegmented.svg");
        }
        Commands::Custom2 {} => {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                let mongo = AsyncMongoClient::new(MongoOptions {
                    uri: "mongodb://localhost:27017".into(),
                    db: "exjobb".into(),
                    raw_sensor_data_collection: "trafikverketflowentries_v2".into(),
                    sensors_collection: "sensors".into(),
                    data_points_collection: "sensordata".into(),
                })
                .await
                .unwrap();

                let metadata = mongo.get_all_sensors().await.unwrap();

                // Min lat max lat min lon max lon
                let mut extent = [f64::MAX, f64::MIN, f64::MAX, f64::MIN];

                for sensor in metadata.iter() {
                    extent[0] = extent[0].min(sensor.location.coordinates[1]);
                    extent[1] = extent[1].max(sensor.location.coordinates[1]);
                    extent[2] = extent[2].min(sensor.location.coordinates[0]);
                    extent[3] = extent[3].max(sensor.location.coordinates[0]);
                }

                let processed_graph: ProcessedGraph =
                    serde_json::from_str(&std::fs::read_to_string("./out/graph.json").unwrap())
                        .unwrap();

                let canvas_size = calc_canvas_size_from_extents(4000, extent);
                let mut canvas_with_path = Canvas::new(canvas_size);
                let mut canvas_just_points = Canvas::new(canvas_size);

                println!("Drawing graph with {}", processed_graph.graph.edge_count());

                for edge in processed_graph.graph.edge_references() {
                    let data = edge.weight();
                    canvas_with_path.draw_polyline(
                        data.polyline.clone(),
                        DrawOptions {
                            color: "#433E45".into(),
                            stroke: 4.0,
                            ..Default::default()
                        },
                    );
                    canvas_just_points.draw_polyline(
                        data.polyline.clone(),
                        DrawOptions {
                            color: "#433E45".into(),
                            stroke: 4.0,
                            ..Default::default()
                        },
                    );
                }

                for sensor in metadata.iter() {
                    let point = Point {
                        latitude: sensor.location.coordinates[1],
                        longitude: sensor.location.coordinates[0],
                    };
                    canvas_with_path.draw_circle(point, "#ff0000", 5.0);
                    canvas_just_points.draw_circle(point, "#ff0000", 5.0);
                }

                let start = PointQuery::new(59.305007, 18.017391, 25.0, -90.0..90.0);
                let end = PointQuery::new(59.356922, 18.032265, 25.0, -45.0..45.0);

                let top_left = Point {
                    latitude: 59.370097,
                    longitude: 17.974399,
                };
                let bottom_right = Point {
                    latitude: 59.293914,
                    longitude: 18.138043,
                };

                fn in_box(point: &Point, top_left: &Point, bottom_right: &Point) -> bool {
                    point.latitude < top_left.latitude
                        && point.latitude > bottom_right.latitude
                        && point.longitude > top_left.longitude
                        && point.longitude < bottom_right.longitude
                }

                let mut graph = processed_graph.graph;
                let mut to_remove = Vec::new();
                for node in graph.node_indices() {
                    let data = graph.node_weight(node).unwrap();
                    if !in_box(&data.point, &top_left, &bottom_right) {
                        to_remove.push(node);
                    }
                }
                for node in to_remove {
                    graph.remove_node(node);
                }

                let tree = processing::build_node_acceleration_structure(&graph);
                let (_, (start_idx, _)) = tree
                    .iter_nearest(
                        &[start.point.latitude, start.point.longitude],
                        &math::geo_distance,
                    )
                    .unwrap()
                    .skip_while(|(_, (_, data))| !start.heading.contains(&data.heading))
                    .next()
                    .unwrap();
                let (_, (end_idx, _)) = tree
                    .iter_nearest(
                        &[end.point.latitude, end.point.longitude],
                        &math::geo_distance,
                    )
                    .unwrap()
                    .skip_while(|(_, (_, data))| !end.heading.contains(&data.heading))
                    .next()
                    .unwrap();
                let path = visitor::shortest_path(
                    &graph,
                    vec![*start_idx, *end_idx],
                    DistanceMetric::Space,
                )
                .unwrap();

                let polyline = path
                    .nodes
                    .windows(2)
                    .flat_map(|edge| {
                        let edge = graph.edges_connecting(edge[0], edge[1]).next().unwrap();
                        edge.weight().polyline.clone()
                    })
                    .collect::<Vec<_>>();
                canvas_with_path.draw_polyline(
                    polyline,
                    DrawOptions {
                        color: "#00ff00".into(),
                        stroke: 10.0,
                        ..Default::default()
                    },
                );

                println!(
                    "With path node count: {}",
                    canvas_with_path.get_node_count()
                );
                println!(
                    "Just points node count: {}",
                    canvas_just_points.get_node_count()
                );

                canvas_with_path.save("./out/allsensorswithpath.svg");
                canvas_just_points.save("./out/allsensors.svg");
            });
        }
        Commands::Custom3 {} => {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                let mongo = AsyncMongoClient::new(MongoOptions {
                    uri: "mongodb://localhost:27017".into(),
                    db: "exjobb".into(),
                    raw_sensor_data_collection: "trafikverketflowentries_v2".into(),
                    sensors_collection: "sensors".into(),
                    data_points_collection: "sensordata".into(),
                })
                .await
                .unwrap();

                let metadata = mongo.get_all_sensors().await.unwrap();

                // Min lat max lat min lon max lon
                let mut large_extent = [f64::MAX, f64::MIN, f64::MAX, f64::MIN];

                for sensor in metadata.iter() {
                    large_extent[0] = large_extent[0].min(sensor.location.coordinates[1]);
                    large_extent[1] = large_extent[1].max(sensor.location.coordinates[1]);
                    large_extent[2] = large_extent[2].min(sensor.location.coordinates[0]);
                    large_extent[3] = large_extent[3].max(sensor.location.coordinates[0]);
                }

                let small_extent = [59.319467, 59.329296, 18.058204, 18.080229];

                let processed_graph: ProcessedGraph =
                    serde_json::from_str(&std::fs::read_to_string("./out/graph.json").unwrap())
                        .unwrap();

                let canvas_size_large = calc_canvas_size_from_extents(4000, large_extent);
                let canvas_size_small = calc_canvas_size_from_extents(4000, small_extent);

                let mut canvas_large = Canvas::new_with_background(canvas_size_large, "#100e16");
                let mut canvas_small = Canvas::new_with_background(canvas_size_small, "#100e16");

                const COLORS: [&str; 25] = [
                    "#006400", "#808000", "#483d8b", "#b22222", "#008080", "#000080", "#9acd32",
                    "#8fbc8f", "#8b008b", "#ff0000", "#ff8c00", "#ffff00", "#00ff00", "#00fa9a",
                    "#8a2be2", "#00ffff", "#0000ff", "#ff00ff", "#1e90ff", "#db7093", "#f0e68c",
                    "#87ceeb", "#ff1493", "#ffa07a", "#ee82ee",
                ];

                let mut i = 0;
                for edge in processed_graph.graph.edge_references() {
                    let data = edge.weight();
                    let color = COLORS[i % COLORS.len()];
                    i += 1;

                    canvas_large.draw_polyline(
                        data.polyline.clone(),
                        DrawOptions {
                            color: color.into(),
                            stroke: 4.0,
                            ..Default::default()
                        },
                    );
                    canvas_small.draw_polyline(
                        data.polyline.clone(),
                        DrawOptions {
                            color: color.into(),
                            stroke: 4.0,
                            ..Default::default()
                        },
                    );
                }

                canvas_large.save("./out/disjoint_large.svg");
                canvas_small.save("./out/disjoint_small.svg");
            });
        }
    }

    println!("Runtime: {:?}", style(start.elapsed()).yellow().bold());
}
