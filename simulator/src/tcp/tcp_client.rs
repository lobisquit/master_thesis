use crate::core::*;
use crate::Message::*;
use crate::utils::*;
use crate::counters::MAINFRAME_ADDR;

static WAITING_TIME_TOLERANCE: f64 = 1.0; // s
static WAITING_TIME_MARGIN: f64 = 0.95; // s

#[derive(Debug, Clone)]
pub enum TcpClientStatus {
    Idle,

    RequestInit,
    RequestWait { session_id: usize },

    DataInit { session_id: usize, new_packet: Packet },
    DataUpdate { session_id: usize, new_packet: Packet },
    DataWait {
        session_id: usize,
        sequence_num: usize,
        sequence_end: usize
    },
    DataACK {
        session_id: usize,
        sequence_num: usize,
        sequence_end: usize
    },
    Unusable { session_id: usize },
    Evaluate { session_id: usize }
}

impl Default for TcpClientStatus {
    fn default() -> Self {
        TcpClientStatus::Idle
    }
}

impl MachineStatus for TcpClientStatus {}

impl TcpClientStatus {
    fn get_session_id(&self) -> Option<usize> {
        use TcpClientStatus::*;

        match *self {
            Idle => None,

            RequestInit => None,
            RequestWait { session_id } => Some(session_id),

            DataInit { session_id, .. } => Some(session_id),
            DataWait { session_id, .. } => Some(session_id),
            DataUpdate { session_id, .. } => Some(session_id),
            DataACK { session_id, .. } => Some(session_id),

            Unusable { session_id } => Some(session_id),
            Evaluate { session_id, .. } => Some(session_id)
        }
    }
}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct TcpClient {
    node_id: NodeAddress,

    next_hop_id: NodeAddress,
    dst_id: NodeAddress,

    window_size: usize,
    t_repeat: f64,
    t_unusable: f64,

    expected_plt: f64,

    #[builder(setter(skip))]
    status: TcpClientStatus,

    #[builder(setter(skip))]
    starting_time: f64,

    #[builder(setter(skip))]
    timeouts: Vec<usize>,

    #[builder(setter(skip))]
    received_chunks: Vec<bool>
}

impl Node for TcpClient {
    fn get_id(&self) -> NodeAddress {
        self.node_id
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        use TcpClientStatus::*;
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
                if let Some(tcp_status) = new_status.downcast_ref::<TcpClientStatus>() {
                    // move to the specified status and apply its operations
                    self.status = tcp_status.clone();

                    match self.status {
                        Idle => {
                            vec![]
                        },

                        RequestInit => {
                            self.starting_time = current_time;

                            // immediately send DATA request
                            let session_id = Message::new_session_id();

                            // start longer timeout, after which the service is
                            // considered unusable
                            let unusable_timeout = Message::new_timeout(
                                MoveToStatus(Box::new( Unusable { session_id } ))
                            );
                            self.timeouts.push(
                                unusable_timeout.get_id().unwrap()
                            );

                            // immediately send DATA request
                            let new_status = RequestWait { session_id };
                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.node_id),

                                self.new_event(current_time + self.t_unusable,
                                               unusable_timeout,
                                               self.node_id)
                            ]
                        },
                        RequestWait { session_id } => {
                            // size in byte of ethernet frame with empty tcp packet
                            let request_size = 24 * 8;

                            let request_type = TcpDataRequest {
                                window_size: self.window_size
                            };
                            let request = Message::new_packet(session_id,
                                                             request_size,
                                                             request_type,
                                                             current_time,
                                                             self.node_id,
                                                             self.dst_id);

                            // repeat the request after a timeout
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(RequestWait { session_id }))
                            );
                            self.timeouts.push(repeat_timeout.get_id().unwrap());

                            vec![
                                self.new_event(current_time,
                                               request,
                                               self.next_hop_id),
                                self.new_event(current_time + self.t_repeat,
                                               repeat_timeout,
                                               self.node_id),
                            ]
                        },
                        DataInit { session_id, new_packet } => {
                            if let TcpData { sequence_end, .. } = new_packet.pkt_type {
                                self.timeouts.clear();

                                self.received_chunks = vec![false; sequence_end];

                                let new_status = DataUpdate {
                                    session_id,
                                    new_packet
                                };
                                vec![
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(new_status)),
                                                   self.node_id)
                                ]
                            }
                            else {
                                panic!("Invalid packet type {:?} in {:?}",
                                       new_packet.pkt_type, self)
                            }
                        },
                        DataUpdate { session_id, new_packet } => {
                            // invalidate all previous timeouts: communication is
                            // still alive
                            self.timeouts.clear();

                            if let TcpData { sequence_num, sequence_end, rtt } = new_packet.pkt_type {
                                self.received_chunks[sequence_num] = true;

                                // read (if any) the RTT from the server

                                if let Some(time) = rtt {
                                    self.t_repeat = time;
                                    self.t_unusable = 10.0 * time;
                                }

                                // k is the next needed element index: first
                                // non-true in array
                                let k = {
                                    let mut tx_index = sequence_end;
                                    for (index, element) in self.received_chunks.iter().enumerate() {
                                        if !element {
                                            tx_index = index;
                                            break;
                                        }
                                    }
                                    tx_index
                                };

                                // immediately send ACK
                                let new_status = DataACK {
                                    session_id,
                                    sequence_num: k,
                                    sequence_end
                                };
                                vec![
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(new_status)),
                                                   self.node_id)
                                ]
                            }
                            else {
                                panic!("Invalid packet type {:?} in {:?}", new_packet.pkt_type, self)
                            }
                        },
                        DataACK { session_id, sequence_num, sequence_end } => {
                            let mut events = vec![];

                            // send ACK to next_hop
                            let pkt_type = TcpACK {
                                sequence_num: sequence_num
                            };

                            // size in byte of ethernet frame with empty tcp packet
                            let ack_size = 24 * 8;
                            let ack = Message::new_packet(session_id,
                                                         ack_size,
                                                         pkt_type,
                                                         current_time,
                                                         self.node_id,
                                                         self.dst_id);

                            events.push(self.new_event(current_time,
                                                       ack,
                                                       self.next_hop_id));

                            // check if sequence_num matches the length of the
                            // boolean array of received packets: in this case
                            // we are done, else continue
                            let new_status = if sequence_num == sequence_end {
                                Evaluate { session_id }
                            }
                            else {
                                DataWait {
                                    session_id,
                                    sequence_num,
                                    sequence_end
                                }
                            };

                            events.push(
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.node_id)
                            );

                            events
                        },
                        DataWait { session_id, sequence_num, sequence_end } => {
                            let mut events = vec![];

                            // long timeout
                            let new_status = Unusable {
                                session_id: session_id
                            };
                            let unusable_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(new_status))
                            );
                            self.timeouts.push(
                                unusable_timeout.get_id().unwrap()
                            );

                            events.push(
                                self.new_event(current_time + self.t_unusable,
                                               unusable_timeout,
                                               self.node_id));

                            // repeat timeout
                            let new_status = DataACK {
                                session_id,
                                sequence_num,
                                sequence_end
                            };
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(new_status))
                            );
                            self.timeouts.push(
                                repeat_timeout.get_id().unwrap()
                            );

                            events.push(
                                self.new_event(current_time + self.t_repeat,
                                               repeat_timeout,
                                               self.node_id));

                            events
                        },
                        Unusable { session_id } => {
                            // invalidate previous timeouts: connection is
                            // considered dead
                            self.timeouts.clear();

                            // TODO mark connection as unusable in metrics

                            let new_status = Evaluate { session_id };
                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.node_id)
                            ]
                        },
                        Evaluate { .. } => {
                            let plt = current_time - self.starting_time;

                            // utility is based on the expected packet load time
                            // tolerance is standard
                            let utility = utility(
                                plt,
                                self.expected_plt + WAITING_TIME_TOLERANCE,
                                WAITING_TIME_TOLERANCE,
                                WAITING_TIME_MARGIN
                            );

                            let report = ReportUtility {
                                utility,
                                node_id: self.get_id()
                            };

                            vec![ self.new_event(current_time,
                                                 MoveToStatus(Box::new(Idle)),
                                                 self.node_id),

                                  self.new_event(current_time,
                                                 report,
                                                 MAINFRAME_ADDR) ]
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
                match self.status.get_session_id() {
                    None => vec![],
                    Some(number) => {
                        // do not send final ACK, as it will be performed when in
                        // IDLE state
                        let new_status = Evaluate { session_id: number };
                        vec![ self.new_event(current_time,
                                             MoveToStatus(Box::new(new_status)),
                                             self.node_id)
                        ]
                    }
                }
            },
            Data(packet) => {
                assert!(packet.dst_node == self.node_id);
                assert!(packet.src_node == self.dst_id);

                if let TcpData { sequence_end, .. } = packet.pkt_type {
                    info!("{}: {:?} received by {:?}", current_time, packet, self);

                    match self.status {
                        Idle => {
                            // server is still retransmitting: say stop
                            let ack_size = 24 * 8;

                            let pkt_type = TcpACK {
                                sequence_num: sequence_end
                            };

                            let ack = Message::new_packet(packet.session_id,
                                                         ack_size,
                                                         pkt_type,
                                                         current_time,
                                                         self.node_id,
                                                         self.dst_id);

                            vec![ self.new_event(current_time,
                                                 ack,
                                                 self.next_hop_id) ]
                        },
                        RequestWait { session_id } => {
                            self.timeouts.clear();

                            let new_status = DataInit {
                                session_id,
                                new_packet: packet
                            };
                            vec![ self.new_event(current_time,
                                                 MoveToStatus(
                                                     Box::new(new_status)),
                                                 self.node_id)
                            ]
                        },
                        DataWait { session_id, .. } => {
                            self.timeouts.clear();

                            let new_status = DataUpdate {
                                session_id,
                                new_packet: packet
                            };
                            vec![ self.new_event(current_time,
                                                 MoveToStatus(
                                                     Box::new(new_status)),
                                                 self.node_id)
                            ]
                        },
                        // reaching zero-time states can happen in an unfortunate
                        // case of simulaneous events: debug only if actually needed
                        _ => panic!("Packet {:?} received in wrong status at {:?}",
                                   packet, self)
                    }
                }
                else {
                    panic!("Wrong packet type {:?} received at {:?}",
                           packet, self)
                }
            },
            _ => vec![]
        }
    }
}
