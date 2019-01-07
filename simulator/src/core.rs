use crate::counters::*;

use std::cmp::Ordering;
use std::sync::atomic::Ordering as AtomicOrdering;

#[derive(Debug, Clone)]
pub struct FiniteF32(f32);

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
pub struct NodeId(usize);

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

impl FiniteF32 {
    pub fn new(number: f32) -> Result<FiniteF32, ()> {
        if number.is_finite() {
            Ok(FiniteF32(number))
        }
        else {
            Err(())
        }
    }
}

impl PartialEq for FiniteF32 {
    fn eq(&self, other: &FiniteF32) -> bool {
        self.0 == other.0
    }
}

impl Eq for FiniteF32 {}

impl Ord for FiniteF32 {
    fn cmp(&self, other: &FiniteF32) -> Ordering {
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

impl PartialOrd for FiniteF32 {
    fn partial_cmp(&self, other: &FiniteF32) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub enum Message {
    // data messages
    DataPacket { id: usize, size: u64, source: NodeId },
    GeneratePacket(bool),
    TxPacket

    // control messages
    // ParamRequest  { param: String },
    // ParamResponse { param: String, value: FiniteF32 },
    // ParamSet      { param: String, value: FiniteF32 }
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
    time: FiniteF32,
    msg: Message,
    dest: NodeId
}

impl Event {
    pub fn new(time: f32, msg: Message, dest: NodeId) -> Result<Event, ()> {
        Ok(Event { time: FiniteF32::new(time)?, msg: msg, dest: dest })
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

pub trait Node {
    fn process_message(&mut self, message: Message, current_time: f32) -> Vec<Event>;

    fn get_id(&self) -> NodeId;
}
