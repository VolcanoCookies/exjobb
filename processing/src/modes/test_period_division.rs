use clap::Args;
use indicatif::ProgressBar;
use mongodb::options::FindOptions;

use crate::{mongo::model::DataPoint, progress::Progress};

#[derive(Debug, Args)]
pub struct TestPeriodDivisionOptions {
    #[clap(short, long, default_value = "mongodb://localhost:27017")]
    pub connection_url: String,
    #[clap(short, long, default_value = "exjobb")]
    pub database: String,
    #[clap(short, long, default_value = "sensordata")]
    pub data_collection: String,
    #[clap(short, long)]
    pub period: usize,
}

pub async fn test_period_division(options: TestPeriodDivisionOptions) {
    let mut progress = Progress::new();

    progress.step_unsized("Connecting to MongoDB");
    let client = mongodb::Client::with_uri_str(options.connection_url).await;
    let client = client.unwrap();
    progress.finish("Connected to MongoDB");

    let db = client.database(&options.database);
    let data_collection = db.collection::<DataPoint>(&options.data_collection);

    progress.step_unsized("Counting documents");
    let total = data_collection
        .estimated_document_count(None)
        .await
        .unwrap();
    progress.finish(format!("{} documents to process", total));

    progress.step_sized(total as usize, "Processing documents");

    let cursor_options = FindOptions::builder().batch_size(10000).build();
    let mut cursor = data_collection.find(None, cursor_options).await.unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    async fn process(data: DataPoint, progress: ProgressBar, period: usize) {
        if data.flow_rate as usize % period != 0 {
            println!(
                "Datapoint {} does not match period, {} % {} != 0",
                data.mongo_id, data.flow_rate, period
            );
        }

        progress.inc(1);
    }

    let pb = progress.get_pb().clone();

    let period = options.period;
    while cursor.advance().await.is_ok() {
        let data = cursor.deserialize_current().unwrap();

        let future = process(data, pb.clone(), period);

        runtime.spawn(future);
    }
    progress.finish("Documents processed");
}
