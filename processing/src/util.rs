use std::{io::Error, ops::Range};

use crate::args::deserialize_f64_null_as_infinity;
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::parse;

#[derive(Debug, Clone, Serialize, Deserialize, Args)]
#[group(required = true, multiple = true)]
pub struct PointQuery {
    #[clap(flatten)]
    pub point: parse::Point,
    #[clap(short, long, default_value = "nan")]
    #[serde(deserialize_with = "deserialize_f64_null_as_infinity")]
    pub radius: f64,
    #[clap(short, long, default_value = "-180..180", value_parser = range_from_str)]
    pub heading: Range<f64>,
}

fn range_from_str(s: &str) -> Result<Range<f64>, Error> {
    let mut parts = s.split("..");
    let start = parts.next().unwrap().parse().unwrap();
    let end = parts.next().unwrap().parse().unwrap();
    Ok(start..end)
}

impl PointQuery {
    pub fn new(latitude: f64, longitude: f64, radius: f64, heading: Range<f64>) -> Self {
        PointQuery {
            point: parse::Point {
                latitude,
                longitude,
            },
            radius,
            heading,
        }
    }
}
