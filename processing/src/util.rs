use std::{io::Error, ops::Range};

use clap::Args;
use petgraph::{graph::NodeIndex, stable_graph::StableDiGraph};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    math::geo_distance,
    parse,
    processing::{build_node_acceleration_structure, EdgeData, NodeData},
};

pub fn find_point(
    graph: &StableDiGraph<NodeData, EdgeData>,
    query: PointQuery,
) -> Result<NodeIndex, Error> {
    let tree = build_node_acceleration_structure(graph);
    let p = [query.point.latitude, query.point.longitude];
    let mut iter = tree
        .iter_nearest(&p, &geo_distance)
        .expect("Failed to find nearest node");
    while let Some((dist, (node, data))) = iter.next() {
        if dist > query.radius {
            break;
        }
        if query.heading.contains(&data.heading) {
            return Ok(*node);
        }
    }

    Err(Error::new(std::io::ErrorKind::NotFound, "No node found"))
}

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

/// A helper to deserialize `f64`, treating JSON null as f64::NAN.
/// See https://github.com/serde-rs/json/issues/202
fn deserialize_f64_null_as_infinity<'de, D: Deserializer<'de>>(des: D) -> Result<f64, D::Error> {
    let optional = Option::<f64>::deserialize(des)?;
    Ok(optional.unwrap_or(f64::INFINITY))
}
