use clap::Args;
use longitude::Location;
use serde::{Deserialize, Serialize};

use crate::output::CanvasSize;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Args)]
#[group(required = true, multiple = true)]
pub struct Point {
    #[clap(short = 'a', long = "lat")]
    pub latitude: f64,
    #[clap(short = 'o', long = "lon")]
    pub longitude: f64,
}

impl Point {
    pub fn within(&self, canvas_size: &CanvasSize) -> bool {
        if self.latitude < canvas_size.min_lat || self.latitude > canvas_size.max_lat {
            return false;
        }
        if self.longitude < canvas_size.min_lon || self.longitude > canvas_size.max_lon {
            return false;
        }
        true
    }
}

impl Into<Location> for Point {
    fn into(self) -> Location {
        Location::from(self.latitude, self.longitude)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoadDirection {
    Forward,
    Backward,
    Both,
    None,
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
pub(crate) struct RawSensorData {
    pub site_id: i32,
    pub vehicle_flow_rate: f32,
    pub average_vehicle_speed: f32,
    pub geometry: SensorGeometry,
    pub specific_lane: String,
    pub measurement_side: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct SensorGeometry {
    pub point: Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SensorData {
    pub site_id: i32,
    pub flow_rate: f64,
    pub average_speed: f64,
    pub point: Point,
    pub lane: i32,
    pub side: Direction,
}

pub fn parse_sensor_data(raw: Vec<RawSensorData>) -> Vec<SensorData> {
    raw.into_iter()
        .map(|raw| SensorData {
            site_id: raw.site_id,
            flow_rate: raw.vehicle_flow_rate as f64,
            average_speed: raw.average_vehicle_speed as f64,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawRoadData {
    pub _county: i32,
    pub _deleted: bool,
    pub direction: RawRoadDirection,
    pub geometry: RoadGeometry,
    pub length: f32,
    pub _modified_time: String,
    pub road_main_number: i32,
    pub road_sub_number: i32,
    pub _time_stamp: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawRoadDirection {
    pub _code: i32,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RoadGeometry {
    pub coordinates: Vec<Point>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadData {
    pub direction: RoadDirection,
    pub main_number: i32,
    pub sub_number: i32,
    pub coordinates: Vec<Point>,
    pub length: f64,
    pub unique_id: i32,
    pub speed_limit: f64,
}

pub fn parse_road_data(raw: Vec<RawRoadData>) -> Vec<RoadData> {
    raw.into_iter()
        .enumerate()
        .map(|(unique_id, raw)| RoadData {
            direction: raw.direction.value.as_str().into(),
            main_number: raw.road_main_number,
            sub_number: raw.road_sub_number,
            coordinates: raw.geometry.coordinates,
            length: raw.length as f64,
            unique_id: unique_id as i32,
            speed_limit: 0.0,
        })
        .collect()
}

pub fn read_roads(path: &str) -> Vec<RoadData> {
    let raw = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&raw).unwrap()
}
