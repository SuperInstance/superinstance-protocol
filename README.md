# SuperInstance Protocol

**SuperInstance Protocol** is a hybrid wire format combining a JSON envelope with MessagePack payload — providing human-readable metadata for routing/inspection and compact binary encoding for efficient data transfer, with ternary conservation enforcement.

## Why It Matters

Every distributed system needs a wire protocol. Pure JSON is human-readable but verbose (20-40% larger than binary). Pure binary (protobuf, msgpack) is compact but opaque — you can't inspect a message without the schema. The "bottle" protocol gives you both: the JSON envelope (id, source, target, action, trits, ttl) is self-describing and routable, while the msgpack payload carries the dense data. The ternary conservation check (Σ trits must be preserved across transformations) provides end-to-end integrity verification that neither JSON nor msgpack alone can offer.

## How It Works

### Bottle Structure

```json
{
  "id": "0190a1b2-...",  // UUIDv7 (time-ordered)
  "ver": 1,               // envelope schema version
  "src": "agent-alpha",   // source agent/service
  "tgt": "fleet-router",  // target agent/service
  "act": "cycle.complete",// namespaced action
  "trits": [-1, 0, 1],    // ternary state (conservation-tracked)
  "enc": "msgpack",       // payload encoding
  "pay": "gQUO...",       // base64-encoded msgpack payload
  "ttl": 30               // time-to-live in seconds
}
```

### Envelope vs Payload Separation

The `BottleHeader` struct contains only envelope fields — parseable without touching the msgpack payload:

```rust
let header: BottleHeader = serde_json::from_str(&envelope_json);
// Route based on src/tgt/act WITHOUT decoding payload
```

Header parsing: **O(1)**. Payload decode: deferred until needed, **O(N)** where N = payload size.

### UUIDv7 Generation

IDs use UUIDv7 (time-ordered), enabling chronological sorting without a separate timestamp:

```
UUIDv7 = [48-bit unix_ms] [12-bit rand_a] [2-bit version] [62-bit rand_b]
```

Collision probability: negligible (80 bits of randomness per millisecond).

### Ternary Conservation

Every transformation (routing, forwarding, processing) must preserve:

```
Σ trits(before) = Σ trits(after)
```

The `verify_conservation()` method checks this invariant. Violations return `BottleError::Conservation { expected, actual }`. Conservation verification: **O(N)** where N = trit count.

### TTL Enforcement

Messages expire after `ttl` seconds. Expired bottles return `BottleError::TtlExpired`:

```
if now - bottle_created > bottle.ttl:
    return Err(TtlExpired { id })
```

TTL check: **O(1)**.

### TypeScript Types

The protocol includes TypeScript type definitions (`types.ts`) for cross-language compatibility:

```typescript
type Trit = -1 | 0 | 1;
interface Bottle { id, ver, src, tgt, act, trits, enc, pay, ttl }
```

## Quick Start

```rust
use superinstance_protocol::{Bottle, BottleHeader};

// Create a bottle
let bottle = Bottle::new(
    "agent-alpha",
    "fleet-router",
    "cycle.complete",
    vec![-1, 0, 1],
    &serde_json::json!({"result": 42}),
    30, // TTL
);

// Serialize to JSON wire format
let json = serde_json::to_string(&bottle)?;

// Parse header only (without decoding payload)
let header: BottleHeader = serde_json::from_str(&json)?;
println!("Action: {}", header.act);

// Verify conservation
let trit_sum: i32 = bottle.trits.iter().map(|&t| t as i32).sum();
println!("Trit sum: {}", trit_sum);
```

## API

| Type | Description |
|------|-------------|
| `Bottle` | Full wire object (envelope + base64 msgpack payload) |
| `BottleHeader` | Envelope-only view for routing without payload decode |
| `BottleError` | JsonDecode, MsgpackDecode, Base64Decode, Conservation, TtlExpired, Validation |
| `Trit` | `i8` type alias for ternary digits {-1, 0, +1} |

Key methods: `Bottle::new()`, `Bottle::decode_payload()`, `Bottle::verify_conservation()`.

## Architecture Notes

SuperInstance Protocol is the universal wire format for fleet communication. In γ + η = C, the trits field carries the conservation quantity C across all transformations. The JSON envelope enables γ (growth — new agents can join by parsing envelopes without schema compilation) while TTL enforcement provides η (avoidance — expired messages are dropped, preventing stale data accumulation). The TypeScript types enable cross-language fleet membership.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md) for the fleet communication architecture.

## References

1. RFC 8949 — "Concise Binary Object Representation (CBOR)." IETF, 2020. (MessagePack predecessor)
2. RFC 9562 — "Universally Unique IDentifiers (UUIDs)." IETF, 2024. (UUIDv7)
3. Newman, S. (2021). *Building Microservices*, 2nd ed. O'Reilly. Chapter 4: Inter-service Communication.

## License

MIT
