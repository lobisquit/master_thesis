use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;
/// see here for terminology: https://www.nsnam.org/docs/models/html/tbf.html

#[derive(Debug)]
pub struct TokenBucketQueue {
    node_id: usize,
    dest_id: usize,

    queue: VecDeque<Message>,
    max_queue: usize,

    // allows for half tokens (for example when the event happens during regeneration)
    tokens: f32,
    last_update_time: f32,
    max_tokens: f32,

    conn_speed: f32,

    // transmission speed under normal conditions (data rate = tx rate)
    token_rate: f32
}

impl TokenBucketQueue {
    fn update_tokens(&mut self, current_time: f32) {
        self.tokens += (current_time - self.last_update_time) * self.token_rate;
        self.last_update_time = current_time;

        if self.tokens > self.max_tokens {
            self.tokens = self.max_tokens;
        }
    }

    fn next_pkt_delay(&self) -> f32 {
        // tx time for packet already in the queue
        let mut queue_size = 0;
        for msg in &self.queue {
            if let DataPacket { size: pkt_size, .. } = msg {
                queue_size += pkt_size;
            }
        }
        (queue_size as f32 - self.tokens) / self.token_rate
    }
}

impl Node for TokenBucketQueue {
    fn process_message(&mut self, message: Message, current_time: f32) -> Vec<Event> {
        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        // new event: time to update the tokens
        self.update_tokens(current_time);

        match message {
            DataPacket { .. } => {
                // destroy packet if queue is full
                if self.queue.len() > self.max_queue {
                    vec![]
                }
                else {
                    let pkt_delay = self.next_pkt_delay();

                    // add packet to the queue
                    self.queue.push_back(message);

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
                    if self.tokens < pkt_size as f32 {
                        panic!("Node {:?} does not have enough tokens to tx pkt{:?}", self, next_pkt);
                    }

                    // pay the tokens required
                    self.tokens -= pkt_size as f32;

                    // schedule reception after connection tx time
                    let tx_time = pkt_size as f32 / self.conn_speed;
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

    fn get_id(&self) -> usize {
        self.node_id
    }
}
