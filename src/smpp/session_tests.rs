//! Unit tests for SMPP session management
//! 
//! These tests verify session management logic based on SMPP protocol requirements:
//! - Sessions must have unique IDs
//! - Sessions track bind type (Transmitter, Receiver, Transceiver)
//! - Sessions can be added, retrieved, and removed
//! - Session count must be accurate

use crate::smpp::session::{Session, SessionManager, BindType};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn test_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345)
}

#[test]
fn test_session_manager_creation() {
    let manager = SessionManager::new();
    assert_eq!(manager.get_all_sessions().len(), 0, "New manager should have no sessions");
}

#[test]
fn test_add_and_get_session() {
    let manager = SessionManager::new();
    let session = Session::new(
        "client1".to_string(),
        BindType::Transceiver,
        test_addr(),
    );
    let session_id = session.id.clone();
    
    manager.add_session(session.clone());
    
    let retrieved = manager.get_session(&session_id);
    assert!(retrieved.is_some(), "Session should be retrievable after adding");
    assert_eq!(retrieved.unwrap().system_id, "client1");
}

#[test]
fn test_remove_session() {
    let manager = SessionManager::new();
    let session = Session::new(
        "client2".to_string(),
        BindType::Transmitter,
        test_addr(),
    );
    let session_id = session.id.clone();
    
    manager.add_session(session);
    assert_eq!(manager.get_all_sessions().len(), 1);
    
    manager.remove_session(&session_id);
    assert_eq!(manager.get_all_sessions().len(), 0, "Session should be removed");
    assert!(manager.get_session(&session_id).is_none(), "Removed session should not be retrievable");
}

#[test]
fn test_multiple_sessions() {
    let manager = SessionManager::new();
    
    // Add multiple sessions with different bind types
    for i in 0..5 {
        let bind_type = match i % 3 {
            0 => BindType::Transmitter,
            1 => BindType::Receiver,
            _ => BindType::Transceiver,
        };
        let session = Session::new(
            format!("client-{}", i),
            bind_type,
            test_addr(),
        );
        manager.add_session(session);
    }
    
    assert_eq!(manager.get_all_sessions().len(), 5, "All sessions should be tracked");
}

#[test]
fn test_session_bind_types() {
    // Verify all bind types are correctly stored
    let manager = SessionManager::new();
    
    let tx_session = Session::new("tx-client".to_string(), BindType::Transmitter, test_addr());
    let rx_session = Session::new("rx-client".to_string(), BindType::Receiver, test_addr());
    let trx_session = Session::new("trx-client".to_string(), BindType::Transceiver, test_addr());
    
    let tx_id = tx_session.id.clone();
    let rx_id = rx_session.id.clone();
    let trx_id = trx_session.id.clone();

    manager.add_session(tx_session);
    manager.add_session(rx_session);
    manager.add_session(trx_session);
    
    let tx = manager.get_session(&tx_id).unwrap();
    let rx = manager.get_session(&rx_id).unwrap();
    let trx = manager.get_session(&trx_id).unwrap();
    
    assert!(matches!(tx.bind_type, BindType::Transmitter));
    assert!(matches!(rx.bind_type, BindType::Receiver));
    assert!(matches!(trx.bind_type, BindType::Transceiver));
}

#[test]
fn test_session_count() {
    let manager = SessionManager::new();
    
    assert_eq!(manager.count(), 0);
    
    let s1 = Session::new("c1".to_string(), BindType::Transceiver, test_addr());
    let s1_id = s1.id.clone();
    manager.add_session(s1);
    assert_eq!(manager.count(), 1);
    
    manager.add_session(Session::new("c2".to_string(), BindType::Transceiver, test_addr()));
    assert_eq!(manager.count(), 2);
    
    manager.remove_session(&s1_id);
    assert_eq!(manager.count(), 1);
}
