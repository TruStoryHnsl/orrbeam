import { useState } from "react";
import { useSunshineStore } from "@/stores/sunshine";
import type { SunshineSettings } from "@/stores/sunshine";
import { useSharedControlStore } from "@/stores/sharedControl";
import { StatusDot } from "@/components/ui/StatusDot";
import { Button } from "@/components/ui/Button";
import { PairAcceptDialog } from "./PairAcceptDialog";

const CODEC_OPTIONS = ["h264", "h265", "av1"];
const FPS_OPTIONS = [30, 60, 90, 120];
const BITRATE_PRESETS = [
  { label: "5 Mbps", value: 5000 },
  { label: "10 Mbps", value: 10000 },
  { label: "20 Mbps", value: 20000 },
  { label: "40 Mbps", value: 40000 },
  { label: "80 Mbps", value: 80000 },
];

export function SunshinePanel() {
  const {
    status,
    gpu,
    monitors,
    settings,
    loading,
    error,
    start,
    stop,
    setMonitor,
    updateSettings,
  } = useSunshineStore();

  const {
    enabled: scEnabled,
    participants,
    loading: scLoading,
    error: scError,
    start: scStart,
    stop: scStop,
    addParticipant,
    removeParticipant,
  } = useSharedControlStore();

  const [pairOpen, setPairOpen] = useState(false);
  const [newParticipant, setNewParticipant] = useState("");

  const isRunning = status?.status === "running";
  const isInstalled = status?.status !== "not_installed";

  return (
    <div className="flex-1 flex flex-col p-4 bg-surface-1 overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <span className="text-sunshine text-lg">&#9788;</span>
          <h2 className="text-sm font-semibold uppercase tracking-wider text-sunshine">Sunshine</h2>
          <span className="text-xs text-neutral-500">Host</span>
        </div>
        {status && <StatusDot status={status.status} />}
      </div>

      {/* Service info */}
      <Section title="Service">
        <InfoRow label="Status" value={status?.status ?? "checking..."} />
        {status?.version && <InfoRow label="Version" value={status.version} />}
      </Section>

      {/* GPU & Encoder */}
      {gpu && (
        <Section title="Encoder">
          <InfoRow label="GPU" value={gpu.name} />
          <InfoRow label="Encoder" value={gpu.encoder} />
          {gpu.driver && <InfoRow label="Driver" value={gpu.driver} />}
        </Section>
      )}

      {/* Monitor selector */}
      {monitors.length > 0 && (
        <Section title="Monitor">
          <div className="space-y-1">
            {monitors.map((m) => {
              const isSelected = settings?.output_name === m.name;
              return (
                <button
                  key={m.name}
                  onClick={() => setMonitor(m.name)}
                  className={`w-full flex items-center justify-between text-xs rounded px-2.5 py-2 transition-colors ${
                    isSelected
                      ? "bg-sunshine/15 border border-sunshine/30 text-sunshine"
                      : "bg-surface-2 border border-transparent text-neutral-300 hover:bg-surface-3"
                  }`}
                >
                  <span className="flex items-center gap-1.5">
                    {isSelected && <span className="text-sunshine">●</span>}
                    {m.name}
                    {m.primary && <span className="text-[10px] text-neutral-500">primary</span>}
                  </span>
                  <span className="text-neutral-500">
                    {m.resolution}
                    {m.refresh_rate && ` @ ${m.refresh_rate}Hz`}
                  </span>
                </button>
              );
            })}
          </div>
        </Section>
      )}

      {/* Stream settings */}
      {settings && <StreamSettings settings={settings} onSave={updateSettings} />}

      {/* Error */}
      {error && (
        <div className="text-xs text-red-400 bg-red-500/10 rounded px-2 py-1.5 mb-3">{error}</div>
      )}

      {/* Actions */}
      <div className="mt-auto pt-4 space-y-2">
        {!isInstalled ? (
          <p className="text-xs text-neutral-500">Sunshine is not installed.</p>
        ) : isRunning ? (
          <>
            <Button variant="danger" onClick={stop} disabled={loading} className="w-full">
              {loading ? "Stopping..." : "Stop Hosting"}
            </Button>
            <Button variant="ghost" onClick={() => setPairOpen(true)} className="w-full" size="sm">
              Accept Pairing
            </Button>
          </>
        ) : (
          <Button variant="sunshine" onClick={start} disabled={loading} className="w-full">
            {loading ? "Starting..." : "Start Hosting"}
          </Button>
        )}
      </div>

      {/* Shared Control */}
      <div className="mt-4 pt-4 border-t border-neutral-800">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-neutral-400 uppercase tracking-wider">
            Control Mode
          </span>
          {/* Solo / Shared-control pill toggle */}
          <div className="flex items-center rounded-full bg-surface-2 border border-neutral-700 p-0.5 gap-0.5">
            <button
              onClick={() => { if (scEnabled) scStop(); }}
              disabled={scLoading || !scEnabled}
              className={`px-2.5 py-0.5 rounded-full text-[11px] font-medium transition-colors ${
                !scEnabled
                  ? "bg-neutral-600 text-neutral-100 shadow-sm"
                  : "text-neutral-500 hover:text-neutral-300"
              }`}
            >
              Solo
            </button>
            <button
              onClick={() => { if (!scEnabled) scStart(); }}
              disabled={scLoading || scEnabled}
              className={`px-2.5 py-0.5 rounded-full text-[11px] font-medium transition-colors ${
                scEnabled
                  ? "bg-sunshine/80 text-neutral-900 shadow-sm"
                  : "text-neutral-500 hover:text-neutral-300"
              }`}
            >
              Shared
            </button>
          </div>
        </div>

        {scEnabled && (
          <div className="space-y-2">
            {/* Participant list */}
            {participants.length > 0 && (
              <div className="space-y-1">
                {participants.map((name) => (
                  <div
                    key={name}
                    className="flex items-center justify-between bg-surface-2 rounded px-2.5 py-1.5"
                  >
                    <span className="text-xs text-neutral-200 font-mono">{name}</span>
                    <button
                      onClick={() => removeParticipant(name)}
                      disabled={scLoading}
                      className="text-[10px] text-neutral-500 hover:text-red-400 transition-colors disabled:opacity-50"
                    >
                      remove
                    </button>
                  </div>
                ))}
              </div>
            )}

            {participants.length === 0 && (
              <p className="text-[11px] text-neutral-600">No participants yet.</p>
            )}

            {/* Add participant input */}
            <div className="flex gap-1.5">
              <input
                type="text"
                value={newParticipant}
                onChange={(e) => setNewParticipant(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newParticipant.trim()) {
                    addParticipant(newParticipant.trim());
                    setNewParticipant("");
                  }
                }}
                placeholder="Participant name"
                maxLength={64}
                className="flex-1 bg-surface-2 text-neutral-200 text-xs rounded px-2 py-1 border border-transparent focus:border-neutral-600 focus:outline-none placeholder-neutral-600"
              />
              <button
                onClick={() => {
                  if (newParticipant.trim()) {
                    addParticipant(newParticipant.trim());
                    setNewParticipant("");
                  }
                }}
                disabled={scLoading || !newParticipant.trim()}
                className="text-xs text-sunshine hover:text-sunshine-bright disabled:text-neutral-600 disabled:cursor-not-allowed transition-colors px-2"
              >
                Add
              </button>
            </div>

            {/* Shared control error */}
            {scError && (
              <div className="text-[11px] text-red-400 bg-red-500/10 rounded px-2 py-1">
                {scError}
              </div>
            )}
          </div>
        )}
      </div>

      <PairAcceptDialog open={pairOpen} onClose={() => setPairOpen(false)} />
    </div>
  );
}

function StreamSettings({
  settings,
  onSave,
}: {
  settings: SunshineSettings;
  onSave: (s: SunshineSettings) => Promise<void>;
}) {
  const [draft, setDraft] = useState(settings);
  const [dirty, setDirty] = useState(false);

  const update = (patch: Partial<SunshineSettings>) => {
    setDraft((d) => ({ ...d, ...patch }));
    setDirty(true);
  };

  const save = async () => {
    await onSave(draft);
    setDirty(false);
  };

  return (
    <Section
      title="Stream"
      action={
        dirty ? (
          <button
            onClick={save}
            className="text-[10px] text-sunshine hover:text-sunshine-bright transition-colors"
          >
            Apply
          </button>
        ) : null
      }
    >
      {/* Codec */}
      <div className="flex items-center justify-between text-xs mb-2">
        <span className="text-neutral-500">Codec</span>
        <div className="flex gap-1">
          {CODEC_OPTIONS.map((c) => (
            <button
              key={c}
              onClick={() => update({ codec: c })}
              className={`px-2 py-0.5 rounded text-[11px] transition-colors ${
                draft.codec === c
                  ? "bg-sunshine/20 text-sunshine border border-sunshine/30"
                  : "bg-surface-3 text-neutral-400 hover:text-neutral-200"
              }`}
            >
              {c.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      {/* FPS */}
      <div className="flex items-center justify-between text-xs mb-2">
        <span className="text-neutral-500">FPS</span>
        <div className="flex gap-1">
          {FPS_OPTIONS.map((f) => (
            <button
              key={f}
              onClick={() => update({ fps: f })}
              className={`px-2 py-0.5 rounded text-[11px] transition-colors ${
                draft.fps === f
                  ? "bg-sunshine/20 text-sunshine border border-sunshine/30"
                  : "bg-surface-3 text-neutral-400 hover:text-neutral-200"
              }`}
            >
              {f}
            </button>
          ))}
        </div>
      </div>

      {/* Bitrate */}
      <div className="flex items-center justify-between text-xs">
        <span className="text-neutral-500">Bitrate</span>
        <select
          value={draft.bitrate ?? 20000}
          onChange={(e) => update({ bitrate: Number(e.target.value) })}
          className="bg-surface-3 text-neutral-200 text-[11px] rounded px-2 py-0.5 border-0 outline-none"
        >
          {BITRATE_PRESETS.map((b) => (
            <option key={b.value} value={b.value}>
              {b.label}
            </option>
          ))}
        </select>
      </div>
    </Section>
  );
}

function Section({
  title,
  action,
  children,
}: {
  title: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="mb-4">
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-xs font-medium text-neutral-400 uppercase tracking-wider">{title}</h3>
        {action}
      </div>
      <div className="space-y-1.5">{children}</div>
    </div>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between text-xs">
      <span className="text-neutral-500">{label}</span>
      <span className="text-neutral-200 font-mono">{value}</span>
    </div>
  );
}
