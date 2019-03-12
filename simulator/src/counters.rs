/// static counters for creating unique and traceable objects
use crate::core::NodeAddress;
use crate::little_graph::NodeId;

use std::sync::atomic::AtomicUsize;

pub static LAST_PKT_ID: AtomicUsize = AtomicUsize::new(1);
pub static LAST_TIMEOUT_ID: AtomicUsize = AtomicUsize::new(1);
pub static LAST_SESSION_ID: AtomicUsize = AtomicUsize::new(1);

// conventions for components ids

pub static MAINFRAME_ID: NodeId = NodeId(0);
pub static CONTROLLER_ADDR: NodeAddress = NodeAddress { node_id: MAINFRAME_ID.0,
                                                       component_id: 0 };
pub static TBF_UPLINK_ID: usize = 10;
pub static NIC_UPLINK_ID: usize = 11;
pub static SWITCH_UPLINK_ID: usize = 12;

pub static TBF_DOWNLINK_ID: usize = 20;
pub static NIC_DOWNLINK_ID: usize = 21;
pub static SWITCH_DOWNLINK_ID: usize = 22;

pub static MIN_CLIENT_ID: usize = 100;

/// Time required by each node to process a single event
pub static PROC_TIME: f64 = 5e-6;
