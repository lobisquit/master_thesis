/// static counters for creating unique and traceable objects
use crate::core::NodeAddress;

use std::sync::atomic::AtomicUsize;

pub static LAST_PKT_ID: AtomicUsize = AtomicUsize::new(1);
pub static LAST_TIMEOUT_ID: AtomicUsize = AtomicUsize::new(1);
pub static LAST_SESSION_ID: AtomicUsize = AtomicUsize::new(1);

pub static MAINFRAME_ADDR: NodeAddress = NodeAddress { node_id: 0, component_id: 0 };
