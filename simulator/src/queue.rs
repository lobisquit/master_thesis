use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct BlockingQueue {
    node_id: NodeId,
    dest_id: NodeId,

    #[builder(setter(skip))]
    queue: VecDeque<Message>,
    max_queue: usize,

    conn_speed: f64
}

impl Node for BlockingQueue {
    fn get_id(&self) -> NodeId {
        self.node_id
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        match message {
            Data { .. } => {
                // put packet in the queue if there is space for it
                if self.queue.len() < self.max_queue {
                    self.queue.push_back(message);

                    // schedule its transmission if it is the queue was empty
                    if self.queue.len() == 1 {
                        return vec![ self.new_event(current_time,
                                                    QueueTransmitPacket,
                                                    self.node_id) ]
                    }
                }

                vec![]
            },

            QueueTransmitPacket => {
                let next_pkt = self.queue.pop_front().expect("Empty queue");

                if let Data(Packet { size: pkt_size, .. }) = next_pkt {
                    // service time is given by connection speed
                    let tx_time = pkt_size as f64 / self.conn_speed;

                    // tx the first packet in the queue
                    let mut events = vec![ self.new_event(current_time + tx_time,
                                                          next_pkt,
                                                          self.dest_id) ];

                    // schedule next one if queue is still not empty
                    if self.queue.len() > 0 {
                        events.push(
                            self.new_event(current_time + tx_time,
                                           QueueTransmitPacket,
                                           self.node_id)
                        )
                    }

                    events
                }
                else {
                    panic!("Invalid element in pkt queue of node {:?}: {:?}", self, next_pkt)
                }
            },

            _ => panic!("Wrong message type received in node {:?}: {:?}",
                       self.node_id, message)
        }
    }
}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct TokenBucketQueue {
    /// see here for terminology: https://www.nsnam.org/docs/models/html/tbf.html

    #[builder(setter(skip))]
    node_id: NodeId,
    dest_id: NodeId,

    #[builder(setter(skip))]
    queue: VecDeque<Message>,
    max_queue: usize,

    #[builder(default = "self.default_tokens()?")]
    tokens: f64,
    max_tokens: f64,

    #[builder(setter(skip))]
    last_update_time: f64,

    token_rate: f64
}

impl TokenBucketQueueBuilder {
    // set to maximum value
    fn default_tokens(&self) -> Result<f64, String> {
        let max_tokens = self.max_tokens.ok_or("Max tokens not set")?;

        Ok(max_tokens)
    }
}

impl TokenBucketQueue {
    fn update_tokens(&mut self, current_time: f64) {
        self.tokens += (current_time - self.last_update_time) * self.token_rate;
        self.last_update_time = current_time;

        if self.tokens > self.max_tokens {
            self.tokens = self.max_tokens;
        }
    }

    fn next_pkt_delay(&self) -> f64 {
        if let Some( Data(Packet { size: pkt_size, .. }) ) = self.queue.get(0) {
            let proc_time = 1e-6;

            if self.tokens > *pkt_size as f64 {
                proc_time
            }
            else {
                (*pkt_size as f64 - self.tokens) / self.token_rate + proc_time
            }
        }
        else {
            panic!("No packet in the queue to compute delay of")
        }
    }
}

impl Node for TokenBucketQueue {
    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        // new event: time to update the tokens
        self.update_tokens(current_time);

        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        match message {
            Data(Packet { size: pkt_size, .. }) => {
                // destroy packet if queue is full
                if self.queue.len() > self.max_queue {
                    vec![]
                }
                else {
                    if self.max_tokens < pkt_size as f64 {
                        panic!("Packet {:?} does not fit the bucket of {:?}", message, self)
                    }

                    // add packet to the queue
                    self.queue.push_back(message);

                    // if packet is the only one, schedule its tx
                    if self.queue.len() == 1 {
                        vec![
                            self.new_event(current_time + self.next_pkt_delay(),
                                           QueueTransmitPacket,
                                           self.node_id)
                        ]
                    }
                    else {
                        vec![]
                    }
                }
            },
            QueueTransmitPacket => {
                let next_pkt = self.queue.pop_front().expect("Empty queue");

                if let Data(Packet { size: pkt_size, .. }) = next_pkt {
                    self.update_tokens(current_time);

                    // safety check
                    if self.tokens < pkt_size as f64 {
                        panic!("Node {:?} does not have enough tokens to tx pkt {:?}", self, next_pkt);
                    }

                    // pay the tokens required
                    self.tokens -= pkt_size as f64;

                    let mut events = vec![
                        self.new_event(current_time,
                                       next_pkt,
                                       self.dest_id)
                    ];

                    // schedule next packet tx if queue is not empty
                    if self.queue.len() > 0 {
                        let next_pkt_delay = self.next_pkt_delay();
                        events.push(
                            self.new_event(current_time + next_pkt_delay,
                                           QueueTransmitPacket,
                                           self.node_id)
                        )
                    }

                    events
                }
                else {
                    panic!("Invalid element in pkt queue of node {:?}: {:?}", self, next_pkt)
                }
            }
            _ => panic!("Wrong message type received in node {:?}: {:?}",
                       self.node_id, message)
        }
    }

    fn get_id(&self) -> NodeId {
        self.node_id
    }
}
