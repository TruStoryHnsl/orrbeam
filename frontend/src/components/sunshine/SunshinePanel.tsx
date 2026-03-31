import { useSunshineStore } from "@/stores/sunshine";
import { StatusDot } from "@/components/ui/StatusDot";
import { Button } from "@/components/ui/Button";

export function SunshinePanel() {
  const { status, gpu, monitors, loading, error, start, stop } =
    useSunshineStore();

  const isRunning = status?.status === "running";
  const isInstalled = status?.status !== "not_installed";

  return (
    <div className="flex-1 flex flex-col p-4 bg-surface-1 overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <span className="text-sunshine text-lg">&#9788;</span>
          <h2 className="text-sm font-semibold uppercase tracking-wider text-sunshine">
            Sunshine
          </h2>
          <span className="text-xs text-neutral-500">Host</span>
        </div>
        {status && <StatusDot status={status.status} />}
      </div>

      {/* Status */}
      <div className="space-y-3 mb-4">
        <InfoRow label="Status" value={status?.status ?? "checking..."} />
        {status?.version && (
          <InfoRow label="Version" value={status.version} />
        )}
        {gpu && (
          <>
            <InfoRow label="Encoder" value={gpu.encoder} />
            <InfoRow label="GPU" value={gpu.name} />
            {gpu.driver && <InfoRow label="Driver" value={gpu.driver} />}
          </>
        )}
      </div>

      {/* Monitors */}
      {monitors.length > 0 && (
        <div className="mb-4">
          <h3 className="text-xs font-medium text-neutral-400 uppercase tracking-wider mb-2">
            Monitors
          </h3>
          <div className="space-y-1">
            {monitors.map((m) => (
              <div
                key={m.name}
                className="flex items-center justify-between text-xs bg-surface-2 rounded px-2 py-1.5"
              >
                <span className="text-neutral-200">
                  {m.name}
                  {m.primary && (
                    <span className="text-sunshine ml-1">*</span>
                  )}
                </span>
                <span className="text-neutral-500">
                  {m.resolution}
                  {m.refresh_rate && ` @ ${m.refresh_rate}Hz`}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="text-xs text-red-400 bg-red-500/10 rounded px-2 py-1.5 mb-4">
          {error}
        </div>
      )}

      {/* Actions */}
      <div className="mt-auto pt-4">
        {!isInstalled ? (
          <p className="text-xs text-neutral-500">
            Sunshine is not installed.
          </p>
        ) : isRunning ? (
          <Button
            variant="danger"
            onClick={stop}
            disabled={loading}
            className="w-full"
          >
            {loading ? "Stopping..." : "Stop Hosting"}
          </Button>
        ) : (
          <Button
            variant="sunshine"
            onClick={start}
            disabled={loading}
            className="w-full"
          >
            {loading ? "Starting..." : "Start Hosting"}
          </Button>
        )}
      </div>
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
