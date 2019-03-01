extern crate simulator;
extern crate rand;

#[macro_use]
extern crate log;
extern crate env_logger;

use std::collections::BinaryHeap;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use std::fs::File;
use std::io::{BufReader, BufRead};
use std::result::Result;
use std::num::ParseIntError;

use env_logger::{Builder, Env};
use simulator::*;

pub static FLAG: bool = false;
pub static GRAPH_PATH: &str = "../data/aachen_net/topology.txt";

fn register_node<'a>(node: &'a mut dyn Node,
                     nodes: &mut HashMap<NodeId, &'a mut Node>) {
    nodes.insert(node.get_id(), node);
}

fn read_graph(path: &str) -> Result<Graph, ParseIntError> {
    let input = File::open(path).expect(&format!("{} not found", path));
    let buffered = BufReader::new(input);

    let mut graph = Graph::default();

    for line in buffered.lines() {
        let content = line.unwrap();
        let pieces = content.split(',').collect::<Vec<&str>>();

        let node = pieces[0].parse::<usize>()?;
        let father = pieces[1].parse::<usize>()?;

        if pieces.len() == 3 {
            let n_lines = pieces[2].parse::<u64>()?;
            graph.add_node(node, father, n_lines);
        }
        else {
            graph.add_node(node, father, 0);
        }
    }

    Ok(graph)
}

fn main() {
    let environment = Env::default().default_filter_or("error");
    Builder::from_env(environment).init();

    let mut controller = ControllerBuilder::default().build().unwrap();

    // read network topology structure
    let graph = read_graph(GRAPH_PATH)
        .expect("Error while parsing graph");

    // dbg!(leaves);

    // let mut client = UdpClientBuilder::default()
    //     .node_id(1)
    //     .next_hop_id(5)
    //     .dst_id(2)
    //     .bitrate(10000)
    //     .t0(2.)
    //     .n(10 as u64)
    //     .build()
    //     .unwrap();

    // let mut server = UdpServerBuilder::default()
    //     .node_id(2)
    //     .next_hop_id(6)
    //     .dst_id(1)
    //     .file_size(1e5 as u64)
    //     .mtu_size(1500 as u64)

    //     .build()
    //     .unwrap();

    // let mut client_to_server = BlockingQueueBuilder::default()
    //     .node_id(3)
    //     .dest_id(2)
    //     .max_queue(40 as usize)
    //     .conn_speed(999.0)
    //     .build()
    //     .unwrap();

    // let mut server_to_client = BlockingQueueBuilder::default()
    //     .node_id(4)
    //     .dest_id(1)
    //     .max_queue(40 as usize)
    //     .conn_speed(1001.0)
    //     .build()
    //     .unwrap();

    // let mut client_to_server_tbf = TokenBucketQueueBuilder::default()
    //     .node_id(5)
    //     .dest_id(3)
    //     .build()
    //     .unwrap();

    // controller.register_tbf(client_to_server_tbf.get_id());

    // let mut server_to_client_tbf = TokenBucketQueueBuilder::default()
    //     .node_id(6)
    //     .dest_id(4)
    //     .build()
    //     .unwrap();

    // let new_params = TokenBucketQueueParamsBuilder::default()
    //     .max_queue(14)
    //     .max_tokens(15.0)
    //     .token_rate(25.0)
    //     .build()
    //     .unwrap();

    // let fire_event = Event {
    //     sender: NodeId(0),
    //     time: 0.0,
    //     message: Message::SetParams(new_params),
    //     recipient: server_to_client_tbf.get_id()
    // };

    // controller.register_tbf(server_to_client_tbf.get_id());

    // let mut nodes: HashMap<NodeId, &mut Node> = HashMap::new();
    // register_node(&mut controller, &mut nodes);
    // register_node(&mut client, &mut nodes);
    // register_node(&mut server, &mut nodes);
    // register_node(&mut client_to_server, &mut nodes);
    // register_node(&mut server_to_client, &mut nodes);
    // register_node(&mut client_to_server_tbf, &mut nodes);
    // register_node(&mut server_to_client_tbf, &mut nodes);

    // let mut event_queue: BinaryHeap<Event> = BinaryHeap::new();

    // event_queue.push(fire_event);

    // let start = Instant::now();
    // let mut n_events = 0;

    // while let Some(event) = event_queue.pop() {
    //     if !FLAG {
    //         debug!(" ");
    //         debug!("{:?}", event);
    //     }

    //     n_events += 1;

    //     let new_events = expand_event(event, &mut nodes);

    //     if !FLAG {
    //         for e in &new_events {
    //             debug!("-> {:?}", e);
    //         }
    //     }

    //     event_queue.extend(new_events);
    // }

    // dbg!(nodes);

    // let duration = start.elapsed();
    // if n_events != 0 {
    //     println!("{:?} for each one of the {} events", duration / n_events, n_events);
    // }
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

#[test]
fn test_graph_structure() {
    // read network topology structure
    let graph = read_graph(GRAPH_PATH)
        .expect("Error while parsing graph");

    let leaves = graph.get_leaves();

    let dslams = leaves
        .iter()
        .map(|id| *graph.get_father(*id).unwrap())
        .collect::<HashSet<GraphId>>();

    let routers = dslams
        .iter()
        .map(|id| *graph.get_father(*id).unwrap())
        .collect::<HashSet<GraphId>>();

    for leaf in &leaves {
        let dslam = graph.get_father(*leaf)
            .expect("leaf has no father");

        assert!(!leaves.contains(dslam));

        let router = graph.get_father(*dslam)
            .expect("dslam has no router");

        assert!(!leaves.contains(router));
        assert!(!dslams.contains(router));

        let mainframe = graph.get_father(*router)
            .expect("router has no father");

        assert!(!leaves.contains(mainframe));
        assert!(!dslams.contains(mainframe));
        assert!(!routers.contains(mainframe));

        assert!(graph.get_father(*mainframe) == None);
    }
}
