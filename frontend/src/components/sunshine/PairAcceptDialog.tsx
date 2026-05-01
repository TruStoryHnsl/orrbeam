import { useState } from "react";
import { invoke } from "@/api/tauri";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";

export function PairAcceptDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [pin, setPin] = useState("");
  const [clientName, setClientName] = useState("");
  const [state, setState] = useState<"idle" | "submitting" | "done" | "error">("idle");
  const [error, setError] = useState("");

  const submit = async () => {
    if (pin.length !== 4) return;
    setState("submitting");
    try {
      const accepted = (await invoke("pair_accept", {
        pin,
        clientName: clientName || undefined,
      })) as boolean;
      if (accepted) {
        setState("done");
      } else {
        setError("PIN was not accepted by Sunshine.");
        setState("error");
      }
    } catch (e) {
      setError(String(e));
      setState("error");
    }
  };

  const handleClose = () => {
    setState("idle");
    setPin("");
    setClientName("");
    setError("");
    onClose();
  };

  return (
    <Modal open={open} onClose={handleClose} title="Accept Pairing">
      {state === "idle" && (
        <div className="space-y-3">
          <p className="text-xs text-neutral-400">
            A remote Moonlight client is trying to pair with your Sunshine. Enter the 4-digit PIN
            shown on the remote machine.
          </p>

          {/* PIN input */}
          <div className="flex justify-center">
            <input
              type="text"
              value={pin}
              onChange={(e) => {
                const v = e.target.value.replace(/\D/g, "").slice(0, 4);
                setPin(v);
              }}
              placeholder="0000"
              className="bg-surface-3 text-center text-2xl font-mono tracking-[0.3em] text-sunshine-bright rounded-lg px-6 py-3 border border-surface-4 outline-none focus:border-sunshine/50 w-40"
              autoFocus
            />
          </div>

          {/* Optional client name */}
          <div className="flex items-center justify-between text-xs">
            <span className="text-neutral-500">Client name (optional)</span>
            <input
              value={clientName}
              onChange={(e) => setClientName(e.target.value)}
              placeholder="remote"
              className="bg-surface-3 text-neutral-200 text-[11px] rounded px-2 py-1 border-0 outline-none w-28 text-right"
            />
          </div>

          <Button
            variant="sunshine"
            onClick={submit}
            disabled={pin.length !== 4}
            className="w-full"
          >
            Accept Pairing
          </Button>
        </div>
      )}

      {state === "submitting" && (
        <div className="text-center py-4">
          <div className="text-xs text-neutral-400 mb-2">Submitting PIN to Sunshine...</div>
          <p className="text-[11px] text-neutral-500">
            This may take up to 15 seconds while waiting for the pairing handshake.
          </p>
        </div>
      )}

      {state === "done" && (
        <div className="space-y-3 text-center py-2">
          <div className="text-green-400 text-sm font-medium">Pairing successful</div>
          <p className="text-xs text-neutral-400">
            The remote client is now authorized to stream from your Sunshine.
          </p>
          <Button variant="ghost" onClick={handleClose} className="w-full">
            Done
          </Button>
        </div>
      )}

      {state === "error" && (
        <div className="space-y-3">
          <div className="text-xs text-red-400 bg-red-500/10 rounded px-3 py-2">{error}</div>
          <Button variant="ghost" onClick={() => setState("idle")} className="w-full">
            Try Again
          </Button>
        </div>
      )}
    </Modal>
  );
}
