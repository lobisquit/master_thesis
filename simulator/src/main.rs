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
    let x = TcpClientBuilder::default()
        .node_id(0)
        .next_hop_id(0)
        .dst_id(0)
        .window_size(0 as usize)
        .t_repeat(0.9)
        .t_unusable(0.10)
        .build()
        .unwrap();

    dbg!(x);

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
