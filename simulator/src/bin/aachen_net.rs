extern crate simulator;
extern crate rand;

#[macro_use]
extern crate log;
extern crate env_logger;

use rand::SeedableRng;
use rand_hc::Hc128Rng;
use rand::distributions::Exp;

// use std::collections::BinaryHeap;
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::time::Instant;

use std::fs::File;
use std::io::{BufReader, BufRead};
use std::result::Result;
use std::num::ParseIntError;

use env_logger::{Builder, Env};
use simulator::*;

pub static GRAPH_PATH: &str = "../data/aachen_net/topology.txt";

fn register_node(node: Box<Node>,
                 nodes: &mut HashMap<NodeAddress, Box<Node>>) {
    nodes.insert(node.get_addr(), node);
}

fn read_graph(path: &str) -> Result<Graph, ParseIntError> {
    let input = File::open(path).expect(&format!("{} not found", path));
    let buffered = BufReader::new(input);

    let mut graph = Graph::default();

    for line in buffered.lines() {

        let err_msg = format!("ERR: {:?}", line);

        let content = line.expect(&err_msg);
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

fn populate_node(node_id: usize,
                 nodes: &mut HashMap<NodeAddress, Box<Node>>,
                 graph: &Graph,
                 controller: &mut Controller,
                 max_queue_uplink: usize,
                 conn_speed_uplink: f64,
                 max_queue_downlink: usize,
                 conn_speed_downlink: f64) {

    // uplink components
    let uplink_tbf = TokenBucketQueueBuilder::default()
        .node_addr(NodeAddress::new(node_id, TBF_UPLINK_ID))
        .dest_addr(NodeAddress::new(node_id, NIC_UPLINK_ID))
        .build()
        .expect("ERR 1");

    controller.register_tbf(uplink_tbf.get_addr());
    register_node(Box::new(uplink_tbf), nodes);

    let err_msg = format!("{}", node_id);
    let father_id: usize = graph.get_father(node_id).expect(&err_msg).into();
    let uplink_nic = BlockingQueueBuilder::default()
        .node_addr(NodeAddress::new(node_id, NIC_UPLINK_ID))
        .dest_addr(NodeAddress::new(father_id, TBF_UPLINK_ID))
        .max_queue(max_queue_uplink)    // bits
        .conn_speed(conn_speed_uplink)  // Mbit/s;
        .build()
        .expect("ERR 3");

    register_node(Box::new(uplink_nic), nodes);

    // downlink components
    let downlink_tbf = TokenBucketQueueBuilder::default()
        .node_addr(NodeAddress::new(node_id, TBF_DOWNLINK_ID))
        .dest_addr(NodeAddress::new(node_id, NIC_DOWNLINK_ID))
        .build()
        .expect("ERR 4");

    controller.register_tbf(downlink_tbf.get_addr());
    register_node(Box::new(downlink_tbf), nodes);

    let downlink_nic = BlockingQueueBuilder::default()
        .node_addr(NodeAddress::new(node_id, NIC_DOWNLINK_ID))
        .dest_addr(NodeAddress::new(node_id, SWITCH_DOWNLINK_ID))
        .max_queue(max_queue_downlink)    // bits
        .conn_speed(conn_speed_downlink)  // Mbit/s;
        .build()
        .expect("ERR 5");

    register_node(Box::new(downlink_nic), nodes);

    let mut downlink_switch = SwitchBuilder::default()
        .node_addr(NodeAddress::new(node_id, SWITCH_DOWNLINK_ID))
        .build()
        .expect("ERR 6");

    // populate routing table
    for (leaf, child) in graph.get_routes(node_id) {
        let next_hop = NodeAddress::new(child.into(), TBF_DOWNLINK_ID);
        downlink_switch.add_route(*leaf, next_hop);
    }

    register_node(Box::new(downlink_switch), nodes);
}

fn main() {
    let environment = Env::default().default_filter_or("error");
    Builder::from_env(environment).init();

    // read network topology structure
    let graph = read_graph(GRAPH_PATH)
        .expect("ERR Error while parsing graph")
        .initialize_routes();

    // randomly generated seed
    let seed: [u8; 32] = [ 99,  8, 83, 32, 34, 69, 53, 54,
                           90, 86, 60, 14, 62, 32, 67, 35,
                           96, 75, 58, 22, 55, 38, 24, 24,
                           85, 85, 14, 96, 11, 38, 85, 64 ];

    let mut controller = ControllerBuilder::default()
        .interarrival( Exp::new(3.0) )
        .rng( Hc128Rng::from_seed(seed) )
        .report_path("prova.txt")
        .build()
        .expect("ERR 7");

    let mut nodes: HashMap<NodeAddress, Box<dyn Node>> = HashMap::new();

    let dslams = graph.get_leaves();

    let routers = dslams
        .iter()
        .map(|dslam| graph.get_father(*dslam).expect("ERR 9"))
        .collect::<HashSet<NodeId>>();

    for dslam in &dslams {
        populate_node(dslam.into(),
                      &mut nodes,
                      &graph,
                      &mut controller,
                      1000, 1e6,
                      1000, 1e6);
    }
    debug!("Initialized DSLAMs");

    for router in routers {
        populate_node(router.into(),
                      &mut nodes,
                      &graph,
                      &mut controller,
                      1000, 1e6,
                      1000, 1e6);
    }
    debug!("Initialized ROUTERS");

    // create (unique) mainframe
    {
        let uplink_nic = BlockingQueueBuilder::default()
            .node_addr(NodeAddress::new(MAINFRAME_ID.into(), TBF_UPLINK_ID))
            .dest_addr(NodeAddress::new(MAINFRAME_ID.into(), SWITCH_UPLINK_ID))
            .max_queue(1000000 as usize) // bits
            .conn_speed(1e5) // Mbit/s;
            .build()
            .expect("ERR 3asd");

        register_node(Box::new(uplink_nic), &mut nodes);

        let uplink_switch = SwitchBuilder::default()
            .node_addr(NodeAddress::new(MAINFRAME_ID.into(), SWITCH_UPLINK_ID))
            .build()
            .expect("ERR 10");

        register_node(Box::new(uplink_switch), &mut nodes);

        let downlink_nic = BlockingQueueBuilder::default()
            .node_addr(NodeAddress::new(MAINFRAME_ID.into(), NIC_DOWNLINK_ID))
            .dest_addr(NodeAddress::new(MAINFRAME_ID.into(), SWITCH_DOWNLINK_ID))
            .max_queue(1000000 as usize) // bits
            .conn_speed(1e5) // Mbit/s;
            .build()
            .expect("ERR 3asd");

        register_node(Box::new(downlink_nic), &mut nodes);

        let mut downlink_switch = SwitchBuilder::default()
            .node_addr(NodeAddress::new(MAINFRAME_ID.into(), SWITCH_DOWNLINK_ID))
            .build()
            .expect("ERR 10a");

        // register all network routes in downlink switch
        for (leaf, child) in graph.get_routes(MAINFRAME_ID) {
            let next_hop = NodeAddress::new(child.into(), TBF_DOWNLINK_ID);
            downlink_switch.add_route(*leaf, next_hop);
        }

        register_node(Box::new(downlink_switch), &mut nodes);

        debug!("Initialized MAINFRAME");
    }

    // create all clients and all servers
    let mut current_id = MIN_CLIENT_ID;

    for dslam in &dslams {
        let client_next_hop = NodeAddress::new(dslam.into(),
                                              TBF_UPLINK_ID);

        let server_next_hop = NodeAddress::new(MAINFRAME_ID.into(),
                                              NIC_DOWNLINK_ID);

        if let Some(n_lines) = graph.get_weight(*dslam) {
            for _ in 0..*n_lines {
                {
                    let server_address = NodeAddress::new(MAINFRAME_ID.into(),
                                                          current_id.into());

                    let client_address = NodeAddress::new(dslam.into(),
                                                         current_id.into());

                    let server = UdpServerBuilder::default()
                        .node_addr(server_address)
                        .next_hop_addr(server_next_hop)
                        .dst_addr(client_address)
                        .file_size(10000 as u64)
                        .mtu_size(1000 as u64)
                        .build()
                        .expect("ERR 11");
                    register_node(Box::new(server), &mut nodes);

                    // create n_lines users per DSLAM and the corresponding server
                    let client = UdpClientBuilder::default()
                        .node_addr(client_address)
                        .next_hop_addr(client_next_hop)
                        .dst_addr(server_address)
                        .bitrate(1000)
                        .t0(2.0)
                        .n(5 as u64)
                        .build()
                        .expect("12");
                    register_node(Box::new(client), &mut nodes);

                    current_id += 1;
                }
                {
                    let server_address = NodeAddress::new(MAINFRAME_ID.into(),
                                                          current_id.into());

                    let client_address = NodeAddress::new(dslam.into(),
                                                          current_id.into());

                    let server = TcpServerBuilder::default()
                        .node_addr(server_address)
                        .next_hop_id(server_next_hop)
                        .dst_addr(client_address)
                        .total_n_packets(10000 as usize)
                        .mtu_size(1000 as u64)
                        .t0(10)
                        .build()
                        .expect("13");

                    register_node(Box::new(server), &mut nodes);

                    // create n_lines users per DSLAM and the corresponding server
                    let client = TcpClientBuilder::default()
                        .node_addr(client_address)
                        .next_hop_addr(client_next_hop)
                        .dst_addr(server_address)
                        .window_size(100 as usize)
                        .t_repeat(5.0)
                        .t_unusable(10.0)
                        .expected_plt(5.0)
                        .build()
                        .expect("14");

                    register_node(Box::new(client), &mut nodes);

                    current_id += 1;
                }
            }
        }
    }
    debug!("Initialized CLIENTs and SERVERs");

    register_node(Box::new(controller), &mut nodes);
    debug!("Initialized CONTROLLER");

    // create fire events for all clients
    let mut event_queue: BinaryHeap<Event> = BinaryHeap::new();
    for (node_id, node) in nodes.iter() {
        let fire_event = Event {
            sender: CONTROLLER_ADDR, // does not matter here
            time: 0.0,
            message: Message::UserSwitchOn,
            recipient: *node_id
        };

        // add event to queue if client
        if let Some(_) = node.downcast_ref::<UdpClient>() {
            event_queue.push(fire_event);
            break;
        }
        else if let Some(_) = node.downcast_ref::<TcpClient>() {
            event_queue.push(fire_event);
            break;
        }
    }

    // start event loop
    let start = Instant::now();
    let mut n_events = 0;

    let detailed_debug = false;
    while let Some(event) = event_queue.pop() {
        if n_events % 1000000 == 0 {
            info!("Reached {}", n_events);
        }

        if !detailed_debug {
            debug!(" ");
            debug!("{:?}", event);
        }

        n_events += 1;

        let new_events = expand_event(event, &mut nodes, detailed_debug);

        if !detailed_debug {
            for e in &new_events {
                debug!("-> {:?}", e);
            }
        }

        event_queue.extend(new_events);
    }

    let duration = start.elapsed();
    if n_events != 0 {
        println!("{:?} for each one of the {} events",
                 duration / n_events,
                 n_events);
    }
}

fn expand_event(original_event: Event,
                nodes: &mut HashMap<NodeAddress, Box<Node>>,
                detailed_debug: bool) -> Vec<Event> {
    if detailed_debug {
        debug!(" ");
        debug!("{:?}", original_event);
    }

    let Event { time, message, recipient, .. } = original_event;

    let destination = match nodes.get_mut(&recipient) {
        Some(node) => node,
        None => panic!("No such node {:?}", recipient)
    };
    let output_events = destination.process_message(message, time.into());

    if detailed_debug {
        for e in &output_events {
            debug!("-> {:?}", e);
        }
    }

    output_events.into_iter().flat_map(
        |event| {
            // check if the event is in the same place and time wrt the original one
            if event.recipient == recipient && event.time == time {
                expand_event(event, nodes, detailed_debug)
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
        .map(|id| *graph.get_father(*id).expect("16"))
        .collect::<HashSet<NodeId>>();

    let routers = dslams
        .iter()
        .map(|id| *graph.get_father(*id).expect("17"))
        .collect::<HashSet<NodeId>>();

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

        assert!(*mainframe == 0.into());

        assert!(!leaves.contains(mainframe));
        assert!(!dslams.contains(mainframe));
        assert!(!routers.contains(mainframe));

        assert!(graph.get_father(*mainframe) == None);
    }

    for router in routers {
        if let Some(n_lines) = graph.get_weight(router) {
            panic!();
        }
    }

    for building in graph.get_leaves() {
        if let Some(n_lines) = graph.get_weight(building) {
            panic!();
        }
    }
}
