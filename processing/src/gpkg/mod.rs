mod model;

use std::time::Instant;

use console::style;
use geo::CoordsIter;
use proj4rs::Proj;
use smol::stream::StreamExt;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

use crate::{
    gpkg::model::RawRoadRow,
    parse::{Point, RoadData},
    processing::{DriveDirection, Metadata},
    progress::eta_bar,
};

pub fn read_database(path: &str, query: Option<String>) -> Vec<RoadData> {
    smol::block_on(async {
        let pool = create_connection_pool(path).await;
        fetch_all_roads(&pool, query).await
    })
}

async fn create_connection_pool(path: &str) -> Pool<Sqlite> {
    let mut path = path.to_string();
    if !path.starts_with("sqlite://") {
        path = format!("sqlite://{}", path);
    }
    SqlitePoolOptions::new().connect(&path).await.unwrap()
}

pub async fn fetch_all_roads(pool: &Pool<Sqlite>, query: Option<String>) -> Vec<RoadData> {
    let from_definition =
        "+proj=utm +zone=33 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs";
    let to_definition = "+proj=longlat +datum=WGS84 +no_defs +type=crs";

    let from = Proj::from_proj_string(&from_definition).unwrap();
    let to = Proj::from_proj_string(&to_definition).unwrap();

    println!(
        "{} Fetching roads from database...",
        style("[1/2]").bold().dim()
    );

    let filter = query.unwrap_or("".into());

    let count_query = format!("SELECT COUNT(*) FROM SverigepaketTP {}", filter);
    let road_count: (i32,) = sqlx::query_as(&count_query).fetch_one(pool).await.unwrap();

    println!(
        "{} Parsing {} roads...",
        style("[2/2]").bold().dim(),
        style(road_count.0).bold()
    );
    let start = Instant::now();
    let pb = eta_bar(road_count.0 as usize);

    let query = format!("SELECT * FROM SverigepaketTP {}", filter);
    let mut roads_stream = sqlx::query_as::<_, RawRoadRow>(&query).fetch(pool);
    let mut road_data = Vec::with_capacity(road_count.0 as usize);

    let mut dropped = 0;

    while let Some(Ok(road)) = roads_stream.next().await {
        if let Some(road_type) = road.road_type {
            if road_type != "bilnät" {
                dropped += 1;
                pb.inc(1);
                continue;
            }
        }

        let mut coords = road
            .geom
            .geometry
            .unwrap()
            .coords_iter()
            .map(|coord| (coord.x, coord.y))
            .collect::<Vec<_>>();
        proj4rs::transform::transform(&from, &to, coords.as_mut_slice()).unwrap();

        let polyline = coords
            .iter()
            .map(|(x, y)| Point {
                latitude: y.to_degrees() as f64,
                longitude: x.to_degrees() as f64,
            })
            .collect::<Vec<_>>();

        let speed_limit_f = road
            .speed_limit_f
            .map(|speed_limit| speed_limit.parse().unwrap_or_default());
        let speed_limit_b = road
            .speed_limit_b
            .map(|speed_limit| speed_limit.parse().unwrap_or_default());

        let speed_limit = match (speed_limit_f, speed_limit_b) {
            (Some(f), Some(b)) => (f + b) / 2.0,
            (Some(f), None) => f,
            (None, Some(b)) => b,
            (None, None) => 0.0,
        };

        let fdf = if let Some(fdf) = road.forbidden_direction_f {
            fdf.parse::<i32>().unwrap() == -1
        } else {
            false
        };
        let fdb = if let Some(fdb) = road.forbidden_direction_b {
            fdb.parse::<i32>().unwrap() == -1
        } else {
            false
        };
        let direction = match (fdf, fdb) {
            (true, true) => crate::parse::RoadDirection::None,
            (true, false) => crate::parse::RoadDirection::Backward,
            (false, true) => crate::parse::RoadDirection::Forward,
            (false, false) => crate::parse::RoadDirection::Both,
        };

        let metadata = Metadata {};

        road_data.push(RoadData {
            main_number: road.main_number,
            sub_number: road.sub_number,
            length: road.length,
            unique_id: road.unique_id,
            coordinates: polyline,
            direction,
            speed_limit,
            metadata,
        });

        pb.inc(1);
    }
    pb.finish_and_clear();

    println!(
        "{:?} Parsed {} roads and dropped {}",
        style(start.elapsed()).bold().dim().yellow(),
        style(road_data.len()).bold(),
        style(dropped).bold()
    );

    road_data
}
