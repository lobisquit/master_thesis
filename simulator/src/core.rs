use crate::counters::*;

use std::collections::VecDeque;

use std::cmp::Ordering;
use std::fmt::Debug;
use std::sync::atomic::Ordering as AtomicOrdering;

#[derive(Debug, Clone, Copy)]
pub struct FiniteF64(f64);

impl Into<f64> for FiniteF64 {
    fn into(self) -> f64 {
        self.0
    }
}

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

impl FiniteF64 {
    pub fn new(number: f64) -> Result<FiniteF64, ()> {
        if number.is_finite() {
            Ok(FiniteF64(number))
        }
        else {
            Err(())
        }
    }
}

impl PartialEq for FiniteF64 {
    fn eq(&self, other: &FiniteF64) -> bool {
        self.0 == other.0
    }
}

impl Eq for FiniteF64 {}

impl Ord for FiniteF64 {
    fn cmp(&self, other: &FiniteF64) -> Ordering {
        if self.0 < other.0 {
            Ordering::Less
        }
        else if self.0 > other.0 {
            Ordering::Greater
        }
        else {
            Ordering::Equal
        }
    }
}

impl PartialOrd for FiniteF64 {
    fn partial_cmp(&self, other: &FiniteF64) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub enum PacketType {
    TcpData {
        sequence_num: usize, // sequence number of current packet
        sequence_end: usize, // sequence number of last packet (total)
    },
    UdpData(bool),
    DataRequest,
    DataStop,
    ACK(usize),
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub enum Message {
    // actual packets (on the wire)

    Packet {
        id: usize,   // unique packet ID across all packets
        session: usize,    // current session  ID
        size: u64,
        pkt_type: PacketType,
        creation_time: FiniteF64,
        src_node: NodeId,
        dst_node: NodeId
    },

    // control messages

    UserSwitch(bool),
    UserPageRequest,

    QueueTransmitPacket
}

impl Message {
    pub fn new_packet(session_id: Option<usize>,
                      size: u64,
                      pkt_type: PacketType,
                      current_time: f64,
                      src_node: NodeId,
                      dst_node: NodeId) -> Message {

        let session_id = session_id.unwrap_or(
            LAST_SESSION_ID.fetch_add(1, AtomicOrdering::SeqCst)
        );

        Message::Packet {
            id: LAST_PKT_ID.fetch_add(1, AtomicOrdering::SeqCst),

            session: session_id,

            size: size,
            pkt_type: pkt_type,
            creation_time: FiniteF64::new(current_time).unwrap(),
            src_node: src_node,
            dst_node: dst_node
        }

    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Event {
    pub time: FiniteF64,
    pub msg: Message,
    pub sender: NodeId,
    pub recipient: NodeId
}

impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        other.time.cmp(&self.time)
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Event) -> Option<Ordering> {
        Some(other.time.cmp(&self.time))
    }
}

pub trait Node: Debug {
    fn get_id(&self) -> NodeId;

    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event>;

    fn new_event(&self, time: f64, msg: Message, recipient: NodeId) -> Event {
        Event {
            time:     FiniteF64::new(time).unwrap(),
            msg:      msg,
            sender: self.get_id(),
            recipient: recipient
        }
    }
}
