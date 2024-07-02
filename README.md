# Data Reliability in Urban Traffic Monitoring - Tools and Dataset

This repository contains the code, and generated datasets for our thesis.

## Datasets

The dataset of trafikverkets historical sensor data can be found in `dataset/trafikverket_historic.tar.gz`, it is a MongoDB dump archive.

To restore it you will need the [mongorestore](https://www.mongodb.com/docs/database-tools/mongorestore/) database tool, as well as a MongoDB database.

Restore it using `mongorestore --gzip --archive="dataset/trafikverket_historic.tar.gz" --nsFrom "exjobb.*" --nsTo "<DESTINATION_DATABASE>.*"`. This will create two collections, `sensors` containing sensor metadata, and `sensordata` containing datapoints for the sensors.

### Data Format

#### Sensor Metadata
```rust
{
    _id: ObjectId
    // Valid GEOJson, can be used to perform mongo GEO queries
    Location: {
        type: "Point",
        // Longitude, Latitude
        coordinates: [f64, f64]
    }
    // Unique id from trafikverket
    SiteId: i32
    // For some sensors, specifies the side of the road it is on
    MeasurementSide: String
    // Type of vehicle this metadata corresponds to,
    // Each sensor gets multiple metadata entries, one for each vehicle type reportd
    VehicleType: String
    // Which lane the sensor sits in
    SpecificLane: i32
    // Period of time to which the sensor extrapolates its data to, 
    // in our case always 60
    Period: i32
}
```

#### Sensor Data
```rust
{
    _id: ObjectId
    // Time of the specific measurement
    Time: Date
    // Which sensor metadata this datapoint belongs to
    SensorId: ObjectId
    // The amount of vehicles per period
    FlowRate: i32
    // Average speed of vehicles
    AverageSpeed: i32
    // Id of the original raw measurement this datapoint originates from
    OriginalId: ObjectId
}
```


## Usage

### Prerequisites

* A MongoDB database
* Node 20 or later
* Rust 1.76.0 or later
* NVDB road sqlite database [link](https://www.nvdb.se/sv/dataleverantor/hamta-data/)
* Bing API key (Only for collection)
* Tomtom API key (Only for collection)
* HERE api key (Only for collection)

API keys are only needed when performing route api collection.

---
### Setup

1. Create a `.env` file in the root directory, use `.env_example` as a template.

### Routing APIs

1. Configure the collection in `src/collect/routes.ts` (at the top of the file).
2. Run the collection using `npm run collect-routes`, results will be saved to the MongoDB database specified in `.env`.

### Trafikverket 

1. Configure the collection in `src/collect/trafikverketflow.ts` (at the top of the file).
2. Run the collection using `npm run collect-trafikverket`, results will be saved to the MongoDB database specified in `.env`.
---
### Simulation

In the below examples commands `cli` is an alias for `cargo run --release --`

You can always view all available subcommands using `cli --help`, and available options for each subcommand using `cli <SUBCOMMAND> --help`

1. Change directories to `processing`.
    * Next steps assume you are in this directory.
2. Extract the road data using `cli extract-gpkg-data -s <PATH_TO_SQLITE_DB>`.
    * We need to extract the road data from the sqlite database and turn it into a more usable format.
    * You can optionally specify a query here to filter the data before converting it, but usually this is not needed.
3. Aggregate sensor data using `cli aggregate-sensor-data`.
    * We need to aggregate sensors after collecting them so that we can query them efficiently.
4. Process the data into a workable graph using `cli process`.
    * We now merge the extracted road data and the aggregated sensors and create a directed graph out of the two.
    * There are a lot of options for this command, fine-tuning how we create the graph. A simple example would be `cli process --max-distance-from-sensors 25000 --merge-overlap-distance 0`
        * `--max-distance-from-sensors 25000` culls any roads not within 25km of any sensor.
        * `--merge-overlap-distance 0` connects overlapping road segments that are within 0m of each other, you almost never want to set this to a higher value.
5. Simulate travel time over a route using `cli live-route`.
    * This will calculate the estimated travel time from our sensor data for some route, at various points in time, like if we were there measuring the travel time constantly.
    * To simulate the same route as in our paper, use `cli live-route --query .\queries\query1km.json --step-size 1m --max-steps 250 --max-sensor-data-age 15m --start-date <DATE> --output .\out\live_route_1km.csv --date-offset 2h --vehicle-type any-vehicle`
        * `--query` specifies which route query file to use, `queries/query1km.json`, `queries/query.2km.json`, `queries/query4km.json`, `queries/query8km.json` cover the 4 routes we simulated in our paper.
        * `--step-size 1m` increments the time by 1 minute between each run.
        * `--max-steps 250` performs 250 runs of the simulation, in this case covering a time period of 4 hours and 9 minutes.
        * `--max-sensor-data-age 15m` ignores sensor data older than 15 minutes in favor of road speed limits.
        * `--start-date` tells us the start date of our simulation, so that we use the correct historical sensor data.
        * `--date-offset 2h` offsets the date by 2 hours, to account for timezones, since dates in our database are UTC.
        * `--vehicle-type any-vehicle` filters sensor data to only include data for `any-vehicle` which is an aggregate for all vehicles, meaning we take all vehicles into account.
        * `--output` specifies where to save the generated csv file to.