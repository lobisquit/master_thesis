use crate::core::*;
use std::collections::VecDeque;
use crate::Message::*;
use std::any::Any;

#[derive(Debug, Clone)]
enum UdpClientStatus {
    Idle,

    RequestInit,
    RequestWait { session_id: usize },

    DataInit { session_id: usize },
    DataWait { session_id: usize },
    DataUpdate { session_id: usize, new_packet: Packet },

    StopWait { session_id: usize },
    Unusable { session_id: usize },
    Evaluation { session_id: usize }
}

impl UdpClientStatus {
    fn get_session_id(&self) -> Option<usize> {
        use UdpClientStatus::*;

        match *self {
            Idle => None,

            RequestInit => None,
            RequestWait { session_id } => Some(session_id),

            DataInit { session_id } => Some(session_id),
            DataWait { session_id } => Some(session_id),
            DataUpdate { session_id, .. } => Some(session_id),


            StopWait { session_id } => Some(session_id),
            Unusable { session_id } => Some(session_id),
            Evaluation { session_id } => Some(session_id)
        }
    }
}

impl MachineStatus for UdpClientStatus {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
pub struct UdpClient {
    node_id: NodeId,
    status: UdpClientStatus,
    next_hop_id: NodeId,
    dst_id: NodeId,

    t0: f64,
    n: u64,
    timeouts: Vec<usize>,

    // metrics
    pkt_delays: Vec<f64>,
    pkt_sizes: Vec<u64>
}

impl UdpClient {
    fn switch_off(&mut self) -> Vec<Event> {
        use UdpClientStatus::*;

        self.status = Idle;

        // TODO log state somewhere before erasing it
        self.pkt_delays.clear();
        self.pkt_sizes.clear();

        vec![]
    }
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
                    self.timeouts.clear();
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
                            // create a new (unique) session id
                            let session_id = Message::new_session_id();

                            // start longer timeout, after which the service is considered unusable
                            let unusable_timeout = Message::new_timeout(
                                MoveToStatus(Box::new( Unusable { session_id } ))
                            );
                            self.timeouts.push(unusable_timeout.get_id().unwrap());

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new( RequestWait { session_id } )),
                                               self.node_id),

                                self.new_event(current_time + self.n as f64 * self.t0,
                                               unusable_timeout,
                                               self.node_id)
                            ]
                        },
                        RequestWait { session_id } => {
                            // size in byte of ethernet frame with empty tcp packet
                            let request_size = 24 * 8;
                            let request = Message::new_packet(*session_id,
                                                             request_size,
                                                             DataRequest,
                                                             current_time,
                                                             self.node_id,
                                                             self.dst_id);

                            // repeat the request after a timeout
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new( RequestWait { session_id: *session_id } ))
                            );
                            self.timeouts.push(repeat_timeout.get_id().unwrap());

                            vec![
                                self.new_event(current_time, request, self.next_hop_id),
                                self.new_event(current_time + self.t0,
                                               repeat_timeout,
                                               self.node_id),
                            ]
                        },

                        DataInit { session_id } => {
                            self.timeouts.clear();

                            let unusable_timeout = Message::new_timeout(
                                MoveToStatus(Box::new( Unusable { session_id: *session_id } ))
                            );
                            self.timeouts.push(unusable_timeout.get_id().unwrap());

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new( DataWait { session_id: *session_id } )),
                                               self.node_id),

                                self.new_event(current_time + self.n as f64 * self.t0,
                                               unusable_timeout,
                                               self.node_id)
                            ]
                        },
                        DataWait { .. } => vec![],
                        DataUpdate { session_id, new_packet } => {
                            dbg!(new_packet);

                            // TODO use new_packet to update the metrics
                            vec![
                                self.new_event(current_time,
                                                MoveToStatus(Box::new( DataWait { session_id: *session_id } )),
                                               self.node_id)
                            ]
                        },

                        StopWait { session_id } => {
                            let request_size = 24 * 8;
                            let request = Message::new_packet(*session_id,
                                                             request_size,
                                                             DataStop,
                                                             current_time,
                                                             self.node_id,
                                                             self.dst_id);

                            // repeat the STOP request after a timeout
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new( StopWait { session_id: *session_id } ))
                            );
                            self.timeouts.push(repeat_timeout.get_id().unwrap());

                            vec![
                                self.new_event(current_time, request, self.next_hop_id),
                                self.new_event(current_time + self.t0,
                                               repeat_timeout,
                                               self.node_id),
                            ]
                        },
                        Unusable { session_id } => {
                            self.timeouts.clear();

                            // TODO mark connection as unusable

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new( StopWait { session_id: *session_id } )),
                                               self.node_id)
                            ]
                        },
                        Evaluation { session_id } => {
                            self.timeouts.clear();

                            // TODO use obtained metrics to evaluate QoS, QoE
                            dbg!(session_id);

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new( Idle )),
                                               self.node_id)
                            ]
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
                    panic!("User request in {:?} received while in status {:?}", self, self.status)
                }
            },
            UserSwitchOff => {
                match self.status.get_session_id() {
                    None => vec![],
                    Some(number) => vec![
                        self.new_event(current_time,
                                       MoveToStatus(Box::new( StopWait { session_id: number } )),
                                       self.node_id)
                    ]
                }
            },
            Data(packet) => {
                match self.status.get_session_id() {
                    None => match packet.pkt_type {
                        UdpData => vec![ self.new_event(current_time,
                                                       MoveToStatus(Box::new( StopWait { session_id: packet.session_id } )),
                                                       self.node_id) ],
                        UdpFinish => vec![ /* stay IDLE */ ],
                        _ => panic!("Unexpected packet {:?} in {:?}", packet, self)
                    },
                    Some(number) => {
                        if number == packet.session_id {
                            match packet.pkt_type {
                                UdpData => vec![ self.new_event(current_time,
                                                               MoveToStatus(Box::new( DataUpdate { session_id: packet.session_id, new_packet: packet } )),
                                                               self.node_id) ],
                                UdpFinish => vec![ self.new_event(current_time,
                                                                 MoveToStatus(Box::new( Idle )),
                                                                 self.node_id) ],
                                _ => panic!("Unexpected packet {:?} in {:?}", packet, self)
                            }
                        }
                        else {
                            // packet belongs to an old session (and was lost in
                            // the network): ignore it
                            vec![]
                        }
                    }
                }
            },
            _ => vec![]
        }
    }
}
