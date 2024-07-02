use clap::ValueEnum;
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Copy, ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum VehicleType {
    AgriculturalVehicle,
    AnyVehicle,
    ArticulatedVehicle,
    Bicycle,
    Bus,
    Car,
    Caravan,
    CarOrLightVehicle,
    CarWithCaravan,
    CarWithTrailer,
    ConstructionOrMaintenanceVehicle,
    FourWheelDrive,
    HighSidedVehicle,
    Lorry,
    Moped,
    Motorcycle,
    MotorcycleWithSideCar,
    Motorscooter,
    Tanker,
    ThreeWheeledVehicle,
    Trailer,
    Tram,
    TwoWheeledVehicle,
    Van,
    VehicleWithCatalyticConverter,
    VehicleWithoutCatalyticConverter,
    VehicleWithCaravan,
    VehicleWithTrailer,
    WithEvenNumberedRegistrationPlates,
    WithOddNumberedRegistrationPlates,
    Other,
}

impl Into<Bson> for VehicleType {
    fn into(self) -> Bson {
        match self {
            VehicleType::AgriculturalVehicle => Bson::String("agriculturalVehicle".to_string()),
            VehicleType::AnyVehicle => Bson::String("anyVehicle".to_string()),
            VehicleType::ArticulatedVehicle => Bson::String("articulatedVehicle".to_string()),
            VehicleType::Bicycle => Bson::String("bicycle".to_string()),
            VehicleType::Bus => Bson::String("bus".to_string()),
            VehicleType::Car => Bson::String("car".to_string()),
            VehicleType::Caravan => Bson::String("caravan".to_string()),
            VehicleType::CarOrLightVehicle => Bson::String("carOrLightVehicle".to_string()),
            VehicleType::CarWithCaravan => Bson::String("carWithCaravan".to_string()),
            VehicleType::CarWithTrailer => Bson::String("carWithTrailer".to_string()),
            VehicleType::ConstructionOrMaintenanceVehicle => {
                Bson::String("constructionOrMaintenanceVehicle".to_string())
            }
            VehicleType::FourWheelDrive => Bson::String("fourWheelDrive".to_string()),
            VehicleType::HighSidedVehicle => Bson::String("highSidedVehicle".to_string()),
            VehicleType::Lorry => Bson::String("lorry".to_string()),
            VehicleType::Moped => Bson::String("moped".to_string()),
            VehicleType::Motorcycle => Bson::String("motorcycle".to_string()),
            VehicleType::MotorcycleWithSideCar => Bson::String("motorcycleWithSideCar".to_string()),
            VehicleType::Motorscooter => Bson::String("motorscooter".to_string()),
            VehicleType::Tanker => Bson::String("tanker".to_string()),
            VehicleType::ThreeWheeledVehicle => Bson::String("threeWheeledVehicle".to_string()),
            VehicleType::Trailer => Bson::String("trailer".to_string()),
            VehicleType::Tram => Bson::String("tram".to_string()),
            VehicleType::TwoWheeledVehicle => Bson::String("twoWheeledVehicle".to_string()),
            VehicleType::Van => Bson::String("van".to_string()),
            VehicleType::VehicleWithCatalyticConverter => {
                Bson::String("vehicleWithCatalyticConverter".to_string())
            }
            VehicleType::VehicleWithoutCatalyticConverter => {
                Bson::String("vehicleWithoutCatalyticConverter".to_string())
            }
            VehicleType::VehicleWithCaravan => Bson::String("vehicleWithCaravan".to_string()),
            VehicleType::VehicleWithTrailer => Bson::String("vehicleWithTrailer".to_string()),
            VehicleType::WithEvenNumberedRegistrationPlates => {
                Bson::String("withEvenNumberedRegistrationPlates".to_string())
            }
            VehicleType::WithOddNumberedRegistrationPlates => {
                Bson::String("withOddNumberedRegistrationPlates".to_string())
            }
            VehicleType::Other => Bson::String("other".to_string()),
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
    pub vehicle_type: VehicleType,
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
            "VehicleType": self.vehicle_type,
            "SpecificLane": self.get_lane_i32(),
            "MeasurementSide": self.get_measurement_side(),
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
    pub vehicle_type: VehicleType,
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
            vehicle_type: data.vehicle_type,
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
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub mongo_id: Option<ObjectId>,
    pub original_id: ObjectId,
    pub sensor_id: ObjectId,
    pub time: DateTime,
    pub flow_rate: f64,
    pub average_speed: f64,
}

impl From<RawSensorData> for DataPoint {
    fn from(data: RawSensorData) -> Self {
        Self {
            mongo_id: None,
            original_id: data.mongo_id.unwrap(),
            sensor_id: data.mongo_id.unwrap(),
            time: data.measurement_time,
            flow_rate: data.flow_rate,
            average_speed: data.average_speed,
        }
    }
}
