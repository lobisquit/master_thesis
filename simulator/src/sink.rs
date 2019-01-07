use crate::core::*;
use crate::Message::*;

#[derive(Debug)]
pub struct SimpleSink {
    id: usize,
    total_packet_size: u64
}

impl Node for SimpleSink {
    fn process_message(&mut self, message: Message, current_time: f32) -> Vec<Event> {
        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        match message {
            DataPacket { id: _, size, source: _ } => {
                self.total_packet_size += size;
            },

            _ => panic!("Wrong message type received in node {:?}: {:?}",
                        self.id, message)
        };

        vec![]
    }

    fn get_id(&self) -> usize {
        self.id
    }
}
