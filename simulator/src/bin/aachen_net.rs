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
use std::fs::remove_file;
use std::io::{BufReader, BufRead};
use std::result::Result;
use std::num::ParseIntError;

use env_logger::{Builder, Env};
use simulator::*;

static GRAPH_PATH: &str = "../data/aachen_net/topology.txt";

fn main() {
    // simulation parameters

    let interarrival_seed: Hc128Rng = Hc128Rng::from_seed(
        [ 99,  8, 83, 32, 34, 69, 53, 54,
          90, 86, 60, 14, 62, 32, 67, 35,
          96, 75, 58, 22, 55, 38, 24, 24,
          85, 85, 14, 96, 11, 38, 85, 64 ]
    );

    let interarrival_distr: Exp = Exp::new(3.0);

    let dslam_upload_speed: f64 = 200e6; // bit/s
    let dslam_upload_buffer: usize = (3e6 * 8.0) as usize; // bits

    let dslam_download_speed: f64 = 1e9; // bit/s
    let dslam_download_buffer: usize = (13e6 * 8.0) as usize; // bits

    let router_upload_speed: f64 = 2e9; // bit/s
    let router_upload_buffer: usize = (3e6 * 8.0) as usize; // bits

    let router_download_speed: f64 = 10e9; // bit/s
    let router_download_buffer: usize = (13e6 * 8.0) as usize; // bits

    let mainframe_upload_speed: f64 = 2e9; // bit/s
    let mainframe_upload_buffer: usize = (12e6 * 8.0) as usize; // bits

    let mainframe_download_speed: f64 = 10e9; // bit/s
    let mainframe_download_buffer: usize = (52e6 * 8.0) as usize; // bits

    let udp_server_file_size: u64 = 1e8 as u64; // bit
    let udp_server_mtu_size: u64 = 548 * 8 as u64; // bit

    let udp_client_bitrate: f64 = 10e6; // bit/s
    let udp_client_timeout: f64 = 5.0; // s
    let udp_client_n_timeouts: u64 = 3;

    let tcp_server_mtu_size: u64 = 512 * 8; // bits
    let tcp_server_n_packets: usize = (1e6 / tcp_server_mtu_size as f64) as usize;
    let tcp_server_first_rtt: f64 = 1.0; // s

    let tcp_client_window: usize = 2000; // n packets
    let tcp_client_t_repeat: f64 = 2.0; // s
    let tcp_client_t_unusable: f64 = 20.0; // s
    let tcp_client_expected_plt: f64 = 5.0; // s

    // setup logs

    let environment = Env::default().default_filter_or("error");
    Builder::from_env(environment).init();

    // read graph

    let graph = read_graph(GRAPH_PATH)
        .expect("ERR Error while parsing graph")
        .initialize_routes();

    let report_path = "results/current.csv";

    // clean path if file already exists
    match remove_file(report_path) {
        Ok(_) => debug!("Report file \"{}\" removed", report_path),
        Err(_) => debug!("Path \"{}\" already clear", report_path)
    }

    // create nodes

    let mut controller = ControllerBuilder::default()
        .interarrival(interarrival_distr)
        .rng(interarrival_seed)
        .report_path(report_path)
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
                      dslam_upload_buffer,
                      dslam_upload_speed,
                      dslam_download_buffer,
                      dslam_download_speed);
    }
    debug!("Initialized {} DSLAMs", dslams.len());

    for router in &routers {
        populate_node(router.into(),
                      &mut nodes,
                      &graph,
                      &mut controller,
                      router_upload_buffer,
                      router_upload_speed,
                      router_download_buffer,
                      router_download_speed);
    }
    debug!("Initialized {} ROUTERs", routers.len());

    // create (unique) mainframe
    {
        let uplink_nic = BlockingQueueBuilder::default()
            .node_addr(NodeAddress::new(MAINFRAME_ID.into(), TBF_UPLINK_ID))
            .dest_addr(NodeAddress::new(MAINFRAME_ID.into(), SWITCH_UPLINK_ID))
            .max_queue(mainframe_upload_buffer)
            .conn_speed(mainframe_upload_speed)
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
            .max_queue(mainframe_download_buffer)
            .conn_speed(mainframe_download_speed)
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

    let mut total_n_lines = 0;
    for dslam in &dslams {
        let client_next_hop = NodeAddress::new(dslam.into(),
                                              TBF_UPLINK_ID);

        let server_next_hop = NodeAddress::new(MAINFRAME_ID.into(),
                                              NIC_DOWNLINK_ID);

        if let Some(n_lines) = graph.get_weight(*dslam) {
            for _ in 0..*n_lines {
                total_n_lines += n_lines;

                {
                    let server_address = NodeAddress::new(MAINFRAME_ID.into(),
                                                          current_id.into());

                    let client_address = NodeAddress::new(dslam.into(),
                                                         current_id.into());

                    let server = UdpServerBuilder::default()
                        .node_addr(server_address)
                        .next_hop_addr(server_next_hop)
                        .dst_addr(client_address)
                        .file_size(udp_server_file_size)
                        .mtu_size(udp_server_mtu_size)
                        .build()
                        .expect("ERR 11");
                    register_node(Box::new(server), &mut nodes);

                    // create n_lines users per DSLAM and the corresponding server
                    let client = UdpClientBuilder::default()
                        .node_addr(client_address)
                        .next_hop_addr(client_next_hop)
                        .dst_addr(server_address)
                        .bitrate(udp_client_bitrate)
                        .t0(udp_client_timeout)
                        .n(udp_client_n_timeouts)
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
                        .total_n_packets(tcp_server_n_packets)
                        .mtu_size(tcp_server_mtu_size)
                        .t0(tcp_server_first_rtt)
                        .build()
                        .expect("13");

                    register_node(Box::new(server), &mut nodes);

                    // create n_lines users per DSLAM and the corresponding server
                    let client = TcpClientBuilder::default()
                        .node_addr(client_address)
                        .next_hop_addr(client_next_hop)
                        .dst_addr(server_address)
                        .window_size(tcp_client_window)
                        .t_repeat(tcp_client_t_repeat)
                        .t_unusable(tcp_client_t_unusable)
                        .expected_plt(tcp_client_expected_plt)
                        .build()
                        .expect("14");

                    register_node(Box::new(client), &mut nodes);

                    current_id += 1;
                }
            }
        }
    }
    debug!("Initialized {} CLIENTs and SERVERs", total_n_lines);

    register_node(Box::new(controller), &mut nodes);
    debug!("Initialized CONTROLLER");

    // create fire events for all clients
    let mut event_queue: BinaryHeap<Event> = BinaryHeap::new();
    for (node_id, node) in &nodes {
        let fire_event = Event {
            sender: CONTROLLER_ADDR, // does not matter here
            time: 0.0,
            message: Message::UserSwitchOn,
            recipient: *node_id
        };

        // add event to queue if client
        if let Some(_) = node.downcast_ref::<UdpClient>() {
            continue;
            event_queue.push(fire_event);
        }
        else if let Some(_) = node.downcast_ref::<TcpClient>() {
            event_queue.push(fire_event);
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
