use mongodb::bson::DateTime;

use crate::{
    mongo::{client::async_client::AsyncMongoClient, model::VehicleType},
    processing::ProcessedGraph,
    visitor::{convert_kmh_to_ms, Path},
};

pub struct DataPointFilter {
    pub timestamp: Option<i64>,
    pub max_age: Option<i64>,
}

impl Default for DataPointFilter {
    fn default() -> Self {
        Self {
            timestamp: None,
            max_age: None,
        }
    }
}

pub async fn calculate_live_travel_time(
    graph: &ProcessedGraph,
    path: &Path,
    mongo: &AsyncMongoClient,
    filter: DataPointFilter,
    vehicle_type: Option<VehicleType>,
) -> f64 {
    let ProcessedGraph {
        graph,
        sensor_store,
    } = graph;

    let mut passed_sensors = Vec::new();
    for node in &path.nodes {
        if let Some(sensor) = sensor_store.get(node) {
            if let Some(vehicle_type) = vehicle_type {
                passed_sensors.extend(sensor.iter().filter(|s| s.vehicle_type == vehicle_type));
            } else {
                passed_sensors.extend(sensor.iter());
            }
        }
    }

    let timestamp = filter
        .timestamp
        .unwrap_or_else(|| DateTime::now().timestamp_millis());
    let max_age = filter.max_age.unwrap_or(timestamp);

    let data = mongo
        .get_sensor_data_at(passed_sensors.into_iter(), timestamp, max_age)
        .await
        .expect("Failed to get sensor data");

    let mut distance = 0.0;
    let mut measurements_distance = Vec::new();

    let mut prev_node = None;

    for node in &path.nodes {
        let edge_length = if let Some(prev_node) = prev_node {
            let edge = graph.edges_connecting(prev_node, *node).next().unwrap();
            edge.weight().distance
        } else {
            0.0
        };

        distance += edge_length;

        let node_data = graph.node_weight(*node).unwrap();
        if node_data.has_sensor {
            let sensors = sensor_store.get(node).unwrap();
            let site_ids = sensors.iter().map(|s| s.site_id).collect::<Vec<_>>();
            let (sum, count) = site_ids
                .iter()
                .filter_map(|id| data.get(id))
                .map(|d| d.average_speed)
                .fold((0.0, 0), |(sum, count), speed| (sum + speed, count + 1));
            let average_speed = sum / count as f64;

            if count > 0 {
                measurements_distance.push((distance, average_speed));
            }
        }

        prev_node = Some(*node);
    }

    if measurements_distance.is_empty() {
        println!("No sensor data found for path");
        println!("At timestamp: {}", DateTime::from_millis(timestamp));
        println!("Max age: {}", DateTime::from_millis(timestamp - max_age));
    }

    let mut iter = measurements_distance.iter();
    let mut prev = iter.next().unwrap();
    // Calculate the travel time from the start of the path to the first sensor
    let mut travel_time = prev.0 / convert_kmh_to_ms(prev.1);

    // Calculate the travel time between sensors
    for next in iter {
        let (prev_distance, prev_speed) = prev;
        let (next_distance, next_speed) = next;
        let distance = next_distance - prev_distance;
        let time = 2.0 * distance / convert_kmh_to_ms(prev_speed + next_speed);
        travel_time += time;
        prev = next;
    }

    // Calculate the travel time from the last sensor to the end of the path
    let (prev_distance, prev_speed) = prev;
    let distance = distance - prev_distance;
    travel_time += distance / convert_kmh_to_ms(*prev_speed);

    travel_time
}
