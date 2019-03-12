use crate::core::*;
use crate::queue::*;
use crate::counters::*;
use crate::Message::*;
use std::collections::HashMap;
use rand_hc::Hc128Rng;
use std::io::Write;
use rand::distributions::Distribution;
use rand::distributions::Exp;
use std::fs::OpenOptions;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct Controller {
    interarrival: Exp,
    rng: Hc128Rng,
    report_path: String,

    #[builder(setter(skip))]
    utilities: HashMap<NodeAddress, f64>,

    #[builder(setter(skip))]
    report: String,

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

    fn flush_report(&mut self) -> std::io::Result<()> {
        info!("Report flushed");

        let mut report_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.report_path)?;

        report_file.write_all(self.report.as_bytes())?;
        self.report = String::new();
        Ok(())
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.flush_report()
            .expect("Unable to flush controller report on drop");
    }
}

impl Node for Controller {
    fn get_addr(&self) -> NodeAddress {
        CONTROLLER_ADDR
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        match message {
            ReportUtility { utility, node_addr, notes } => {
                self.utilities.insert(node_addr, utility);
                info!("Utility {} reported by client {}",
                      utility,
                      node_addr.component_id);

                self.report.push_str(
                    &format!("{},{:.14},{}\n", node_addr.component_id, utility, notes)
                );

                // flush report every now and then
                if self.report.len() > 1e4 as usize {
                    self.flush_report().unwrap();
                }

                if current_time < 50.0 {
                    let interarrival_time = self.interarrival.sample(&mut self.rng);

                    vec![
                        self.new_event(current_time + interarrival_time,
                                       UserSwitchOn,
                                       node_addr)
                    ]
                }
                else {
                    vec![]
                }
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
