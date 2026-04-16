import { useEffect, useRef, useState } from "react";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { onPeeringProgress } from "@/api/tauri";
import type { PeeringProgress } from "@/types/peers";

// Ordered stage list — matches the Rust state machine in src-tauri/src/commands/remote.rs
const STAGES = [
  { key: "resolving", label: "Resolving peer" },
  { key: "probing", label: "Probing orrbeam on peer" },
  { key: "remote_starting", label: "Starting Sunshine on peer" },
  { key: "pin_generating", label: "Submitting pairing PIN" },
  { key: "paired_parallel", label: "Completing pairing" },
  { key: "streaming_local", label: "Starting local Moonlight" },
  { key: "done", label: "Connected" },
];

type StageState = "pending" | "active" | "done" | "error";

function StageRow({
  label,
  state,
  detail,
}: {
  label: string;
  state: StageState;
  detail?: string | null;
}) {
  return (
    <div
      className={`flex items-start gap-2 py-1 transition-opacity ${state === "pending" ? "opacity-40" : "opacity-100"}`}
    >
      <span
        className={`mt-0.5 text-xs w-4 flex-shrink-0 ${
          state === "done"
            ? "text-green-400"
            : state === "error"
              ? "text-red-400"
              : state === "active"
                ? "text-moonlight animate-pulse"
                : "text-neutral-600"
        }`}
      >
        {state === "done" ? "\u2713" : state === "error" ? "\u2717" : state === "active" ? "\u25cf" : "\u25cb"}
      </span>
      <div className="flex flex-col gap-0.5 min-w-0">
        <span
          className={`text-xs ${
            state === "error"
              ? "text-red-400"
              : state === "done"
                ? "text-neutral-300"
                : state === "active"
                  ? "text-neutral-200 font-medium"
                  : "text-neutral-500"
          }`}
        >
          {label}
        </span>
        {detail && state === "error" && (
          <span className="text-[10px] text-red-400/80 break-all">{detail}</span>
        )}
        {detail && state === "active" && (
          <span className="text-[10px] text-neutral-500 break-all">{detail}</span>
        )}
      </div>
    </div>
  );
}

interface Props {
  open: boolean;
  peerName: string;
  onClose: () => void;
  /** Called when done — parent can refresh node list */
  onSuccess?: () => void;
}

export function PeeringProgressModal({ open, peerName, onClose, onSuccess }: Props) {
  const [activeStage, setActiveStage] = useState<string | null>(null);
  const [completedStages, setCompletedStages] = useState<Set<string>>(new Set());
  const [failedStage, setFailedStage] = useState<string | null>(null);
  const [failedDetail, setFailedDetail] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [isDone, setIsDone] = useState(false);
  const unlistenRef = useRef<(() => void) | null>(null);

  const reset = () => {
    setActiveStage(null);
    setCompletedStages(new Set());
    setFailedStage(null);
    setFailedDetail(null);
    setLogs([]);
    setIsDone(false);
  };

  useEffect(() => {
    if (!open) return;
    reset();

    onPeeringProgress((progress: PeeringProgress) => {
      setLogs((prev) => [
        ...prev,
        `[${progress.stage}] ${progress.detail ?? ""}${progress.error ? " ERROR: " + progress.error : ""}`,
      ]);

      if (progress.error) {
        setFailedStage(progress.stage);
        setFailedDetail(progress.error);
        setActiveStage(null);
        return;
      }

      if (progress.stage === "done") {
        setCompletedStages((prev) => new Set([...prev, progress.stage]));
        setActiveStage(null);
        setIsDone(true);
        return;
      }

      // Mark previous active stage as complete, set new active
      setActiveStage((prev) => {
        if (prev) setCompletedStages((s) => new Set([...s, prev]));
        return progress.stage;
      });
    }).then((unlisten) => {
      unlistenRef.current = unlisten;
    });

    return () => {
      unlistenRef.current?.();
      unlistenRef.current = null;
    };
  }, [open]);

  // Auto-close on done after 1s
  useEffect(() => {
    if (isDone) {
      const t = setTimeout(() => {
        onSuccess?.();
        onClose();
      }, 1000);
      return () => clearTimeout(t);
    }
  }, [isDone, onClose, onSuccess]);

  const handleCopyLogs = async () => {
    try {
      await navigator.clipboard.writeText(logs.join("\n"));
    } catch {
      // clipboard unavailable
    }
  };

  const getStageState = (key: string): StageState => {
    if (failedStage === key) return "error";
    if (completedStages.has(key)) return "done";
    if (activeStage === key) return "active";
    return "pending";
  };

  return (
    <Modal open={open} onClose={onClose} title={`Connecting to ${peerName}`}>
      <div className="space-y-4">
        <div className="space-y-0.5">
          {STAGES.map((s) => (
            <StageRow
              key={s.key}
              label={s.label}
              state={getStageState(s.key)}
              detail={
                (failedStage === s.key && failedDetail) ||
                (activeStage === s.key ? null : null)
              }
            />
          ))}
        </div>

        {failedStage && (
          <div className="space-y-2">
            <div className="text-xs text-red-400 bg-red-500/10 rounded px-3 py-2">
              {failedDetail ?? "Connection failed"}
            </div>
            <div className="flex gap-2">
              <Button variant="ghost" onClick={handleCopyLogs} size="sm" className="flex-1">
                Copy logs
              </Button>
              <Button variant="ghost" onClick={onClose} size="sm" className="flex-1">
                Close
              </Button>
            </div>
          </div>
        )}

        {isDone && (
          <div className="text-center text-xs text-green-400 animate-pulse">
            Connected. Closing\u2026
          </div>
        )}

        {!failedStage && !isDone && (
          <Button variant="ghost" onClick={onClose} className="w-full" size="sm">
            Cancel
          </Button>
        )}
      </div>
    </Modal>
  );
}
