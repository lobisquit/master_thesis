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

use env_logger::{Builder, Env};

fn main() {
    let environment = Env::default().default_filter_or("error");
    Builder::from_env(environment).init();


    let mut sink = SimpleSinkBuilder::default().build().unwrap();

    let mut tbq = TokenBucketQueueBuilder::default()
        .dest_id(sink.get_id())
        .max_queue(10 as usize)
        .max_tokens(10000.)
        .conn_speed(1200.)
        .token_rate(1000.)
        .build().unwrap();

    let mut source = DeterministicSourceBuilder::default()
        .delta_t(1.)
        .dest_id(tbq.get_id())
        .packet_size(1000 as u64)
        .build()
        .unwrap();

    let fire_event = Event::new(0.,
                               Message::StartTx,
                               source.get_id()).unwrap();

    let stop_event = Event::new(3.,
                               Message::StopTx,
                               source.get_id()).unwrap();

    let mut nodes: HashMap<NodeId, &mut Node> = HashMap::new();
    println!("{:?}", sink);
    println!("{:?}", source);
    println!("{:?}", tbq);

    nodes.insert(sink.get_id().clone(), &mut sink);
    nodes.insert(source.get_id().clone(), &mut source);
    nodes.insert(tbq.get_id().clone(), &mut tbq);

    let mut event_queue: BinaryHeap<Event> = BinaryHeap::new();
    event_queue.push(fire_event);
    event_queue.push(stop_event);

    while let Some(event) = event_queue.pop() {
        println!("\n\nCurrent event: {:?}", event);

        let Event { time, msg, dest } = event;

        // println!("{:?}", time);

        let destination = nodes.get_mut(&dest).unwrap();
        let new_events = destination.process_message(msg, time.into());

        println!("\n\nNew events:");
        for event in &new_events {
            println!("\n - {:?}", event);
        }

        event_queue.extend(new_events);

        let t: f32 = time.into();
        if t > 3. {
            ::std::process::exit(1)
        }
        // println!("{:?}", event_queue);
    }
    // println!("{:?}", n);
    // println!("{:?}", nu);
}
