# Verifying the Control Plane — orrion ↔ orrpheus

This document is a placeholder for the end-to-end smoke-test runbook (WI-17).
Full verification steps will be written once both nodes have the v2 build deployed.

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

## Full runbook

TODO (WI-17): Write `docs/verifying_control_plane.md` with exact commands, expected
JSON output, and screenshots of each UI state. Requires both nodes at the same commit.
