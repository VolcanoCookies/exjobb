use crate::{
    math::extents,
    output::{Canvas, DrawOptions},
    parse::RoadData,
};

const COLORS: [&str; 25] = [
    "#006400", "#808000", "#483d8b", "#b22222", "#008080", "#000080", "#9acd32", "#8fbc8f",
    "#8b008b", "#ff0000", "#ff8c00", "#ffff00", "#00ff00", "#00fa9a", "#8a2be2", "#00ffff",
    "#0000ff", "#ff00ff", "#1e90ff", "#db7093", "#f0e68c", "#87ceeb", "#ff1493", "#ffa07a",
    "#ee82ee",
];

pub fn draw_roads(road_data: Vec<RoadData>, unique_ids: Vec<i32>) -> Canvas {
    if unique_ids.is_empty() {
        panic!("No unique ids provided");
    } else if unique_ids.len() > COLORS.len() {
        panic!("Too many unique ids provided");
    }

    let extent = road_data
        .iter()
        .flat_map(|r| r.coordinates.iter())
        .cloned()
        .collect::<Vec<_>>();
    let extent = extents(&extent);

    let mut canvas = Canvas::from_extents(4000, extent);

    let mut i = 0;
    for road in road_data {
        if unique_ids.contains(&road.unique_id) {
            canvas.draw_polyline(
                road.coordinates,
                DrawOptions {
                    color: COLORS[i],
                    stroke: 1.0,
                    stroke_dasharray: "2,2",
                    ..Default::default()
                },
            );
        }
        i += 1;
    }

    return canvas;
}
