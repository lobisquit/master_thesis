#[macro_use] extern crate log;
extern crate env_logger;

#[macro_use]
extern crate derive_builder;

mod core;
mod counters;
mod queue;
mod switch;
mod udp;

use self::core::*;
use self::udp::*;

fn main() {
    let ptype = PacketType::ACK(44 as usize);

    let x = UdpClient {
        node_id: 0.into(),
        status: udp::UdpClientStatus::default(),
        next_hop_id: 1.into(),
        dst_id: 10.into(),

        bitrate: 10.0,
        t0: 14.0,
        n: 12,
        timeouts: vec![]
    };

    dbg!(x);
}
