use petgraph::{graph::NodeIndex, stable_graph::StableDiGraph};

use crate::{
    custom_bfs::CustomBfs,
    processing::{EdgeData, NodeData},
};

pub struct Path {
    pub nodes: Vec<NodeIndex>,
    pub length: f32,
}

pub fn shortest_path(
    graph: &StableDiGraph<NodeData, EdgeData>,
    points: Vec<NodeIndex>,
) -> Option<Path> {
    let mut path = Vec::new();
    let mut length = 0.0;

    let mut iter = points.iter();
    let mut start = iter.next()?;

    path.push(*start);

    for end in iter {
        let p = shortest_path_singular(graph, *start, *end)?;

        path.extend(p.nodes.iter().skip(1));
        length += p.length;
        start = end;
    }

    Some(Path {
        nodes: path,
        length,
    })
}

fn shortest_path_singular(
    graph: &StableDiGraph<NodeData, EdgeData>,
    start: NodeIndex,
    end: NodeIndex,
) -> Option<Path> {
    let mut search = CustomBfs::new(graph, start);
    while let Some((idx, dist, path)) = search.next(&graph) {
        if idx == end {
            return Some(Path {
                nodes: path,
                length: dist,
            });
        }
    }

    None
}
