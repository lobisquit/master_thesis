use crate::core::*;
use crate::Message::*;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct DeterministicSource {
    #[builder(setter(skip))]
    id: NodeId,

    delta_t: f32,
    dest_id: NodeId,
    packet_size: u64
}

impl Node for DeterministicSource {
    fn process_message(&mut self, message: Message, current_time: f32) -> Vec<Event> {
        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        match message {
            GeneratePacket(go_on) => if go_on {
                vec![
                    // shedule next packet transmission
                    Event::new(current_time + self.delta_t,
                              Message::GeneratePacket(go_on),
                              self.id).unwrap(),

                    // send packet to destination immediately
                    Event::new(current_time,
                              Message::new_packet(self.packet_size, self.id),
                              self.dest_id).unwrap(),
                ]
            }
            else {
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
