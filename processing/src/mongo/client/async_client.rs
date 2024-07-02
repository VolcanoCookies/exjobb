use std::collections::HashMap;

use mongodb::{
    bson::{doc, DateTime},
    options::FindOneOptions,
};

use crate::mongo::model::{DataPoint, SensorMetadata};

use super::{Collections, MongoOptions};

pub struct AsyncMongoClient {
    collections: Collections,
}

impl AsyncMongoClient {
    pub async fn new(options: MongoOptions) -> Result<Self, mongodb::error::Error> {
        let client = mongodb::Client::with_uri_str(&options.uri).await?;
        let db = client.database(&options.db);

        let raw_sensor_data = db.collection(&options.raw_sensor_data_collection);
        let sensors = db.collection(&options.sensors_collection);
        let data_points = db.collection(&options.data_points_collection);

        Ok(Self {
            collections: Collections {
                raw_sensor_data,
                sensors,
                data_points,
            },
        })
    }

    pub async fn get_all_sensors(&self) -> Result<Vec<SensorMetadata>, mongodb::error::Error> {
        let collection = self.collections.sensors.clone();
        let mut cursor = collection.find(None, None).await?;
        let mut acc = Vec::new();

        while cursor.advance().await? {
            acc.push(cursor.deserialize_current().unwrap());
        }

        Ok(acc)
    }

    pub async fn get_sensor_data_at<'a, I: Iterator<Item = &'a SensorMetadata>>(
        &self,
        sensors: I,
        timestamp: i64,
        max_age: i64,
    ) -> mongodb::error::Result<HashMap<i32, DataPoint>> {
        let mut data = HashMap::new();
        let diff = timestamp - max_age;

        let max_timestamp = DateTime::from_millis(timestamp);
        let min_timestamp = DateTime::from_millis(diff);

        for sensor in sensors {
            let data_point = self
                .collections
                .data_points
                .find_one(
                    doc! {
                        "SensorId": sensor.mongo_id.unwrap(),
                        "Time": { "$lte": max_timestamp, "$gte": min_timestamp},
                    },
                    FindOneOptions::builder().sort(doc! { "Time": -1 }).build(),
                )
                .await?;

            if let Some(data_point) = data_point {
                data.insert(sensor.site_id, data_point);
            }
        }

        Ok(data)
    }
}
