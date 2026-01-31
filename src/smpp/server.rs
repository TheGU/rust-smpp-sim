use crate::config::AppConfig;
use std::sync::Arc;
use std::str::FromStr;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use rusmpp::{tokio_codec::CommandCodec, Command, Pdu, CommandStatus};
use rusmpp::types::COctetString;
use futures::{SinkExt, StreamExt};
use crate::smpp::session::{Session, SessionManager, BindType};
use crate::smpp::queue::{MessageQueue, QueuedMessage};

pub async fn start_smpp_server(
    config: Arc<AppConfig>,
    session_manager: Arc<SessionManager>,
    message_queue: Arc<MessageQueue>,
) -> std::io::Result<()> {
    let addr = format!("0.0.0.0:{}", config.smpp.port);
    let listener = TcpListener::bind(&addr).await?;
    
    tracing::info!("SMPP Server started/listening on {}", addr);

    loop {
        let (socket, _) = listener.accept().await?;
        let config_clone = config.clone();
        let session_manager = session_manager.clone();
        let message_queue = message_queue.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, config_clone, session_manager, message_queue).await {
                tracing::error!("Connection error: {}", e);
            }
        });
    }
}

use tokio::sync::mpsc;

async fn handle_connection(socket: TcpStream, config: Arc<AppConfig>, session_manager: Arc<SessionManager>, message_queue: Arc<MessageQueue>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let remote_addr = socket.peer_addr()?;
    tracing::info!("New connection from {}", remote_addr);

    // Use rusmpp codec for framing
    let framed = Framed::new(socket, CommandCodec::new());
    let (mut sink, mut stream) = framed.split();
    
    // Channel for sending PDUs from other parts of the application (e.g. LifecycleManager) to this socket
    let (tx, mut rx) = mpsc::channel(100);

    // Track current session ID if authenticated
    let mut current_session_id: Option<String> = None;

    loop {
        tokio::select! {
            // Handle incoming PDU from client
            Some(command_result) = stream.next() => {
                match command_result {
                    Ok(command) => {
                        tracing::debug!("Received Command from {}: {:?}", remote_addr, command);
                        
                        // Pass tx.clone() so handle_command can give it to a new Session
                        if let Some(resp) = handle_command(&command, &config, &session_manager, &message_queue, &mut current_session_id, remote_addr, tx.clone()).await {
                            sink.send(resp).await?;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error decoding PDU from {}: {}", remote_addr, e);
                        break;
                    }
                }
            }
            // Handle outgoing PDU from server (e.g. Delivery Receipt)
            Some(command) = rx.recv() => {
                tracing::debug!("Sending async Command to {}: {:?}", remote_addr, command);
                sink.send(command).await?;
            }
            else => break,
        }
    }
    
    if let Some(session_id) = current_session_id {
        session_manager.remove_session(&session_id);
        tracing::info!("Session {} disconnected", session_id);
    }

    tracing::info!("Connection closed for {}", remote_addr);
    Ok(())
}

fn authenticate(system_id: &str, password: &str, config: &AppConfig) -> bool {
    // Check default account
    if system_id == config.smpp.system_id && password == config.smpp.password {
        return true;
    }
    // Check additional accounts
    for account in &config.smpp.accounts {
        if system_id == account.system_id && password == account.password {
            return true;
        }
    }
    false
}

pub(crate) async fn handle_command(
    command: &Command, 
    config: &AppConfig, 
    session_manager: &SessionManager,
    message_queue: &MessageQueue,
    current_session_id: &mut Option<String>,
    remote_addr: std::net::SocketAddr,
    sender: mpsc::Sender<Command>,
) -> Option<Command> {
    if let Some(pdu_ref) = command.pdu() {
        let pdu = pdu_ref.clone();
        match pdu {
            Pdu::BindTransmitter(req) => {
                tracing::info!("BindTransmitter: {:?}", req);
                // AUTH CHECK
                if authenticate(&req.system_id.to_string(), &req.password.to_string(), config) {
                     let address_range = if req.address_range.to_string().is_empty() { None } else { Some(req.address_range.to_string()) };
                     let session = Session::new(req.system_id.to_string(), BindType::Transmitter, remote_addr, sender, address_range);
                     *current_session_id = Some(session.id.clone());
                     session_manager.add_session(session);
                     
                     Some(Command::builder()
                        .status(CommandStatus::EsmeRok)
                        .sequence_number(command.sequence_number())
                        .pdu(Pdu::BindTransmitterResp(rusmpp::pdus::BindTransmitterResp::new(
                            req.system_id.clone(),
                            Some(rusmpp::values::InterfaceVersion::Smpp5_0),
                        )))
                     )
                } else {
                    tracing::warn!("Auth failed for system_id: {}", req.system_id.to_string());
                    Some(Command::builder()
                        .status(CommandStatus::EsmeRbindfail)
                        .sequence_number(command.sequence_number())
                        .pdu(Pdu::BindTransmitterResp(rusmpp::pdus::BindTransmitterResp::new(
                             req.system_id.clone(),
                             None
                        )))
                    )
                }
            }
            Pdu::BindReceiver(req) => {
                tracing::info!("BindReceiver: {:?}", req);
                 // AUTH CHECK
                if authenticate(&req.system_id.to_string(), &req.password.to_string(), config) {
                     let address_range = if req.address_range.to_string().is_empty() { None } else { Some(req.address_range.to_string()) };
                     let session = Session::new(req.system_id.to_string(), BindType::Receiver, remote_addr, sender, address_range);
                     *current_session_id = Some(session.id.clone());
                     session_manager.add_session(session);

                    Some(Command::builder()
                        .status(CommandStatus::EsmeRok)
                        .sequence_number(command.sequence_number())
                        .pdu(Pdu::BindReceiverResp(rusmpp::pdus::BindReceiverResp::new(
                            req.system_id.clone(),
                            Some(rusmpp::values::InterfaceVersion::Smpp5_0),
                        )))
                     )
                } else {
                     Some(Command::builder()
                        .status(CommandStatus::EsmeRbindfail)
                        .sequence_number(command.sequence_number())
                        .pdu(Pdu::BindReceiverResp(rusmpp::pdus::BindReceiverResp::new(
                             req.system_id.clone(),
                             None
                        )))
                    )
                }
            }
            Pdu::BindTransceiver(req) => {
                tracing::info!("BindTransceiver: {:?}", req);
                 // AUTH CHECK
                if authenticate(&req.system_id.to_string(), &req.password.to_string(), config) {
                     let address_range = if req.address_range.to_string().is_empty() { None } else { Some(req.address_range.to_string()) };
                     let session = Session::new(req.system_id.to_string(), BindType::Transceiver, remote_addr, sender, address_range);
                     *current_session_id = Some(session.id.clone());
                     session_manager.add_session(session);
                     
                     Some(Command::builder()
                        .status(CommandStatus::EsmeRok)
                        .sequence_number(command.sequence_number())
                        .pdu(Pdu::BindTransceiverResp(rusmpp::pdus::BindTransceiverResp::new(
                            req.system_id.clone(),
                            Some(rusmpp::values::InterfaceVersion::Smpp5_0),
                        )))
                     )
                } else {
                     Some(Command::builder()
                        .status(CommandStatus::EsmeRbindfail)
                        .sequence_number(command.sequence_number())
                        .pdu(Pdu::BindTransceiverResp(rusmpp::pdus::BindTransceiverResp::new(
                             req.system_id.clone(),
                             None
                        )))
                    )
                }
            }
            Pdu::SubmitSm(req) => {
                // Check if session is bound
                if current_session_id.is_none() {
                    tracing::warn!("SubmitSM without bound session");
                    return Some(Command::builder()
                        .status(CommandStatus::EsmeRinvbndsts)
                        .sequence_number(command.sequence_number())
                        .pdu(Pdu::SubmitSmResp(rusmpp::pdus::SubmitSmResp::new(
                            COctetString::from_str("").unwrap_or_default(),
                            vec![],
                        )))
                    );
                }
                
                let message_id = message_queue.next_message_id();
                tracing::info!("SubmitSM: message_id={}, dest={}", message_id, req.destination_addr.to_string());
                
                // Queue the message for potential delivery report
                let queued_msg = QueuedMessage {
                    message_id: message_id.clone(),
                    source_addr: req.source_addr.to_string(),
                    dest_addr: req.destination_addr.to_string(),
                    short_message: req.short_message().as_ref().to_vec(),
                    data_coding: 0, // TODO: extract from req.data_coding
                    session_id: current_session_id.clone().unwrap_or_default(),
                    submitted_at: std::time::Instant::now(),
                };
                message_queue.add_pending_dr(queued_msg);
                
                Some(Command::builder()
                    .status(CommandStatus::EsmeRok)
                    .sequence_number(command.sequence_number())
                    .pdu(Pdu::SubmitSmResp(rusmpp::pdus::SubmitSmResp::new(
                        COctetString::from_str(&message_id).unwrap_or_default(),
                        vec![],
                    )))
                )
            }
            Pdu::EnquireLink => {
                tracing::debug!("EnquireLink");
                 Some(Command::builder()
                    .status(CommandStatus::EsmeRok)
                    .sequence_number(command.sequence_number())
                    .pdu(Pdu::EnquireLinkResp)
                 )
            }
            Pdu::Unbind => {
                 tracing::info!("Unbind");
                 if let Some(sid) = current_session_id.take() {
                     session_manager.remove_session(&sid);
                 }
                 
                 Some(Command::builder()
                    .status(CommandStatus::EsmeRok)
                    .sequence_number(command.sequence_number())
                    .pdu(Pdu::UnbindResp)
                 )   
            }
            _ => {
                tracing::warn!("Unhandled Command: {:?}", command);
                None
            }
        }
    } else {
        tracing::warn!("Command without PDU: {:?}", command);
        None
    }
}
