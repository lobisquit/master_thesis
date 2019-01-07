use crate::counters::*;

use std::cmp::Ordering;
use std::sync::atomic::Ordering as AtomicOrdering;

#[derive(Debug)]
pub struct FiniteF32(f32);

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

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Message {
    // data messages
    DataPacket { id: usize, size: u64, source: usize },
    GeneratePacket,
    TxPacket

    // control messages
    // ParamRequest  { param: String },
    // ParamResponse { param: String, value: FiniteF32 },
    // ParamSet      { param: String, value: FiniteF32 }
}

impl Message {
    pub fn new_packet(size: u64, source: usize) -> Message {
        Message::DataPacket {
            id: LAST_PKT_ID.fetch_add(1, AtomicOrdering::SeqCst),
            size: size,
            source: source
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Event {
    time: FiniteF32,
    msg: Message,
    dest: usize
}

impl Event {
    pub fn new(time: f32, msg: Message, dest: usize) -> Result<Event, ()> {
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
    fn get_id(&self) -> usize;
}
