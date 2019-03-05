use crate::core::*;
use crate::queue::*;
use crate::counters::*;
use crate::Message::*;
use std::collections::HashMap;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct Controller {
    #[builder(setter(skip))]
    utilities: HashMap<NodeAddress, f64>,

    #[builder(setter(skip))]
    tbf_params: HashMap<NodeAddress, TokenBucketQueueParams>
}

impl Controller {
    pub fn register_tbf(&mut self, tbf_id: NodeAddress) {
        // default value is the starting one for all TBF
        let default_params = TokenBucketQueueParams::default();

        // store values in memory
        self.tbf_params.insert(tbf_id, default_params);
    }
}

impl Node for Controller {
    fn get_addr(&self) -> NodeAddress {
        CONTROLLER_ADDR
    }

    fn process_message(&mut self, message: Message, _current_time: f64) -> Vec<Event> {
        match message {
            ReportUtility { utility, node_addr, notes } => {
                self.utilities.insert(node_addr, utility);

                vec![]
            },
            RecomputeParams => {
                // TODO use utilities information to modify TBF parameters, with
                // the ultimate goal to improve the network. Report all TBF
                // needed changes via messages

                vec![]
            }
            _ => panic!("Invalid element in pkt queue of node {:?}: {:?}", self, message)
        }
    }
}
