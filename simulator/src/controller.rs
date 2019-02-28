use crate::core::*;
use crate::counters::*;
use crate::Message::*;
use std::collections::HashMap;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct Controller {
    node_id: NodeId,

    #[builder(setter(skip))]
    utilities: HashMap<NodeId, f64>,

    #[builder(setter(skip))]
    tbf_params: HashMap<NodeId, HashMap<String, f64>>
}

impl Controller {
    fn register_tbf(&mut self, tbf_id: NodeId) -> Event {
        let mut default_params = HashMap::new();
        default_params.insert("max_queue".into(), 4.);
        default_params.insert("max_tokens".into(), 3.);
        default_params.insert("token_rate".into(), 14.);

        // store values in memory
        self.tbf_params.insert(tbf_id, default_params.clone());

        // report them to the node, via messages sent *before* simulation starts
        self.new_event(-1.0, SetParams(default_params), tbf_id)
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
                // the ultimate goal to improve the network

                vec![]
            }
            _ => panic!("Invalid element in pkt queue of node {:?}: {:?}", self, message)
        }
    }
}
