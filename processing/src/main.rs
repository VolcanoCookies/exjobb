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
use modes::{AggregateOptions, InspectOptions, TestPeriodDivisionOptions};
use mongo::client::MongoOptions;
use parse::{parse_road_data, parse_sensor_data};
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
        #[clap(long, default_value = "./out/graph.bin")]
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
        #[clap(long, default_value = "./out/graph.bin")]
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
            let graph = bitcode::deserialize(&std::fs::read(&input).unwrap()).unwrap();
            let canvas = modes::shortest_path(graph, desired_path, cull_to_path_distance, metric);
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
            let graph = bitcode::deserialize(&std::fs::read(&input).unwrap()).unwrap();
            let canvas = modes::inspect(graph, options);
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
    }

    println!("Runtime: {:?}", style(start.elapsed()).yellow().bold());
}
