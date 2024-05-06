use std::collections::{HashMap, VecDeque};

use fixedbitset::FixedBitSet;
use petgraph::{
    graph::NodeIndex,
    stable_graph::StableDiGraph,
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};

use crate::parse::Point;

#[derive(Clone)]
pub struct StackNode<N> {
    idx: NodeIndex,
    distance: f64,
    data: N,
    path: Vec<NodeIndex>,
}

impl<N> StackNode<N> {
    fn new(node: NodeIndex, distance: f64, data: N, path: Vec<NodeIndex>) -> Self {
        StackNode {
            idx: node,
            distance,
            data,
            path,
        }
    }
}

#[derive(Clone)]
pub struct CustomBfs<N, E> {
    /// The queue of nodes to visit
    pub stack: VecDeque<StackNode<N>>,
    /// The map of discovered nodes
    pub discovered: FixedBitSet,
    pub distances: HashMap<NodeIndex, f64>,
    pub paths: HashMap<NodeIndex, Vec<NodeIndex>>,
    pub distance_fn: fn(&N, &N, &E) -> f64,
}

impl<N, E> CustomBfs<N, E>
where
    N: PartialEq + Copy + Positionable,
    E: Clone,
{
    /// Create a new **Bfs**, using the graph's visitor map, and put **start**
    /// in the stack of nodes to visit.
    pub fn new(
        graph: &StableDiGraph<N, E>,
        start: NodeIndex,
        distance_fn: fn(&N, &N, &E) -> f64,
    ) -> Self
    where
        N: PartialEq + Copy + Positionable,
    {
        let discovered = graph.visit_map();
        let mut stack = VecDeque::new();
        let start_data = graph.node_weight(start).unwrap();
        stack.push_front(StackNode::new(start, 0.0, *start_data, vec![]));
        let distances = HashMap::new();
        let paths = HashMap::new();
        CustomBfs {
            stack,
            discovered,
            distances,
            paths,
            distance_fn,
        }
    }

    /// Return the next node in the bfs, or **None** if the traversal is done.
    pub fn next(&mut self, graph: &StableDiGraph<N, E>) -> Option<(NodeIndex, f64, Vec<NodeIndex>)>
    where
        N: PartialEq + Copy + Positionable,
    {
        while let Some(node) = self.stack.pop_front() {
            if self.discovered.visit(node.idx) {
                // First time visiting node, add its distance
                self.distances.insert(node.idx, node.distance);
                // Add the path to the node
                let mut path = node.path.clone();
                self.paths.insert(node.idx, path.clone());
                path.push(node.idx);

                for edge in graph.edges(node.idx) {
                    let to = edge.target();
                    let to_data = graph.node_weight(to).unwrap();

                    let edge_data = edge.weight();

                    let distance = (self.distance_fn)(&node.data, to_data, edge_data);
                    if distance.is_infinite() {
                        continue;
                    }

                    let distance = node.distance + distance;
                    self.insert_sorted(StackNode::new(to, distance, *to_data, path.clone()));
                }
                return Some((node.idx, node.distance, node.path));
            }
        }
        None
    }

    pub fn next_undirected(
        &mut self,
        graph: &StableDiGraph<N, E>,
    ) -> Option<(NodeIndex, f64, Vec<NodeIndex>)>
    where
        N: PartialEq + Copy + Positionable,
    {
        while let Some(node) = self.stack.pop_front() {
            if self.discovered.visit(node.idx) {
                // First time visiting node, add its distance
                self.distances.insert(node.idx, node.distance);
                // Add the path to the node
                let mut path = node.path.clone();
                self.paths.insert(node.idx, path.clone());
                path.push(node.idx);

                let edges_out = graph.edges_directed(node.idx, Outgoing);
                let edges_in = graph.edges_directed(node.idx, Incoming);
                let edges = edges_out.chain(edges_in);

                for edge in edges {
                    let to = if edge.source() == node.idx {
                        edge.target()
                    } else {
                        edge.source()
                    };
                    let to_data = graph.node_weight(to).unwrap();

                    let edge_data = edge.weight();

                    let distance = (self.distance_fn)(&node.data, to_data, edge_data);
                    if distance.is_infinite() {
                        continue;
                    }

                    let distance = node.distance + distance;
                    self.insert_sorted(StackNode::new(to, distance, *to_data, path.clone()));
                }
                return Some((node.idx, node.distance, node.path));
            }
        }
        None
    }

    fn insert_sorted(&mut self, node: StackNode<N>) {
        match self.stack.binary_search_by(|n| {
            n.distance
                .partial_cmp(&node.distance)
                .expect(format!("{} {}", n.distance, node.distance).as_str())
        }) {
            Ok(i) => self.stack.insert(i, node),
            Err(i) => self.stack.insert(i, node),
        }
    }
}

pub trait Positionable {
    fn point(&self) -> Point;
}
