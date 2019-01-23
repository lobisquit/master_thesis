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

    let x = UdpClientBuilder::default()
        .node_id(4)
        .next_hop_id(12)
        .dst_id(64)
        .bitrate(10.0)
        .t0(14.0)
        .n(12 as u64)
        .build()
        .unwrap();

    dbg!(x);

    let y = UdpServerBuilder::default()
        .node_id(15)
        .next_hop_id(17)
        .dst_id(18)
        .file_size(14000 as u64)
        .mtu_size(53 as u64)
        .build()
        .unwrap();

    dbg!(y);

}
