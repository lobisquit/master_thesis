#[macro_use] extern crate log;
extern crate env_logger;

mod sink;
mod core;
mod counters;
mod source;
mod token_bucket_queue;

use self::core::*;
use std::collections::BinaryHeap;

use env_logger::{Builder, Env};
use log::Level;

fn main() {
    let environment = Env::default().default_filter_or("debug");
    Builder::from_env(environment).init();

    let mut event_queue: BinaryHeap<Event> = BinaryHeap::new();

    let msg1 = Message::TxPacket;
    let msg2 = Message::TxPacket;

    let e1 = Event::new(0.1, msg1, 1).unwrap();
    let e2 = Event::new(0.2, msg2, 2).unwrap();

    event_queue.push(e1);
    event_queue.push(e2);

    while let Some(event) = event_queue.pop() {
        println!("{:?}", event);
    }
}
