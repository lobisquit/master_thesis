use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;
use std::any::Any;

#[derive(Debug, Clone)]
enum UdpServerStatus {
    Idle,

    DataSend { session_id: usize, bitrate: f64, data_sent: u64 },
    DataWait { session_id: usize, bitrate: f64, data_sent: u64 },

    FinishSend { session_id: usize }
}

impl Default for UdpServerStatus {
    fn default() -> Self {
        UdpServerStatus::Idle
    }
}

impl UdpServerStatus {
    fn get_session_id(&self) -> Option<usize> {
        use UdpServerStatus::*;

        match *self {
            Idle => None,

            DataSend { session_id, .. } => Some(session_id),
            DataWait { session_id, .. } => Some(session_id),

            FinishSend { session_id }  => Some(session_id)
        }
    }
}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct UdpServer {
    node_id: NodeId,

    #[builder(setter(skip))]
    status: UdpServerStatus,

    next_hop_id: NodeId,
    dst_id: NodeId,

    file_size: u64,
    mtu_size: u64,

    #[builder(setter(skip))]
    timeouts: Vec<usize>
}

impl Node for UdpServer {
    fn get_id(&self) -> NodeId {
        self.node_id
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        use UdpServerStatus::*;
        use PacketType::*;

        // first of all, handle timeouts if they are still active
        match message {
            Timeout { expire_message, id } => {
                if self.timeouts.contains(&id) {
                    vec![ self.new_event(current_time,
                                         *expire_message,
                                         self.get_id()) ]
                }
                else {
                    vec![]
                }
            },
            MoveToStatus(new_status) => {
                if let Some(udp_status) = new_status.as_any().downcast_ref::<UdpServerStatus>() {
                    self.status = udp_status.clone();

                    match self.status {
                        Idle => vec![],
                        DataSend { session_id, bitrate, data_sent } => {
                            if data_sent < self.file_size {
                                let data_packet = Message::new_packet(
                                    session_id,
                                    self.mtu_size,
                                    UdpData,
                                    current_time,
                                    self.node_id,
                                    self.dst_id
                                );

                                let new_status = DataWait {
                                    session_id: session_id,
                                    bitrate: bitrate,
                                    data_sent: data_sent + self.mtu_size
                                };

                                vec![
                                    // send packet to the new status now
                                    self.new_event(current_time,
                                                   data_packet,
                                                   self.next_hop_id),

                                    // move immediately to waiting state
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(
                                                       new_status
                                                   )),
                                                   self.get_id())
                                ]
                            }
                            else {
                                // just tell the user that the stream has ended
                                vec![
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(
                                                       FinishSend { session_id }
                                                   )),
                                                   self.get_id())
                                ]
                            }
                        },
                        DataWait { session_id, bitrate, data_sent } => {
                            // schedule departure of next packet
                            let timeout = Message::new_timeout(
                                MoveToStatus(Box::new(
                                    DataSend { session_id, bitrate, data_sent }
                                ))
                            );
                            self.timeouts.push(timeout.get_id().unwrap());

                            let wait_time = self.mtu_size as f64 / bitrate;
                            vec![ self.new_event(current_time + wait_time,
                                                 timeout,
                                                 self.node_id) ]
                        },
                        FinishSend { session_id } => {
                            // tell the user that stream has ended
                            let finish_packet = Message::new_packet(
                                session_id,
                                self.mtu_size,
                                UdpFinish { file_size: self.file_size },
                                current_time,
                                self.node_id,
                                self.dst_id
                            );

                            vec![
                                self.new_event(current_time,
                                               finish_packet,
                                               self.get_id()),

                                self.new_event(current_time,
                                               MoveToStatus(Box::new(Idle)),
                                               self.get_id())
                            ]
                        }
                    }
                }
                else {
                    panic!("Invalid status {:?} for {:?}", new_status, self)
                }
            },
            Data(packet) => {
                match packet.pkt_type {
                    // only answer to data requests
                    UdpDataRequest { bitrate } => {
                        // cancel current service (if any) and start a new one:
                        // if one was running, user must have timed-out
                        let new_status = DataSend {
                            session_id: packet.session_id,
                            bitrate: bitrate,
                            data_sent: 0
                        };

                        vec![ self.new_event(current_time,
                                             MoveToStatus(Box::new(new_status)),
                                             self.get_id()) ]
                    },
                    UdpFinishRequest => {
                        // ignore finish request if old (must be duplicate)
                        if let Some(number) = self.status.get_session_id() {
                            if number != packet.session_id {
                                return vec![]
                            }
                        }

                        vec![
                            // anytime if the user tell we are done, we are done
                            self.new_event(current_time,
                                           MoveToStatus(Box::new(FinishSend {
                                               session_id: packet.session_id
                                           })),
                                           self.get_id())
                        ]
                    },
                    _ => panic!("Unexpected packet type for {:?} in {:?}",
                               packet, self)
                }
            },
            _ => panic!("Unexpected message {:?} in {:?}", message, self)
        }
    }
}
