use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;

/// See here for terminology: https://www.nsnam.org/docs/models/html/tbf.html

#[derive(Debug, Builder, Clone)]
pub struct TokenBucketQueueParams {
    max_queue: usize,
    max_tokens: f64,
    token_rate: f64
}

impl Default for TokenBucketQueueParams {
    fn default() -> Self {
        TokenBucketQueueParams {
            // TODO set better starting values
            max_queue: 100,
            max_tokens: 10000.0,
            token_rate: 10000.0
        }
    }
}

#[derive(Debug, Builder, Clone)]
#[builder(setter(into))]
pub struct TokenBucketQueue {
    node_id: NodeId,
    dest_id: NodeId,

    #[builder(setter(skip))]
    params: TokenBucketQueueParams,

    #[builder(default="self.default_tokens()?")]
    tokens: f64,

    #[builder(setter(skip))]
    last_update_time: f64,

    #[builder(setter(skip))]
    status: TokenBucketQueueStatus,

    #[builder(setter(skip))]
    queue: VecDeque<Packet>,

    #[builder(setter(skip))]
    n_pkt_served: usize,

    #[builder(setter(skip))]
    n_pkt_lost: usize,
}

impl TokenBucketQueueBuilder {
    // set to maximum value
    fn default_tokens(&self) -> Result<f64, String> {
        let max_tokens = TokenBucketQueueParams::default().max_tokens;

        Ok(max_tokens)
    }
}

#[derive(Debug, Clone)]
enum TokenBucketQueueStatus {
    Idle,
    Transmitting,
    Decide,
    Wait
}

impl Default for TokenBucketQueueStatus {
    fn default() -> Self {
        TokenBucketQueueStatus::Idle
    }
}

impl MachineStatus for TokenBucketQueueStatus {}

impl TokenBucketQueue {
    fn update_tokens(&mut self, current_time: f64) {
        self.tokens += (current_time - self.last_update_time) * self.params.token_rate;
        self.last_update_time = current_time;

        if self.tokens > self.params.max_tokens {
            self.tokens = self.params.max_tokens;
        }
    }

    fn next_pkt_delay(&self) -> f64 {
        if let Some( Packet { size: pkt_size, .. }) = self.queue.get(0) {
            if self.tokens > *pkt_size as f64 {
                0.0
            }
            else {
                (*pkt_size as f64 - self.tokens) / self.params.token_rate
            }
        }
        else {
            panic!("No packet in the queue to compute delay of")
        }
    }
}

impl Node for TokenBucketQueue {
    fn get_id(&self) -> NodeId {
        self.node_id
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        use TokenBucketQueueStatus::*;

        match message {
            Data(packet) => {
                match &mut self.status {
                    Idle => {
                        self.queue.push_back(packet);

                        vec![
                            self.new_event(current_time,
                                           MoveToStatus(Box::new(Transmitting)),
                                           self.get_id())
                        ]
                    },
                    Transmitting | Wait => {
                        // put packet in the queue if there is space for it
                        if self.queue.len() < self.params.max_queue {
                            self.queue.push_back(packet);
                        }
                        else {
                            // track lost packets
                            self.n_pkt_lost += 1;
                        }
                        vec![]
                    },
                    _ => panic!("{:?} arrived in wrong state at {:?}", packet, self)
                }
            },
            MoveToStatus(new_status) => {
                if let Some(status) = new_status.downcast_ref::<TokenBucketQueueStatus>() {
                    self.status = status.clone();

                    match status {
                        Idle => vec![],
                        Transmitting => {
                            let next_pkt = self.queue.pop_front().expect("Empty queue");

                            // track delivered packet
                            self.n_pkt_served += 1;

                            // tx the first packet in the queue
                            vec![ self.new_event(current_time,
                                                 Data(next_pkt),
                                                 self.dest_id),

                                  self.new_event(current_time,
                                                 MoveToStatus(Box::new(Decide)),
                                                 self.get_id())
                            ]
                        },
                        Decide => {
                            if self.queue.len() == 0 {
                                vec![ self.new_event(current_time,
                                                     MoveToStatus(Box::new(Idle)),
                                                     self.get_id()) ]
                            }
                            else {
                                vec![ self.new_event(current_time,
                                                     MoveToStatus(Box::new(
                                                         Wait
                                                     )),
                                                     self.get_id()) ]
                            }
                        },
                        Wait => {
                            // update the number of tokens
                            self.update_tokens(current_time);

                            // evaluate when the number of tokens is enough to
                            // send the next packet
                            let delay = self.next_pkt_delay();

                            vec![ self.new_event(current_time + delay,
                                                 MoveToStatus(Box::new(
                                                     Transmitting
                                                 )),
                                                 self.get_id()) ]
                        }
                    }
                }
                else {
                    panic!("Invalid status {:?} for {:?}", new_status, self)
                }
            },
            SetParams(params) => {
                self.params = params;

                // if max_tokens is lowered, lower max_tokens as well
                if self.tokens > self.params.max_tokens {
                    self.tokens = self.params.max_tokens;
                }

                // if max_queue is lowered, remove all new packets in excess
                // in a newer-first fashion
                while self.queue.len() > self.params.max_queue {
                    self.queue.pop_back();
                }

                vec![]
            },
            _ => panic!("Invalid message {:?} for {:?}", message, self)
        }
    }
}
