# Verifying the Control Plane — orrion ↔ orrpheus

## Prerequisite checklist

- [ ] orrbeam v2 running on **orrion** (`corr@orrion`, 192.168.1.152)
- [ ] orrbeam v2 running on **orrpheus** (`coltonorr@orrpheus`, 192.168.1.132)
- [ ] Port 47782 reachable between the two nodes (orrtellite mesh or LAN)
- [ ] Any old v1 Python daemon killed (`pkill -f 'orrbeam.*daemon'`)

## Quick smoke tests

```bash
# 1. Verify server is listening (run on each node)
ss -tlnp | grep 47782          # orrion (Linux)
lsof -i :47782                  # orrpheus (macOS)

# 2. Fetch hello from each node (from the other node)
curl --insecure https://192.168.1.152:47782/v1/hello | jq .   # from orrpheus → orrion
curl --insecure https://192.168.1.132:47782/v1/hello | jq .   # from orrion → orrpheus

# 3. Confirm 401 before trust is established
curl --insecure https://192.168.1.132:47782/v1/status | jq .
# Expected: {"code":"unknown_key","message":"..."}
```

## Mutual trust flow (UI)

1. On orrion, open Settings → Peers → Add peer → "Request mutual trust"
2. Enter `192.168.1.132:47782` (or orrtellite address)
3. `MutualTrustPendingModal` appears on orrion
4. On orrpheus, `MutualTrustRequestDialog` pops up — click Approve
5. Both nodes now show each other in the Trusted Peers list

## Remote connect flow (UI)

1. On orrion, Moonlight panel shows orrpheus NodeCard with "Connect (remote)" button
2. Click it — `PeeringProgressModal` opens and steps through:
   - Resolving peer → Probing → Starting Sunshine → Submitting PIN → Pairing → Streaming
3. orrpheus Sunshine starts automatically
4. moonlight-qt on orrion connects to orrpheus desktop

## Failure paths to verify

| Scenario | Expected behavior |
|----------|-------------------|
| Remove orrion from orrpheus trusted_peers.yaml | Next connect attempt: `unknown_key` in progress modal |
| Tamper cert_sha256 in trusted_peers.yaml | TLS error: `cert pin mismatch` |
| Replay a signed request | Server returns `replay` |
| 60+ second clock skew | Server returns `clock_skew` |
| orrpheus orrbeam not running | Hard error: "peer unreachable" with Retry button |

---

## §19 Security Checklist — Static Verification

Each item below has a runnable command (grep or clippy) that can be executed without live nodes,
or an explicit **manual step** with expected output for items that require a running instance.

### §19.1 Signature correctness

**19.1a** — Canonical string identical in `sign` and `verify`:

```bash
grep -n "canonical\|to_sign\|signing_input" \
  crates/orrbeam-core/src/wire.rs
# Expected: the same string construction expression appears in both the sign()
# and verify() paths. Side-by-side the two blocks manually to confirm they match.
```

**19.1b** — Server uses raw body bytes, no serde round-trip:

```bash
grep -n "serde_json::to_vec\|to_bytes" \
  crates/orrbeam-net/src/server/mod.rs \
  crates/orrbeam-net/src/server/handlers.rs 2>/dev/null || \
grep -rn "serde_json::to_vec\|to_bytes" crates/orrbeam-net/src/server/
# Expected: body is captured via to_bytes(body, 64 * 1024) BEFORE any serde parse.
# No serde_json::to_vec call on the parsed value may appear in the hash path.
```

**19.1c** — `verify_strict` used, not `verify`:

```bash
grep -rn "\.verify\b" crates/orrbeam-core/src/wire.rs
# Expected: zero hits for bare .verify( — only .verify_strict( is acceptable.
grep -rn "verify_strict" crates/orrbeam-core/src/wire.rs
# Expected: at least one hit.
```

**19.1d** — Empty signature header rejected explicitly:

```bash
grep -rn "empty\|is_empty\|X-Orrbeam-Signature" \
  crates/orrbeam-net/src/server/
# Expected: explicit check that rejects a missing or empty signature header
# before attempting base64 decode. Look for an early return / error variant.
```

---

### §19.2 Replay protection

**19.2a** — Every authenticated route hits `require_signed`:

```bash
grep -n "route\|require_signed" crates/orrbeam-net/src/server/mod.rs
# Expected: all /v1/* routes that are not hello/mutual-trust are wrapped in
# the require_signed layer_fn. Audit the router definition — every .route()
# call inside the authenticated subtree must pass through the middleware.
```

**19.2b** — Nonce cache is per-key-id, not global:

```bash
grep -rn "HashMap\|BTreeMap\|nonce" crates/orrbeam-net/src/server/nonce.rs
# Expected: the cache is keyed by a key-id (fingerprint or similar), so
# replaying a nonce from key A does not block key B.
```

**19.2c** — GC task is actually spawned:

```bash
grep -rn "tokio::spawn\|spawn_nonce_gc\|gc_task\|start_gc" \
  crates/orrbeam-net/src/server/
# Expected: a tokio::spawn call wiring up the GC loop — not just a function
# definition that is never called.
```

**19.2d** — Eviction test exists:

```bash
grep -rn "evict\|expired\|gc\|Expired" \
  crates/orrbeam-net/src/server/nonce.rs
# Expected: a #[test] or #[tokio::test] block exercising nonce eviction.
```

---

### §19.3 TLS pinning

**19.3a** — `danger_accept_invalid_certs` confined to bootstrap and mutual-trust:

```bash
grep -rn "danger_accept_invalid_certs" crates/
# Expected: exactly 3 hits — in bootstrap_hello, send_mutual_trust_request,
# poll_mutual_trust_request. Any additional hit is a security bug.
```

**19.3b** — `PinnedVerifier` uses constant-time compare:

```bash
grep -rn "ct_eq\|constant_time\|subtle\|ConstantTimeEq\|eq.*sha256\|sha256.*eq" \
  crates/orrbeam-net/src/client/
# Expected: the cert fingerprint comparison in PinnedVerifier uses a
# constant-time equality function, not plain ==.
```

**19.3c** — rustls ClientConfig restricted to TLS 1.3:

```bash
grep -rn "tls13\|with_safe_default\|protocol_versions\|TLS13\|tls_1_3" \
  crates/orrbeam-net/src/client/
# Expected: explicit restriction to TLS 1.3 only (no TLS 1.2 offered).
```

**19.3d** — Name verification skipped with explanatory comment:

```bash
grep -rn "verify_hostname\|ServerCertVerifier\|disable_name\|no_name" \
  crates/orrbeam-net/src/client/
# Expected: ServerCertVerifier impl or ClientConfig disables hostname
# verification with a comment like "// Hostname is not meaningful for
# self-signed certs; pinning is enforced by fingerprint instead."
```

---

### §19.4 Permission coverage

**19.4a** — Every authenticated handler checks its permission bit:

```bash
grep -rn "permissions\.\|can_start\|can_stop\|can_pair\|can_peer" \
  crates/orrbeam-net/src/server/handlers.rs 2>/dev/null || \
grep -rn "permissions\." crates/orrbeam-net/src/server/
# Expected: each handler that performs a privileged action reads the
# appropriate peer.permissions.* field and returns 403 if unset.
```

**19.4b** — Unauthenticated handlers never read PeerContext:

```bash
grep -rn "PeerContext\|peer_context\|peer\.permissions" \
  crates/orrbeam-net/src/server/
# Expected: PeerContext only appears in handlers behind require_signed.
# hello, mutual_trust_request, mutual_trust_poll must not reference it.
```

**19.4c** — `trusted_full()` covers all current permission fields:

```bash
grep -n "trusted_full\|PeerPermissions" \
  crates/orrbeam-core/src/peers.rs
# Expected: trusted_full() sets every field in PeerPermissions to true.
# Manually compare the struct definition with the trusted_full() body.
```

---

### §19.5 No secrets in logs

**19.5a** — PIN is redacted in logs:

```bash
grep -rn "tracing\|log!\|info!\|debug!\|warn!\|error!" \
  crates/orrbeam-net/src/ src-tauri/src/ \
  | grep -i "pin"
# Expected: any PIN-related log line uses a redacted placeholder like
# "PIN <redacted>" — never the actual PIN value.
```

**19.5b** — Signing key bytes not in tracing:

```bash
grep -rn "signing_key\|sign_key\|secret_key\|private_key" \
  crates/orrbeam-core/src/ crates/orrbeam-net/src/ src-tauri/src/ \
  | grep -v "^.*//\|load\|save\|path\|file\|pem\|test"
# Expected: no tracing macro receives a raw signing key. Debug impls on
# Identity / ControlState must mask the key bytes.
```

**19.5c** — Sunshine password not in tracing:

```bash
grep -rn "sunshine_password\|password" \
  crates/orrbeam-net/src/ src-tauri/src/ \
  | grep -i "info!\|debug!\|warn!\|error!\|tracing"
# Expected: zero hits — the password must never be passed to a tracing macro.
```

**19.5d** — Custom Debug on ControlState and Identity masks key material:

```bash
grep -rn "impl.*Debug.*ControlState\|impl.*Debug.*Identity\|fmt::Debug" \
  crates/orrbeam-net/src/ crates/orrbeam-core/src/
# Expected: manual Debug implementations that print "<redacted>" for key
# material, rather than derived Debug which would dump all fields.
```

---

### §19.6 No panic paths

**19.6a** — clippy unwrap/expect check on orrbeam-net:

```bash
cargo clippy -p orrbeam-net -- -D clippy::unwrap_used -D clippy::expect_used 2>&1
# Expected: no warnings or errors. Any unwrap/expect in non-test code fails this check.
```

**19.6b** — Body buffering uses 64 KiB limit:

```bash
grep -rn "to_bytes\|body.*bytes\|64 \* 1024\|65536" \
  crates/orrbeam-net/src/server/
# Expected: to_bytes(body, 64 * 1024) pattern — never to_bytes(body, usize::MAX)
# or similar unbounded read.
```

---

### §19.7 File permissions

**19.7a** — Sensitive files written with 0o600:

```bash
grep -rn "0o600\|set_permissions\|mode(0o6" \
  crates/orrbeam-core/src/tls.rs \
  crates/orrbeam-core/src/peers.rs \
  crates/orrbeam-core/src/identity.rs 2>/dev/null || \
grep -rn "0o600" crates/orrbeam-core/src/
# Expected: trusted_peers.yaml, control.key.pem, signing.key are written
# with Unix permissions 0o600.
```

**19.7b** — Parent directories created before writes:

```bash
grep -rn "create_dir_all\|create_dir" \
  crates/orrbeam-core/src/tls.rs \
  crates/orrbeam-core/src/peers.rs \
  crates/orrbeam-core/src/identity.rs 2>/dev/null || \
grep -rn "create_dir_all" crates/orrbeam-core/src/
# Expected: at least one create_dir_all call before each file write in
# the save paths for the sensitive files listed above.
```

---

### §19.8 Concurrency

**19.8a** — No blocking_lock in request path:

```bash
grep -rn "blocking_lock\|blocking_write\|blocking_read" \
  crates/orrbeam-net/src/server/
# Expected: zero hits. blocking_lock blocks the tokio executor thread.
```

**19.8b** — Middleware uses read(), handlers use write() only for touch_last_seen:

```bash
grep -rn "\.read()\|\.write()" \
  crates/orrbeam-net/src/server/
# Expected: require_signed and other middleware use .read() on the peer store.
# .write() appears only in the touch_last_seen update path — not in read-only handlers.
```

---

### §19.9 Mutual trust anti-abuse

**19.9a** — Rate limit on /v1/mutual-trust-request:

```bash
grep -rn "rate_limit\|RateLimiter\|3.*per.*min\|per_minute\|trust.*request" \
  crates/orrbeam-net/src/server/
# Expected: a rate limiter capping mutual-trust-request to 3/min/IP and
# max 1 pending globally. Manual step: confirm the limit constants match
# the spec (3 per minute per IP, 1 global pending cap).
```

**19.9b** — 60 s expiry enforced:

```bash
grep -rn "60\|expiry\|expires_at\|pending.*timeout\|Duration::from_secs" \
  crates/orrbeam-net/src/server/
# Expected: a 60-second timeout constant applied to pending trust requests,
# enforced either via a GC task or a lazy expiry check on read.
```

**19.9c** — Approval requires explicit user action (manual):

**Manual step**: In the orrbeam UI on the receiving node, confirm that:
1. No auto-accept path exists (no `auto_approve` flag, no config option to bypass).
2. The `MutualTrustRequestDialog` requires the user to click "Approve" — the request cannot be fulfilled by another API call or a config file edit while the app is running.

Expected: clicking the dialog is the sole code path that transitions a trust request from `pending` → `approved`.

---

### §19.10 Plan compliance spot checks

**19.10a** — No /v1/loop or /v1/connect-back:

```bash
grep -rn "v1/loop\|v1/connect.back\|connect_back\|loop_endpoint" \
  crates/orrbeam-net/src/ src-tauri/src/
# Expected: zero hits.
```

**19.10b** — No CLI surface added:

```bash
grep -rn "headless.accept.pin\|orrbeam.cli\|clap\|structopt\|ArgMatches" \
  src-tauri/src/ crates/
# Expected: zero hits. No CLI argument parsing in non-test code.
```

**19.10c** — No plaintext HTTP listener on 47782:

```bash
grep -rn "47782\|bind.*47782\|listen.*47782" crates/orrbeam-net/src/
# Expected: port 47782 binds only via axum-server's TLS acceptor — no
# plain http::Server or TcpListener without TLS wrapping this port.
```

**19.10d** — mDNS TXT exposes only fingerprint and cert_sha256:

```bash
grep -rn "TxtRecord\|txt_record\|mdns.*txt\|_orrbeam" \
  crates/orrbeam-net/src/
# Expected: TXT record construction includes only node_name, fingerprint,
# cert_sha256, and control_port. No private key bytes, no cert PEM.
```
