# superinstance-protocol

[![crates.io](https://img.shields.io/crates/v/superinstance-protocol.svg)](https://crates.io/crates/superinstance-protocol)
[![docs.rs](https://docs.rs/superinstance-protocol/badge.svg)](https://docs.rs/superinstance-protocol)
[![license](https://img.shields.io/crates/l/superinstance-protocol.svg)](https://crates.io/crates/superinstance-protocol)

Wire format for agent-to-agent messaging with built-in ternary conservation auditing.

## Quick Start

```rust
use superinstance_protocol::{Bottle, audit_strict};

// Create a bottle carrying typed payload data
#[derive(serde::Serialize, serde::Deserialize)]
struct CycleReport { quality: f64, count: u32 }

let payload = CycleReport { quality: 0.95, count: 42 };
let bottle = Bottle::new(
    "forgemaster",       // src
    "fleet-edge",        // tgt
    "cycle.complete",    // action
    vec![-1, 0, 1, 0],  // trits (ternary charge)
    &payload,            // typed payload
    300,                 // TTL in seconds
)?;

// Encode to JSON wire format
let wire = bottle.encode()?;

// Decode on the other side
let decoded = Bottle::decode(&wire)?;
let report: CycleReport = decoded.decode_payload()?;

// Audit conservation between input → output
audit_strict(&input_bottle, &output_bottle)?;
```

## Key Types

| Type | Role |
|---|---|
| `Bottle` | Full wire object: JSON envelope + base64-encoded msgpack payload |
| `BottleHeader` | Envelope-only view for routing/inspection without touching the payload |
| `BottleError` | Errors for decode, encode, conservation, TTL, and validation failures |
| `Trit` | Type alias for `i8` — values are `-1`, `0`, or `1` |

**Free functions:** `audit()` returns `bool`, `audit_strict()` returns `Result<(), BottleError>`.

## Why

Agents passing messages need a wire format that is:
- **Inspectable** — the JSON envelope (`BottleHeader`) can be parsed without deserializing the payload
- **Efficient** — the payload is msgpack, not JSON
- **Auditable** — every bottle carries a ternary charge (`trits`) whose sum is conserved across transformations, enabling structural integrity checks without knowing the payload schema

## Features

- **Hybrid encoding** — JSON envelope for routing, msgpack payload for density
- **UUIDv7 identifiers** — time-sortable, monotonic bottle IDs
- **TTL enforcement** — bottles expire based on creation time + TTL
- **Ternary conservation** — `trit_sum()` and `audit()` / `audit_strict()` verify charge is preserved
- **Header-only parsing** — `Bottle::decode_header()` reads envelope fields without touching the payload
- **Typed payloads** — any `Serialize`/`Deserialize` type goes in, same type comes out
- **Raw payload support** — `Bottle::new_raw()` for pre-encoded bytes
- **Empty bottles** — `Bottle::new_empty()` for signal-only messages

## License

MIT OR Apache-2.0
