use crate::core::*;
use crate::Message::*;
use std::cmp::max;

#[derive(Debug, Clone)]
enum TcpServerStatus {
    Idle,
    InitSession,

    TransmitDecide,
    TransmitPacket ,
    TransmitRepeat,
    TransmitWait
}

#[derive(Debug, Default, Clone)]
struct TcpConnParams {
    session_id: usize,
    n: usize,
    a: usize,
    b: usize
}

impl Default for TcpServerStatus {
    fn default() -> Self {
        TcpServerStatus::Idle
    }
}

impl MachineStatus for TcpServerStatus {}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct TcpServer {
    node_id: NodeId,

    #[builder(setter(skip))]
    status: TcpServerStatus,

    #[builder(setter(skip))]
    conn_params: TcpConnParams,

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
                        InitSession => {
                            let new_status = TransmitDecide;

                            self.conn_params.a = 0;
                            self.conn_params.b = 0;

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.get_id())
                            ]
                        },
                        TransmitDecide => {
                            // time to decide: cancel everything else
                            self.timeouts.clear();

                            if self.conn_params.b == self.total_n_packets {
                                // wait if all packets in window have been transmitted
                                vec![
                                    self.new_event(current_time,
                                                   MoveToStatus(Box::new(TransmitWait)),
                                                   self.get_id())
                                ]
                            }
                            else {
                                if self.conn_params.b < self.conn_params.a + self.conn_params.n {
                                    vec![
                                        self.new_event(current_time,
                                                       MoveToStatus(
                                                           Box::new(TransmitPacket)
                                                       ),
                                                       self.get_id())
                                    ]
                                }
                                else {
                                    vec![
                                        self.new_event(current_time,
                                                       MoveToStatus(
                                                           Box::new(TransmitWait)
                                                       ),
                                                       self.get_id())
                                    ]
                                }
                            }
                        },
                        TransmitPacket => {
                            let data_packet = Message::new_packet(
                                self.conn_params.session_id,
                                self.mtu_size,
                                TcpData {
                                    sequence_num: self.conn_params.b,
                                    sequence_end: self.total_n_packets
                                },
                                current_time,
                                self.node_id,
                                self.dst_id
                            );

                            self.conn_params.b = self.conn_params.b + 1;

                            // prepare retransmission
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(TransmitDecide))
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
                        TransmitWait => {
                            let timeout = Message::new_timeout(
                                MoveToStatus(Box::new(TransmitRepeat))
                            );
                            self.timeouts.push(timeout.get_id().unwrap());

                            vec![ self.new_event(current_time + self.t0,
                                                 timeout,
                                                 self.node_id) ]
                        },
                        TransmitRepeat => {
                            // reset sent window, to send unACKed packets again
                            self.conn_params.b = self.conn_params.a;

                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(TransmitDecide)),
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
                        self.conn_params.session_id = packet.session_id;
                        self.conn_params.n = window_size;

                        vec![
                            self.new_event(current_time,
                                           MoveToStatus(Box::new(InitSession)),
                                           self.get_id()) ]
                    },
                    TcpACK { sequence_num, .. } => {
                        if let Idle = self.status {
                            // ignore when connection has ended
                            vec![]
                        }
                        else if self.conn_params.session_id != packet.session_id || sequence_num <= self.conn_params.a {
                            // ignore old ACKs
                            vec![]
                        }
                        else {
                            assert!(sequence_num <= self.total_n_packets);

                            self.conn_params.a = max(self.conn_params.a, sequence_num);
                            self.conn_params.b = max(self.conn_params.b, self.conn_params.a);

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
                                if let TransmitWait = self.status {
                                    // move to decide only when waiting:
                                    // leave the repeat timeout of packet
                                    // transmissions untouched

                                    vec![
                                        self.new_event(current_time,
                                                       MoveToStatus(
                                                           Box::new(TransmitDecide)
                                                       ),
                                                       self.get_id())
                                    ]
                                }
                                else {
                                    vec![]
                                }
                            }
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
