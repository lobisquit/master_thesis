/// static counters for creating unique and traceable objects
use std::sync::atomic::AtomicUsize;

pub static LAST_PKT_ID: AtomicUsize = AtomicUsize::new(0);
pub static LAST_NODE_ID: AtomicUsize = AtomicUsize::new(0);
