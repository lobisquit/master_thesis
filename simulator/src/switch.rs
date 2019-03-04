use crate::core::*;
use crate::core::Message::*;
use crate::counters::MIN_CLIENT_ID;
use std::collections::HashMap;
use crate::little_graph::NodeId;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct Switch {
    node_addr: NodeAddress,

    #[builder(setter(skip))]
    routing_table: HashMap<NodeId, NodeAddress>
}

impl Switch {
    pub fn add_route(&mut self, dst_node: NodeId, next_hop: NodeAddress) {
        self.routing_table.insert(dst_node, next_hop);
    }
}

impl Node for Switch {
    fn get_addr(&self) -> NodeAddress {
        self.node_addr
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        match message {
            Data(Packet { dst_node, .. }) => {
                if dst_node.node_id == self.node_addr.node_id {
                    assert!(dst_node.component_id >= MIN_CLIENT_ID);

                    // direct deliver if destination is for the same node
                    vec![ self.new_event(current_time,
                                         message,
                                         dst_node) ]
                }
                else if let Some(next_hop) = self.routing_table.get(&dst_node.node_id.into()) {
                    vec![ self.new_event(current_time,
                                         message,
                                         *next_hop) ]
                }
                else {
                    panic!("Invalid destination {:?} for node {:?}", dst_node, self)
                }
            },
            _ => panic!("Invalid element in pkt queue of node {:?}: {:?}", self, message)

        }
    }
}
