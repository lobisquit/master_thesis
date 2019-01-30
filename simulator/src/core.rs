use crate::counters::*;

use std::cmp::Ordering;
use std::fmt::Debug;
use std::sync::atomic::Ordering as AtomicOrdering;
use downcast_rs::Downcast;
use std::collections::VecDeque;
use std::iter::Iterator;

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct NodeId(pub usize);

impl Into<usize> for &NodeId {
    fn into(self) -> usize {
        self.0
    }
}

impl Into<usize> for NodeId {
    fn into(self) -> usize {
        self.0
    }
}

impl Into<NodeId> for usize {
    fn into(self) -> NodeId {
        NodeId(self)
    }
}

impl Default for NodeId {
    fn default() -> NodeId {
        NodeId( LAST_NODE_ID.fetch_add(1, AtomicOrdering::SeqCst) )
    }
}

pub trait MachineStatus : Debug + Downcast {}
impl_downcast!(MachineStatus);

#[derive(Debug, Clone, Copy)]
pub enum PacketType {
    TcpDataRequest {
        window_size: usize
    },
    TcpData {
        sequence_num: usize, // sequence number of current packet
        sequence_end: usize, // sequence number of last packet (total)
    },
    TcpACK {
        sequence_num: usize
    },
    UdpDataRequest { bitrate: f64 },
    UdpData,
    UdpFinishRequest,
    UdpFinish { file_size: u64 },
    DataStop
}

#[derive(Debug, Clone, Copy)]
pub struct Packet {
    id: usize,            // unique packet ID across all packets
    pub session_id: usize,    // current session  ID
    pub size: u64,
    pub pkt_type: PacketType,
    pub creation_time: f64,
    pub src_node: NodeId,
    pub dst_node: NodeId
}

#[derive(Debug)]
pub enum Message {
    // actual packets (on the wire)

    Data(Packet),

    // control messages

    Timeout { expire_message: Box<Message>, id: usize },

    UserSwitchOn,
    UserSwitchOff,

    MoveToStatus(Box<MachineStatus>),

    QueueTransmitPacket
}

impl Message {
    pub fn new_session_id() -> usize {
        LAST_SESSION_ID.fetch_add(1, AtomicOrdering::SeqCst)
    }

    pub fn new_packet(session_id: usize,
                      size: u64,
                      pkt_type: PacketType,
                      current_time: f64,
                      src_node: NodeId,
                      dst_node: NodeId) -> Message {

        Message::Data(Packet {
            id: LAST_PKT_ID.fetch_add(1, AtomicOrdering::SeqCst),

            session_id: session_id,
            size: size,
            pkt_type: pkt_type,
            creation_time: current_time,
            src_node: src_node,
            dst_node: dst_node
        })
    }

    pub fn new_timeout(expire_message: Message) -> Message {
        Message::Timeout {
            id: LAST_TIMEOUT_ID.fetch_add(1, AtomicOrdering::SeqCst),
            expire_message: Box::new(expire_message)
        }
    }

    pub fn get_id(&self) -> Option<usize> {
        match self {
            Message::Data(Packet { id, .. }) => Some(*id),
            Message::Timeout { id, .. } => Some(*id),
            _ => None
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub recipient: NodeId,
    pub time: f64,
    pub message: Message,
    pub sender: NodeId,
}

impl Eq for Event {}

impl PartialEq for Event {
    fn eq(&self, other: &Event) -> bool {
        other.time == self.time
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        other.time.partial_cmp(&self.time).unwrap()
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Event) -> Option<Ordering> {
        let time_order = other.time.partial_cmp(&self.time)
            .expect("NaN in events times?");

        if let Ordering::Equal = time_order {
            // in doubt, give less priority to external data packets
            if let Message::Data(_) = self.message {
                return Some(Ordering::Less);
            }
            if let Message::Data(_) = other.message {
                return Some(Ordering::Greater);
            }
        }

        Some(time_order)
    }
}

pub trait Node: Debug {
    fn get_id(&self) -> NodeId;

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event>;

    fn new_event(&self, time: f64, message: Message, recipient: NodeId) -> Event {
        Event {
            time: time,
            message: message,
            sender: self.get_id(),
            recipient: recipient
        }
    }
}

pub fn median<'a, T: Iterator<Item=&'a f64>>(data: T) -> f64 {
    let mut data_vec: Vec<f64> = data.map(|x| *x).collect();
    data_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mid = data_vec.len() / 2;
    if data_vec.len() % 2 == 0 {
        (data_vec[mid - 1] + data_vec[mid]) / 2.0
    } else {
        data_vec[mid]
    }
}
