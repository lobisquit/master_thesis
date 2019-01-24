use crate::counters::*;
use std::any::Any;

use std::cmp::Ordering;
use std::fmt::Debug;
use std::sync::atomic::Ordering as AtomicOrdering;

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

pub trait MachineStatus : Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static + Debug> MachineStatus for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}


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
    pub time: f64,
    pub msg: Message,
    pub sender: NodeId,
    pub recipient: NodeId
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
        other.time.partial_cmp(&self.time)
    }
}

pub trait Node: Debug {
    fn get_id(&self) -> NodeId;

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event>;

    fn new_event(&self, time: f64, msg: Message, recipient: NodeId) -> Event {
        Event {
            time:     time,
            msg:      msg,
            sender: self.get_id(),
            recipient: recipient
        }
    }

    fn handle_timeout(&mut self, time: f64, message: Message) -> Vec<Event> {
        if let Message::Timeout { expire_message, .. } = message {
            vec![ self.new_event(time, *expire_message, self.get_id()) ]
        }
        else {
            panic!("Timeout message expected, got {:?}", message)
        }
    }
}
