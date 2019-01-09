use crate::core::*;
use crate::Message::*;

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct TrackingSink {
    #[builder(setter(skip))]
    id: NodeId,
    #[builder(setter(skip))]
    total_packet_size: u64
}

impl Node for TrackingSink {
    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        debug!("Node {:?} received message {:?} at time {}", self, message, current_time);

        match message {
            DataPacket { id, size, source } => {
                vec![
                    // shedule next packet transmission
                    Event::new(current_time,
                              Message::SuccessPacket { id, size },
                              source).unwrap()
                ]
            },

            _ => panic!("Wrong message type received in node {:?}: {:?}",
                        self.id, message)
        }
    }

    fn get_id(&self) -> NodeId {
        self.id
    }
}
