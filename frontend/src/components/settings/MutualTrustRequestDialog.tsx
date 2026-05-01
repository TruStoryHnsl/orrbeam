import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { usePeersStore } from "@/stores/peers";
import type { PendingMutualTrustSummary } from "@/types/peers";

interface Props {
  open: boolean;
  onClose: () => void;
  request: PendingMutualTrustSummary | null;
}

/** Receiver-side dialog: shown when an inbound mutual trust request arrives. */
export function MutualTrustRequestDialog({ open, onClose, request }: Props) {
  const approveMutualTrust = usePeersStore((s) => s.approveMutualTrust);
  const rejectMutualTrust = usePeersStore((s) => s.rejectMutualTrust);

  if (!request) return null;

  const fp = request.initiator_fingerprint;
  const fpSnippet = fp.length > 8 ? fp.slice(0, 8) + "…" : fp;

  const handleApprove = async () => {
    try {
      await approveMutualTrust(request.request_id);
    } finally {
      onClose();
    }
  };

  const handleReject = async () => {
    try {
      await rejectMutualTrust(request.request_id);
    } finally {
      onClose();
    }
  };

  return (
    <Modal
      open={open}
      onClose={handleReject}
      title="Incoming mutual trust request"
    >
      <div className="space-y-4">
        <p className="text-xs text-neutral-400">
          <span className="text-neutral-200 font-medium">{request.initiator_name}</span> wants to
          establish mutual trust with this node. Both nodes will trust each other after approval.
        </p>

        <div className="bg-surface-3 rounded-lg px-3 py-3 space-y-2">
          <div className="flex flex-col gap-0.5">
            <span className="text-[10px] text-neutral-500 uppercase tracking-wide">
              Initiator name
            </span>
            <span className="text-sm text-neutral-200 font-medium">{request.initiator_name}</span>
          </div>

          <div className="flex flex-col gap-0.5">
            <span className="text-[10px] text-neutral-500 uppercase tracking-wide">
              Fingerprint (first 8 chars)
            </span>
            <span className="text-sm font-mono text-neutral-300">{fpSnippet}</span>
          </div>

          {request.note && (
            <div className="flex flex-col gap-0.5">
              <span className="text-[10px] text-neutral-500 uppercase tracking-wide">Note</span>
              <span className="text-xs text-neutral-300 italic">{request.note}</span>
            </div>
          )}

          <div className="flex flex-col gap-0.5">
            <span className="text-[10px] text-neutral-500 uppercase tracking-wide">Received</span>
            <span className="text-xs text-neutral-400">
              {new Date(request.created_at).toLocaleString()}
            </span>
          </div>
        </div>

        <p className="text-[11px] text-neutral-500">
          Verify the fingerprint out of band before approving. You can check it in the{" "}
          <em>About</em> tab on the initiator machine.
        </p>

        <div className="flex gap-2">
          <Button variant="ghost" onClick={handleReject} className="flex-1">
            Reject
          </Button>
          <Button variant="moonlight" onClick={handleApprove} className="flex-1">
            Approve
          </Button>
        </div>
      </div>
    </Modal>
  );
}
