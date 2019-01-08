use crate::counters::*;

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
pub enum Message {
    // data messages
    DataPacket { id: usize, size: u64, source: NodeId },
    TxPacket,

    GeneratePacket,
    StartTx,
    StopTx

    // control messages
    // ParamRequest  { param: String },
    // ParamResponse { param: String, value: FiniteF64 },
    // ParamSet      { param: String, value: FiniteF64 }
}

impl Message {
    pub fn new_packet(size: u64, source: NodeId) -> Message {
        Message::DataPacket {
            id: LAST_PKT_ID.fetch_add(1, AtomicOrdering::SeqCst),
            size: size,
            source: source
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Event {
    pub time: FiniteF64,
    pub msg: Message,
    pub dest: NodeId
}

impl Event {
    pub fn new(time: f64, msg: Message, dest: NodeId) -> Result<Event, ()> {
        Ok(Event { time: FiniteF64::new(time)?, msg: msg, dest: dest })
    }
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
    fn process_message(&mut self, message: Message, current_time: f64) -> Vec<Event>;

    fn get_id(&self) -> NodeId;
}
