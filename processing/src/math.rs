use crate::parse::Point;

pub fn dist(a: Point, b: Point) -> f32 {
    // Distance using latitude and longitude
    // acos(sin(lat1)*sin(lat2)+cos(lat1)*cos(lat2)*cos(lon2-lon1))*6371 (6371 is Earth radius in km.)

    let lat1 = a.latitude.to_radians();
    let lon1 = a.longitude.to_radians();
    let lat2 = b.latitude.to_radians();
    let lon2 = b.longitude.to_radians();

    let dlat = lat2 - lat1;
    let dlon = lon2 - lon1;

    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    6371000.0 * c
}

pub fn extent<T>(vec: Vec<T>, func: fn(&T) -> f32) -> (f32, f32) {
    let min = vec.iter().map(func).fold(f32::INFINITY, f32::min);
    let max = vec.iter().map(func).fold(f32::NEG_INFINITY, f32::max);
    (min, max)
}
