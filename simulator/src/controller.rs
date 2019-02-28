use crate::core::*;
use crate::queue::*;
use crate::counters::*;
use crate::Message::*;
use std::collections::HashMap;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct Controller {
    #[builder(default="CONTROLLER_ID")]
    node_id: NodeId,

    #[builder(setter(skip))]
    utilities: HashMap<NodeId, f64>,

    #[builder(setter(skip))]
    tbf_params: HashMap<NodeId, TokenBucketQueueParams>
}

impl Controller {
    pub fn register_tbf(&mut self, tbf_id: NodeId) {
        // default value is the starting one for all TBF
        let default_params = TokenBucketQueueParams::default();

        // store values in memory
        self.tbf_params.insert(tbf_id, default_params);
    }
}

impl Node for Controller {
    fn get_id(&self) -> NodeId {
        CONTROLLER_ID
    }

    fn process_message(&mut self, message: Message, _current_time: f64) -> Vec<Event> {
        match message {
            ReportUtility { utility, node_id } => {
                self.utilities.insert(node_id, utility);

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
