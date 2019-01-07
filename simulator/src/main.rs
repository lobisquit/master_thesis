#[macro_use] extern crate log;
extern crate env_logger;

#[macro_use]
extern crate derive_builder;

mod sink;
mod core;
mod counters;
mod source;
mod token_bucket_queue;

use self::core::*;
use self::token_bucket_queue::*;
use std::collections::BinaryHeap;

use env_logger::{Builder, Env};
fn main() {
    let environment = Env::default().default_filter_or("debug");
    Builder::from_env(environment).init();

    let mut event_queue: BinaryHeap<Event> = BinaryHeap::new();

    let msg1 = Message::TxPacket;
    let msg2 = Message::GeneratePacket(true);

    let tbq1 = TokenBucketQueueBuilder::default()
        .dest_id(0)
        .max_queue(0 as usize)
        .max_tokens(10.)
        .conn_speed(0.)
        .token_rate(0.)
        .build().unwrap();

    let tbq2 = TokenBucketQueueBuilder::default()
        .dest_id(0)
        .max_queue(0 as usize)
        .max_tokens(0.)
        .conn_speed(0.)
        .token_rate(0.)
        .build().unwrap();

    let e1 = Event::new(0.1, msg1, tbq1.get_id()).unwrap();
    let e2 = Event::new(0.2, msg2, tbq2.get_id()).unwrap();

    println!("{:?}", tbq1);
    println!("{:?}", tbq2);

    event_queue.push(e1);
    event_queue.push(e2);

    let n = NodeId::default();
    let nu: usize = (&n).into();

    println!("{:?}", n);
    println!("{:?}", nu);

    while let Some(event) = event_queue.pop() {
        println!("{:?}", event);
    }
}
