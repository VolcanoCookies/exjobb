use std::ops::RangeInclusive;

use clap::Args;
use console::style;
use num_traits::real;
use petgraph::{
    graph::NodeIndex,
    stable_graph::StableDiGraph,
    visit::{IntoEdgeReferences, Visitable},
};
use serde::{Deserialize, Serialize};

use crate::{
    math::geo_distance,
    output::{Canvas, DrawOptions},
    parse::SensorData,
    processing::{build_node_acceleration_structure, EdgeData, NodeData},
    progress::Progress,
    util::{find_point, PointQuery},
    visitor::{
        self, calculate_travel_distance, calculate_travel_time_sensors, convert_ms_to_kmh,
        DistanceMetric, TravelTime,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorMode {
    Deviation {
        range: RangeInclusive<f64>,
        step_size: f64,
    },
    SetSpeed {
        speeds: Vec<f64>,
    },
}

impl SensorMode {
    pub fn count(&self) -> usize {
        match self {
            SensorMode::Deviation { range, step_size } => {
                f64::ceil((range.end() - range.start()) / step_size) as usize
            }
            SensorMode::SetSpeed { speeds } => speeds.len(),
        }
    }

    pub fn modifications(&self) -> Vec<SensorModification> {
        match self {
            SensorMode::Deviation { range, step_size } => {
                let mut deviation = *range.start();
                let mut modifications = Vec::new();
                while deviation <= *range.end() {
                    modifications.push(SensorModification::Deviation {
                        modifier: deviation,
                    });
                    deviation += step_size;
                }
                modifications
            }
            SensorMode::SetSpeed { speeds } => speeds
                .iter()
                .map(|speed| SensorModification::SetSpeed { speed: *speed })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorModification {
    Deviation { modifier: f64 },
    SetSpeed { speed: f64 },
}

impl SensorModification {
    pub fn apply(&self, sensor: SensorData) -> SensorData {
        match self {
            SensorModification::Deviation { modifier } => {
                let mut sensor = sensor.clone();
                sensor.average_speed *= modifier;
                sensor
            }
            SensorModification::SetSpeed { speed } => {
                let mut sensor = sensor.clone();
                sensor.average_speed = *speed;
                sensor
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationPathQuery {
    Raw(Vec<PointQuery>),
    File(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationSetup {
    pub paths: Vec<SimulationPathQuery>,
    pub metric: DistanceMetric,
    pub sensors: Vec<SensorSetup>,
    pub sensor_mode: SensorMode,
    pub fake_data_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorSetup {
    pub site_id: i32,
}

#[derive(Debug, Clone, Args)]
pub struct SimulationOptions {
    #[clap(short, long, default_value = "false", default_missing_value = "true")]
    pub ignore_missing_sensors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedPath {
    pub nodes: Vec<NodeIndex>,
    pub length: f64,
    pub travel_time: TravelTime,
    pub path_index: usize,
    pub modification: SensorModification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub paths: Vec<SimulatedPath>,
    pub sensor_mode: SensorMode,
}

pub struct ExtendedPath {
    pub path: visitor::Path,
    pub length: f64,
}

pub fn simulate(
    graph: &mut StableDiGraph<NodeData, EdgeData>,
    setup: SimulationSetup,
    options: SimulationOptions,
    output_path: &str,
) {
    let mut progress = Progress::new();

    progress.step_sized(setup.paths.len(), "Finding node waypoints");
    let mut paths = Vec::new();
    for path in &setup.paths {
        let path = match path {
            SimulationPathQuery::Raw(path) => path.clone(),
            SimulationPathQuery::File(path) => {
                serde_json::from_str(&std::fs::read_to_string(path).expect("Failed to read file"))
                    .expect("Failed to parse file")
            }
        };

        let path = path
            .iter()
            .map(|query| find_point(graph, query.clone()).expect("Failed to find point"))
            .collect::<Vec<_>>();
        paths.push(path);
        progress.tick();
    }
    progress.finish("Waypoints found");

    progress.step_sized(paths.len(), "Simulating paths");
    let paths = paths
        .into_iter()
        .map(|path| {
            let path =
                visitor::shortest_path(graph, path, setup.metric).expect("Failed to find path");
            let length = calculate_travel_distance(graph, &path.nodes);
            let res = ExtendedPath { path, length };
            progress.tick();
            res
        })
        .collect::<Vec<_>>();
    progress.finish("Paths simulated");

    // Ensure all sensors exist in the paths
    progress.step_unsized("Ensuring all sensors exist on paths");
    let existing_sensors = paths
        .iter()
        .map(|ext| ext.path.nodes.clone())
        .flat_map(|nodes| {
            nodes
                .clone()
                .into_iter()
                .filter_map(|node| graph[node].sensor)
        })
        .map(|sensor| sensor.site_id)
        .collect::<Vec<_>>();
    for sensor in &setup.sensors {
        if !existing_sensors.contains(&sensor.site_id) {
            if options.ignore_missing_sensors {
                println!(
                    "Sensor {} not found on any path",
                    style(sensor.site_id).bold()
                );
            } else {
                panic!(
                    "Sensor {} not found on any path, existing sensors are: {:?}",
                    sensor.site_id, existing_sensors
                );
            }
        }
    }
    progress.finish("Found all sensors");

    progress.step_unsized("Trimming graph to path");
    let mut stripped_graph = graph.clone();
    let mut keep = stripped_graph.visit_map();
    for path in &paths {
        for node in path.path.nodes.iter() {
            keep.insert(node.index());
            progress.tick();
        }
    }
    stripped_graph.retain_nodes(|_, node| keep.contains(node.index()));
    progress.finish(format!(
        "Graph trimmed to {} nodes",
        style(stripped_graph.node_count()).bold()
    ));

    progress.step_sized(
        setup.sensor_mode.count() * paths.len(),
        "Simulating sensors",
    );
    let mut results = Vec::new();
    let sensors = setup
        .sensors
        .iter()
        .map(|sensor| sensor.site_id)
        .collect::<Vec<_>>();
    for (index, ExtendedPath { path, length }) in paths.iter().enumerate() {
        for modification in setup.sensor_mode.modifications() {
            let modifications = vec![modification.clone(); setup.sensors.len()];
            let input = sensors
                .iter()
                .cloned()
                .zip(modifications)
                .collect::<Vec<_>>();
            let travel_time =
                simulate_modifications(&stripped_graph, &setup.clone(), &path, &input);
            let result = SimulatedPath {
                nodes: path.nodes.clone(),
                length: *length,
                travel_time,
                path_index: index,
                modification,
            };
            results.push(result);
            progress.tick();
        }
    }
    progress.finish("Sensors simulated");

    let result = SimulationResult {
        paths: results,
        sensor_mode: setup.sensor_mode,
    };

    save_as_csv(result, output_path);
    progress.step_single(format!(
        "Simulation results saved to {}",
        style(output_path).bold()
    ));

    let mut canvas = Canvas::from_graph(4000, &graph);

    let tree = build_node_acceleration_structure(&stripped_graph);
    graph.retain_nodes(|frozen, node| {
        let data = frozen.node_weight(node).unwrap();
        let p = [data.point.latitude, data.point.longitude];
        let mut iter = tree.iter_nearest(&p, &geo_distance).unwrap();
        let (dist, _) = iter.next().unwrap();
        dist < 500.0
    });

    progress.step_sized(graph.edge_count(), "Drawing edges");
    for edge in graph.edge_references() {
        let data = edge.weight();
        canvas.draw_polyline(
            data.polyline.clone(),
            DrawOptions {
                color: "gray".into(),
                stroke: 1.0,
                ..Default::default()
            },
        );
        progress.tick();
    }
    progress.finish(format!("Drew {} edges", style(graph.edge_count()).bold()));

    progress.step_sized(paths.len(), "Drawing paths");
    for path in paths.iter() {
        let path = &path.path;

        let opts = DrawOptions {
            color: "blue".into(),
            stroke: 1.0,
            ..Default::default()
        };

        path.nodes.windows(2).for_each(|nodes| {
            let edge = graph.find_edge(nodes[0], nodes[1]).unwrap();
            let data = &graph[edge];
            canvas.draw_polyline(data.polyline.clone(), opts.clone());
        });

        let start = path.nodes.first().unwrap();
        let end = path.nodes.last().unwrap();
        let start_data = graph.node_weight(*start).unwrap();
        let end_data = graph.node_weight(*end).unwrap();

        canvas.draw_circle(start_data.point, "green", 5.0);
        canvas.draw_circle(end_data.point, "red", 5.0);

        progress.tick();
    }
    progress.finish(format!("Drew {} paths", style(paths.len()).bold()));

    canvas.save("./out/output.svg");
}

fn simulate_modifications(
    graph: &StableDiGraph<NodeData, EdgeData>,
    setup: &SimulationSetup,
    path: &visitor::Path,
    sensors: &Vec<(i32, SensorModification)>,
) -> TravelTime {
    let mut graph = graph.clone();
    for node in &path.nodes {
        if let Some(sensor) = graph[*node].sensor {
            for (site_id, modification) in sensors {
                if *site_id == sensor.site_id {
                    let real_speed = sensor.average_speed;
                    let sensor = modification.apply(sensor);
                    let fake_speed = sensor.average_speed;

                    let real_vehicle_count = sensor.flow_rate;

                    let fake_vehicle_count = real_vehicle_count * (fake_speed - real_speed)
                        / (setup.fake_data_speed - fake_speed);

                    println!(
                        "{} Count: {:.2} -> {:.2} \t Speed: {:.2} -> {:.2}",
                        sensor.site_id,
                        real_vehicle_count,
                        fake_vehicle_count,
                        real_speed,
                        fake_speed
                    );

                    graph[*node].sensor = Some(sensor);
                }
            }
        }
    }

    calculate_travel_time_sensors(&graph, path)
}

pub fn save_as_csv(result: SimulationResult, file_path: &str) {
    let mut writer = csv::Writer::from_path(file_path).expect("Failed to open file");
    writer
        .write_record(&[
            "path_index",
            "length",
            "travel_time",
            "modification_value",
            "modification_mode",
        ])
        .expect("Failed to write header");
    for path in result.paths {
        writer
            .write_record(&[
                path.path_index.to_string(),
                path.length.to_string(),
                path.travel_time.time.to_string(),
                match path.modification {
                    SensorModification::Deviation { modifier } => modifier.to_string(),
                    SensorModification::SetSpeed { speed } => speed.to_string(),
                },
                match path.modification {
                    SensorModification::Deviation { .. } => "deviation".to_string(),
                    SensorModification::SetSpeed { .. } => "set_speed".to_string(),
                },
            ])
            .expect("Failed to write record");
    }
    writer.flush().expect("Failed to flush writer");
}
