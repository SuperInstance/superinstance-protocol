/**
 * SuperInstance Hybrid Bottle Protocol — TypeScript types
 *
 * Wire format: JSON envelope with an opaque base64-encoded msgpack payload.
 * The envelope is the contract; the payload is an implementation detail.
 */

/** Ternary digit: -1, 0, or +1 */
export type Trit = -1 | 0 | 1;

/** Bottle envelope + payload — full wire object */
export interface Bottle {
  id: string;          // uuidv7
  ver: number;         // envelope schema version
  src: string;         // source agent / service
  tgt: string;         // target agent / service
  act: string;         // namespaced action (e.g. "cycle.complete")
  trits: Trit[];       // ternary state — conservation law applies
  enc: "msgpack";      // payload encoding
  pay: string;         // base64-encoded payload
  ttl: number;         // time-to-live in seconds
}

/** Lightweight header — parseable without touching the payload */
export interface BottleHeader {
  id: string;
  ver: number;
  src: string;
  tgt: string;
  act: string;
  trits: Trit[];
  enc: string;
  ttl: number;
}

/**
 * Compute the ternary sum of a bottle's trits.
 */
export function tritSum(bottle: { trits: Trit[] }): number {
  return bottle.trits.reduce((sum, t) => sum + t, 0);
}

/**
 * Conservation audit: returns true if the ternary charge is conserved
 * between input and output bottles.
 */
export function audit(input: { trits: Trit[] }, output: { trits: Trit[] }): boolean {
  return tritSum(input) === tritSum(output);
}

/**
 * Encode a Bottle to its JSON wire format (Uint8Array).
 */
export function encode(bottle: Bottle): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(bottle));
}

/**
 * Decode a full Bottle from its JSON wire format.
 */
export function decode(data: Uint8Array | string): Bottle {
  const json = typeof data === "string" ? data : new TextDecoder().decode(data);
  return JSON.parse(json) as Bottle;
}

/**
 * Decode only the header from the wire format — no payload deserialization.
 */
export function decodeHeader(data: Uint8Array | string): BottleHeader {
  const json = typeof data === "string" ? data : new TextDecoder().decode(data);
  // Parse and strip the `pay` field
  const raw = JSON.parse(json) as Bottle;
  const { pay: _pay, ...header } = raw;
  return header as unknown as BottleHeader;
}

/**
 * Check if a bottle's TTL has expired based on its uuidv7 timestamp.
 *
 * uuidv7 encodes unix_ms in the first 48 bits (first 12 hex chars).
 */
export function isExpired(bottle: Bottle): boolean {
  const unixMs = uuidv7ToTimestamp(bottle.id);
  const nowMs = Date.now();
  return nowMs > unixMs + bottle.ttl * 1000;
}

/**
 * Extract the unix-millisecond timestamp from a uuidv7 string.
 */
export function uuidv7ToTimestamp(id: string): number {
  // First 12 hex chars = 48-bit unix_ms
  const hex = id.replace(/-/g, "").slice(0, 12);
  return parseInt(hex, 16);
}

/**
 * Validate a bottle: TTL check + structural checks.
 */
export function validate(bottle: Bottle): { ok: boolean; error?: string } {
  if (isExpired(bottle)) {
    return { ok: false, error: `TTL expired for bottle ${bottle.id}` };
  }
  if (bottle.ver !== 1) {
    return { ok: false, error: `Unsupported envelope version: ${bottle.ver}` };
  }
  if (bottle.trits.some((t) => t !== -1 && t !== 0 && t !== 1)) {
    return { ok: false, error: "trits must contain only -1, 0, or 1" };
  }
  return { ok: true };
}

/**
 * Create a new bottle with a generated uuidv7 id.
 *
 * Note: Requires a uuidv7 library or a runtime that provides crypto.randomUUID()
 * (which generates v4; for true v7, use a library like `uuid` package).
 */
export function createBottle(
  src: string,
  tgt: string,
  act: string,
  trits: Trit[],
  payload: Uint8Array,
  ttl: number,
): Bottle {
  const id = uuidv7();
  return {
    id,
    ver: 1,
    src,
    tgt,
    act,
    trits,
    enc: "msgpack",
    pay: btoa(String.fromCharCode(...payload)),
    ttl,
  };
}

/** Minimal uuidv7 generator (time-sortable). */
function uuidv7(): string {
  const now = Date.now();
  const hex = (n: number, len: number) => n.toString(16).padStart(len, "0");

  const unixMs = now;
  const timeHex = hex(unixMs, 12);

  // ver=7 in the 4 bits after time
  const randA = Math.floor(Math.random() * 0x0fff) | 0x7000;
  const randB = (Math.floor(Math.random() * 0x3fffffff) | 0x80000000) >>> 0;

  return (
    timeHex.slice(0, 8) + "-" +
    timeHex.slice(8, 12) + "-" +
    hex(randA, 4) + "-" +
    hex(randB >>> 16, 4) + "-" +
    hex(randB & 0xffff, 4) +
    hex(Math.floor(Math.random() * 0xffff), 4) +
    hex(Math.floor(Math.random() * 0xffff), 4)
  );
}
