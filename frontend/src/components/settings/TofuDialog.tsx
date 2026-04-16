import { useState } from "react";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { usePeersStore } from "@/stores/peers";
import type { HelloPayload, PeerDraft } from "@/types/peers";

/** Format a hex string into space-separated groups of N chars. */
function groupHex(hex: string, n = 4): string {
  const chunks: string[] = [];
  for (let i = 0; i < hex.length; i += n) chunks.push(hex.slice(i, i + n));
  return chunks.join(" ");
}

type TofuState = "idle" | "fetching" | "review" | "persisting" | "done" | "error";

interface Props {
  open: boolean;
  onClose: () => void;
  /** Pre-fetched hello payload (passed when triggered from manual fetch flow) */
  hello: HelloPayload | null;
  /** Address used to fetch this peer (e.g. "100.66.55.59") */
  address: string;
  /** Port used */
  port: number;
  /** Called after peer is successfully saved */
  onSuccess?: () => void;
}

export function TofuDialog({ open, onClose, hello, address, port, onSuccess }: Props) {
  const confirmPeer = usePeersStore((s) => s.confirmPeer);
  const fetchPeerHello = usePeersStore((s) => s.fetchPeerHello);

  const [state, setState] = useState<TofuState>(hello ? "review" : "idle");
  const [current, setCurrent] = useState<HelloPayload | null>(hello);
  const [verified, setVerified] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Override-able name the user can edit before saving
  const [peerName, setPeerName] = useState(hello?.node_name ?? "");

  // Sync when hello prop changes (parent re-opens dialog with new data)
  const effectiveHello = current ?? hello;

  const reset = () => {
    setState(hello ? "review" : "idle");
    setCurrent(hello);
    setVerified(false);
    setError(null);
    setPeerName(hello?.node_name ?? "");
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleFetch = async () => {
    setState("fetching");
    setError(null);
    try {
      const payload = await fetchPeerHello(address, port);
      setCurrent(payload);
      setPeerName(payload.node_name);
      setState("review");
    } catch (e) {
      setError(String(e));
      setState("error");
    }
  };

  const handleSave = async () => {
    if (!effectiveHello) return;
    setState("persisting");
    try {
      const draft: PeerDraft = {
        name: peerName.trim() || effectiveHello.node_name,
        ed25519_fingerprint: effectiveHello.ed25519_fingerprint,
        ed25519_public_key_b64: effectiveHello.ed25519_public_key_b64,
        cert_sha256: effectiveHello.cert_sha256,
        address,
        control_port: port,
        tags: [],
        note: null,
      };
      await confirmPeer(draft);
      setState("done");
      setTimeout(() => {
        handleClose();
        onSuccess?.();
      }, 800);
    } catch (e) {
      setError(String(e));
      setState("error");
    }
  };

  return (
    <Modal open={open} onClose={handleClose} title="Trust new peer">
      {state === "idle" && (
        <div className="space-y-3">
          <p className="text-xs text-neutral-400">
            Fetch the fingerprint from{" "}
            <span className="font-mono text-neutral-200">
              {address}:{port}
            </span>{" "}
            and verify it out of band before trusting.
          </p>
          <Button variant="moonlight" onClick={handleFetch} className="w-full">
            Fetch fingerprint
          </Button>
        </div>
      )}

      {state === "fetching" && (
        <div className="text-center py-4">
          <div className="text-xs text-neutral-400 mb-2">Contacting peer…</div>
          <div className="animate-pulse text-moonlight">&#9679; &#9679; &#9679;</div>
        </div>
      )}

      {state === "review" && effectiveHello && (
        <div className="space-y-4">
          <div className="space-y-3">
            <div className="flex flex-col gap-0.5">
              <span className="text-xs text-neutral-500 uppercase tracking-wide">Node name</span>
              <input
                type="text"
                value={peerName}
                onChange={(e) => setPeerName(e.target.value)}
                className="bg-surface-3 rounded px-2 py-1 text-sm text-neutral-200 border border-surface-4 focus:outline-none focus:border-moonlight/40"
                placeholder={effectiveHello.node_name}
              />
              <span className="text-[10px] text-neutral-600">
                Edit to override the display name
              </span>
            </div>

            <div className="flex flex-col gap-0.5">
              <span className="text-xs text-neutral-500 uppercase tracking-wide">
                Ed25519 fingerprint
              </span>
              <span className="text-sm font-mono text-neutral-200 break-all">
                {effectiveHello.ed25519_fingerprint}
              </span>
            </div>

            <div className="flex flex-col gap-0.5">
              <span className="text-xs text-neutral-500 uppercase tracking-wide">
                TLS cert SHA-256
              </span>
              <span className="text-sm font-mono text-neutral-200 break-all leading-relaxed">
                {groupHex(effectiveHello.cert_sha256, 4)}
              </span>
            </div>

            <div className="flex flex-col gap-1 text-xs text-neutral-500">
              <span>
                OS: <span className="text-neutral-300">{effectiveHello.os}</span>
              </span>
              <span>
                Port:{" "}
                <span className="text-neutral-300 font-mono">{effectiveHello.control_port}</span>
              </span>
            </div>
          </div>

          <label className="flex items-start gap-2 cursor-pointer group">
            <input
              type="checkbox"
              checked={verified}
              onChange={(e) => setVerified(e.target.checked)}
              className="mt-0.5 accent-moonlight"
            />
            <span className="text-xs text-neutral-400 group-hover:text-neutral-300 transition-colors">
              I have verified this fingerprint out of band
            </span>
          </label>

          <Button
            variant="moonlight"
            onClick={handleSave}
            disabled={!verified}
            className="w-full disabled:opacity-40 disabled:cursor-not-allowed"
          >
            Trust and save
          </Button>
        </div>
      )}

      {state === "persisting" && (
        <div className="text-center py-4">
          <div className="text-xs text-neutral-400 mb-2">Saving peer…</div>
          <div className="animate-pulse text-moonlight">&#9679; &#9679; &#9679;</div>
        </div>
      )}

      {state === "done" && (
        <div className="text-center py-4">
          <div className="text-green-400 text-sm font-medium mb-1">Peer trusted</div>
          <div className="text-xs text-neutral-500">Closing…</div>
        </div>
      )}

      {state === "error" && (
        <div className="space-y-3">
          <div className="text-xs text-red-400 bg-red-500/10 rounded px-3 py-2">{error}</div>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={reset} className="flex-1">
              Try again
            </Button>
            <Button variant="ghost" onClick={handleClose} className="flex-1">
              Close
            </Button>
          </div>
        </div>
      )}
    </Modal>
  );
}
