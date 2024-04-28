mod custom_bfs;
mod gpkg;
mod math;
mod modes;
mod output;
mod parse;
mod processing;
mod progress;
mod util;
mod visitor;

use clap::{Parser, Subcommand};
use console::style;
use human_bytes::human_bytes;
use modes::InspectOptions;
use parse::{parse_road_data, parse_sensor_data};
use processing::NodeCollapse;
use visitor::DistanceMetric;

use crate::{
    modes::{SimulationOptions, SimulationSetup},
    parse::{read_roads, read_sensors},
    util::PointQuery,
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
        #[clap(long, default_value = "./out/graph.bin")]
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
        #[clap(long, default_value = "../roadData.json")]
        road_data: String,
        #[clap(long, default_value = "../sensorData.json")]
        sensor_data: String,
        #[clap(long, default_value = "./out/graph.bin")]
        output: String,
        #[clap(short, long, default_value = "false", default_missing_value = "true")]
        dedup_road_data: bool,
        #[clap(short, long)]
        max_distance: Option<f64>,
        #[clap(short = 'M', long, default_missing_value = "0")]
        merge_overlapping_distance: Option<f64>,
        #[clap(short, long, default_value = "none")]
        collapse_nodes: NodeCollapse,
        #[clap(short, long, default_value = "false", default_missing_value = "true")]
        remove_disjoint_nodes: bool,
        #[clap(
            short = 'D',
            long,
            default_value = "false",
            default_missing_value = "true"
        )]
        dedup_edges: bool,
        #[clap(
            short = 'v',
            long,
            default_value = "-1",
            default_missing_value = "20.0"
        )]
        connect_distance: f64,
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
    Simulate {
        #[clap(long, default_value = "./out/graph.bin")]
        input: String,
        #[clap(long, default_value = "./out/data.csv")]
        output: String,
        #[clap(short, long, default_value = "./simulations/sim1.json")]
        setup: String,
        #[clap(flatten)]
        options: SimulationOptions,
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
            let graph = bitcode::deserialize(&std::fs::read(&input).unwrap()).unwrap();
            let canvas = modes::draw_disjoint(graph);
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
        }
        Commands::Process {
            road_data,
            sensor_data,
            output,
            dedup_road_data,
            max_distance,
            merge_overlapping_distance,
            collapse_nodes,
            remove_disjoint_nodes,
            dedup_edges,
            connect_distance,
        } => {
            let opts = processing::GraphProcessingOptions {
                dedup_road_data,
                max_distance_from_sensors: max_distance.unwrap_or(f64::INFINITY),
                merge_overlap_distance: merge_overlapping_distance.unwrap_or(f64::NAN),
                collapse_nodes,
                remove_disjoint_nodes,
                dedup_edges,
                connect_distance,
            };

            let road_data = read_roads(&road_data);
            let sensor_data = read_sensors(&sensor_data);

            let graph = processing::parse_data(road_data, sensor_data, opts);
            let data = bitcode::serialize(&graph).unwrap();
            std::fs::write(output.clone(), data).unwrap();
            let size = std::fs::metadata(output).unwrap().len();
            println!("Graph size: {} bytes", human_bytes(size as f64));
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
    }

    println!("Runtime: {:?}", style(start.elapsed()).yellow().bold());
}
