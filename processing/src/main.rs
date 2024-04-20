mod custom_bfs;
mod math;
mod modes;
mod output;
mod parse;
mod processing;
mod visitor;

use std::ops::Range;

use clap::{Parser, Subcommand, ValueEnum};
use human_bytes::human_bytes;
use petgraph::visit::Visitable;
use processing::NodeCollapse;

use crate::parse::{read_roads, read_sensors};

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
    DrawRoad {
        #[clap(long, default_value = "graph.bin")]
        input: String,
        #[clap(long, default_value = "graph.svg")]
        output: String,
        #[clap(short, long)]
        unique_ids: Vec<i32>,
    },
    ShortestPath {
        #[clap(long, default_value = "graph.bin")]
        input: String,
        #[clap(long, default_value = "graph.svg")]
        output: String,
        #[clap(short, long, default_value = "false", default_missing_value = "true")]
        print_path_roads: bool,
    },
    DrawDisjoint {
        #[clap(long, default_value = "graph.bin")]
        input: String,
        #[clap(long, default_value = "graph.svg")]
        output: String,
    },
    DrawReachable {
        #[clap(long, default_value = "graph.bin")]
        input: String,
        #[clap(long, default_value = "graph.svg")]
        output: String,
        #[clap(short = 'a', long = "lat")]
        latitude: f32,
        #[clap(short = 'o', long = "lon")]
        longitude: f32,
        #[clap(short, long)]
        range: f32,
        #[clap(short, long, default_value = "false", default_missing_value = "true")]
        inverse: bool,
    },
    Process {
        #[clap(long, default_value = "../roadData.json")]
        road_data: String,
        #[clap(long, default_value = "../sensorData.json")]
        sensor_data: String,
        #[clap(long, default_value = "graph.bin")]
        output: String,
        #[clap(short, long, default_value = "false", default_missing_value = "true")]
        dedup_road_data: bool,
        #[clap(short, long)]
        max_distance: Option<f32>,
        #[clap(short = 'M', long, default_missing_value = "0")]
        merge_overlapping_distance: Option<f32>,
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
        #[clap(short = 'v', long, default_value = "20.0")]
        connect_distance: f32,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    ShortestPath,
    DrawRoad,
}

fn main() {
    let args: Cli = Cli::parse();

    match args.commands {
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
            print_path_roads,
        } => {
            let graph = bitcode::deserialize(&std::fs::read(&input).unwrap()).unwrap();
            let canvas = modes::shortest_path(graph, print_path_roads);
            svg::save(&output, &canvas).unwrap();
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
                max_distance_from_sensors: max_distance.unwrap_or(f32::INFINITY),
                merge_overlap_distance: merge_overlapping_distance.unwrap_or(f32::NAN),
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
    }
}

#[derive(Debug, Clone)]
pub struct PointQuery {
    pub point: parse::Point,
    pub radius: f32,
    pub heading: Range<f32>,
}

impl PointQuery {
    pub fn new(latitude: f32, longitude: f32, radius: f32, heading: Range<f32>) -> Self {
        PointQuery {
            point: parse::Point {
                latitude,
                longitude,
            },
            radius,
            heading,
        }
    }
}
