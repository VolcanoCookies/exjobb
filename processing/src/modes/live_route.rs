use std::{
    fs,
    ops::Deref,
    time::{Instant, SystemTime},
};

use clap::Args;
use mongodb::bson::DateTime;

use crate::{
    math::geo_distance,
    mongo::{
        client::{async_client::AsyncMongoClient, MongoOptions},
        model::VehicleType,
    },
    processing::{build_node_acceleration_structure, ProcessedGraph},
    progress::Progress,
    travel_time::{self, DataPointFilter},
    util::PointQuery,
    visitor::{self, convert_ms_to_kmh},
};

#[derive(Debug, Clone)]
pub struct ParseableDate(i64);

impl std::str::FromStr for ParseableDate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let timestamp = if s == "now" {
            DateTime::now().timestamp_millis()
        } else if let Ok(timestamp) = s.parse::<i64>() {
            timestamp
        } else {
            DateTime::parse_rfc3339_str(s).unwrap().timestamp_millis()
        };

        Ok(ParseableDate(timestamp))
    }
}

impl Deref for ParseableDate {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct ParseableDuration(i64);

impl std::str::FromStr for ParseableDuration {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let duration = if let Ok(duration) = s.parse::<i64>() {
            duration
        } else {
            let first_char = s.chars().next().unwrap();
            let duration = if first_char == '-' {
                let duration = s[1..s.len() - 1].parse::<i64>().unwrap();
                -duration
            } else {
                s[..s.len() - 1].parse::<i64>().unwrap()
            };

            let last_char = s.chars().last().unwrap();
            match last_char {
                's' => duration * 1000,
                'm' => duration * 60 * 1000,
                'h' => duration * 3600 * 1000,
                'd' => duration * 24 * 3600 * 1000,
                _ => panic!("Invalid duration"),
            }
        };

        Ok(ParseableDuration(duration))
    }
}

impl Deref for ParseableDuration {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Args)]
pub struct LiveRouteOptions {
    #[clap(flatten)]
    pub mongo_options: MongoOptions,
    #[clap(short, long)]
    pub query: String,
    #[clap(short, long, default_value = "now")]
    pub start_date: ParseableDate,
    #[clap(short, long, default_value = "0s")]
    pub date_offset: ParseableDuration,
    #[clap(short, long)]
    pub step_size: ParseableDuration,
    #[clap(short, long)]
    pub max_steps: i64,
    #[clap(short, long, default_value = "./out/graph.json")]
    pub graph_path: String,
    #[clap(short, long)]
    pub max_sensor_data_age: ParseableDuration,
    #[clap(short, long, default_value = "./out/live_route.csv")]
    pub output: String,
    #[clap(short, long, default_value = "anyVehicle")]
    pub vehicle_type: VehicleType,
}

pub async fn live_route(options: LiveRouteOptions) {
    let mut progress = Progress::new();

    progress.step_unsized("Connecting to MongoDB");
    let client = AsyncMongoClient::new(options.mongo_options.clone())
        .await
        .expect("Failed to connect to MongoDB");
    progress.finish("");

    progress.step_unsized("Reading graph");
    let ProcessedGraph {
        graph,
        sensor_store,
    } = serde_json::from_str(fs::read_to_string(&options.graph_path).unwrap().as_str()).unwrap();
    progress.finish(format!(
        "Loaded graph with {} nodes and {} edges",
        graph.node_count(),
        graph.edge_count()
    ));

    progress.step_unsized("Reading query");
    let query: Vec<PointQuery> =
        serde_json::from_str(fs::read_to_string(&options.query).unwrap().as_str()).unwrap();
    progress.finish(format!("Loaded query: {:?}", query));

    progress.step_sized(query.len(), "Finding shortest path");
    let tree = build_node_acceleration_structure(&graph);
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

            progress.tick();
            panic!("No node found for query {:?}", query);
        })
        .collect::<Vec<_>>();
    progress.finish("Found nodes");

    progress.step_unsized("Finding shortest path");
    let path = visitor::shortest_path(&graph, points, visitor::DistanceMetric::Time)
        .expect("No path found");
    let distance = path.nodes.windows(2).fold(0.0, |acc, nodes| {
        let edge = graph.edges_connecting(nodes[0], nodes[1]).next().unwrap();
        acc + edge.weight().distance
    });
    let average_speed = distance / path.length;
    progress.finish(format!(
        "Shortest path time: {}s, distance: {}, average speed: {:.1}km/h",
        path.length,
        distance,
        convert_ms_to_kmh(average_speed)
    ));

    let processed_graph = ProcessedGraph {
        graph,
        sensor_store,
    };

    progress.step_sized(options.max_steps as usize, "Simulating route");
    let mut data = Vec::new();
    for i in 0..options.max_steps {
        let current_time = *options.start_date + i * *options.step_size;

        let live_travel_time = travel_time::calculate_live_travel_time(
            &processed_graph,
            &path,
            &client,
            DataPointFilter {
                timestamp: Some(current_time),
                max_age: Some(*options.max_sensor_data_age),
            },
            Some(options.vehicle_type),
        )
        .await;

        let date = DateTime::from_millis(current_time + *options.date_offset);
        let date = date.try_to_rfc3339_string().unwrap();
        let date = date.replace("T", " ").replace("Z", "");
        data.push((date, live_travel_time));

        progress.tick();
    }
    progress.finish("Simulation finished");

    progress.step_unsized("Writing output");
    let mut writer = csv::Writer::from_path(&options.output).unwrap();
    writer.write_record(&["time", "travelTimeSensors"]).unwrap();
    for (time, travel_time) in data {
        let _ = writer.write_record(&[time.to_string(), travel_time.to_string()]);
    }
    writer.flush().unwrap();
    progress.finish("Output written");
}
