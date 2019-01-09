use crate::core::*;
use crate::Message::*;
use std::collections::HashMap;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct DeterministicSource {
    #[builder(setter(skip))]
    id: NodeId,

    #[builder(setter(skip))]
    active: bool,

    #[builder(setter(skip))]
    packet_sizes: HashMap<usize, u64>,

    #[builder(setter(skip))]
    success_counter: usize,
    #[builder(setter(skip))]
    packet_counter: usize,

    #[builder(setter(skip))]
    packets_tx_time: HashMap<usize, f64>,
    #[builder(setter(skip))]
    delays: Vec<f64>,

    delta_t: f64,
    dest_id: NodeId,
    packet_size: u64,

    conn_speed: f64
}

impl Node for DeterministicSource {
    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        match message {
            StartTx => {
                self.active = true;

                vec![
                    // shedule next packet transmission
                    Event::new(current_time,
                              Message::GeneratePacket,
                              self.id).unwrap()
                ]
            },
            StopTx => {
                self.active = false;
                vec![]
            },

            GeneratePacket => {
                if self.active {
                    let tx_time = self.packet_size as f64 / self.conn_speed;

                    // NOTE pointless for deterministic source like this
                    let new_packet = Message::new_packet(self.packet_size, self.id);

                    if let DataPacket { id, size, .. } = new_packet {
                        // store information about the sent packet
                        self.packet_sizes.insert(id, size);
                        self.packets_tx_time.insert(id, current_time);

                        self.packet_counter += 1;
                    }

                    vec![
                        // shedule next packet transmission
                        Event::new(current_time + self.delta_t,
                                   Message::GeneratePacket,
                                   self.id).unwrap(),

                        // send packet to destination immediately
                        Event::new(current_time + tx_time,
                                   new_packet,
                                   self.dest_id).unwrap(),
                    ]
                }
                else {
                    vec![]
                }
            },
            SuccessPacket { id, size: _size } => {
                let tx_time = self.packets_tx_time.get(&id).unwrap();
                self.delays.push(current_time - tx_time);

                self.success_counter += 1;

                vec![]
            },
            _ => panic!("Wrong message type received in node {:?}: {:?}",
                       self.id, message)
        }
    }

    fn get_id(&self) -> NodeId {
        self.id
    }
}
