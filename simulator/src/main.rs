#[macro_use] extern crate log;
extern crate env_logger;

#[macro_use]
extern crate derive_builder;

mod core;
mod counters;
mod token_bucket_queue;

use self::core::*;
use self::token_bucket_queue::*;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::time::Instant;

use env_logger::{Builder, Env};

fn main() {
    let ptype = PacketType::ACK(44 as usize);


    let x = Message::new_packet(Some(1),
                               100,
                               ptype,
                               0.0,
                               NodeId(10),
                               NodeId(10));
    dbg!(x);
}
