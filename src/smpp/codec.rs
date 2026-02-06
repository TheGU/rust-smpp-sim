//! Custom SMPP Codec wrapper for version compatibility
//!
//! This module provides a wrapper around rusmpp's CommandCodec that adds
//! compatibility with SMPP 3.4 clients. The Node.js `smpp` package (v0.3.x)
//! uses SMPP 3.4 which has subtle differences in PDU encoding.

use bytes::{BytesMut, BufMut};
use rusmpp::{tokio_codec::CommandCodec, Command};
use tokio_util::codec::{Decoder, Encoder};
use std::io;

/// SMPP protocol version for compatibility mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmppVersion {
    /// SMPP 3.4 - lenient mode for older clients
    V34,
    /// SMPP 5.0 - strict mode (default)
    V50,
}

impl SmppVersion {
    pub fn from_str(s: &str) -> Self {
        match s {
            "3.4" | "34" | "3" => SmppVersion::V34,
            _ => SmppVersion::V50,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            SmppVersion::V34 => "3.4",
            SmppVersion::V50 => "5.0",
        }
    }
}

/// Custom SMPP codec that wraps CommandCodec with version compatibility
pub struct SmppCodec {
    inner: CommandCodec,
    version: SmppVersion,
}

impl SmppCodec {
    /// Create a new SmppCodec with the specified version compatibility
    pub fn new(version: SmppVersion) -> Self {
        Self {
            inner: CommandCodec::new(),
            version,
        }
    }

    /// Get the configured SMPP version
    pub fn version(&self) -> SmppVersion {
        self.version
    }
}

impl Decoder for SmppCodec {
    type Item = Command;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // For SMPP 5.0, use standard decoding
        if self.version == SmppVersion::V50 {
            return self.inner.decode(src).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, e.to_string())
            });
        }

        // For SMPP 3.4, try standard decoding first
        // Keep a copy of the buffer position in case we need to retry
        let original_len = src.len();
        
        match self.inner.decode(src) {
            Ok(result) => Ok(result),
            Err(e) => {
                let err_msg = e.to_string();
                
                // Check if this is a null terminator issue
                if err_msg.contains("Not null terminated") || 
                   err_msg.contains("COctetString") {
                    tracing::debug!("SMPP 3.4 compatibility: attempting to fix PDU encoding");
                    
                    // For SMPP 3.4 compatibility, we need to handle the case where
                    // the PDU might have different encoding for COctetString fields.
                    // 
                    // The issue is that some SMPP 3.4 implementations don't properly
                    // null-terminate certain strings, or use different PDU layouts.
                    //
                    // We'll try to fix the buffer by ensuring proper null terminators.
                    if let Some(fixed_buf) = try_fix_pdu_nulls(src, original_len) {
                        *src = fixed_buf;
                        return self.inner.decode(src).map_err(|e| {
                            io::Error::new(io::ErrorKind::InvalidData, 
                                format!("SMPP 3.4 decode failed after fix attempt: {}", e))
                        });
                    }
                }
                
                Err(io::Error::new(io::ErrorKind::InvalidData, err_msg))
            }
        }
    }
}

impl Encoder<Command> for SmppCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Command, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.inner.encode(item, dst).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })
    }
}

impl Encoder<&Command> for SmppCodec {
    type Error = io::Error;

    fn encode(&mut self, item: &Command, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.inner.encode(item, dst).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })
    }
}

/// Attempt to fix PDU null terminator issues for SMPP 3.4 compatibility
/// 
/// This function analyzes the PDU structure and ensures COctetString fields
/// are properly null-terminated.
fn try_fix_pdu_nulls(src: &mut BytesMut, original_len: usize) -> Option<BytesMut> {
    // Ensure we have at least the SMPP header (16 bytes)
    if src.len() < 16 {
        return None;
    }
    
    // Read command length from first 4 bytes (big-endian)
    let cmd_len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
    
    // Verify we have the full PDU
    if src.len() < cmd_len {
        return None;
    }

    // Read command ID to determine PDU type
    let cmd_id = u32::from_be_bytes([src[4], src[5], src[6], src[7]]);
    
    // Handle bind_transmitter (0x00000002), bind_receiver (0x00000001), 
    // bind_transceiver (0x00000009)
    match cmd_id {
        0x00000001 | 0x00000002 | 0x00000009 => {
            // Bind PDU structure after 16-byte header:
            // - system_id (COctetString, max 16)
            // - password (COctetString, max 9)
            // - system_type (COctetString, max 13)
            // - interface_version (1 byte)
            // - addr_ton (1 byte)
            // - addr_npi (1 byte)
            // - address_range (COctetString, max 41)
            
            tracing::debug!("Attempting to fix bind PDU (cmd_id: 0x{:08x})", cmd_id);
            
            // Create a new buffer with fixed content
            let mut fixed = BytesMut::with_capacity(cmd_len + 10); // Extra space for nulls
            
            // Copy header (first 16 bytes)
            fixed.put_slice(&src[0..16]);
            
            let mut pos = 16;
            let mut fixed_count = 0;
            
            // Fix each COctetString field
            for field_name in &["system_id", "password", "system_type"] {
                let (end_pos, added_null) = copy_coctet_string(&src[pos..cmd_len], &mut fixed);
                if added_null {
                    fixed_count += 1;
                    tracing::debug!("Added null terminator for {}", field_name);
                }
                pos += end_pos;
                if pos >= cmd_len {
                    break;
                }
            }
            
            // Copy remaining bytes (interface_version, addr_ton, addr_npi, address_range)
            if pos < cmd_len {
                // Copy 3 fixed bytes
                let remaining_fixed = std::cmp::min(3, cmd_len - pos);
                fixed.put_slice(&src[pos..pos + remaining_fixed]);
                pos += remaining_fixed;
                
                // Copy address_range (COctetString)
                if pos < cmd_len {
                    let (end_pos, added_null) = copy_coctet_string(&src[pos..cmd_len], &mut fixed);
                    if added_null {
                        fixed_count += 1;
                        tracing::debug!("Added null terminator for address_range");
                    }
                    pos += end_pos;
                }
                
                // Copy any remaining bytes (TLVs, etc.)
                if pos < cmd_len {
                    fixed.put_slice(&src[pos..cmd_len]);
                }
            }
            
            if fixed_count > 0 {
                // Update command length in the fixed buffer
                let new_len = fixed.len() as u32;
                fixed[0..4].copy_from_slice(&new_len.to_be_bytes());
                
                // Keep remaining unprocessed data
                if original_len > cmd_len {
                    fixed.put_slice(&src[cmd_len..original_len]);
                }
                
                tracing::info!("SMPP 3.4 compatibility: fixed {} null terminators", fixed_count);
                return Some(fixed);
            }
        }
        _ => {
            // For other PDU types, we don't have specific fixes yet
            tracing::debug!("No specific fix for PDU type 0x{:08x}", cmd_id);
        }
    }
    
    None
}

/// Copy a COctetString field, ensuring it's null-terminated
/// Returns (bytes_consumed_from_source, was_null_added)
fn copy_coctet_string(src: &[u8], dst: &mut BytesMut) -> (usize, bool) {
    // Find null terminator or end of source
    let null_pos = src.iter().position(|&b| b == 0);
    
    match null_pos {
        Some(pos) => {
            // Include the null terminator
            dst.put_slice(&src[0..=pos]);
            (pos + 1, false)
        }
        None => {
            // No null terminator found - add one
            dst.put_slice(src);
            dst.put_u8(0);
            (src.len(), true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smpp_version_from_str() {
        assert_eq!(SmppVersion::from_str("3.4"), SmppVersion::V34);
        assert_eq!(SmppVersion::from_str("34"), SmppVersion::V34);
        assert_eq!(SmppVersion::from_str("3"), SmppVersion::V34);
        assert_eq!(SmppVersion::from_str("5.0"), SmppVersion::V50);
        assert_eq!(SmppVersion::from_str("5"), SmppVersion::V50);
        assert_eq!(SmppVersion::from_str("invalid"), SmppVersion::V50);
    }

    #[test]
    fn test_smpp_version_as_str() {
        assert_eq!(SmppVersion::V34.as_str(), "3.4");
        assert_eq!(SmppVersion::V50.as_str(), "5.0");
    }
}
