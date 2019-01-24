// use crate::core::*;
// use crate::Message::*;

// #[derive(Debug, Clone)]
// pub enum TcpClientStatus {
//     Idle,

//     RequestInit,
//     RequestWait { session_id: usize },

//     DataWait { session_id: usize },
//     DataUpdate { session_id: usize, new_packet: Packet },

//     FinishWait { session_id: usize },
//     Unusable { session_id: usize },
//     Evaluate { session_id: usize, file_size: u64 }
// }

// impl Default for TcpClientStatus {
//     fn default() -> Self {
//         TcpClientStatus::Idle
//     }
// }

// impl TcpClientStatus {
//     fn get_session_id(&self) -> Option<usize> {
//         use TcpClientStatus::*;

//         match *self {
//             Idle => None,

//             RequestInit => None,
//             RequestWait { session_id } => Some(session_id),
//             DataWait { session_id } => Some(session_id),
//             DataUpdate { session_id, .. } => Some(session_id),


//             FinishWait { session_id } => Some(session_id),
//             Unusable { session_id } => Some(session_id),
//             Evaluate { session_id, .. } => Some(session_id)
//         }
//     }
// }

// #[derive(Debug, Builder)]
// #[builder(setter(into))]
// pub struct TcpClient {
//     node_id: NodeId,

//     #[builder(setter(skip))]
//     status: TcpClientStatus,

//     next_hop_id: NodeId,
//     dst_id: NodeId,

//     bitrate: f64,
//     t0: f64,
//     n: u64,

//     #[builder(setter(skip))]
//     timeouts: Vec<usize>
// }

// impl Node for TcpClient {
//     fn get_id(&self) -> NodeId {
//         self.node_id
//     }

//     fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event> {
//         use TcpClientStatus::*;
//         use PacketType::*;

//         // first of all, handle timeouts if they are still active
//         match message {
//             Timeout { expire_message, id } => {
//                 if self.timeouts.contains(&id) {
//                     vec![ self.new_event(current_time,
//                                          *expire_message,
//                                          self.get_id()) ]
//                 }
//                 else {
//                     vec![]
//                 }
//             },
//             MoveToStatus(new_status) => {
//                 if let Some(tcp_status) = new_status.as_any().downcast_ref::<TcpClientStatus>() {
//                     self.status = tcp_status.clone();
//                     match &self.status {
//                         Idle => vec![],

//                         RequestInit => {
//                             // immediately send DATA request
//                             let session_id = Message::new_session_id();

//                             // start longer timeout, after which the ser vice is
//                             // considered unusable
//                             let unusable_timeout = Message::new_timeout(
//                                 MoveToStatus(Box::new( Unusable { session_id } ))
//                             );
//                             self.timeouts.push(
//                                 unusable_timeout.get_id().unwrap()
//                             );
//                             let timeout_delay = self.n as f64 * self.t0;

//                             // immediately send DATA request
//                             let new_status = RequestWait { session_id };
//                             vec![
//                                 self.new_event(current_time,
//                                                MoveToStatus(Box::new(new_status)),
//                                                self.node_id),

//                                 self.new_event(current_time + timeout_delay,
//                                                unusable_timeout,
//                                                self.node_id)
//                             ]
//                         },
//                         RequestWait { session_id } => {
//                             // size in byte of ethernet frame with empty tcp packet
//                             let request_size = 24 * 8;

//                             let pkt_type = TcpDataRequest {
//                                 bitrate: self.bitrate
//                             };
//                             let request = Message::new_packet(*session_id,
//                                                              request_size,
//                                                              pkt_type,
//                                                              current_time,
//                                                              self.node_id,
//                                                              self.dst_id);

//                             // repeat the request after a timeout
//                             let repeat_timeout = Message::new_timeout(
//                                 MoveToStatus(Box::new(self.status.clone()))
//                             );
//                             self.timeouts.push(
//                                 repeat_timeout.get_id().unwrap()
//                             );

//                             vec![
//                                 self.new_event(current_time,
//                                                request,
//                                                self.next_hop_id),
//                                 self.new_event(current_time + self.t0,
//                                                repeat_timeout,
//                                                self.node_id),
//                             ]
//                         },
//                         DataUpdate { session_id, new_packet } => {
//                             // invalidate all previous timeouts: communication is
//                             // still alive
//                             self.timeouts.clear();

//                             // TODO use new_packet to update the metrics
//                             dbg!(new_packet);

//                             let new_status = DataWait {
//                                 session_id: *session_id
//                             };
//                             vec![
//                                 self.new_event(current_time,
//                                                MoveToStatus(Box::new(new_status)),
//                                                self.node_id)
//                             ]
//                         },
//                         DataWait { session_id } => {
//                             let new_status = Unusable {
//                                 session_id: *session_id
//                             };
//                             let unusable_timeout = Message::new_timeout(
//                                 MoveToStatus(Box::new(new_status))
//                             );
//                             self.timeouts.push(
//                                 unusable_timeout.get_id().unwrap()
//                             );

//                             let long_delay = self.n as f64 * self.t0;
//                             vec![ self.new_event(current_time + long_delay,
//                                                  unusable_timeout,
//                                                  self.node_id) ]
//                         },
//                         FinishWait { session_id } => {
//                             // communicate the server that it has to stop sending
//                             // packets
//                             let request_size = 24 * 8;
//                             let request = Message::new_packet(*session_id,
//                                                              request_size,
//                                                              TcpFinishRequest,
//                                                              current_time,
//                                                              self.node_id,
//                                                              self.dst_id);

//                             // repeat the FINISH request after a timeout
//                             let new_status = FinishWait {
//                                 session_id: *session_id
//                             };
//                             let repeat_timeout = Message::new_timeout(
//                                 MoveToStatus(Box::new(new_status))
//                             );
//                             self.timeouts.push(repeat_timeout.get_id().unwrap());

//                             vec![ self.new_event(current_time,
//                                                  request,
//                                                  self.next_hop_id),
//                                   self.new_event(current_time + self.t0,
//                                                  repeat_timeout,
//                                                  self.node_id) ]
//                         },
//                         Unusable { session_id } => {
//                             // invalidate previous timeouts: connection is
//                             // considered dead
//                             self.timeouts.clear();

//                             // TODO mark connection as unusable in metrics
//                             let new_status = FinishWait {
//                                 session_id: *session_id
//                             };
//                             vec![
//                                 self.new_event(current_time,
//                                                MoveToStatus(Box::new(new_status)),
//                                                self.node_id)
//                             ]
//                         },
//                         Evaluate { session_id, file_size } => {
//                             // FINISH packet received: connection is closed
//                             self.timeouts.clear();

//                             // TODO use obtained metrics to compute QoS, QoE
//                             dbg!(session_id);
//                             dbg!(file_size);

//                             vec![ self.new_event(current_time,
//                                                  MoveToStatus(Box::new(Idle)),
//                                                  self.node_id) ]
//                         }
//                     }
//                 }
//                 else {
//                     panic!("Invalid status {:?} for {:?}", new_status, self)
//                 }
//             },
//             // external events
//             UserSwitchOn => {
//                 if let Idle = self.status {
//                     vec![
//                         self.new_event(current_time,
//                                        MoveToStatus(Box::new( RequestInit )),
//                                        self.node_id)
//                     ]
//                 }
//                 else {
//                     panic!("User request in {:?} received while in status {:?}",
//                            self, self.status)
//                 }
//             },
//             UserSwitchOff => {
//                 // stop the server (request FINISH packet) to exit gracefully
//                 match self.status.get_session_id() {
//                     None => vec![],
//                     Some(number) => {
//                         let new_status = FinishWait {
//                             session_id: number
//                         };
//                         vec![ self.new_event(current_time,
//                                              MoveToStatus(Box::new(new_status)),
//                                              self.node_id)
//                         ]
//                     }
//                 }
//             },
//             Data(packet) => {
//                 match self.status.get_session_id() {
//                     // no active connection: this is an old packet,
//                     // received out of order wrt the FINISH packet
//                     None => vec![],

//                     Some(number) => {
//                         if number == packet.session_id {
//                             match packet.pkt_type {
//                                 TcpData => {
//                                     let new_status = DataUpdate {
//                                         session_id: packet.session_id,
//                                         new_packet: packet
//                                     };
//                                     vec![ self.new_event(current_time,
//                                                          MoveToStatus(
//                                                              Box::new(new_status)
//                                                          ),
//                                                          self.node_id) ]
//                                 },
//                                 TcpFinish { file_size }=> {
//                                     let new_status = Evaluate {
//                                         session_id: number,
//                                         file_size: file_size
//                                     };
//                                     vec![ self.new_event(current_time,
//                                                          MoveToStatus(
//                                                              Box::new(new_status)
//                                                          ),
//                                                          self.node_id) ]
//                                 },
//                                 _ => panic!("Unexpected packet {:?} in {:?}",
//                                            packet, self)
//                             }
//                         }
//                         else {
//                             // packet belongs to an old session and arrived after
//                             // its FINISH packet: ignore
//                             vec![]
//                         }
//                     }
//                 }
//             },
//             _ => vec![]
//         }
//     }
// }
