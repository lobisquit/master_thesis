use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;
use crate::counters::*;

#[derive(Debug, Builder, Clone)]
#[builder(setter(into))]
pub struct BlockingQueue {
    node_addr: NodeAddress,
    dest_addr: NodeAddress,

    max_queue: usize,
    conn_speed: f64,

    #[builder(setter(skip))]
    status: BlockingQueueStatus,

    #[builder(setter(skip))]
    queue: VecDeque<Packet>,

    #[builder(setter(skip))]
    n_pkt_served: usize,

    #[builder(setter(skip))]
    n_pkt_lost: usize,
}

#[derive(Debug, Clone)]
enum BlockingQueueStatus {
    Idle,
    Transmitting,
    Decide
}

impl Default for BlockingQueueStatus {
    fn default() -> Self {
        BlockingQueueStatus::Idle
    }
}

impl MachineStatus for BlockingQueueStatus {}

impl Node for BlockingQueue {
    fn get_addr(&self) -> NodeAddress {
        self.node_addr
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        use BlockingQueueStatus::*;

        match message {
            Data(packet) => {
                match &mut self.status {
                    Idle => {
                        self.queue.push_back(packet);

                        vec![
                            self.new_event(current_time,
                                           MoveToStatus(Box::new(Transmitting)),
                                           self.get_addr())
                        ]
                    },
                    Transmitting => {
                        // put packet in the queue if there is space for it
                        if self.queue.len() < self.max_queue {
                            self.queue.push_back(packet);
                            debug!("Queue size of {:?}: {}",
                                   self.get_addr(),
                                   self.queue.len());
                        }
                        else {
                            info!("Packet lost for queue {:?}", self.get_addr());

                            // track lost packets
                            self.n_pkt_lost += 1;
                        }
                        vec![]
                    },
                    _ => panic!("{:?} arrived in wrong state at {:?}", packet, self)
                }
            },
            MoveToStatus(new_status) => {
                if let Some(status) = new_status.downcast_ref::<BlockingQueueStatus>() {
                    self.status = status.clone();

                    match status {
                        Idle => vec![],
                        Transmitting => {
                            let next_pkt = self.queue.pop_front().expect("Empty queue");

                            // track delivered packet
                            self.n_pkt_served += 1;

                            // service time is given by connection speed
                            // let delta: f64 = (self.rng.gen::<f64>() - 0.5) / 10.0;
                            // let tx_time = pkt_size as f64 / (self.conn_speed + delta);
                            let tx_time = next_pkt.size as f64 / self.conn_speed;

                            // tx the first packet in the queue
                            vec![ self.new_event(current_time + tx_time + PROC_TIME,
                                                 Data(next_pkt),
                                                 self.dest_addr),

                                  self.new_event(current_time + tx_time + PROC_TIME,
                                                 MoveToStatus(Box::new(Decide)),
                                                 self.get_addr())
                            ]
                        },
                        Decide => {
                            if self.queue.len() == 0 {
                                vec![ self.new_event(current_time,
                                                     MoveToStatus(Box::new(Idle)),
                                                     self.get_addr()) ]
                            }
                            else {
                                vec![ self.new_event(current_time,
                                                     MoveToStatus(Box::new(
                                                         Transmitting
                                                     )),
                                                     self.get_addr()) ]
                            }
                        }
                    }
                }
                else {
                    panic!("Invalid status {:?} for {:?}", new_status, self)
                }
            },
            _ => panic!("Invalid message {:?} for {:?}", message, self)
        }
    }
}
