use crate::core::*;
use crate::Message::*;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct DeterministicSource {
    #[builder(setter(skip))]
    id: NodeId,

    #[builder(setter(skip))]
    active: bool,

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

                    vec![
                        // shedule next packet transmission
                        Event::new(current_time + self.delta_t,
                                   Message::GeneratePacket,
                                   self.id).unwrap(),

                        // send packet to destination immediately
                        Event::new(current_time + tx_time,
                                   Message::new_packet(self.packet_size, self.id),
                                   self.dest_id).unwrap(),
                    ]
                }
                else {
                    vec![]
                }
            },
            _ => panic!("Wrong message type received in node {:?}: {:?}",
                       self.id, message)
        }
    }

    fn get_id(&self) -> NodeId {
        self.id
    }
}
