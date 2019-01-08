use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;
/// see here for terminology: https://www.nsnam.org/docs/models/html/tbf.html

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct TokenBucketQueue {
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

    conn_speed: f64,
    token_rate: f64
}

impl TokenBucketQueueBuilder {
    // set to maximum value
    fn default_tokens(&self) -> Result<f64, String> {
        let max_tokens = self.max_tokens.ok_or("Max tokens not set")?;

        return Ok(max_tokens);
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
        // tx time for packet already in the queue
        let mut queue_size = 0;
        for msg in &self.queue {
            if let DataPacket { size: pkt_size, .. } = msg {
                queue_size += pkt_size;
            }
        }

        println!("tokens {}, queue_size {}", self.tokens, queue_size);

        let proc_time = 1e-6;

        if self.tokens > queue_size as f64 {
            proc_time
        }
        else {
            (queue_size as f64 - self.tokens) / self.token_rate + proc_time
        }
    }
}

impl Node for TokenBucketQueue {
    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        // new event: time to update the tokens
        self.update_tokens(current_time);

        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        match message {
            DataPacket { .. } => {
                // destroy packet if queue is full
                if self.queue.len() > self.max_queue {
                    vec![]
                }
                else {
                    // add packet to the queue
                    self.queue.push_back(message);

                    let pkt_delay = self.next_pkt_delay();

                    // schedule reception of packet from destination at
                    // appropriate time
                    vec![
                        Event::new(current_time + pkt_delay,
                                  TxPacket,
                                  self.node_id).unwrap()
                    ]
                }
            },
            TxPacket => {
                let next_pkt = self.queue.pop_front().unwrap();

                if let DataPacket { size: pkt_size, .. } = next_pkt {
                    self.update_tokens(current_time);

                    // safety check
                    if self.tokens < pkt_size as f64 {
                        panic!("Node {:?} does not have enough tokens to tx pkt{:?}", self, next_pkt);
                    }

                    // pay the tokens required
                    self.tokens -= pkt_size as f64;

                    // schedule reception after connection tx time
                    let tx_time = pkt_size as f64 / self.conn_speed;
                    vec![
                        Event::new(current_time + tx_time,
                                   next_pkt,
                                   self.dest_id).unwrap()
                    ]
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
