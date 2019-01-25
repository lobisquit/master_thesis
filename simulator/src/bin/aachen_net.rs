extern crate env_logger;
extern crate downcast_rs;
extern crate derive_builder;
extern crate simulator;

use simulator::*;

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
