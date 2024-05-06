pub mod async_client;

use clap::Args;
use mongodb::Collection;

use super::model::{DataPoint, RawSensorData, SensorMetadata};

pub struct Collections {
    pub raw_sensor_data: Collection<RawSensorData>,
    pub sensors: Collection<SensorMetadata>,
    pub data_points: Collection<DataPoint>,
}

#[derive(Debug, Args, Clone)]
pub struct MongoOptions {
    #[clap(long, default_value = "mongodb://localhost:27017")]
    pub uri: String,
    #[clap(long, default_value = "exjobb")]
    pub db: String,
    #[clap(long, default_value = "trafikverketflowentries_v2")]
    pub raw_sensor_data_collection: String,
    #[clap(long, default_value = "sensors")]
    pub sensors_collection: String,
    #[clap(long, default_value = "sensordata")]
    pub data_points_collection: String,
}
