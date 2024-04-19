use std::io::Read;

use longitude::Location;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Point {
    pub latitude: f32,
    pub longitude: f32,
}

impl Into<Location> for Point {
    fn into(self) -> Location {
        Location::from(self.latitude as f64, self.longitude as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoadDirection {
    Forward,
    Backward,
    Both,
}
impl From<&str> for RoadDirection {
    fn from(s: &str) -> Self {
        match s {
            "Med" => RoadDirection::Forward,
            "Mot" => RoadDirection::Backward,
            "southEastBound" => RoadDirection::Backward,
            "southBound" => RoadDirection::Backward,
            "eastBound" => RoadDirection::Backward,
            "northWestBound" => RoadDirection::Forward,
            "northBound" => RoadDirection::Forward,
            "westBound" => RoadDirection::Forward,
            "unknown" => RoadDirection::Both,
            _ => panic!("Invalid direction string: {}", s),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
    Unknown,
}

impl Direction {
    pub fn from_points(a: Point, b: Point) -> Self {
        let dx = b.longitude - a.longitude;
        let dy = b.latitude - a.latitude;

        if dx > 0.0 && dy > 0.0 {
            Direction::NorthEast
        } else if dx > 0.0 && dy < 0.0 {
            Direction::SouthEast
        } else if dx < 0.0 && dy > 0.0 {
            Direction::NorthWest
        } else if dx < 0.0 && dy < 0.0 {
            Direction::SouthWest
        } else if dx > 0.0 {
            Direction::East
        } else if dx < 0.0 {
            Direction::West
        } else if dy > 0.0 {
            Direction::North
        } else {
            Direction::South
        }
    }
}

impl From<&str> for Direction {
    fn from(s: &str) -> Self {
        match s {
            "northBound" => Direction::North,
            "southBound" => Direction::South,
            "eastBound" => Direction::East,
            "westBound" => Direction::West,
            "northEastBound" => Direction::NorthEast,
            "northWestBound" => Direction::NorthWest,
            "southEastBound" => Direction::SouthEast,
            "southWestBound" => Direction::SouthWest,
            "unknown" => Direction::Unknown,
            _ => panic!("Invalid direction string: {}", s),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SensorDataRaw {
    pub site_id: i32,
    pub vehicle_flow_rate: f32,
    pub average_vehicle_speed: f32,
    pub geometry: SensorGeometry,
    pub specific_lane: String,
    pub measurement_side: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SensorGeometry {
    pub point: Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SensorData {
    pub site_id: i32,
    pub flow_rate: f32,
    pub average_speed: f32,
    pub point: Point,
    pub lane: i32,
    pub side: Direction,
}

fn parse_sensor_data(raw: Vec<SensorDataRaw>) -> Vec<SensorData> {
    raw.into_iter()
        .map(|raw| SensorData {
            site_id: raw.site_id,
            flow_rate: raw.vehicle_flow_rate,
            average_speed: raw.average_vehicle_speed,
            point: raw.geometry.point,
            lane: parse_lane(raw.specific_lane.as_str()),
            side: raw.measurement_side.as_str().into(),
        })
        .collect()
}

fn parse_lane(lane: &str) -> i32 {
    let lane = lane.trim_start_matches("lane");
    lane.parse().unwrap()
}

pub fn read_sensors(path: &str) -> Vec<SensorData> {
    let mut reader = std::fs::File::open(path).unwrap();
    let mut raw = String::new();
    reader.read_to_string(&mut raw).unwrap();
    let sensor_data_raw = serde_json::from_str::<Vec<SensorDataRaw>>(&raw).unwrap();

    parse_sensor_data(sensor_data_raw)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawRoadData {
    pub county: i32,
    pub deleted: bool,
    pub direction: RawRoadDirection,
    pub geometry: RoadGeometry,
    pub length: f32,
    pub modified_time: String,
    pub road_main_number: i32,
    pub road_sub_number: i32,
    pub time_stamp: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawRoadDirection {
    pub code: i32,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RoadGeometry {
    pub coordinates: Vec<Point>,
}

#[derive(Debug, Clone)]
pub struct RoadData {
    pub direction: RoadDirection,
    pub main_number: i32,
    pub sub_number: i32,
    pub coordinates: Vec<Point>,
    pub length: f32,
    pub unique_id: i32,
}

fn parse_road_data(raw: Vec<RawRoadData>) -> Vec<RoadData> {
    raw.into_iter()
        .enumerate()
        .map(|(unique_id, raw)| RoadData {
            direction: raw.direction.value.as_str().into(),
            main_number: raw.road_main_number,
            sub_number: raw.road_sub_number,
            coordinates: raw.geometry.coordinates,
            length: raw.length,
            unique_id: unique_id as i32,
        })
        .collect()
}

pub fn read_roads(path: &str) -> Vec<RoadData> {
    let mut reader = std::fs::File::open(path).unwrap();
    let mut raw = String::new();
    reader.read_to_string(&mut raw).unwrap();
    let road_data_raw = serde_json::from_str::<Vec<RawRoadData>>(&raw).unwrap();

    parse_road_data(road_data_raw)
}
