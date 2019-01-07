use crate::core::*;
use crate::Message::*;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct SimpleSink {
    #[builder(setter(skip))]
    id: NodeId,
    #[builder(setter(skip))]
    total_packet_size: u64
}

impl Node for SimpleSink {
    fn process_message(&mut self, message: Message, current_time: f32) -> Vec<Event> {
        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        match message {
            DataPacket { size, .. } => {
                self.total_packet_size += size;
            },

            _ => panic!("Wrong message type received in node {:?}: {:?}",
                        self.id, message)
        };

        vec![]
    }

    fn get_id(&self) -> NodeId {
        self.id
    }
}
