/// static counters for creating unique and traceable objects
use crate::core::NodeId;

use std::sync::atomic::AtomicUsize;

pub static LAST_PKT_ID: AtomicUsize = AtomicUsize::new(1);
pub static LAST_TIMEOUT_ID: AtomicUsize = AtomicUsize::new(1);
pub static LAST_SESSION_ID: AtomicUsize = AtomicUsize::new(1);

pub static LAST_NODE_ID: AtomicUsize = AtomicUsize::new(1);
pub static CONTROLLER_ID: NodeId = NodeId(0);
