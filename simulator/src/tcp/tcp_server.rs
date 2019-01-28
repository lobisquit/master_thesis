use crate::core::*;
use crate::Message::*;
use std::cmp::max;

#[derive(Debug, Clone)]
enum TcpServerStatus {
    Idle,
    InitSession { session_id: usize, n: usize },

    TransmitDecide { session_id: usize, n: usize, a: usize, b: usize },
    TransmitPacket { session_id: usize, n: usize, a: usize, b: usize },
    TransmitRepeat { session_id: usize, n: usize, a: usize, b: usize },
    TransmitWait   { session_id: usize, n: usize, a: usize, b: usize }
}

impl Default for TcpServerStatus {
    fn default() -> Self {
        TcpServerStatus::Idle
    }
}

impl MachineStatus for TcpServerStatus {}

impl TcpServerStatus {
    fn get_conn_params(&self) -> Option<[usize; 4]> {
        use TcpServerStatus::*;

        match *self {
            Idle => None,
            InitSession { .. } => None,
            TransmitDecide { session_id, n, a, b, .. } => Some([session_id, n, a, b]),
            TransmitPacket { session_id, n, a, b, .. } => Some([session_id, n, a, b]),
            TransmitRepeat { session_id, n, a, b, .. } => Some([session_id, n, a, b]),
            TransmitWait   { session_id, n, a, b, .. } => Some([session_id, n, a, b])
        }
    }
}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct TcpServer {
    node_id: NodeId,

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
                        Idle => {
                            self.timeouts.clear();
                            vec![]
                        },
                        InitSession { session_id, n } => {
                            let new_status = TransmitDecide {
                                session_id: session_id,
                                n: n,
                                a: 0,
                                b: 0
                            };

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.get_id())
                            ]
                        },
                        TransmitDecide { session_id, n, a, b } => {
                            // time to decide: cancel everything else
                            self.timeouts.clear();

                            if b == self.total_n_packets {
                                // wait if all packets have been transmitted
                                let new_status = TransmitWait { session_id,
                                                                n,
                                                                a,
                                                                b };

                                vec![
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(new_status)),
                                                   self.get_id())
                                ]
                            }
                            else {
                                if b < a + n {
                                    let new_status = TransmitPacket {
                                        session_id,
                                        n,
                                        a,
                                        b
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
                                                                    a,
                                                                    b };

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
                        TransmitPacket { session_id, n, a, b } => {
                            let data_packet = Message::new_packet(
                                session_id,
                                self.mtu_size,
                                TcpData {
                                    sequence_num: b,
                                    sequence_end: self.total_n_packets
                                },
                                current_time,
                                self.node_id,
                                self.dst_id
                            );

                            // prepare retransmission
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(
                                    TransmitDecide { session_id,
                                                     n,
                                                     a,
                                                     b: b + 1 }
                                ))
                            );
                            self.timeouts.push(repeat_timeout.get_id().unwrap());

                            // update state, in case an ACK invalidates the
                            // upgrade timeout
                            self.status = TransmitPacket { session_id, n, a,
                                                           b: b + 1 };

                            vec![
                                self.new_event(current_time + self.t0 / 2.,
                                               repeat_timeout,
                                               self.get_id()),

                                self.new_event(current_time,
                                               data_packet,
                                               self.next_hop_id)
                            ]
                        },
                        TransmitWait { session_id, n, a, b } => {
                            let timeout = Message::new_timeout(
                                MoveToStatus(Box::new(
                                    TransmitRepeat { session_id, n, a, b }
                                ))
                            );
                            self.timeouts.push(timeout.get_id().unwrap());

                            vec![ self.new_event(current_time + self.t0,
                                                 timeout,
                                                 self.node_id) ]
                        },
                        TransmitRepeat { session_id, n, a, .. } => {
                            // reset sent window, to send unACKed packets again
                            let new_status = TransmitDecide { session_id,
                                                              n,
                                                              a,
                                                              b: a };

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
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
                assert!(packet.dst_node == self.node_id);
                assert!(packet.src_node == self.dst_id);

                info!("{}: {:?} received by {:?}", current_time, packet, self);

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
                    TcpACK { sequence_num, .. } => {
                        if let Some([session_id, n, a, b]) = self.status.get_conn_params() {
                            if session_id != packet.session_id || sequence_num <= a {
                                // ignore old ACKs
                                vec![]
                            }
                            else {
                                assert!(sequence_num <= self.total_n_packets);

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
                                    if let TransmitWait { .. } = self.status {
                                        // move to decide only when waiting:
                                        // leave the repeat timeout of packet
                                        // transmissions untouched

                                        let new_status = TransmitDecide {
                                            session_id,
                                            n,
                                            a: max(a, sequence_num),
                                            b: max(b, a)
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
                                        vec![]
                                    }
                                }
                            }
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
