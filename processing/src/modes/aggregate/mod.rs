use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use clap::Args;
use indicatif::ProgressBar;
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::{CreateCollectionOptions, FindOptions, IndexOptions, TimeseriesOptions},
    Client, IndexModel,
};

use crate::{
    mongo::{self, client::MongoOptions, model::VehicleType},
    progress::Progress,
};

use crate::mongo::model::{DataPoint, MeasurementSide, RawSensorData, SensorMetadata};

#[derive(Debug, Args)]
pub struct AggregateOptions {
    #[clap(flatten)]
    mongo_options: MongoOptions,
}

pub async fn aggregate(options: AggregateOptions) {
    let mut progress = Progress::new();

    let mongo_options = options.mongo_options;

    progress.step_unsized("Connecting to MongoDB");
    let client = Client::with_uri_str(mongo_options.uri).await;
    let client = client.unwrap();
    progress.finish("Connected to MongoDB");

    let db = client.database(&mongo_options.db);
    let input_collection =
        db.collection::<RawSensorData>(&mongo_options.raw_sensor_data_collection);
    let sensor_collection = db.collection::<SensorMetadata>(&mongo_options.sensors_collection);
    let data_collection = db.collection::<DataPoint>(&mongo_options.data_points_collection);

    let _ = db
        .create_collection(
            &mongo_options.data_points_collection,
            CreateCollectionOptions::builder()
                .timeseries(
                    TimeseriesOptions::builder()
                        .time_field("Time".into())
                        .meta_field(Some("SensorId".into()))
                        .build(),
                )
                .build(),
        )
        .await;

    progress.step_unsized("Creating indexes");
    let sensor_geo_index = IndexModel::builder()
        .keys(doc! {
            "location": "2dsphere",
        })
        .build();
    let sensor_index = IndexModel::builder()
        .options(IndexOptions::builder().unique(true).build())
        .keys(doc! {
            "SiteId": 1,
            "VehicleType": 1,
            "SpecificLane": 1,
            "MeasurementSide": 1,
        })
        .build();
    let sensor_id_index = IndexModel::builder()
        .keys(doc! {
            "_id": 1,
        })
        .build();

    sensor_collection
        .create_index(sensor_geo_index, None)
        .await
        .unwrap();
    sensor_collection
        .create_index(sensor_index, None)
        .await
        .unwrap();
    sensor_collection
        .create_index(sensor_id_index, None)
        .await
        .unwrap();

    let data_sensor_index = IndexModel::builder()
        .keys(doc! {
            "SensorId": 1,
            "Time": 1,
        })
        .build();
    data_collection
        .create_index(data_sensor_index, None)
        .await
        .unwrap();
    progress.finish("Indexes created");

    progress.step_unsized("Counting documents");
    let total = input_collection
        .estimated_document_count(None)
        .await
        .unwrap();
    progress.finish(format!("{} documents to process", total));

    progress.step_sized(total as usize, "Processing documents");

    let sensor_id_cache = HashMap::<(i32, MeasurementSide, i32, VehicleType), ObjectId>::new();
    let sensor_id_cache = Arc::new(RwLock::new(sensor_id_cache));

    let options = FindOptions::builder().batch_size(10000).build();
    let mut cursor = input_collection.find(None, options).await.unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    async fn process(
        data: RawSensorData,
        progress: ProgressBar,
        sensor_collection: mongodb::Collection<SensorMetadata>,
        data_collection: mongodb::Collection<DataPoint>,
        sensor_id_cache: Arc<RwLock<HashMap<(i32, MeasurementSide, i32, VehicleType), ObjectId>>>,
    ) {
        let key = (
            data.site_id,
            data.get_measurement_side(),
            data.get_lane_i32(),
            data.vehicle_type,
        );

        let existing_sensor_id = {
            let sensor_id_cache = sensor_id_cache.read().unwrap();
            sensor_id_cache.get(&key).cloned()
        };

        let sensor_id = match existing_sensor_id {
            Some(sensor_id) => sensor_id,
            None => {
                let find_one = sensor_collection.find_one(data.filter(), None);
                let existing = find_one.await.unwrap();

                match existing {
                    Some(existing) => existing.mongo_id.unwrap(),
                    None => {
                        // Acquite write lock before inserting new sensor to prevent duplicates

                        let insert = sensor_collection
                            .insert_one(&data.clone().into(), None)
                            .await;

                        match insert {
                            Ok(inserted) => {
                                {
                                    let mut write_cache = sensor_id_cache.write().unwrap();
                                    write_cache
                                        .insert(key, inserted.inserted_id.as_object_id().unwrap());
                                }

                                inserted.inserted_id.as_object_id().unwrap()
                            }
                            Err(_) => {
                                let find_one = sensor_collection.find_one(data.filter(), None);
                                let existing = find_one.await.unwrap();
                                existing.unwrap().mongo_id.unwrap()
                            }
                        }
                    }
                }
            }
        };

        let mut data_point: DataPoint = data.into();
        data_point.sensor_id = sensor_id;

        let _ = data_collection.insert_one(&data_point, None).await;
        progress.inc(1);
    }

    let pb = progress.get_pb().clone();

    while cursor.advance().await.is_ok() {
        let data = cursor.deserialize_current().unwrap();

        let future = process(
            data,
            pb.clone(),
            sensor_collection.clone(),
            data_collection.clone(),
            sensor_id_cache.clone(),
        );

        runtime.spawn(future);
    }
    progress.finish("Documents processed");
}
