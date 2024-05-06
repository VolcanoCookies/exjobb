use std::str::FromStr;

use serde::{Deserialize, Deserializer};

/// A helper to deserialize `f64`, treating JSON null as f64::NAN.
/// See https://github.com/serde-rs/json/issues/202
pub fn deserialize_f64_null_as_infinity<'de, D: Deserializer<'de>>(
    des: D,
) -> Result<f64, D::Error> {
    let optional = Option::<f64>::deserialize(des)?;
    Ok(optional.unwrap_or(f64::INFINITY))
}

pub fn parse_f64_nan_inf(s: &str) -> Result<f64, <f64 as FromStr>::Err> {
    let v = match s {
        "nan" => f64::NAN,
        "inf" => f64::INFINITY,
        "-inf" => f64::NEG_INFINITY,
        _ => s.parse()?,
    };
    Ok(v)
}
