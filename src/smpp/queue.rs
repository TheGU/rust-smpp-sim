use std::sync::atomic::{AtomicU32, Ordering};
use dashmap::DashMap;
use serde::Serialize;

#[allow(dead_code)]
use tokio::sync::mpsc;

/// Represents a submitted message in the queue
#[derive(Debug, Clone, Serialize)]
pub struct QueuedMessage {
    pub message_id: String,
    pub source_addr: String,
    pub dest_addr: String,
    #[serde(skip)]
    pub short_message: Vec<u8>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub data_coding: u8,
    pub session_id: String,
    #[serde(skip)]
    #[allow(dead_code)]
    pub submitted_at: std::time::Instant,
}

/// Thread-safe message queue for outbound messages (MT -> Delivery Reports)
pub struct MessageQueue {
    /// Messages pending delivery reports
    pending_dr: DashMap<String, QueuedMessage>,
    /// All received messages for display
    all_messages: DashMap<String, QueuedMessage>,
    /// Counter for generating message IDs
    message_id_counter: AtomicU32,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            pending_dr: DashMap::new(),
            all_messages: DashMap::new(),
            message_id_counter: AtomicU32::new(1),
        }
    }

    /// Generate a unique message ID
    pub fn next_message_id(&self) -> String {
        let id = self.message_id_counter.fetch_add(1, Ordering::SeqCst);
        format!("{:08X}", id)
    }

    /// Add a message to both queues
    pub fn add_pending_dr(&self, msg: QueuedMessage) {
        self.all_messages.insert(msg.message_id.clone(), msg.clone());
        self.pending_dr.insert(msg.message_id.clone(), msg);
    }

    /// Get recent messages for display (latest 50)
    pub fn get_recent_messages(&self) -> Vec<QueuedMessage> {
        self.all_messages.iter().map(|r| r.value().clone()).collect()
    }

    /// Get total message count
    #[allow(dead_code)]
    pub fn message_count(&self) -> usize {
        self.all_messages.len()
    }

    /// Get count of pending delivery reports
    pub fn pending_dr_count(&self) -> usize {
        self.pending_dr.len()
    }

    /// Get all pending messages (snapshot for lifecycle processing)
    pub fn get_pending_messages(&self) -> Vec<QueuedMessage> {
        self.pending_dr.iter().map(|r| r.value().clone()).collect()
    }

    /// Remove a message from the pending DR queue
    pub fn remove_pending_dr(&self, message_id: &str) {
        self.pending_dr.remove(message_id);
    }
}

/// Queue for MO (Mobile Originated) messages to be delivered to ESMEs
#[allow(dead_code)]
pub struct MoMessageQueue {
    /// Channel sender for broadcasting MO messages
    tx: mpsc::Sender<MoMessage>,
    /// Channel receiver (will be cloned for subscribers)
    rx: std::sync::Mutex<Option<mpsc::Receiver<MoMessage>>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MoMessage {
    pub source_addr: String,
    pub dest_addr: String,
    pub short_message: String,
}

#[allow(dead_code)]
impl MoMessageQueue {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self {
            tx,
            rx: std::sync::Mutex::new(Some(rx)),
        }
    }

    /// Inject an MO message (from web UI or test)
    pub async fn inject(&self, msg: MoMessage) -> Result<(), mpsc::error::SendError<MoMessage>> {
        self.tx.send(msg).await
    }

    /// Take the receiver (can only be called once)
    pub fn take_receiver(&self) -> Option<mpsc::Receiver<MoMessage>> {
        self.rx.lock().ok()?.take()
    }
    
    /// Get a clone of the sender for injection
    pub fn get_sender(&self) -> mpsc::Sender<MoMessage> {
        self.tx.clone()
    }
}
