mod custom_bfs;
mod math;
mod modes;
mod output;
mod parse;
mod processing;
mod visitor;

use std::ops::Range;

use clap::{Parser, Subcommand, ValueEnum};
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
    #[clap(short, long, default_value = "../roadData.json")]
    road_data: String,
    #[clap(short, long, default_value = "../sensorData.json")]
    sensor_data: String,
    #[clap(short, long, default_value = "output.svg")]
    output: String,
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    DrawRoad {
        #[clap(short, long)]
        unique_ids: Vec<i32>,
    },
    ShortestPath {
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
        #[clap(short, long, default_value = "false", default_missing_value = "true")]
        print_path_roads: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    ShortestPath,
    DrawRoad,
}

fn main() {
    let args: Cli = Cli::parse();

    let road_data = read_roads(&args.road_data);
    let sensor_data = read_sensors(&args.sensor_data);

    println!("Number of roads: {}", road_data.len());
    println!("Number of sensors: {}", sensor_data.len());

    match args.commands {
        Commands::DrawRoad { unique_ids } => {
            let canvas = modes::draw_roads(road_data.clone(), unique_ids);
            canvas.save(&args.output);
        }
        Commands::ShortestPath {
            dedup_road_data,
            max_distance,
            merge_overlapping_distance,
            collapse_nodes,
            remove_disjoint_nodes,
            dedup_edges,
            print_path_roads,
        } => {
            let opts = processing::GraphProcessingOptions {
                dedup_road_data,
                max_distance_from_sensors: max_distance.unwrap_or(f32::INFINITY),
                merge_overlap_distance: merge_overlapping_distance.unwrap_or(f32::NAN),
                collapse_nodes,
                remove_disjoint_nodes,
                dedup_edges,
            };

            let document = modes::shortest_path(
                road_data.clone(),
                sensor_data.clone(),
                opts,
                print_path_roads,
            );
            svg::save(&args.output, &document).unwrap();
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
