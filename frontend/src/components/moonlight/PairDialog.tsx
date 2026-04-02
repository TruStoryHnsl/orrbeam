import { useState } from "react";
import { invoke } from "@/api/tauri";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";

interface PairInitResult {
  pin: string;
  target: string;
  started: boolean;
}

export function PairInitiateDialog({
  open,
  onClose,
  address,
  nodeName,
}: {
  open: boolean;
  onClose: () => void;
  address: string;
  nodeName: string;
}) {
  const [state, setState] = useState<"idle" | "pairing" | "done" | "error">(
    "idle",
  );
  const [pin, setPin] = useState("");
  const [error, setError] = useState("");

  const startPairing = async () => {
    setState("pairing");
    try {
      const result = (await invoke("pair_initiate", {
        address,
      })) as PairInitResult;
      setPin(result.pin);
      setState("done");
    } catch (e) {
      setError(String(e));
      setState("error");
    }
  };

  const handleClose = () => {
    setState("idle");
    setPin("");
    setError("");
    onClose();
  };

  return (
    <Modal open={open} onClose={handleClose} title={`Pair with ${nodeName}`}>
      {state === "idle" && (
        <div className="space-y-3">
          <p className="text-xs text-neutral-400">
            This will initiate a Moonlight pairing handshake with{" "}
            <span className="text-neutral-200">{nodeName}</span>'s Sunshine.
            A 4-digit PIN will be generated — enter it on the remote machine to
            complete pairing.
          </p>
          <Button
            variant="moonlight"
            onClick={startPairing}
            className="w-full"
          >
            Start Pairing
          </Button>
        </div>
      )}

      {state === "pairing" && (
        <div className="text-center py-4">
          <div className="text-xs text-neutral-400 mb-2">
            Initiating pairing handshake...
          </div>
          <div className="animate-pulse text-moonlight">&#9679; &#9679; &#9679;</div>
        </div>
      )}

      {state === "done" && (
        <div className="space-y-4">
          <p className="text-xs text-neutral-400">
            Moonlight pairing started. Enter this PIN on{" "}
            <span className="text-neutral-200">{nodeName}</span>'s Sunshine web
            UI or Orrbeam app:
          </p>
          <div className="text-center">
            <div className="inline-block bg-surface-3 rounded-lg px-8 py-4 border border-moonlight/30">
              <span className="text-3xl font-mono font-bold tracking-[0.3em] text-moonlight-bright">
                {pin}
              </span>
            </div>
          </div>
          <p className="text-[11px] text-neutral-500 text-center">
            The pairing will complete automatically once the PIN is entered on
            the remote machine.
          </p>
          <Button variant="ghost" onClick={handleClose} className="w-full">
            Done
          </Button>
        </div>
      )}

      {state === "error" && (
        <div className="space-y-3">
          <div className="text-xs text-red-400 bg-red-500/10 rounded px-3 py-2">
            {error}
          </div>
          <Button variant="ghost" onClick={handleClose} className="w-full">
            Close
          </Button>
        </div>
      )}
    </Modal>
  );
}
