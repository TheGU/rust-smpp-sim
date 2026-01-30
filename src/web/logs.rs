use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::broadcast;

const MAX_LOG_LINES: usize = 200;

/// Shared log buffer for real-time log streaming
pub struct LogBuffer {
    logs: RwLock<VecDeque<String>>,
    tx: broadcast::Sender<String>,
}

impl LogBuffer {
    pub fn new() -> Arc<Self> {
        let (tx, _) = broadcast::channel(100);
        Arc::new(Self {
            logs: RwLock::new(VecDeque::with_capacity(MAX_LOG_LINES)),
            tx,
        })
    }

    /// Add a log line to the buffer and broadcast it
    pub fn push(&self, line: String) {
        {
            let mut logs = self.logs.write();
            if logs.len() >= MAX_LOG_LINES {
                logs.pop_front();
            }
            logs.push_back(line.clone());
        }
        let _ = self.tx.send(line);
    }

    /// Get all current logs
    pub fn get_all(&self) -> Vec<String> {
        self.logs.read().iter().cloned().collect()
    }

    /// Subscribe to new log lines
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}

/// Custom tracing layer that writes to LogBuffer
use tracing_subscriber::Layer;

pub struct LogBufferLayer {
    buffer: Arc<LogBuffer>,
}

impl LogBufferLayer {
    pub fn new(buffer: Arc<LogBuffer>) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for LogBufferLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        use std::fmt::Write;
        
        let mut message = String::new();
        let meta = event.metadata();
        
        // Format: [LEVEL] target: message
        let _ = write!(message, "[{}] {}: ", meta.level(), meta.target());
        
        struct Visitor<'a>(&'a mut String);
        impl tracing::field::Visit for Visitor<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    let _ = write!(self.0, "{:?}", value);
                } else {
                    let _ = write!(self.0, " {}={:?}", field.name(), value);
                }
            }
        }
        
        event.record(&mut Visitor(&mut message));
        self.buffer.push(message);
    }
}
