use crate::core::*;
use crate::Message::*;
use std::cmp::max;

#[derive(Debug, Clone)]
enum TcpServerStatus {
    Idle,
    InitSession { session_id: usize, n: usize },

    TransmitDecide { session_id: usize, n: usize, a: usize },
    TransmitPacket { session_id: usize, n: usize, a: usize },
    TransmitRepeat { session_id: usize, n: usize, a: usize },
    TransmitWait   { session_id: usize, n: usize, a: usize },
    ReceiveUpdate  { session_id: usize, n: usize, a: usize, packet: Packet }
}

impl Default for TcpServerStatus {
    fn default() -> Self {
        TcpServerStatus::Idle
    }
}

impl MachineStatus for TcpServerStatus {}

impl TcpServerStatus {
    fn get_conn_params(&self) -> Option<[usize; 3]> {
        use TcpServerStatus::*;

        match *self {
            Idle => None,
            InitSession { .. } => None,
            TransmitDecide { session_id, n, a, .. } => Some([session_id, n, a]),
            TransmitPacket { session_id, n, a, .. } => Some([session_id, n, a]),
            TransmitRepeat { session_id, n, a, .. } => Some([session_id, n, a]),
            TransmitWait   { session_id, n, a, .. } => Some([session_id, n, a]),
            ReceiveUpdate  { session_id, n, a, .. } => Some([session_id, n, a]),
        }
    }
}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct TcpServer {
    node_id: NodeId,

    #[builder(setter(skip))]
    b: usize,

    #[builder(setter(skip))]
    status: TcpServerStatus,

    next_hop_id: NodeId,
    dst_id: NodeId,

    total_n_packets: usize,
    mtu_size: u64,
    t0: f64,

    #[builder(setter(skip))]
    timeouts: Vec<usize>
}

impl Node for TcpServer {
    fn get_id(&self) -> NodeId {
        self.node_id
    }

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
        use TcpServerStatus::*;
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
                if let Some(tcp_status) = new_status.downcast_ref::<TcpServerStatus>() {
                    self.status = tcp_status.clone();

                    match self.status {
                        Idle => vec![],
                        InitSession { session_id, n } => {
                            let new_status = TransmitDecide {
                                session_id: session_id,
                                n: n,
                                a: 0
                            };

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.get_id())
                            ]
                        },
                        TransmitDecide { session_id, n, a } => {
                            if self.b == self.total_n_packets {
                                // wait if all packets have been transmitted
                                let new_status = TransmitWait { session_id,
                                                                n,
                                                                a };

                                vec![
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(new_status)),
                                                   self.get_id())
                                ]
                            }
                            else {
                                if self.b < a + n {
                                    let new_status = TransmitPacket {
                                        session_id,
                                        n,
                                        a
                                    };

                                    vec![
                                        self.new_event(current_time,
                                                       MoveToStatus(
                                                           Box::new(new_status)
                                                       ),
                                                       self.get_id())
                                    ]
                                }
                                else {
                                    let new_status = TransmitWait { session_id,
                                                                    n,
                                                                    a };

                                    vec![
                                        self.new_event(current_time,
                                                       MoveToStatus(
                                                           Box::new(new_status)
                                                       ),
                                                       self.get_id())
                                    ]
                                }
                            }
                        },
                        TransmitPacket { session_id, n, a } => {
                            assert!(self.b < self.total_n_packets);

                            let data_packet = Message::new_packet(
                                session_id,
                                self.mtu_size,
                                TcpData {
                                    sequence_num: self.b,
                                    sequence_end: self.total_n_packets
                                },
                                current_time,
                                self.node_id,
                                self.dst_id
                            );

                            // update value of b
                            self.b = self.b + 1;

                            // prepare retransmission
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(
                                    TransmitDecide { session_id,
                                                     n,
                                                     a }
                                ))
                            );
                            self.timeouts.push(repeat_timeout.get_id().unwrap());


                            vec![
                                self.new_event(current_time + self.t0 / 2.,
                                               repeat_timeout,
                                               self.get_id()),

                                self.new_event(current_time,
                                               data_packet,
                                               self.next_hop_id)
                            ]
                        },
                        TransmitWait { session_id, n, a } => {
                            let timeout = Message::new_timeout(
                                MoveToStatus(Box::new(
                                    TransmitRepeat { session_id, n, a }
                                ))
                            );
                            self.timeouts.push(timeout.get_id().unwrap());

                            vec![ self.new_event(current_time + self.t0,
                                                 timeout,
                                                 self.node_id) ]
                        },
                        TransmitRepeat { session_id, n, a, .. } => {
                            // reset sent window, to send unACKed packets again
                            self.b = a;
                            let new_status = TransmitDecide { session_id,
                                                              n,
                                                              a };

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.get_id())
                            ]
                        },
                        ReceiveUpdate { session_id, n, a, packet } => {
                            if let Packet {
                                session_id: pkt_session_id,
                                pkt_type: TcpACK { sequence_num },
                                ..
                            } = packet {
                                assert!(sequence_num <= self.total_n_packets);

                                if session_id != pkt_session_id || sequence_num <= a {
                                    // ignore previus ACKs
                                    vec![]
                                }
                                else {
                                    self.timeouts.clear();

                                    if sequence_num == self.total_n_packets {
                                        // final ACK
                                        vec![
                                            self.new_event(current_time,
                                                           MoveToStatus(
                                                               Box::new(Idle)
                                                           ),
                                                           self.get_id())
                                        ]
                                    }
                                    else {
                                        self.b = max(self.b, a);

                                        let new_status = TransmitDecide {
                                            session_id,
                                            n,
                                            a: max(a, sequence_num),
                                        };
                                        vec![
                                            self.new_event(current_time,
                                                           MoveToStatus(
                                                               Box::new(new_status)
                                                           ),
                                                           self.get_id())
                                        ]
                                    }
                                }
                            }
                            else {
                                panic!("Wrong setup of current state of {:?}: invalid packet", self)
                            }
                        }
                    }
                }
                else {
                    panic!("Invalid status {:?} for {:?}", new_status, self)
                }
            },
            Data(packet) => {
                assert!(packet.dst_node == self.node_id);
                assert!(packet.src_node == self.dst_id);

                info!("{:?} received by {:?}", packet, self);

                match packet.pkt_type {
                    // always drop current session if user is requesting another
                    // round
                    TcpDataRequest { window_size } => {
                        let new_status = InitSession {
                            session_id: packet.session_id,
                            n: window_size
                        };

                        vec![
                            self.new_event(current_time,
                                           MoveToStatus(Box::new(new_status)),
                                           self.get_id())
                        ]
                    },
                    TcpACK { .. } => {
                        if let Some([session_id, n, a]) = self.status.get_conn_params() {
                            // checks on wheater the packet is useful or not will
                            // be performed in ReceiveUpdate
                            let new_status = ReceiveUpdate { session_id,
                                                             n,
                                                             a,
                                                             packet };

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.get_id())
                            ]
                        }
                        else {
                            // ignore when IDLE: old packets
                            vec![]
                        }
                    },
                    _ => panic!("Unexpected packet type for {:?} in {:?}",
                               packet, self)
                }
            },
            _ => panic!("Unexpected message {:?} in {:?}",
                       message, self)
        }
    }
}
