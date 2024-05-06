use mongodb::bson::{doc, oid::ObjectId, Bson, DateTime, Document};
use serde::{Deserialize, Serialize};

use crate::custom_bfs::Positionable;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub enum MeasurementSide {
    Unknown,
    NorthBound,
    SouthBound,
    EastBound,
    WestBound,
    NorthWestBound,
    NorthEastBound,
    SouthWestBound,
    SouthEastBound,
}

impl Into<Bson> for MeasurementSide {
    fn into(self) -> Bson {
        match self {
            MeasurementSide::Unknown => Bson::String("Unknown".to_string()),
            MeasurementSide::NorthBound => Bson::String("NorthBound".to_string()),
            MeasurementSide::SouthBound => Bson::String("SouthBound".to_string()),
            MeasurementSide::EastBound => Bson::String("EastBound".to_string()),
            MeasurementSide::WestBound => Bson::String("WestBound".to_string()),
            MeasurementSide::NorthWestBound => Bson::String("NorthWestBound".to_string()),
            MeasurementSide::NorthEastBound => Bson::String("NorthEastBound".to_string()),
            MeasurementSide::SouthWestBound => Bson::String("SouthWestBound".to_string()),
            MeasurementSide::SouthEastBound => Bson::String("SouthEastBound".to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct RawSensorData {
    #[serde(rename = "_id")]
    pub mongo_id: Option<ObjectId>,
    pub site_id: i32,
    pub measurement_time: DateTime,
    #[serde(rename = "MeasurementOrCalculationPeriod")]
    pub period: i32,
    pub vehicle_type: String,
    #[serde(rename = "VehicleFlowRate")]
    pub flow_rate: f64,
    #[serde(rename = "AverageVehicleSpeed")]
    pub average_speed: f64,
    pub modified_time: DateTime,
    pub specific_lane: String,
    pub measurement_side: String,
    #[serde(rename = "location")]
    pub location: Location,
}

impl Positionable for RawSensorData {
    fn point(&self) -> crate::parse::Point {
        crate::parse::Point {
            latitude: self.location.coordinates[0],
            longitude: self.location.coordinates[1],
        }
    }
}

impl RawSensorData {
    pub fn filter(&self) -> Document {
        doc! {
            "SiteId": self.site_id,
        }
    }

    pub fn get_lane_i32(&self) -> i32 {
        self.specific_lane[4..].chars().as_str().parse().unwrap()
    }

    pub fn get_measurement_side(&self) -> MeasurementSide {
        match self.measurement_side.as_str() {
            "unknown" => MeasurementSide::Unknown,
            "northBound" => MeasurementSide::NorthBound,
            "southBound" => MeasurementSide::SouthBound,
            "eastBound" => MeasurementSide::EastBound,
            "westBound" => MeasurementSide::WestBound,
            "northWestBound" => MeasurementSide::NorthWestBound,
            "northEastBound" => MeasurementSide::NorthEastBound,
            "southWestBound" => MeasurementSide::SouthWestBound,
            "southEastBound" => MeasurementSide::SouthEastBound,
            _ => MeasurementSide::Unknown,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Location {
    #[serde(rename = "type")]
    pub _type: String,
    pub coordinates: [f64; 2],
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SensorMetadata {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub mongo_id: Option<ObjectId>,
    pub site_id: i32,
    pub location: Location,
    pub measurement_side: MeasurementSide,
    pub specific_lane: i32,
    pub period: i32,
}

impl From<RawSensorData> for SensorMetadata {
    fn from(data: RawSensorData) -> Self {
        let measurement_side = data.get_measurement_side();
        let lane = data.get_lane_i32();
        Self {
            mongo_id: None,
            site_id: data.site_id,
            location: data.location,
            measurement_side,
            specific_lane: lane,
            period: data.period,
        }
    }
}

impl Positionable for SensorMetadata {
    fn point(&self) -> crate::parse::Point {
        crate::parse::Point {
            latitude: self.location.coordinates[1],
            longitude: self.location.coordinates[0],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DataPoint {
    pub mongo_id: ObjectId,
    pub sensor_id: ObjectId,
    pub time: DateTime,
    pub flow_rate: f64,
    pub average_speed: f64,
}

impl From<RawSensorData> for DataPoint {
    fn from(data: RawSensorData) -> Self {
        Self {
            mongo_id: data.mongo_id.unwrap(),
            sensor_id: data.mongo_id.unwrap(),
            time: data.measurement_time,
            flow_rate: data.flow_rate,
            average_speed: data.average_speed,
        }
    }
}
