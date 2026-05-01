import { useEffect, useRef, useState } from "react";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { usePeersStore } from "@/stores/peers";
import type { MutualTrustInitResult } from "@/types/peers";

const TIMEOUT_MS = 30_000;
const POLL_INTERVAL_MS = 3_000;

interface Props {
  open: boolean;
  onClose: () => void;
  /** Result from requestMutualTrust — contains the request_id and receiver hello. */
  result: MutualTrustInitResult | null;
}

type Status = "waiting" | "approved" | "rejected" | "timeout" | "cancelled";

/** Initiator-side modal: shown while waiting for the remote peer to approve. */
export function MutualTrustPendingModal({ open, onClose, result }: Props) {
  const fetchInbound = usePeersStore((s) => s.fetchInbound);
  const peers = usePeersStore((s) => s.peers);

  const [status, setStatus] = useState<Status>("waiting");
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Check if peer has appeared in the trusted list (approval signal)
  const peerName = result?.receiver_hello.node_name;
  const isApproved = peers.some((p) => p.name === peerName);

  useEffect(() => {
    if (!open || !result) return;

    setStatus("waiting");

    // Poll inbound requests every 3s to refresh the list
    pollRef.current = setInterval(() => {
      fetchInbound().catch(() => {});
    }, POLL_INTERVAL_MS);

    // Timeout after 30s
    timerRef.current = setTimeout(() => {
      setStatus("timeout");
    }, TIMEOUT_MS);

    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [open, result, fetchInbound]);

  // Detect approval via peer appearing in the store
  useEffect(() => {
    if (status === "waiting" && isApproved) {
      setStatus("approved");
      if (pollRef.current) clearInterval(pollRef.current);
      if (timerRef.current) clearTimeout(timerRef.current);
      setTimeout(onClose, 1200);
    }
  }, [isApproved, status, onClose]);

  const handleCancel = () => {
    setStatus("cancelled");
    if (pollRef.current) clearInterval(pollRef.current);
    if (timerRef.current) clearTimeout(timerRef.current);
    onClose();
  };

  if (!result) return null;

  const peerDisplay = result.receiver_hello.node_name;
  const fpSnippet = result.receiver_hello.ed25519_fingerprint.slice(0, 8) + "…";

  return (
    <Modal open={open} onClose={handleCancel} title="Mutual trust request sent">
      {status === "waiting" && (
        <div className="space-y-4">
          <div className="text-center py-2">
            <div className="text-xs text-neutral-400 mb-3">
              Waiting for{" "}
              <span className="text-neutral-200 font-medium">{peerDisplay}</span> to approve…
            </div>
            <div className="animate-pulse text-moonlight text-lg">&#9679; &#9679; &#9679;</div>
          </div>

          <div className="bg-surface-3 rounded-lg px-3 py-2 space-y-1">
            <div className="flex justify-between text-xs">
              <span className="text-neutral-500">Peer</span>
              <span className="text-neutral-200">{peerDisplay}</span>
            </div>
            <div className="flex justify-between text-xs">
              <span className="text-neutral-500">Fingerprint</span>
              <span className="font-mono text-neutral-300">{fpSnippet}</span>
            </div>
          </div>

          <p className="text-[11px] text-neutral-500">
            A dialog has appeared on <span className="text-neutral-400">{peerDisplay}</span>. Ask
            the user there to approve. Times out in 30 s.
          </p>

          <Button variant="ghost" onClick={handleCancel} className="w-full">
            Cancel
          </Button>
        </div>
      )}

      {status === "approved" && (
        <div className="text-center py-4 space-y-2">
          <div className="text-green-400 text-sm font-medium">Mutual trust established</div>
          <div className="text-xs text-neutral-500">
            Both nodes now trust each other. Closing…
          </div>
        </div>
      )}

      {status === "timeout" && (
        <div className="space-y-3">
          <div className="text-xs text-amber-400 bg-amber-500/10 rounded px-3 py-2">
            The request timed out — the peer did not respond within 30 seconds.
          </div>
          <Button variant="ghost" onClick={handleCancel} className="w-full">
            Close
          </Button>
        </div>
      )}

      {status === "rejected" && (
        <div className="space-y-3">
          <div className="text-xs text-red-400 bg-red-500/10 rounded px-3 py-2">
            The peer rejected the trust request.
          </div>
          <Button variant="ghost" onClick={handleCancel} className="w-full">
            Close
          </Button>
        </div>
      )}
    </Modal>
  );
}
