use clap::Args;
use mongodb::{bson::doc, options::FindOptions, Client};

use crate::{mongo::client::MongoOptions, progress::Progress};

use crate::mongo::model::DataPoint;

#[derive(Debug, Args)]
pub struct FindGapsOptions {
    #[clap(short, long)]
    max_time_between: i32,
    #[clap(flatten)]
    mongo_options: MongoOptions,
}

pub async fn find_gaps(options: FindGapsOptions) {
    let mut progress = Progress::new();

    progress.step_unsized("Connecting to MongoDB");
    let client = Client::with_uri_str(options.mongo_options.uri).await;
    let client = client.unwrap();
    progress.finish("Connected to MongoDB");

    progress.step_unsized("Estimating data length");
    let length = client
        .database(&options.mongo_options.db)
        .collection::<DataPoint>(&options.mongo_options.data_points_collection)
        .estimated_document_count(None)
        .await
        .unwrap() as usize;
    progress.finish(format!("Estimated data length: {}", length));

    progress.step_sized(length, "Reading data points");
    let mut cursor = client
        .database(&options.mongo_options.db)
        .collection::<DataPoint>(&options.mongo_options.data_points_collection)
        .find(
            None,
            FindOptions::builder().sort(doc! { "Time": 1 }).build(),
        )
        .await
        .unwrap();

    let mut times = Vec::with_capacity(length);

    let pb = progress.get_pb();
    let mut i = 0;
    while cursor.advance().await.unwrap() {
        let point = cursor.deserialize_current().unwrap();
        let time = point.time.timestamp_millis() / 1000;
        times.push(time as u32);
        i += 1;
        if i % 1000 == 0 {
            pb.inc(1000);
        }
    }
    progress.finish("Read data points");

    progress.step_unsized("Finding gaps");
    for pair in times.windows(2) {
        let diff = pair[1] - pair[0];
        if diff > options.max_time_between as u32 {
            println!("Gap between {} and {}", pair[0], pair[1]);
        }
    }
}
