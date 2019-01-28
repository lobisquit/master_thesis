extern crate simulator;
extern crate rand;

#[macro_use]
extern crate log;
extern crate env_logger;

use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::time::Instant;

use env_logger::{Builder, Env};
use simulator::*;

pub static FLAG: bool = false;

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
        .total_n_packets(30 as usize)
        .mtu_size(10000 as u64)
        .t0(1.0)
        .build()
        .unwrap();

    let mut client_to_server = BlockingQueueBuilder::default()
        .node_id(3)
        .dest_id(2)
        .max_queue(40 as usize)
        .conn_speed(999.0)
        .build()
        .unwrap();

    let mut server_to_client = BlockingQueueBuilder::default()
        .node_id(4)
        .dest_id(1)
        .max_queue(40 as usize)
        .conn_speed(1001.0)
        .build()
        .unwrap();

    let fire_event = Event {
        sender: NodeId(1),
        time: 0.0,
        message: Message::UserSwitchOn,
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
        if !FLAG {
            debug!(" ");
            debug!("{:?}", event);
        }

        n_events += 1;

        let new_events = expand_event(event, &mut nodes);

        if !FLAG {
            for e in &new_events {
                debug!("-> {:?}", e);
            }
        }

        event_queue.extend(new_events);
    }
    let duration = start.elapsed();
    println!("{:?} for each one of the {} events", duration / n_events, n_events);
}

fn expand_event(original_event: Event, nodes: &mut HashMap<NodeId, &mut Node>) -> Vec<Event> {
    if FLAG {
        debug!(" ");
        debug!("{:?}", original_event);
    }

    let Event { time, message, recipient, .. } = original_event;

    let destination = nodes.get_mut(&recipient).unwrap();
    let output_events = destination.process_message(message, time.into());

    if FLAG {
        for e in &output_events {
            debug!("-> {:?}", e);
        }
    }

    output_events.into_iter().flat_map(
        |event| {
            // check if the event is in the same place and time wrt the original one
            if event.recipient == recipient && event.time == time {
                expand_event(event, nodes)
            }
            else {
                vec![event]
            }
        }
    ).collect()
}
