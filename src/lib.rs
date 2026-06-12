//! SuperInstance Hybrid Bottle Protocol
//!
//! Wire format: JSON envelope with an opaque msgpack payload.
//! The envelope is the contract; the payload is an implementation detail.
//! Ternary conservation: sum of `trits` is preserved across transformations.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rmp_serde::from_slice;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

/// Ternary digit.
pub type Trit = i8;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum BottleError {
    #[error("JSON decode failed: {0}")]
    JsonDecode(#[from] serde_json::Error),
    #[error("MessagePack decode failed: {0}")]
    MsgpackDecode(#[from] rmp_serde::decode::Error),
    #[error("MessagePack encode failed: {0}")]
    MsgpackEncode(#[from] rmp_serde::encode::Error),
    #[error("Base64 decode failed: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("UTF-8 decode failed: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Conservation violation: expected sum {expected}, got {actual}")]
    Conservation { expected: i32, actual: i32 },
    #[error("TTL expired for bottle {id}")]
    TtlExpired { id: Uuid },
    #[error("Validation failed: {reason}")]
    Validation { reason: String },
}

// ---------------------------------------------------------------------------
// BottleHeader — envelope-only, parseable without touching the payload
// ---------------------------------------------------------------------------

/// Lightweight view of the envelope fields. Useful for routing/inspection
/// without deserialising the payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleHeader {
    pub id: Uuid,
    pub ver: u32,
    pub src: String,
    pub tgt: String,
    pub act: String,
    pub trits: Vec<i8>,
    pub enc: String,
    pub ttl: u32,
}

// ---------------------------------------------------------------------------
// Bottle — full wire object
// ---------------------------------------------------------------------------

/// A SuperInstance bottle: JSON envelope carrying an opaque msgpack payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bottle {
    pub id: Uuid,
    pub ver: u32,
    pub src: String,
    pub tgt: String,
    pub act: String,
    pub trits: Vec<i8>,
    /// Encoding identifier (always "msgpack").
    pub enc: String,
    pub pay: String, // base64-encoded payload
    pub ttl: u32,
}

impl Bottle {
    /// Construct a new bottle. Generates a uuidv7 `id`, sets `ver` to 1,
    /// and msgpack-encodes `payload` into the base64 `pay` field.
    pub fn new(
        src: impl Into<String>,
        tgt: impl Into<String>,
        act: impl Into<String>,
        trits: Vec<i8>,
        payload: &impl Serialize,
        ttl: u32,
    ) -> Result<Self, BottleError> {
        let encoded = rmp_serde::to_vec(payload)?;
        Ok(Self {
            id: Uuid::now_v7(),
            ver: 1,
            src: src.into(),
            tgt: tgt.into(),
            act: act.into(),
            trits,
            enc: "msgpack".into(),
            pay: B64.encode(encoded),
            ttl,
        })
    }

    /// Create a bottle with raw payload bytes. Msgpack-wraps the bytes,
    /// then base64-encodes into `pay`.
    pub fn new_raw(
        src: impl Into<String>,
        tgt: impl Into<String>,
        act: impl Into<String>,
        trits: Vec<i8>,
        raw_payload: Vec<u8>,
        ttl: u32,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            ver: 1,
            src: src.into(),
            tgt: tgt.into(),
            act: act.into(),
            trits,
            enc: "msgpack".into(),
            pay: B64.encode(rmp_serde::to_vec(&raw_payload).unwrap()),
            ttl,
        }
    }

    /// Convenience: create a bottle with no structured payload (empty msgpack).
    pub fn new_empty(
        src: impl Into<String>,
        tgt: impl Into<String>,
        act: impl Into<String>,
        trits: Vec<i8>,
        ttl: u32,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            ver: 1,
            src: src.into(),
            tgt: tgt.into(),
            act: act.into(),
            trits,
            enc: "msgpack".into(),
            pay: B64.encode(rmp_serde::to_vec(&()).unwrap()),
            ttl,
        }
    }

    /// Encode the full bottle to its JSON wire format.
    pub fn encode(&self) -> Result<Vec<u8>, BottleError> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Decode a bottle from its JSON wire format.
    pub fn decode(data: &[u8]) -> Result<Self, BottleError> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Parse just the header (envelope) from the wire bytes, ignoring the
    /// payload entirely.
    pub fn decode_header(data: &[u8]) -> Result<BottleHeader, BottleError> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Decode the msgpack payload into a typed value.
    pub fn decode_payload<T: for<'de> Deserialize<'de>>(&self) -> Result<T, BottleError> {
        let bytes = B64.decode(&self.pay)?;
        Ok(from_slice(&bytes)?)
    }

    /// Validate the bottle. Checks:
    /// 1. Conservation: sum of trits is preserved (caller must audit against
    ///    the input bottle).
    /// 2. TTL not expired.
    pub fn validate(&self) -> Result<(), BottleError> {
        // TTL check
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u64;
        // uuidv7 timestamp is milliseconds since epoch, stored in the first 48 bits
        let created_ms = self.id_to_timestamp_ms();
        let created_sec = created_ms / 1000;
        if now > created_sec + self.ttl as u64 {
            return Err(BottleError::TtlExpired { id: self.id });
        }
        Ok(())
    }

    /// Compute the ternary sum of this bottle's trits.
    pub fn trit_sum(&self) -> i32 {
        self.trits.iter().map(|&t| t as i32).sum()
    }

    /// Extract the uuidv7 timestamp (milliseconds since epoch) from the id.
    fn id_to_timestamp_ms(&self) -> u64 {
        // UUIDv7 stores unix_ms in the first 48 bits
        let bytes = self.id.into_bytes();
        ((bytes[0] as u64) << 40)
            | ((bytes[1] as u64) << 32)
            | ((bytes[2] as u64) << 24)
            | ((bytes[3] as u64) << 16)
            | ((bytes[4] as u64) << 8)
            | (bytes[5] as u64)
    }
}

// ---------------------------------------------------------------------------
// Conservation audit
// ---------------------------------------------------------------------------

/// Verify that the ternary charge is conserved between input and output bottles.
/// Returns `true` if `input.trit_sum() == output.trit_sum()`.
pub fn audit(input: &Bottle, output: &Bottle) -> bool {
    input.trit_sum() == output.trit_sum()
}

/// Strict audit: returns `Err` with details on failure.
pub fn audit_strict(input: &Bottle, output: &Bottle) -> Result<(), BottleError> {
    let expected = input.trit_sum();
    let actual = output.trit_sum();
    if expected == actual {
        Ok(())
    } else {
        Err(BottleError::Conservation { expected, actual })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let original = Bottle::new_empty("forgemaster", "fleet-edge", "cycle.complete", vec![-1, 0, 1, 0, 1], 300);
        let wire = original.encode().unwrap();
        let decoded = Bottle::decode(&wire).unwrap();
        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.src, "forgemaster");
        assert_eq!(decoded.tgt, "fleet-edge");
        assert_eq!(decoded.act, "cycle.complete");
        assert_eq!(decoded.trits, vec![-1, 0, 1, 0, 1]);
        assert_eq!(decoded.enc, "msgpack");
        assert_eq!(decoded.ttl, 300);
    }

    #[test]
    fn conservation_holds_for_identity_bottle() {
        let b = Bottle::new_empty("a", "b", "noop", vec![-1, 0, 1, 0, 1], 300);
        // An "identity" transformation produces the same bottle
        assert!(audit(&b, &b));
    }

    #[test]
    fn conservation_fails_for_modified_trits() {
        let input = Bottle::new_empty("a", "b", "transform", vec![-1, 0, 1], 300);
        let output = Bottle::new_empty("b", "c", "transform.done", vec![1, 1, 1], 300);
        // sum(input) = 0, sum(output) = 3 → conservation violation
        assert!(!audit(&input, &output));
    }

    #[test]
    fn header_parseable_without_payload() {
        let b = Bottle::new_empty("forgemaster", "fleet-edge", "cycle.complete", vec![-1, 0, 1], 300);
        let wire = b.encode().unwrap();
        // Decode only the header — structurally the same JSON, just a different type
        let header = Bottle::decode_header(&wire).unwrap();
        assert_eq!(header.src, "forgemaster");
        assert_eq!(header.tgt, "fleet-edge");
        assert_eq!(header.act, "cycle.complete");
        assert_eq!(header.trits, vec![-1, 0, 1]);
        assert_eq!(header.enc, "msgpack");
        assert_eq!(header.ttl, 300);
        // header has no `pay` field — we never touched the payload
    }

    #[test]
    fn ttl_expired_detection() {
        // Create a bottle with TTL = 0 seconds — already expired
        // We construct manually to use a past uuidv7
        let mut b = Bottle::new_empty("a", "b", "test", vec![0], 0);
        // Force ttl to 0 — since uuidv7 is "now", even ttl=0 should be tight
        // but we need a truly expired bottle. Manually craft one.
        let past_id = {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let past_ms = now_ms - 3_600_000; // 1 hour ago
            // Manually construct UUIDv7 bytes: 48-bit timestamp, then version/variant bits
            let ts = past_ms;
            let mut bytes = [0u8; 16];
            bytes[0] = ((ts >> 40) & 0xFF) as u8;
            bytes[1] = ((ts >> 32) & 0xFF) as u8;
            bytes[2] = ((ts >> 24) & 0xFF) as u8;
            bytes[3] = ((ts >> 16) & 0xFF) as u8;
            bytes[4] = ((ts >> 8) & 0xFF) as u8;
            bytes[5] = (ts & 0xFF) as u8;
            // version 7 in high nibble of byte 6
            bytes[6] = 0x70 | 0x01; // ver=7, rand_a=0x01
            bytes[7] = 0x23;
            // variant 10xx in high bits of byte 8
            bytes[8] = 0x80 | 0x45;
            bytes[9] = 0x67;
            bytes[10] = 0x89;
            bytes[11] = 0xAB;
            bytes[12] = 0xCD;
            bytes[13] = 0xEF;
            bytes[14] = 0x01;
            bytes[15] = 0x23;
            Uuid::from_bytes(bytes)
        };
        b.id = past_id;
        b.ttl = 60; // 60 seconds TTL, but created an hour ago
        assert!(b.validate().is_err());
    }

    #[test]
    fn trits_sum_preserved() {
        let trits = vec![-1, -1, 0, 1, 1];
        let sum: i32 = trits.iter().map(|&t| t as i32).sum();
        assert_eq!(sum, 0);

        let b1 = Bottle::new_empty("a", "b", "step1", trits.clone(), 300);
        let b2 = Bottle::new_empty("b", "c", "step2", trits.clone(), 300);
        assert_eq!(b1.trit_sum(), b2.trit_sum());
        assert!(audit(&b1, &b2));
    }

    #[test]
    fn payload_roundtrip_with_typed_data() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct CycleData {
            quality: f64,
            crates: u32,
        }

        let payload = CycleData { quality: 0.95, crates: 42 };
        let b = Bottle::new("forgemaster", "fleet-edge", "cycle.complete", vec![1, 0, -1], &payload, 300).unwrap();
        let decoded: CycleData = b.decode_payload().unwrap();
        assert_eq!(decoded, payload);
    }
}
