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
    pub nodes_fathers: HashMap<NodeId, NodeId>,
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

    pub fn get_weight<N>(&mut self, node_id: N) -> Option<&u64>
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
}
