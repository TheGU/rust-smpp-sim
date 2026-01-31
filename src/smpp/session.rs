use dashmap::DashMap;
use uuid::Uuid;
use serde::Serialize;
use regex;

#[allow(dead_code)]
use rusmpp::values::InterfaceVersion;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum BindType {
    Transmitter,
    Receiver,
    Transceiver,
}

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub id: String,
    pub system_id: String,
    pub bind_type: BindType,
    #[allow(dead_code)]
    #[serde(skip)]
    pub interface_version: Option<InterfaceVersion>,
    #[serde(serialize_with = "serialize_addr")]
    pub addr: std::net::SocketAddr,
    #[serde(skip)]
    pub sender: mpsc::Sender<Command>,
    #[serde(skip)]
    pub address_range: Option<String>,
}

fn serialize_addr<S>(addr: &std::net::SocketAddr, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&addr.to_string())
}

use tokio::sync::mpsc;
use rusmpp::Command;

impl Session {
    pub fn new(system_id: String, bind_type: BindType, addr: std::net::SocketAddr, sender: mpsc::Sender<Command>, address_range: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            system_id,
            bind_type,
            interface_version: None,
            addr,
            sender,
            address_range,
        }
    }
    
    pub async fn send_command(&self, command: Command) -> Result<(), mpsc::error::SendError<Command>> {
        self.sender.send(command).await
    }
}

pub struct SessionManager {
    // Map Session ID -> Session
    sessions: DashMap<String, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn add_session(&self, session: Session) {
        self.sessions.insert(session.id.clone(), session);
    }

    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }
    
    #[allow(dead_code)]
    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    pub fn get_all_sessions(&self) -> Vec<Session> {
        self.sessions.iter().map(|r| r.value().clone()).collect()
    }

    /// Find a suitable session for an MO message dest_addr.
    /// Simplified matching: check if session has a range, and if dest_addr starts with it (regex support is complex here, sticking to prefix or exact match for now, or just regex if easy).
    /// SMPP spec says address_range is regex.
    pub fn find_subscriber(&self, dest_addr: &str) -> Option<Session> {
        // Simple strategy: First Receiver/Transceiver that matches.
        // If range is null, maybe catch-all? Usually null means no routing. // SMPPSim behavior: matches address_range.
        
        for entry in self.sessions.iter() {
            let session = entry.value();
            // Skip Transmitters
            match session.bind_type {
                BindType::Transmitter => continue,
                _ => {}
            }
            
            if let Some(range) = &session.address_range {
                // Try Regex match
                if let Ok(re) = regex::Regex::new(range) {
                    if re.is_match(dest_addr) {
                        return Some(session.clone());
                    }
                } else {
                    // Fallback to simple prefix match if regex fails to compile
                    if dest_addr.starts_with(range) {
                         return Some(session.clone());
                    }
                }
            }
        }
        None
    }
}
