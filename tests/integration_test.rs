use rust_smpp_sim::config::{AppConfig, SmppConfig, ServerConfig, LogConfig, LifecycleConfig, MoServiceConfig};
use rust_smpp_sim::smpp::server::start_smpp_server;
use rust_smpp_sim::smpp::session::SessionManager;
use rust_smpp_sim::smpp::queue::MessageQueue;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::time::Duration;
use rusmpp::{
    tokio_codec::CommandCodec, Command, Pdu, CommandStatus,
    pdus::{BindTransmitter, SubmitSm},
    values::{InterfaceVersion, Ton, Npi, PriorityFlag, ServiceType, ReplaceIfPresentFlag, RegisteredDelivery, DataCoding, EsmClass},
    types::{COctetString, OctetString, EmptyOrFullCOctetString},
};
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};
use std::str::FromStr;

#[tokio::test]
async fn test_smpp_flow() {
    // Setup Configuration
    let port = 2777; // Use a different port to be safe
    let system_id = "testsys";
    let password = "pass";

    let config = Arc::new(AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8082,
        },
        smpp: SmppConfig {
            system_id: system_id.to_string(),
            password: password.to_string(),
            port: port,
            max_sessions: 10,
            accounts: vec![],
            version: "5.0".to_string(),
        },
        log: LogConfig {
            level: "info".to_string(),
        },
        lifecycle: LifecycleConfig::default(),
        mo_service: MoServiceConfig::default(),
    });

    let session_manager = Arc::new(SessionManager::new());
    let message_queue = Arc::new(MessageQueue::new());

    // Start Server
    let server_config = config.clone();
    let server_session_manager = session_manager.clone();
    let server_message_queue = message_queue.clone();

    tokio::spawn(async move {
        start_smpp_server(server_config, server_session_manager, server_message_queue).await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Connect Client
    let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await.expect("Failed to connect");
    let mut framed = Framed::new(stream, CommandCodec::new());

    // 1. Bind Transmitter
    let bind_req = Command::builder()
        .status(CommandStatus::EsmeRok)
        .sequence_number(1)
        .pdu(Pdu::BindTransmitter(BindTransmitter::new(
            COctetString::from_str(system_id).unwrap(),
            COctetString::from_str(password).unwrap(),
            COctetString::from_str("").unwrap(),
            InterfaceVersion::Smpp5_0,
            Ton::Unknown,
            Npi::Unknown,
            COctetString::from_str("").unwrap(),
        )));

    framed.send(bind_req).await.expect("Failed to send BindTransmitter");

    let resp = framed.next().await.expect("Stream closed").expect("Decoding error");
    match resp.pdu() {
        Some(Pdu::BindTransmitterResp(r)) => {
            assert_eq!(resp.status(), CommandStatus::EsmeRok);
            assert_eq!(r.system_id.to_string(), system_id);
        }
        _ => panic!("Expected BindTransmitterResp, got {:?}", resp),
    }

    // 2. Submit SM
    // Using default() for wrapped types or try from_str where applicable
    let submit_req = Command::builder()
        .status(CommandStatus::EsmeRok)
        .sequence_number(2)
        .pdu(Pdu::SubmitSm(SubmitSm::new(
            ServiceType::default(),
            Ton::Unknown,
            Npi::Unknown,
            COctetString::from_str("source").unwrap(),
            Ton::Unknown,
            Npi::Unknown,
            COctetString::from_str("dest").unwrap(),
            EsmClass::default(),
            0, // ProtocolId (u8)
            PriorityFlag::default(),
            EmptyOrFullCOctetString::from_str("").unwrap(), // ScheduleDeliveryTime
            EmptyOrFullCOctetString::from_str("").unwrap(), // ValidityPeriod
            RegisteredDelivery::default(),
            ReplaceIfPresentFlag::default(),
            DataCoding::default(),
            0, // SmDefaultMsgId (u8)
            OctetString::from_str("Hello Rust").unwrap(),
            vec![],
        )));

    framed.send(submit_req).await.expect("Failed to send SubmitSm");

    let resp = framed.next().await.expect("Stream closed").expect("Decoding error");
    match resp.pdu() {
        Some(Pdu::SubmitSmResp(r)) => {
            assert_eq!(resp.status(), CommandStatus::EsmeRok);
            assert!(!r.message_id().to_string().is_empty());
        }
        _ => panic!("Expected SubmitSmResp, got {:?}", resp),
    }

    // 3. Unbind
    let unbind_req = Command::builder()
        .status(CommandStatus::EsmeRok)
        .sequence_number(3)
        .pdu(Pdu::Unbind);

    framed.send(unbind_req).await.expect("Failed to send Unbind");

    let resp = framed.next().await.expect("Stream closed").expect("Decoding error");
    match resp.pdu() {
        Some(Pdu::UnbindResp) => {
            assert_eq!(resp.status(), CommandStatus::EsmeRok);
        }
        _ => panic!("Expected UnbindResp, got {:?}", resp),
    }

    // Verify Session is gone
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(session_manager.get_all_sessions().len(), 0);
}
