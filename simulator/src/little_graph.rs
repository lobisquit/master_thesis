use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct NodeId(pub usize);

impl Into<usize> for &NodeId {
    fn into(self) -> usize {
        self.0
    }
}

impl Into<usize> for NodeId {
    fn into(self) -> usize {
        self.0
    }
}

impl Into<NodeId> for usize {
    fn into(self) -> NodeId {
        NodeId(self)
    }
}

#[derive(Default, Debug)]
pub struct Graph {
    nodes_fathers: HashMap<NodeId, NodeId>,
    nodes_weight: HashMap<NodeId, u64>,
    nodes_leaf_child: HashMap<NodeId, HashMap<NodeId, NodeId>>
}

impl Graph {
    pub fn add_node<N>(&mut self, node_id: N, father_id: N, weight: u64)
    where N: Into<NodeId> + Copy {
        if weight != 0 {
            self.nodes_weight.insert(node_id.into(), weight);
        }
        self.nodes_fathers.insert(node_id.into(), father_id.into());
    }

    pub fn get_father<N>(&self, node_id: N) -> Option<NodeId>
    where N: Into<NodeId> {
        match self.nodes_fathers.get(&node_id.into()) {
            Some(n) => Some(*n),
            None => None
        }
    }

    pub fn get_weight<N>(&self, node_id: N) -> Option<&u64>
    where N: Into<NodeId> {
        self.nodes_weight.get(&node_id.into())
    }

    pub fn get_leaves(&self) -> Vec<NodeId> {
        let non_leaves = self.nodes_fathers.values()
            .map(|x| *x)
            .collect::<Vec<NodeId>>();

        self.nodes_fathers.keys()
            .filter(|candidate| !non_leaves.contains(candidate))
            .map(|x| *x)
            .collect::<Vec<NodeId>>()
    }

    pub fn get_children<N>(&self, node_id: N) -> Vec<NodeId>
    where N: Into<NodeId> {
        let node_id: NodeId = node_id.into();

        self.nodes_fathers.iter()
            .filter(|(_, v)| **v == node_id)
            .map(|(k, _)| *k)
            .collect::<Vec<NodeId>>()
    }

    pub fn initialize_routes<'a>(mut self) -> Graph {
        // create empty entries in map
        self.nodes_leaf_child = HashMap::new();

        let all_nodes = self.nodes_fathers.iter()
            .flat_map(|(k, v)| vec![k, v])
            .map(|x| *x)
            .collect::<HashSet<NodeId>>();

        for node in all_nodes {
            self.nodes_leaf_child.insert(node, HashMap::new());
        }

        for leaf in self.get_leaves() {
            let mut current_node = leaf;
            while let Some(father) = self.get_father(current_node) {
                // insert entry in the right place
                self.nodes_leaf_child
                    .get_mut(&father)
                    .expect("Father key not in nodes_leaf_child map")
                    .insert(leaf, current_node);

                current_node = father;
            }
        }
        self
    }

    pub fn get_routes<N>(&self, node_id: N) -> &HashMap<NodeId, NodeId>
    where N: Into<NodeId> {
        self.nodes_leaf_child
            .get(&node_id.into())
            .expect("Node not in nodes_leaf_child map")
    }
}
