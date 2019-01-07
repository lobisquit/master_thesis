use crate::core::*;
use crate::Message::*;

#[derive(Debug)]
pub struct DeterministicSource {
    delta_t: f32,
    id: usize,
    dest_id: usize,
    packet_size: u64
}

impl Node for DeterministicSource {
    fn process_message(&mut self, message: Message, current_time: f32) -> Vec<Event> {
        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        match message {
            GeneratePacket => vec![
                // shedule next packet transmission
                Event::new(current_time + self.delta_t,
                          Message::GeneratePacket,
                          self.id).unwrap(),

                // send packet to destination immediately
                Event::new(current_time,
                           Message::new_packet(self.packet_size, self.id),
                           self.dest_id).unwrap(),
            ],

            _ => panic!("Wrong message type received in node {:?}: {:?}",
                       self.id, message)
        }
    }

    fn get_id(&self) -> usize {
        self.id
    }
}
