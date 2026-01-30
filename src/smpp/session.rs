use dashmap::DashMap;
use uuid::Uuid;
use serde::Serialize;

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
}

fn serialize_addr<S>(addr: &std::net::SocketAddr, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&addr.to_string())
}

impl Session {
    pub fn new(system_id: String, bind_type: BindType, addr: std::net::SocketAddr) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            system_id,
            bind_type,
            interface_version: None,
            addr,
        }
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
}
