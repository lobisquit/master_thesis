use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;
use std::any::Any;

#[derive(Debug, Clone)]
enum UdpClientStatus {
    Idle,

    RequestInit,
    RequestWait { session_id: usize },

    DataWait { session_id: usize },
    DataUpdate { session_id: usize, new_packet: Packet },

    FinishWait { session_id: usize },
    Unusable { session_id: usize },
    Evaluate { session_id: usize }
}

impl UdpClientStatus {
    fn get_session_id(&self) -> Option<usize> {
        use UdpClientStatus::*;

        match *self {
            Idle => None,

            RequestInit => None,
            RequestWait { session_id } => Some(session_id),
            DataWait { session_id } => Some(session_id),
            DataUpdate { session_id, .. } => Some(session_id),


            FinishWait { session_id } => Some(session_id),
            Unusable { session_id } => Some(session_id),
            Evaluate { session_id } => Some(session_id)
        }
    }
}

#[derive(Debug)]
pub struct UdpClient {
    node_id: NodeId,
    status: UdpClientStatus,
    next_hop_id: NodeId,
    dst_id: NodeId,

    bitrate: f64,
    t0: f64,
    n: u64,
    timeouts: Vec<usize>
}

impl Node for UdpClient {
    fn get_id(&self) -> NodeId {
        self.node_id
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        use UdpClientStatus::*;
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
                if let Some(udp_status) = new_status.as_any().downcast_ref::<UdpClientStatus>() {
                    self.status = udp_status.clone();
                    match &self.status {
                        Idle => vec![],

                        RequestInit => {
                            // start longer timeout, after which the service is
                            // considered unusable
                            let unusable_timeout = Message::new_timeout(
                                MoveToStatus(Box::new( Unusable { session_id } ))
                            );
                            self.timeouts.push(
                                unusable_timeout.get_id().unwrap()
                            );
                            let timeout_delay = self.n as f64 * self.t0;

                            // create a new (unique) session id and immediately
                            // send DATA request
                            let new_status = RequestWait {
                                session_id: Message::new_session_id()
                            };
                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.node_id),

                                self.new_event(current_time + timeout_delay,
                                               unusable_timeout,
                                               self.node_id)
                            ]
                        },
                        RequestWait { session_id } => {
                            // size in byte of ethernet frame with empty tcp packet
                            let request_size = 24 * 8;

                            let pkt_type = UdpDataRequest {
                                bitrate: self.bitrate
                            };
                            let request = Message::new_packet(*session_id,
                                                             request_size,
                                                             pkt_type,
                                                             current_time,
                                                             self.node_id,
                                                             self.dst_id);

                            // repeat the request after a timeout
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(self.status))
                            );
                            self.timeouts.push(
                                repeat_timeout.get_id().unwrap()
                            );

                            vec![
                                self.new_event(current_time,
                                               request,
                                               self.next_hop_id),
                                self.new_event(current_time + self.t0,
                                               repeat_timeout,
                                               self.node_id),
                            ]
                        },
                        DataUpdate { session_id, new_packet } => {
                            // invalidate all previous timeouts: communication is
                            // still alive
                            self.timeouts.clear();

                            // TODO use new_packet to update the metrics
                            dbg!(new_packet);

                            let new_status = DataWait {
                                session_id: *session_id
                            };
                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.node_id)
                            ]
                        },
                        DataWait { session_id } => {
                            let new_status = Unusable {
                                session_id: *session_id
                            };
                            let unusable_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(new_status))
                            );
                            self.timeouts.push(
                                unusable_timeout.get_id().unwrap()
                            );

                            let long_delay = self.n as f64 * self.t0;
                            vec![ self.new_event(current_time + long_delay,
                                                 unusable_timeout,
                                                 self.node_id) ]
                        },
                        FinishWait { session_id } => {
                            // communicate the server that it has to stop sending
                            // packets
                            let request_size = 24 * 8;
                            let request = Message::new_packet(*session_id,
                                                             request_size,
                                                             UdpFinishRequest,
                                                             current_time,
                                                             self.node_id,
                                                             self.dst_id);

                            // repeat the FINISH request after a timeout
                            let new_status = FinishWait {
                                session_id: *session_id
                            };
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(new_status))
                            );
                            self.timeouts.push(repeat_timeout.get_id().unwrap());

                            vec![ self.new_event(current_time,
                                                 request,
                                                 self.next_hop_id),
                                  self.new_event(current_time + self.t0,
                                                 repeat_timeout,
                                                 self.node_id) ]
                        },
                        Unusable { session_id } => {
                            // invalidate previous timeouts: connection is
                            // considered dead
                            self.timeouts.clear();

                            // TODO mark connection as unusable in metrics
                            let new_status = FinishWait {
                                session_id: *session_id
                            };
                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.node_id)
                            ]
                        },
                        Evaluate { session_id } => {
                            // FINISH packet received: connection is closed
                            self.timeouts.clear();

                            // TODO use obtained metrics to compute QoS, QoE
                            dbg!(session_id);

                            vec![ self.new_event(current_time,
                                                 MoveToStatus(Box::new(Idle)),
                                                 self.node_id) ]
                        }
                    }
                }
                else {
                    panic!("Invalid status {:?} for {:?}", new_status, self)
                }
            },
            // external events
            UserSwitchOn => {
                if let Idle = self.status {
                    vec![
                        self.new_event(current_time,
                                       MoveToStatus(Box::new( RequestInit )),
                                       self.node_id)
                    ]
                }
                else {
                    panic!("User request in {:?} received while in status {:?}",
                           self, self.status)
                }
            },
            UserSwitchOff => {
                // stop the server (request FINISH packet) to exit gracefully
                match self.status.get_session_id() {
                    None => vec![],
                    Some(number) => {
                        let new_status = FinishWait {
                            session_id: number
                        };
                        vec![ self.new_event(current_time,
                                             MoveToStatus(Box::new(new_status)),
                                             self.node_id)
                    ]
                }
            },
            Data(packet) => {
                match self.status.get_session_id() {
                    // no active connection: this is an old packet,
                    // received out of order wrt the FINISH packet
                    None => vec![],

                    Some(number) => {
                        if number == packet.session_id {
                            match packet.pkt_type {
                                UdpData => {
                                    let new_status = DataUpdate {
                                        session_id: packet.session_id,
                                        new_packet: packet
                                    };
                                    vec![ self.new_event(current_time,
                                                         MoveToStatus(
                                                             Box::new(new_status)
                                                         ),
                                                         self.node_id) ]
                                },
                                UdpFinish => {
                                    let new_status = Evaluate {
                                        session_id: number
                                    };
                                    vec![ self.new_event(current_time,
                                                         MoveToStatus(
                                                             Box::new(new_status)
                                                         ),
                                                         self.node_id) ]
                                },
                                _ => panic!("Unexpected packet {:?} in {:?}",
                                           packet, self)
                            }
                        }
                        else {
                            // packet belongs to an old session and arrived after
                            // its FINISH packet: ignore
                            vec![]
                        }
                    }
                }
            },
            _ => vec![]
        }
    }
}

#[derive(Debug, Clone)]
enum UdpServerStatus {
    Idle,

    DataSend { session_id: usize, bitrate: f64, data_sent: u64 },
    DataWait { session_id: usize, bitrate: f64, data_sent: u64 },

    FinishSend { session_id: usize }
}

impl UdpServerStatus {
    fn get_session_id(&self) -> Option<usize> {
        use UdpServerStatus::*;

        match *self {
            Idle => None,

            DataSend { session_id, .. } => Some(session_id),
            DataWait { session_id, .. } => Some(session_id),

            FinishSend { session_id, .. }  => Some(session_id)
        }
    }
}

#[derive(Debug)]
pub struct UdpServer {
    node_id: NodeId,
    status: UdpServerStatus,
    next_hop_id: NodeId,
    dst_id: NodeId,
    file_size: u64,
    mtu_size: u64,
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
                                let data_packet = Message::new_packet(session_id,
                                                                     self.mtu_size,
                                                                     UdpData,
                                                                     current_time,
                                                                     self.node_id,
                                                                     self.dst_id);

                                vec![
                                    self.new_event(current_time,
                                                   data_packet,
                                                   self.next_hop_id),

                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(
                                                       DataWait {
                                                           session_id: session_id,
                                                           bitrate: bitrate,
                                                           data_sent: data_sent + self.mtu_size
                                                       }
                                                   )),
                                                   self.get_id())
                                ]
                            }
                            else {
                                vec![
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(FinishSend { session_id })),
                                                   self.get_id())
                                ]
                            }
                        },
                        DataWait { session_id, bitrate, data_sent } => {
                            let timeout = Message::new_timeout(
                                MoveToStatus(Box::new( DataSend { session_id, bitrate, data_sent } ))
                            );
                            self.timeouts.push(timeout.get_id().unwrap());

                            vec![
                                self.new_event(current_time + self.mtu_size as f64 / bitrate,
                                               timeout,
                                               self.node_id),
                            ]
                        },
                        FinishSend { session_id } => {
                            let finish_packet = Message::new_packet(session_id,
                                                                   self.mtu_size,
                                                                   UdpData,
                                                                   current_time,
                                                                   self.node_id,
                                                                   self.dst_id);
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
                    UdpDataRequest { bitrate } => {
                        if let Some(number) = self.status.get_session_id() {
                            if number != packet.session_id {
                                // cancel current service and start a new one:
                                // user has timeout the one we are serving
                                vec![
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(DataSend {
                                                       session_id: packet.session_id,
                                                       bitrate: bitrate,
                                                       data_sent: 0
                                                   })),
                                                   self.get_id())
                                ]
                            }
                            else {
                                vec![]
                            }
                        }
                        else {
                            vec![]
                        }
                    },
                    UdpFinishRequest => vec![
                        self.new_event(current_time,
                                       MoveToStatus(Box::new(FinishSend {
                                           session_id: packet.session_id
                                       })),
                                       self.get_id())
                    ],
                    _ => panic!("Unexpected packet type for {:?} in {:?}", packet, self)
                }
            }
            _ => panic!("Unexpected message {:?} in {:?}", message, self)
        }
    }
}
