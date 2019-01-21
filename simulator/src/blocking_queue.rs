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
            Packet { .. } => {
                // put packet in the queue if there is space for it
                if self.queue.len() < self.max_queue {
                    self.queue.push_back(message);

                    // schedule its transmission if it is the queue was empty
                    if self.queue.len() == 1 {
                        return vec![ self.new_event(current_time,
                                                    QueueTransmitPacket,
                                                    self.node_id).unwrap() ]
                    }
                }

                vec![]
            },

            QueueTransmitPacket => {
                let next_pkt = self.queue.pop_front().expect("Empty queue");

                if let Packet { size: pkt_size, .. } = next_pkt {
                    // service time is given by connection speed
                    let tx_time = pkt_size as f64 / self.conn_speed;

                    // tx the first packet in the queue
                    let mut events = vec![ self.new_event(current_time + tx_time,
                                                          next_pkt,
                                                          self.node_id).unwrap() ];

                    // schedule next one if queue is still not empty
                    if self.queue.len() > 0 {
                        events.push(
                            self.new_event(current_time + tx_time,
                                           QueueTransmitPacket,
                                           self.node_id).unwrap()
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
