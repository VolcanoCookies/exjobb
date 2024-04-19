use std::collections::{HashMap, VecDeque};

use fixedbitset::FixedBitSet;
use petgraph::{
    graph::NodeIndex,
    stable_graph::StableDiGraph,
    visit::{VisitMap, Visitable},
};

use crate::{math::dist, parse::Point};

#[derive(Clone)]
pub struct StackNode {
    idx: NodeIndex,
    distance: f32,
    point: Point,
    path: Vec<NodeIndex>,
}

impl StackNode {
    fn new(node: NodeIndex, distance: f32, point: Point, path: Vec<NodeIndex>) -> Self {
        StackNode {
            idx: node,
            distance,
            point,
            path,
        }
    }
}

#[derive(Clone)]
pub struct CustomBfs {
    /// The queue of nodes to visit
    pub stack: VecDeque<StackNode>,
    /// The map of discovered nodes
    pub discovered: FixedBitSet,
    pub distances: HashMap<NodeIndex, f32>,
    pub paths: HashMap<NodeIndex, Vec<NodeIndex>>,
}

impl CustomBfs {
    /// Create a new **Bfs**, using the graph's visitor map, and put **start**
    /// in the stack of nodes to visit.
    pub fn new<N, E>(graph: &StableDiGraph<N, E>, start: NodeIndex) -> Self
    where
        N: PartialEq + Copy + Positionable,
    {
        let discovered = graph.visit_map();
        let mut stack = VecDeque::new();
        let start_data = graph.node_weight(start).unwrap();
        stack.push_front(StackNode::new(start, 0.0, start_data.point(), vec![]));
        let distances = HashMap::new();
        let paths = HashMap::new();
        CustomBfs {
            stack,
            discovered,
            distances,
            paths,
        }
    }

    /// Return the next node in the bfs, or **None** if the traversal is done.
    pub fn next<N, E>(
        &mut self,
        graph: &StableDiGraph<N, E>,
    ) -> Option<(NodeIndex, f32, Vec<NodeIndex>)>
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

                for succ in graph.neighbors(node.idx) {
                    let next_data = graph.node_weight(succ).unwrap();
                    let distance = node.distance + dist(node.point, next_data.point());
                    self.insert_sorted(StackNode::new(
                        succ,
                        distance,
                        next_data.point(),
                        path.clone(),
                    ));
                }
                return Some((node.idx, node.distance, node.path));
            }
        }
        None
    }

    fn insert_sorted(&mut self, node: StackNode) {
        match self
            .stack
            .binary_search_by(|n| n.distance.partial_cmp(&node.distance).unwrap())
        {
            Ok(i) => self.stack.insert(i, node),
            Err(i) => self.stack.insert(i, node),
        }
    }
}

pub trait Positionable {
    fn point(&self) -> Point;
}
