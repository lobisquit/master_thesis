use crate::utils::mean;
use crate::core::*;
use crate::Message::*;
use crate::counters::CORE_ID;
use crate::utility::*;

static PKT_LOSS_LIMIT: f64 = 5e-2;
static PKT_LOSS_TOLERANCE: f64 = 1e-2;
static PKT_LOSS_MARGIN: f64 = 0.95;

static AVG_DELAY_LIMIT: f64 = 4.0; // s
static AVG_DELAY_TOLERANCE: f64 = 1.0; // s
static AVG_DELAY_MARGIN: f64 = 0.95; // s

#[derive(Debug, Clone)]
pub enum UdpClientStatus {
    Idle,

    RequestInit,
    RequestWait { session_id: usize },

    DataWait { session_id: usize },
    DataUpdate { session_id: usize, new_packet: Packet },

    FinishWait { session_id: usize },
    Unusable { session_id: usize },
    Evaluate { session_id: usize, file_size: u64 }
}

impl Default for UdpClientStatus {
    fn default() -> Self {
        UdpClientStatus::Idle
    }
}

impl MachineStatus for UdpClientStatus {}

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
            Evaluate { session_id, .. } => Some(session_id)
        }
    }
}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct UdpClient {
    node_id: NodeId,

    next_hop_id: NodeId,
    dst_id: NodeId,

    bitrate: f64,
    t0: f64,
    n: u64,

    #[builder(setter(skip))]
    delays: Vec<f64>,

    #[builder(setter(skip))]
    received_data: u64,

    #[builder(setter(skip))]
    status: UdpClientStatus,

    #[builder(setter(skip))]
    timeouts: Vec<usize>,

    #[builder(setter(skip))]
    starting_time: f64
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
                if let Some(udp_status) = new_status.downcast_ref::<UdpClientStatus>() {
                    // move to the specified status and apply its operations
                    self.status = udp_status.clone();

                    match self.status {
                        Idle => {
                            self.delays.clear();
                            self.received_data = 0;
                            self.timeouts.clear();

                            vec![]
                        },

                        RequestInit => {
                            // register start of current session
                            self.starting_time = current_time;

                            // immediately send DATA request
                            let session_id = Message::new_session_id();

                            // start longer timeout, after which the ser vice is
                            // considered unusable
                            let unusable_timeout = Message::new_timeout(
                                MoveToStatus(Box::new( Unusable { session_id } ))
                            );
                            self.timeouts.push(
                                unusable_timeout.get_id().unwrap()
                            );
                            let timeout_delay = self.n as f64 * self.t0;

                            // immediately send DATA request
                            let new_status = RequestWait { session_id };
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
                            let request = Message::new_packet(session_id,
                                                             request_size,
                                                             pkt_type,
                                                             current_time,
                                                             self.node_id,
                                                             self.dst_id);

                            // repeat the request after a timeout
                            let repeat_timeout = Message::new_timeout(
                                MoveToStatus(Box::new(self.status.clone()))
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

                            // remember packet delay
                            let delay = current_time - new_packet.creation_time;
                            self.delays.push(delay);

                            // mark the portion of data as received
                            self.received_data += new_packet.size;

                            let new_status = DataWait {
                                session_id: session_id
                            };
                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.node_id)
                            ]
                        },
                        DataWait { session_id } => {
                            let new_status = Unusable {
                                session_id: session_id
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
                            let request = Message::new_packet(session_id,
                                                             request_size,
                                                             UdpFinishRequest,
                                                             current_time,
                                                             self.node_id,
                                                             self.dst_id);

                            // repeat the FINISH request after a timeout
                            let new_status = FinishWait {
                                session_id: session_id
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
                                session_id: session_id
                            };
                            vec![
                                self.new_event(current_time,
                                               MoveToStatus(Box::new(new_status)),
                                               self.node_id)
                            ]
                        },
                        Evaluate { file_size, .. } => {
                            // FINISH packet received: connection is closed
                            self.timeouts.clear();

                            let pkt_loss = 1.0 -
                                self.received_data as f64 /
                                file_size as f64;

                            let avg_delay = mean(&self.delays);

                            // let throughput = file_size  as f64 /
                            //     (current_time - self.starting_time);

                            let current_utility =
                                utility(pkt_loss,
                                        PKT_LOSS_LIMIT + PKT_LOSS_TOLERANCE,
                                        PKT_LOSS_TOLERANCE,
                                        PKT_LOSS_MARGIN) *
                                utility(avg_delay,
                                        AVG_DELAY_LIMIT + AVG_DELAY_TOLERANCE,
                                        AVG_DELAY_TOLERANCE,
                                        AVG_DELAY_MARGIN);

                            vec![ self.new_event(current_time,
                                                 MoveToStatus(Box::new(Idle)),
                                                 self.node_id),

                                  self.new_event(current_time,
                                                 ReportUtility(current_utility),
                                                 CORE_ID) ]
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
                    vec![ self.new_event(current_time,
                                         MoveToStatus(Box::new( RequestInit )),
                                         self.node_id) ]
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
                                             self.node_id) ]
                    }
                }
            },
            Data(packet) => {
                info!("{}: {:?} received by {:?}", current_time, packet, self);

                assert!(packet.dst_node == self.node_id);
                assert!(packet.src_node == self.dst_id);

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
                                UdpFinish { file_size }=> {
                                    let new_status = Evaluate {
                                        session_id: number,
                                        file_size: file_size
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
