use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;
use std::collections::HashMap;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct Switch {
    node_id: NodeId,

    #[builder(setter(skip))]
    routing_table: HashMap<NodeId, NodeId>
}

impl Switch {
    fn add_route(&mut self, dst_node: NodeId, next_hop: NodeId) {
        self.routing_table.insert(dst_node, next_hop);
    }
}

impl Node for Switch {
    fn get_id(&self) -> NodeId {
        self.node_id
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        match message {
            Data(Packet { dst_node, .. }) => {
                match self.routing_table.get(&dst_node) {
                    Some(next_hop) => {
                        vec![ self.new_event(current_time,
                                             message,
                                             *next_hop) ]
                    },
                    None => panic!("Invalid destination {:?} for node {:?}", dst_node, self)
                }
            },
            _ => panic!("Invalid element in pkt queue of node {:?}: {:?}", self, message)

        }
    }
}
