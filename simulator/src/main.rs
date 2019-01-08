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
use self::source::*;
use self::sink::*;
use self::token_bucket_queue::*;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::time::Instant;

use env_logger::{Builder, Env};

fn main() {
    let environment = Env::default().default_filter_or("error");
    Builder::from_env(environment).init();

    let mut sink = SimpleSinkBuilder::default().build().unwrap();

    let mut tbq = TokenBucketQueueBuilder::default()
        .dest_id(sink.get_id())
        .max_queue(10 as usize)
        .max_tokens(10000.)
        .tokens(1100.)
        .conn_speed(1200.)
        .token_rate(500.)
        .build().unwrap();

    let mut source = DeterministicSourceBuilder::default()
        .delta_t(1.)
        .dest_id(tbq.get_id())
        .packet_size(1000 as u64)
        .conn_speed(1200.)
        .build()
        .unwrap();

    let fire_event = Event::new(0.,
                               Message::StartTx,
                               source.get_id()).unwrap();

    let stop_event = Event::new(100000.,
                               Message::StopTx,
                               source.get_id()).unwrap();

    let mut nodes: HashMap<NodeId, &mut Node> = HashMap::new();
    println!("{:?}", sink);
    println!("{:?}", source);
    println!("{:?}", tbq);

    nodes.insert(sink.get_id(),   &mut sink);
    nodes.insert(source.get_id(), &mut source);
    nodes.insert(tbq.get_id(),    &mut tbq);

    let mut event_queue: BinaryHeap<Event> = BinaryHeap::new();
    event_queue.push(fire_event);
    event_queue.push(stop_event);

    let start = Instant::now();
    let mut n_events = 0;

    while let Some(event) = event_queue.pop() {
        n_events += 1;

        debug!("\nCurrent event: {:?}", event);

        let Event { time, msg, dest } = event;

        let destination = nodes.get_mut(&dest).unwrap();
        let new_events = destination.process_message(msg, time.into());

        debug!("New events:");
        for event in &new_events {
            debug!("- {:?}", event);
        }

        event_queue.extend(new_events);

        // let t: f64 = time.into();
        // if t > 3. {
        //     ::std::process::exit(1)
        // }
    }
    println!("{:?}", start.elapsed() / n_events);
}
