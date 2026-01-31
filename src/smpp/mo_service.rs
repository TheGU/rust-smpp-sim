use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::config::AppConfig;
use crate::smpp::session::SessionManager;
use crate::smpp::queue::{MoMessageQueue, MoMessage};
use std::fs::File;
use std::io::{BufRead, BufReader};
use rusmpp::{Command, Pdu, CommandStatus};
use rusmpp::types::{COctetString, OctetString, EmptyOrFullCOctetString};
use rusmpp::values::{
    Ton, Npi, EsmClass, PriorityFlag, RegisteredDelivery, ReplaceIfPresentFlag, DataCoding,
    ServiceType
};
use std::str::FromStr;
use hex;

pub async fn start_mo_service_task(
    config: Arc<AppConfig>,
    session_manager: Arc<SessionManager>,
    mo_queue: Arc<MoMessageQueue>,
) {
    if !config.mo_service.enabled {
        tracing::info!("MO Service disabled");
        return;
    }
    
    tracing::info!("MO Service started");
    
    let rate = config.mo_service.delivery_messages_per_minute;
    let period_ms = if rate > 0 { 60000 / rate as u64 } else { 1000 }; // Default wait if 0/error
    
    // We need a loop that:
    // 1. Reads user injection from queue (Web UI)
    // 2. Reads from CSV (Simulated traffic) - simpler to run two tasks or combine?
    // Let's run CSV injector as a separate loop if enabled.
    
    // Task 1: Web Injection Dispatcher
    let queue_manager = session_manager.clone();
    let queue_recv = mo_queue.clone();
    tokio::spawn(async move {
        process_injected_messages(queue_recv, queue_manager).await;
    });

    if rate > 0 {
        // Task 2: CSV Injection
         loop {
             match File::open(&config.mo_service.file_path) {
                 Ok(file) => {
                     let reader = BufReader::new(file);
                     for line in reader.lines() {
                         if let Ok(content) = line {
                             if content.trim().is_empty() || content.starts_with('#') { continue; }
                             
                             let parts: Vec<&str> = content.split(',').collect();
                             if parts.len() >= 3 {
                                 let source = parts[0].trim();
                                 let dest = parts[1].trim();
                                 let msg_content = parts[2..].join(","); // Join remaining in case msg has comma
                                 
                                 // Inject
                                 let mo = MoMessage {
                                     source_addr: source.to_string(),
                                     dest_addr: dest.to_string(),
                                     short_message: msg_content,
                                 };
                                 
                                 dispatch_mo(&mo, &session_manager).await;
                                 
                                 // Wait for rate limit
                                 sleep(Duration::from_millis(period_ms)).await;
                             }
                         }
                     }
                 }
                 Err(e) => {
                     tracing::error!("Failed to open MO messages file {}: {}", config.mo_service.file_path, e);
                     sleep(Duration::from_secs(10)).await; // Wait before retry
                 }
             }
             // Loop file again? SMPPSim behavior dictates looping usually or restart?
             // Assuming loop for traffic generation
             tracing::info!("MO CSV file finished, restarting...");
         }
    }
}

async fn process_injected_messages(mo_queue: Arc<MoMessageQueue>, session_manager: Arc<SessionManager>) {
    // We need to take the receiver from mutex
    if let Some(mut rx) = mo_queue.take_receiver() {
        while let Some(msg) = rx.recv().await {
            dispatch_mo(&msg, &session_manager).await;
        }
    } else {
        tracing::error!("Failed to take MO queue receiver - already taken?");
    }
}

async fn dispatch_mo(msg: &MoMessage, session_manager: &SessionManager) {
    // Find subscriber
    if let Some(session) = session_manager.find_subscriber(&msg.dest_addr) {
        tracing::info!("Delivering MO from {} to {} via session {}", msg.source_addr, msg.dest_addr, session.id);
        
        if let Some(pdu) = create_deliver_sm(msg) {
             if let Err(e) = session.send_command(pdu).await {
                 tracing::error!("Failed to send MO to session {}: {}", session.id, e);
             }
        }
    } else {
        tracing::warn!("No suitable session found for MO to {}", msg.dest_addr);
        // Add to "Delayed Inbound Queue"? (TODO)
        // For now just drop/log
    }
}

fn create_deliver_sm(msg: &MoMessage) -> Option<Command> {
    // Determine if binary
    let (short_message, data_coding) = if msg.short_message.starts_with("0x") {
        if let Ok(bytes) = hex::decode(&msg.short_message[2..]) {
            (OctetString::from_bytes(bytes.into()).unwrap_or_default(), DataCoding::default()) // 8-bit binary
        } else {
             (OctetString::from_str(&msg.short_message).unwrap_or_default(), DataCoding::default())
        }
    } else {
        (OctetString::from_str(&msg.short_message).unwrap_or_default(), DataCoding::default())
    };

    Some(Command::builder()
        .status(CommandStatus::EsmeRok)
        .sequence_number(0)
        .pdu(Pdu::DeliverSm(rusmpp::pdus::DeliverSm::new(
             ServiceType::default(),
             
             // Source Addr (The sender of the MO)
             Ton::Unknown,
             Npi::Unknown,
             COctetString::from_str(&msg.source_addr).unwrap_or_default(),
             
             // Dest Addr (The ESME receiving it)
             Ton::Unknown,
             Npi::Unknown,
             COctetString::from_str(&msg.dest_addr).unwrap_or_default(),
             
             // esm_class
             EsmClass::default(),
             
             0,
             PriorityFlag::default(),
             EmptyOrFullCOctetString::default(),
             EmptyOrFullCOctetString::default(),
             RegisteredDelivery::default(),
             ReplaceIfPresentFlag::DoNotReplace,
             data_coding,
             0,
             short_message,
             vec![]
        )))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_deliver_sm_binary() {
        let msg = MoMessage {
            source_addr: "src".to_string(),
            dest_addr: "dst".to_string(),
            short_message: "0x000102".to_string(),
        };
        
        // This function doesn't use Config or other complex types
        let cmd_opt = create_deliver_sm(&msg);
        assert!(cmd_opt.is_some());
        
        let cmd = cmd_opt.unwrap();
        if let Some(Pdu::DeliverSm(_)) = cmd.pdu() {
            // Success
        } else {
            panic!("Expected DeliverSm");
        }
    }
}
