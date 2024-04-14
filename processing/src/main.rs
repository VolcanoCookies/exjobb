mod graph;
mod math;
mod output;
mod parse;

use crate::parse::{read_roads, read_sensors};

fn main() {
    let road_data = read_roads("../roadData.json");
    let sensor_data = read_sensors("../sensorData.json");

    println!("Number of roads: {}", road_data.len());
    println!("Number of sensors: {}", sensor_data.len());

    let graph = graph::parse_data(road_data, sensor_data);

    let document = output::render(2000, &graph);
    svg::save("output.svg", &document).unwrap();
}
