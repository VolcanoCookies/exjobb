use longitude::Location;

use crate::parse::Point;

pub fn dist(a: Point, b: Point) -> f64 {
    let a: Location = a.into();
    let b: Location = b.into();

    a.distance(&b).meters()
}

pub fn midpoint(a: Point, b: Point) -> Point {
    Point {
        latitude: (a.latitude + b.latitude) / 2.0,
        longitude: (a.longitude + b.longitude) / 2.0,
    }
}

pub fn point_line_dist(point: Point, line_start: Point, line_end: Point) -> f64 {
    let lat1 = line_start.latitude.to_radians();
    let lon1 = line_start.longitude.to_radians();
    let lat2 = line_end.latitude.to_radians();
    let lon2 = line_end.longitude.to_radians();
    let lat3 = point.latitude.to_radians();
    let lon3 = point.longitude.to_radians();

    let y = (lon3 - lon1).sin() * lat3.cos();
    let x = lat1.cos() * lat3.sin() - lat1.sin() * lat3.cos() * (lon3 - lon1).cos();
    let bearing1 = y.atan2(x);

    let y2 = (lon2 - lon1).sin() * lat2.cos();
    let x2 = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * (lon2 - lon1).cos();
    let bearing2 = y2.atan2(x2);

    let d_lon = lon3 - lon1;

    let distance_ac =
        ((lat1.sin() * lat3.sin()) + (lat1.cos() * lat3.cos() * (d_lon).cos())).acos() * 6371000.0;
    let min_distance =
        ((distance_ac / 6371000.0).sin() * (bearing1 - bearing2).sin()).asin() * 6371000.0;

    min_distance
}

pub fn point_line_dist_approx(point: Point, line_start: Point, line_end: Point) -> f64 {
    let a = dist(point, line_start);
    let b = dist(point, line_end);

    let midpoint = Point {
        latitude: (line_start.latitude + line_end.latitude) / 2.0,
        longitude: (line_start.longitude + line_end.longitude) / 2.0,
    };
    let c = dist(point, midpoint);

    a.min(b).min(c)
}

pub fn line_heading(start: Point, end: Point) -> f64 {
    let lat1 = start.latitude.to_radians();
    let lon1 = start.longitude.to_radians();
    let lat2 = end.latitude.to_radians();
    let lon2 = end.longitude.to_radians();

    let dlon = lon2 - lon1;

    let y = dlon.sin() * lat2.cos();
    let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();

    y.atan2(x).to_degrees()
}

pub fn lerp<T, F>(a: T, b: T, t: F) -> T
where
    T: std::ops::Add<Output = T> + std::ops::Sub<Output = T> + std::ops::Mul<F, Output = T> + Copy,
{
    a + (b - a) * t
}

pub fn angle_average(angles: &Vec<f64>) -> f64 {
    let mut x = 0.0;
    let mut y = 0.0;
    for angle in angles {
        x += angle.to_radians().cos();
        y += angle.to_radians().sin();
    }

    y.atan2(x).to_degrees()
}

pub fn angle_diff(a: f64, b: f64) -> f64 {
    let diff = (a - b + 180.0) % 360.0 - 180.0;
    if diff < -180.0 {
        diff + 360.0
    } else {
        diff
    }
}

pub fn geo_distance(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != 2 || b.len() != 2 {
        panic!("Invalid input");
    }
    let a = Point {
        latitude: a[0],
        longitude: a[1],
    };
    let b = Point {
        latitude: b[0],
        longitude: b[1],
    };
    dist(a, b)
}
