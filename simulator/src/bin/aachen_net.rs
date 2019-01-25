extern crate simulator;

#[macro_use]
extern crate log;
extern crate env_logger;

use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::time::Instant;

use env_logger::{Builder, Env};
use simulator::*;

fn main() {
    let environment = Env::default().default_filter_or("error");
    Builder::from_env(environment).init();

    let mut client = TcpClientBuilder::default()
        .node_id(1)
        .next_hop_id(3)
        .dst_id(2)
        .window_size(10 as usize)
        .t_repeat(2.0)
        .t_unusable(20.0)
        .build()
        .unwrap();

    let mut server = TcpServerBuilder::default()
        .node_id(2)
        .next_hop_id(4)
        .dst_id(1)
        .total_n_packets(3 as usize)
        .mtu_size(1000 as u64)
        .t0(1.0)
        .build()
        .unwrap();

    let mut client_to_server = BlockingQueueBuilder::default()
        .node_id(3)
        .dest_id(2)
        .max_queue(100 as usize)
        .conn_speed(1000.0)
        .build()
        .unwrap();

    let mut server_to_client = BlockingQueueBuilder::default()
        .node_id(4)
        .dest_id(1)
        .max_queue(100 as usize)
        .conn_speed(1000.0)
        .build()
        .unwrap();

    let fire_event = Event {
        time: 0.0,
        msg: Message::UserSwitchOn,
        recipient: NodeId(1)
    };

    let mut nodes: HashMap<NodeId, &mut Node> = HashMap::new();
    nodes.insert(client.get_id(), &mut client);
    nodes.insert(server.get_id(), &mut server);
    nodes.insert(client_to_server.get_id(), &mut client_to_server);
    nodes.insert(server_to_client.get_id(), &mut server_to_client);

    let mut event_queue: BinaryHeap<Event> = BinaryHeap::new();
    event_queue.push(fire_event);

    let start = Instant::now();
    let mut n_events = 0;

    while let Some(event) = event_queue.pop() {
        n_events += 1;

        debug!(" ");
        debug!("{:?}", event);

        let Event { time, msg, recipient } = event;

        let destination = nodes.get_mut(&recipient).unwrap();
        let new_events = destination.process_message(msg, time.into());

        for event in &new_events {
            debug!("-> {:?}", event);
        }

        event_queue.extend(new_events);
    }
    println!("{:?}", start.elapsed() / n_events);
}
