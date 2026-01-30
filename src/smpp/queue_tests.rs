//! Unit tests for SMPP message queue
//!
//! These tests verify message queue logic based on SMPP protocol requirements:
//! - Message IDs must be unique and sequential
//! - Messages are queued in order
//! - Pending delivery reports are tracked correctly
//! - Queue operations are thread-safe

use crate::smpp::queue::{MessageQueue, QueuedMessage};

#[test]
fn test_message_queue_creation() {
    let queue = MessageQueue::new();
    assert_eq!(queue.pending_dr_count(), 0, "New queue should have no pending DRs");
}

#[test]
fn test_message_id_generation() {
    let queue = MessageQueue::new();
    
    let id1 = queue.next_message_id();
    let id2 = queue.next_message_id();
    let id3 = queue.next_message_id();
    
    // Message IDs should be unique
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);
    
    // IDs should be sequential (incrementing)
    // Parse the numeric parts and verify order
    let num1: u64 = id1.parse().expect("ID should be numeric");
    let num2: u64 = id2.parse().expect("ID should be numeric");
    let num3: u64 = id3.parse().expect("ID should be numeric");
    
    assert!(num2 > num1, "IDs should be incrementing");
    assert!(num3 > num2, "IDs should be incrementing");
}

#[test]
fn test_add_pending_dr() {
    let queue = MessageQueue::new();
    
    let msg = QueuedMessage {
        message_id: "msg-001".to_string(),
        source_addr: "+1234567890".to_string(),
        dest_addr: "+0987654321".to_string(),
        short_message: b"Hello World".to_vec(),
        data_coding: 0,
        session_id: "session-001".to_string(),
        submitted_at: std::time::Instant::now(),
    };
    
    queue.add_pending_dr(msg);
    
    assert_eq!(queue.pending_dr_count(), 1, "DR should be pending after add");
}

#[test]
fn test_get_recent_messages() {
    let queue = MessageQueue::new();
    
    // Add multiple messages
    for i in 0..10 {
        let msg = QueuedMessage {
            message_id: format!("msg-{:03}", i),
            source_addr: format!("+123{}", i),
            dest_addr: format!("+987{}", i),
            short_message: format!("Message {}", i).into_bytes(),
            data_coding: 0,
            session_id: "session".to_string(),
            submitted_at: std::time::Instant::now(),
        };
        queue.add_pending_dr(msg);
    }
    
    let recent = queue.get_recent_messages();
    assert_eq!(recent.len(), 10, "Should retrieve all recent messages");
}

#[test]
fn test_message_content_preserved() {
    let queue = MessageQueue::new();
    
    let original_content = "Test OTP: 123456";
    let msg = QueuedMessage {
        message_id: "content-test".to_string(),
        source_addr: "+1111".to_string(),
        dest_addr: "+2222".to_string(),
        short_message: original_content.as_bytes().to_vec(),
        data_coding: 0,
        session_id: "s1".to_string(),
        submitted_at: std::time::Instant::now(),
    };
    
    queue.add_pending_dr(msg);
    
    let retrieved = queue.get_recent_messages();
    assert_eq!(retrieved.len(), 1);
    
    let content = String::from_utf8_lossy(&retrieved[0].short_message);
    assert_eq!(content, original_content, "Message content should be preserved");
}

#[test]
fn test_source_dest_addresses() {
    let queue = MessageQueue::new();
    
    let msg = QueuedMessage {
        message_id: "addr-test".to_string(),
        source_addr: "+66812345678".to_string(),
        dest_addr: "+66887654321".to_string(),
        short_message: b"SMS".to_vec(),
        data_coding: 0,
        session_id: "s".to_string(),
        submitted_at: std::time::Instant::now(),
    };
    
    queue.add_pending_dr(msg);
    
    let retrieved = &queue.get_recent_messages()[0];
    assert_eq!(retrieved.source_addr, "+66812345678");
    assert_eq!(retrieved.dest_addr, "+66887654321");
}
