//! Unit tests for SMPP server logic
//!
//! These tests verify the server's response to various SMPP PDUs.
//! We mock the environment by creating a local AppConfig, SessionManager, and MessageQueue.

use crate::smpp::server::handle_command;
use crate::config::AppConfig;
use crate::smpp::session::{Session, SessionManager, BindType};
use crate::smpp::queue::MessageQueue;
use tokio::sync::mpsc;
use rusmpp::{
    Command, Pdu, CommandStatus,
    pdus::{
        BindTransmitter,
    },
    types::{COctetString},
    values::{InterfaceVersion, Ton, Npi},
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

fn test_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345)
}


fn test_config() -> AppConfig {
    AppConfig {
        server: crate::config::ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
        },
        smpp: crate::config::SmppConfig {
            system_id: "user".to_string(),
            password: "pass".to_string(),
            port: 2775,
            max_sessions: 50,
            accounts: vec![],
            version: "5.0".to_string(),
        },
        log: crate::config::LogConfig {
            level: "info".to_string(),
        },
        lifecycle: crate::config::LifecycleConfig::default(),
        mo_service: crate::config::MoServiceConfig::default(),
    }
}

#[tokio::test]
async fn test_bind_transmitter_success() {
    let config = test_config();
    let session_manager = SessionManager::new();
    let message_queue = MessageQueue::new();
    let mut current_session_id: Option<String> = None;
    let (tx, _rx) = mpsc::channel(1);

    let bind_req = BindTransmitter::new(
        COctetString::from_str("user").unwrap(),
        COctetString::from_str("pass").unwrap(),
        COctetString::empty(),
        InterfaceVersion::Smpp5_0, // Not Option
        Ton::Unknown,
        Npi::Unknown,
        COctetString::empty(),
    );

    // Command::new(status, sequence_number, pdu)
    let command = Command::new(CommandStatus::EsmeRok, 1, Pdu::BindTransmitter(bind_req));

    // Arguments: command, config, session_manager, message_queue, current_session_id, remote_addr, sender
    let resp = handle_command(
        &command,
        &config,
        &session_manager,
        &message_queue,
        &mut current_session_id,
        test_addr(),
        tx
    ).await;

    assert!(resp.is_some());
    let resp_cmd = resp.unwrap();
    assert_eq!(resp_cmd.status, CommandStatus::EsmeRok);
    
    if let Some(Pdu::BindTransmitterResp(body)) = resp_cmd.pdu() {
        assert_eq!(body.system_id.to_string(), "user");
    } else {
        panic!("Expected BindTransmitterResp, got {:?}", resp_cmd.pdu());
    }

    assert!(current_session_id.is_some());
    let session = session_manager.get_session(&current_session_id.unwrap());
    assert!(session.is_some());
    assert!(matches!(session.unwrap().bind_type, BindType::Transmitter));
}

#[tokio::test]
async fn test_bind_failure_bad_creds() {
    let config = test_config();
    let session_manager = SessionManager::new();
    let message_queue = MessageQueue::new();
    let mut current_session_id: Option<String> = None;
    let (tx, _rx) = mpsc::channel(1);

    let bind_req = BindTransmitter::new(
        COctetString::from_str("bad").unwrap(),
        COctetString::from_str("bad").unwrap(),
        COctetString::empty(),
        InterfaceVersion::Smpp5_0,
        Ton::Unknown,
        Npi::Unknown,
        COctetString::empty(),
    );
    
    let command = Command::new(CommandStatus::EsmeRok, 2, Pdu::BindTransmitter(bind_req));

    let resp = handle_command(
        &command,
        &config,
        &session_manager,
        &message_queue,
        &mut current_session_id,
        test_addr(),
        tx
    ).await;

    assert!(resp.is_some());
    let resp_cmd = resp.unwrap();
    assert_eq!(resp_cmd.status, CommandStatus::EsmeRbindfail);
    assert!(current_session_id.is_none());
}

#[tokio::test]
async fn test_enquire_link() {
    let config = test_config();
    let session_manager = SessionManager::new();
    let message_queue = MessageQueue::new();
    let mut current_session_id: Option<String> = None;
    let (tx, _rx) = mpsc::channel(1);

    let command = Command::new(CommandStatus::EsmeRok, 3, Pdu::EnquireLink);
    
    let resp = handle_command(
        &command,
        &config,
        &session_manager,
        &message_queue,
        &mut current_session_id,
        test_addr(),
        tx
    ).await;
    
    assert!(resp.is_some());
    let resp_cmd = resp.unwrap();
    assert_eq!(resp_cmd.status, CommandStatus::EsmeRok);
    assert!(matches!(resp_cmd.pdu(), Some(Pdu::EnquireLinkResp)));
}

#[tokio::test]
async fn test_unbind() {
    let config = test_config();
    let session_manager = SessionManager::new();
    let message_queue = MessageQueue::new();
    let (tx, _rx) = mpsc::channel(1);
    
    // Manually create session
    let session = Session::new("test_sys".to_string(), BindType::Transmitter, test_addr(), tx.clone(), None);
    let sid = session.id.clone();
    session_manager.add_session(session);
    let mut current_session_id: Option<String> = Some(sid.clone());
    
    let command = Command::new(CommandStatus::EsmeRok, 4, Pdu::Unbind);
    
    let resp = handle_command(
        &command,
        &config,
        &session_manager,
        &message_queue,
        &mut current_session_id,
        test_addr(),
        tx
    ).await;
    
    assert!(resp.is_some());
    let resp_cmd = resp.unwrap();
    assert_eq!(resp_cmd.status, CommandStatus::EsmeRok);
    assert!(matches!(resp_cmd.pdu(), Some(Pdu::UnbindResp)));
    
    assert!(current_session_id.is_none());
    assert!(session_manager.get_session(&sid).is_none());
}
