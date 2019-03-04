use std::collections::HashMap;

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
    pub nodes_weight: HashMap<NodeId, u64>
}

impl Graph {
    pub fn add_node<N>(&mut self, node_id: N, father_id: N, weight: u64)
    where N: Into<NodeId> + Copy {
        if weight != 0 {
            self.nodes_weight.insert(node_id.into(), weight);
        }
        self.nodes_fathers.insert(node_id.into(), father_id.into());
    }

    pub fn get_father<N>(&self, node_id: N) -> Option<&NodeId>
    where N: Into<NodeId> {
        self.nodes_fathers.get(&node_id.into())
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

    fn is_ancestor(&self, wanted_node: NodeId, leaf: NodeId) -> bool {
        let mut current_node = leaf;
        while let Some(node) = self.get_father(current_node) {
            if *node == wanted_node {
                return true;
            }
            else {
                current_node = *node;
            }
        }
        false
    }

    pub fn get_routes<N>(&self, node_id: N) -> HashMap<NodeId, NodeId>
    where N: Into<NodeId> {
        let children = self.get_children(node_id.into());
        let leaves = self.get_leaves();

        let mut routes = HashMap::new();

        for child in children {
            let child_leaves = leaves
                .iter()
                .filter(|leaf| self.is_ancestor(child, **leaf))
                .map(|x| *x)
                .collect::<Vec<NodeId>>();

            for leaf in child_leaves {
                routes.insert(leaf, child);
            }
        }
        routes
    }
}
