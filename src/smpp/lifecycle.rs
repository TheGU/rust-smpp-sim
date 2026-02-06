use std::sync::Arc;
use tokio::time::{sleep, Duration};
use rand::Rng;
use crate::config::AppConfig;
use crate::smpp::session::{SessionManager, BindType};
use crate::smpp::queue::{MessageQueue, QueuedMessage};
use rusmpp::{Command, Pdu, CommandStatus};
use rusmpp::types::{COctetString, OctetString, EmptyOrFullCOctetString};
use rusmpp::values::{
    Ton, Npi, EsmClass, PriorityFlag, RegisteredDelivery, ReplaceIfPresentFlag, DataCoding,
    ServiceType, MessagingMode, MessageType, Ansi41Specific, GsmFeatures
};
use std::str::FromStr;

pub async fn start_lifecycle_task(
    config: Arc<AppConfig>,
    session_manager: Arc<SessionManager>,
    message_queue: Arc<MessageQueue>,
) {
    tracing::info!("Lifecycle Manager started");
    
    loop {
        sleep(Duration::from_millis(config.lifecycle.message_state_check_frequency_ms)).await;
        
        process_pending_messages(&config, &session_manager, &message_queue).await;
        // cleanup_old_messages(&config, &message_queue).await; // TODO: Implement cleanup
    }
}

async fn process_pending_messages(
    config: &AppConfig,
    session_manager: &SessionManager,
    message_queue: &MessageQueue,
) {
    // We need to iterate over pending_dr messages
    // pending_dr is DashMap<String, QueuedMessage>
    
    let pending_msgs: Vec<QueuedMessage> = message_queue.get_pending_messages();
    
    for msg in pending_msgs {
        if let Some(final_state) = check_transition(&msg, config) {
            // Transition occurred!
            tracing::info!("Message {} transitioning to {:?}", msg.message_id, final_state);
            
            // 1. Generate Delivery Receipt
            if let Some(pdu) = create_delivery_receipt(&msg, final_state, config) {
                 // 2. Find Session
                 if let Some(session) = session_manager.get_session(&msg.session_id) {
                     let can_receive = match session.bind_type {
                         BindType::Receiver | BindType::Transceiver => true,
                         BindType::Transmitter => true,
                     };
                     
                     if can_receive {
                         if let Err(e) = session.send_command(pdu).await {
                             tracing::error!("Failed to send DR to session {}: {}", session.id, e);
                         } else {
                             tracing::info!("Sent DR for {} to session {}", msg.message_id, session.id);
                         }
                     }
                 } else {
                     tracing::warn!("Session {} not found for DR of message {}", msg.session_id, msg.message_id);
                 }
            }
            
            // 3. Remove from pending queue (it's handled)
            message_queue.remove_pending_dr(&msg.message_id);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum MessageState {
    Delivered,
    // Expired, // Reserved for future implementation
    // Deleted, // Reserved for future implementation
    Undeliverable,
    Accepted,
    // Unknown, // Reserved for future implementation
    Rejected,
}

fn check_transition(msg: &QueuedMessage, config: &AppConfig) -> Option<MessageState> {
    let now = std::time::Instant::now();
    let elapsed = now.duration_since(msg.submitted_at).as_millis() as u64;
    
    if elapsed >= config.lifecycle.max_time_enroute_ms {
        // Time to transition!
        let mut rng = rand::rng();
        let roll = rng.random_range(0..100);
        
        let mut cumulative = 0;
        
        cumulative += config.lifecycle.percent_delivered;
        if roll < cumulative { return Some(MessageState::Delivered); }
        
        cumulative += config.lifecycle.percent_undeliverable;
        if roll < cumulative { return Some(MessageState::Undeliverable); }
        
        cumulative += config.lifecycle.percent_accepted;
        if roll < cumulative { return Some(MessageState::Accepted); }
        
        cumulative += config.lifecycle.percent_rejected;
        if roll < cumulative { return Some(MessageState::Rejected); }
        
        // Default fallthrough calculation
        Some(MessageState::Delivered) 
    } else {
        None
    }
}

fn create_delivery_receipt(msg: &QueuedMessage, state: MessageState, _config: &AppConfig) -> Option<Command> {
    // Format: id:IIIIIIII sub:001 dlvrd:001 submit date:YYMMDDhhmm done date:YYMMDDhhmm stat:DELIVRD err:000 text:..........
    let now = chrono::Local::now();
    let submit_date = now.format("%y%m%d%H%M").to_string(); // Approximate
    let done_date = now.format("%y%m%d%H%M").to_string();
    
    let stat_str = match state {
        MessageState::Delivered => "DELIVRD",
        // MessageState::Expired => "EXPIRED",
        // MessageState::Deleted => "DELETED",
        MessageState::Undeliverable => "UNDELIV",
        MessageState::Accepted => "ACCEPTD",
        // MessageState::Unknown => "UNKNOWN",
        MessageState::Rejected => "REJECTD",
    };

    let short_message = format!(
        "id:{} sub:001 dlvrd:001 submit date:{} done date:{} stat:{} err:000 text:{}",
        msg.message_id, submit_date, done_date, stat_str, String::from_utf8_lossy(&msg.short_message).chars().take(20).collect::<String>()
    );

    // EsmClass: Message Type = SMSC Delivery Receipt (0x04)
    // Mode = Default (Store and Forward)
    let esm_class = EsmClass::new(
        MessagingMode::default(),
        MessageType::default(),
        Ansi41Specific::default(),
        GsmFeatures::default()
    );

    Some(Command::builder()
        .status(CommandStatus::EsmeRok)
        .sequence_number(0) // Server initiated, usually 0 or monotonic
        .pdu(Pdu::DeliverSm(rusmpp::pdus::DeliverSm::new(
             // Service Type
             ServiceType::default(), 
             
             // Source Addr
             Ton::Unknown,
             Npi::Unknown,
             COctetString::from_str(&msg.dest_addr).unwrap_or_default(), 
             
             // Dest Addr
             Ton::Unknown,
             Npi::Unknown,
             COctetString::from_str(&msg.source_addr).unwrap_or_default(), 
             
             // esm_class
             esm_class,
             
             // protocol_id
             0,
             
             // priority_flag
             PriorityFlag::default(),
             
             // schedule_delivery_time
             EmptyOrFullCOctetString::default(),
             
             // validity_period
             EmptyOrFullCOctetString::default(),
             
             // registered_delivery
             RegisteredDelivery::default(),
             
             // replace_if_present_flag
             ReplaceIfPresentFlag::DoNotReplace,
             
             // data_coding
             DataCoding::default(),
             
             // sm_default_msg_id
             0,
             
             // short_message
             OctetString::from_str(&short_message).unwrap_or_default(),
             
             // tlvs
             vec![]
        )))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*; 
    use crate::smpp::queue::QueuedMessage;
    use std::time::Instant;

    #[test]
    fn test_create_delivery_receipt_structure() {
        let msg = QueuedMessage {
            message_id: "test1".to_string(),
            source_addr: "src".to_string(),
            dest_addr: "dst".to_string(),
            short_message: b"hello".to_vec(),
            data_coding: 0,
            session_id: "sess".to_string(),
            submitted_at: Instant::now(),
        };
        
        let config = AppConfig {
            server: ServerConfig { host: "".into(), port: 0 },
            smpp: SmppConfig { system_id: "".into(), password: "".into(), port: 0, max_sessions: 0, accounts: vec![], version: "5.0".into() },
            log: LogConfig { level: "info".into() },
            lifecycle: LifecycleConfig::default(),
            mo_service: MoServiceConfig::default(),
        };
        
        let pdu = create_delivery_receipt(&msg, MessageState::Delivered, &config);
        
        assert!(pdu.is_some());
        let command = pdu.unwrap();
        if let Some(Pdu::DeliverSm(req)) = command.pdu() {
            assert_eq!(req.source_addr.to_string(), "dst");
            assert_eq!(req.destination_addr.to_string(), "src");
        } else {
            panic!("Expected DeliverSm PDU");
        }
    }
}
