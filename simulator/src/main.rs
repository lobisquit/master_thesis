#[macro_use] extern crate log;
extern crate env_logger;

#[macro_use]
extern crate derive_builder;

mod core;
mod counters;
mod queue;
mod switch;
mod tcp;

use self::core::*;
use self::tcp::*;

fn main() {
    // let x = TcpClientBuilder::default()
    //     .node_id(4)
    //     .next_hop_id(12)
    //     .dst_id(64)
    //     .bitrate(10.0)
    //     .t0(14.0)
    //     .n(12 as u64)
    //     .build()
    //     .unwrap();

    // dbg!(x);

    let y = TcpServerBuilder::default()
        .node_id(0)
        .next_hop_id(0)
        .dst_id(0)
        .total_n_packets(0 as usize)
        .mtu_size(0 as u64)
        .t0(0)
        .build()
        .unwrap();

    dbg!(y);

}
