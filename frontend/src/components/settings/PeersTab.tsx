import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { usePeersStore } from "@/stores/peers";
import { usePlatformStore } from "@/stores/platform";
import type { TrustedPeer, PendingMutualTrustSummary, MutualTrustInitResult } from "@/types/peers";
import { TofuDialog } from "./TofuDialog";
import { MutualTrustRequestDialog } from "./MutualTrustRequestDialog";
import { MutualTrustPendingModal } from "./MutualTrustPendingModal";

// ── Helpers ──────────────────────────────────────────────────────────────────

function snip(s: string, n = 8): string {
  return s.length > n ? s.slice(0, n) + "\u2026" : s;
}

function parseAddress(raw: string): { address: string; port: number } | null {
  const trimmed = raw.trim();
  const lastColon = trimmed.lastIndexOf(":");
  if (lastColon > 0) {
    const host = trimmed.slice(0, lastColon);
    const port = parseInt(trimmed.slice(lastColon + 1), 10);
    if (host && !isNaN(port) && port > 0 && port < 65536) {
      return { address: host, port };
    }
  }
  return null;
}

// ── Sub-components ────────────────────────────────────────────────────────────

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-[10px] font-medium text-neutral-500 uppercase tracking-wider mb-2">
      {children}
    </h3>
  );
}

function PermSummary({ p }: { p: TrustedPeer["permissions"] }) {
  const grants: string[] = [];
  if (p.can_start_sunshine) grants.push("start");
  if (p.can_stop_sunshine) grants.push("stop");
  if (p.can_submit_pin) grants.push("pin");
  if (p.can_list_peers) grants.push("peers");
  return (
    <span className="text-[10px] text-neutral-500">
      {grants.length ? grants.join(", ") : "read-only"}
    </span>
  );
}

function PeerRow({
  peer,
  onRemove,
}: {
  peer: TrustedPeer;
  onRemove: () => void;
}) {
  return (
    <div className="flex flex-col gap-1 bg-surface-2 rounded-lg px-3 py-2 border border-surface-3">
      <div className="flex items-start justify-between gap-2">
        <div className="flex flex-col gap-0.5 min-w-0">
          <span className="text-sm text-neutral-200 font-medium">{peer.name}</span>
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-[10px] font-mono text-neutral-400">
              {snip(peer.ed25519_fingerprint)}
            </span>
            <span className="text-[10px] font-mono text-neutral-500">
              {snip(peer.cert_sha256)}
            </span>
            {peer.tags.map((t) => (
              <span key={t} className="text-[10px] bg-surface-3 rounded px-1 text-neutral-500">
                {t}
              </span>
            ))}
          </div>
          <PermSummary p={peer.permissions} />
          {peer.last_seen_at && (
            <span className="text-[10px] text-neutral-600">
              Last seen: {new Date(peer.last_seen_at).toLocaleString()}
            </span>
          )}
        </div>
        <button
          onClick={onRemove}
          className="px-2 py-1 rounded text-[10px] text-neutral-500 hover:text-red-400 hover:bg-red-500/10 transition-colors flex-shrink-0"
          title="Remove peer"
        >
          Remove
        </button>
      </div>
    </div>
  );
}

function InboundRequestRow({
  req,
  onSelect,
}: {
  req: PendingMutualTrustSummary;
  onSelect: () => void;
}) {
  return (
    <div className="flex items-center justify-between bg-amber-500/5 border border-amber-500/20 rounded-lg px-3 py-2 gap-2">
      <div className="flex flex-col gap-0.5 min-w-0">
        <span className="text-sm text-neutral-200 font-medium">{req.initiator_name}</span>
        <span className="text-[10px] font-mono text-neutral-400">
          {snip(req.initiator_fingerprint)}
        </span>
        {req.note && (
          <span className="text-[10px] text-neutral-500 italic">{req.note}</span>
        )}
      </div>
      <Button variant="moonlight" onClick={onSelect} className="text-xs py-1 px-3 flex-shrink-0">
        Review
      </Button>
    </div>
  );
}

// ── Add-peer form ─────────────────────────────────────────────────────────────

type AddMode = "mutual" | "manual";

interface AddPeerFormProps {
  onTofuReady: (address: string, port: number) => void;
  onMutualResult: (result: MutualTrustInitResult) => void;
}

function AddPeerForm({ onTofuReady, onMutualResult }: AddPeerFormProps) {
  const requestMutualTrust = usePeersStore((s) => s.requestMutualTrust);
  const [mode, setMode] = useState<AddMode>("mutual");
  const [rawAddr, setRawAddr] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const parsed = parseAddress(rawAddr);

  const handleMutual = async () => {
    if (!parsed) {
      setErr("Enter address:port");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      const result = await requestMutualTrust(parsed.address, parsed.port);
      onMutualResult(result);
      setRawAddr("");
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleManual = () => {
    if (!parsed) {
      setErr("Enter address:port");
      return;
    }
    setErr(null);
    onTofuReady(parsed.address, parsed.port);
  };

  return (
    <div className="space-y-3 bg-surface-2 rounded-lg px-3 py-3 border border-surface-3">
      <div className="flex gap-4">
        {(["mutual", "manual"] as AddMode[]).map((m) => (
          <label key={m} className="flex items-center gap-1.5 cursor-pointer">
            <input
              type="radio"
              name="add-mode"
              value={m}
              checked={mode === m}
              onChange={() => {
                setMode(m);
                setErr(null);
              }}
              className="accent-moonlight"
            />
            <span className="text-xs text-neutral-300">
              {m === "mutual" ? "Request mutual trust" : "Manual (one-way)"}
            </span>
          </label>
        ))}
      </div>

      <input
        type="text"
        value={rawAddr}
        onChange={(e) => {
          setRawAddr(e.target.value);
          setErr(null);
        }}
        placeholder="address:port  (e.g. 100.66.55.59:47782)"
        className="w-full bg-surface-3 rounded px-2 py-1.5 text-xs text-neutral-200 border border-surface-4 focus:outline-none focus:border-moonlight/40 font-mono placeholder:font-sans placeholder:text-neutral-600"
      />

      {err && <div className="text-[11px] text-red-400">{err}</div>}

      {mode === "mutual" ? (
        <Button
          variant="moonlight"
          onClick={handleMutual}
          disabled={busy || !rawAddr.trim()}
          className="w-full disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {busy ? "Sending\u2026" : "Request mutual trust"}
        </Button>
      ) : (
        <Button
          variant="ghost"
          onClick={handleManual}
          disabled={!rawAddr.trim()}
          className="w-full disabled:opacity-40 disabled:cursor-not-allowed"
        >
          Fetch fingerprint
        </Button>
      )}
    </div>
  );
}

// ── Identity section ──────────────────────────────────────────────────────────

function IdentitySection() {
  const identity = usePlatformStore((s) => s.identity);
  const info = usePlatformStore((s) => s.info);
  const [copied, setCopied] = useState(false);

  const handleShare = async () => {
    const blob = {
      node_name: info?.hostname ?? "unknown",
      ed25519_fingerprint: identity?.fingerprint ?? "",
      public_key: identity?.public_key ?? [],
    };
    try {
      await navigator.clipboard.writeText(JSON.stringify(blob, null, 2));
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard unavailable in some contexts
    }
  };

  return (
    <div className="bg-surface-2 rounded-lg px-3 py-3 border border-surface-3 space-y-1">
      <div className="flex items-center justify-between">
        <div className="flex flex-col gap-0.5">
          <span className="text-[10px] text-neutral-500 uppercase tracking-wide">This node</span>
          <span className="text-sm text-neutral-200 font-medium">{info?.hostname ?? "\u2014"}</span>
          {identity?.fingerprint && (
            <span className="text-[10px] font-mono text-neutral-400">
              {snip(identity.fingerprint)}
            </span>
          )}
        </div>
        <button
          onClick={handleShare}
          className="px-2 py-1 rounded text-[10px] text-neutral-500 hover:text-neutral-300 hover:bg-surface-3 transition-colors"
          title="Copy identity JSON for out-of-band verification"
        >
          {copied ? "Copied!" : "Share"}
        </button>
      </div>
    </div>
  );
}

// ── Main PeersTab ─────────────────────────────────────────────────────────────

export function PeersTab() {
  const { peers, inboundMutualTrust, loading, error, fetch, fetchInbound, removePeer } =
    usePeersStore();

  // TOFU dialog
  const [tofuOpen, setTofuOpen] = useState(false);
  const [tofuAddress, setTofuAddress] = useState("");
  const [tofuPort, setTofuPort] = useState(47782);

  // Mutual trust pending modal (initiator side)
  const [pendingResult, setPendingResult] = useState<MutualTrustInitResult | null>(null);
  const [pendingOpen, setPendingOpen] = useState(false);

  // Mutual trust review dialog (receiver side)
  const [reviewRequest, setReviewRequest] = useState<PendingMutualTrustSummary | null>(null);
  const [reviewOpen, setReviewOpen] = useState(false);

  // Load on mount + poll every 3s for inbound requests
  useEffect(() => {
    fetch();
    fetchInbound();
    const interval = setInterval(() => {
      fetchInbound();
    }, 3_000);
    return () => clearInterval(interval);
  }, [fetch, fetchInbound]);

  const handleTofuReady = (address: string, port: number) => {
    setTofuAddress(address);
    setTofuPort(port);
    setTofuOpen(true);
  };

  const handleMutualResult = (result: MutualTrustInitResult) => {
    setPendingResult(result);
    setPendingOpen(true);
  };

  const handleRemovePeer = async (name: string) => {
    if (!confirm(`Remove ${name} from trusted peers?`)) return;
    try {
      await removePeer(name);
    } catch {
      // error already stored in peers store
    }
  };

  return (
    <div className="space-y-5 p-4">
      {/* Your identity */}
      <div>
        <SectionTitle>Your identity</SectionTitle>
        <IdentitySection />
      </div>

      {/* Inbound mutual trust requests */}
      {inboundMutualTrust.length > 0 && (
        <div>
          <SectionTitle>Inbound trust requests</SectionTitle>
          <div className="space-y-2">
            {inboundMutualTrust.map((req) => (
              <InboundRequestRow
                key={req.request_id}
                req={req}
                onSelect={() => {
                  setReviewRequest(req);
                  setReviewOpen(true);
                }}
              />
            ))}
          </div>
        </div>
      )}

      {/* Trusted peers */}
      <div>
        <SectionTitle>Trusted peers ({peers.length})</SectionTitle>
        {loading && peers.length === 0 ? (
          <div className="text-xs text-neutral-500 py-2">Loading\u2026</div>
        ) : error ? (
          <div className="text-xs text-red-400">{error}</div>
        ) : peers.length === 0 ? (
          <div className="text-xs text-neutral-500 py-2">
            No trusted peers yet. Add one below.
          </div>
        ) : (
          <div className="space-y-2">
            {peers.map((p) => (
              <PeerRow key={p.name} peer={p} onRemove={() => handleRemovePeer(p.name)} />
            ))}
          </div>
        )}
      </div>

      {/* Add peer */}
      <div>
        <SectionTitle>Add peer</SectionTitle>
        <AddPeerForm onTofuReady={handleTofuReady} onMutualResult={handleMutualResult} />
      </div>

      {/* Dialogs (rendered outside the scrollable flow) */}
      <TofuDialog
        open={tofuOpen}
        onClose={() => setTofuOpen(false)}
        hello={null}
        address={tofuAddress}
        port={tofuPort}
        onSuccess={() => fetch()}
      />

      <MutualTrustRequestDialog
        open={reviewOpen}
        onClose={() => {
          setReviewOpen(false);
          fetchInbound();
        }}
        request={reviewRequest}
      />

      <MutualTrustPendingModal
        open={pendingOpen}
        onClose={() => {
          setPendingOpen(false);
          fetch();
        }}
        result={pendingResult}
      />
    </div>
  );
}
