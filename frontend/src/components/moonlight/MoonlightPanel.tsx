import { useState } from "react";
import { useMoonlightStore } from "@/stores/moonlight";
import type { MoonlightNode } from "@/stores/moonlight";
import { StatusDot } from "@/components/ui/StatusDot";
import { Button } from "@/components/ui/Button";
import { PairInitiateDialog } from "./PairDialog";

const RESOLUTION_OPTIONS = [
  { label: "720p", value: "1280x720" },
  { label: "1080p", value: "1920x1080" },
  { label: "1440p", value: "2560x1440" },
  { label: "4K", value: "3840x2160" },
];

export function MoonlightPanel() {
  const { status, nodes, connectedTo, loading, error, connect, disconnect } = useMoonlightStore();

  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [windowed, setWindowed] = useState(false);
  const [resolution, setResolution] = useState("1920x1080");
  const [app, setApp] = useState("Desktop");
  const [pairTarget, setPairTarget] = useState<{ address: string; name: string } | null>(null);

  const isInstalled = status?.status !== "not_installed";
  const onlineNodes = nodes.filter((n) => n.state !== "offline");
  const offlineNodes = nodes.filter((n) => n.state === "offline");

  const handleConnect = () => {
    if (!selectedNode) return;
    connect(selectedNode, app, windowed, resolution);
  };

  return (
    <div className="flex-1 flex flex-col p-4 bg-surface-1 overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <span className="text-moonlight text-lg">&#9789;</span>
          <h2 className="text-sm font-semibold uppercase tracking-wider text-moonlight">
            Moonlight
          </h2>
          <span className="text-xs text-neutral-500">Client</span>
        </div>
        {status && <StatusDot status={status.status} />}
      </div>

      {/* Status */}
      <Section title="Service">
        <InfoRow
          label="Status"
          value={connectedTo ? `Streaming from ${connectedTo}` : (status?.status ?? "checking...")}
        />
        {status?.version && <InfoRow label="Version" value={status.version} />}
      </Section>

      {/* Active connection — disconnect button */}
      {connectedTo && (
        <div className="mb-4">
          <Button variant="danger" onClick={disconnect} disabled={loading} className="w-full">
            {loading ? "Disconnecting..." : "Disconnect"}
          </Button>
        </div>
      )}

      {/* Stream settings (shown when not connected) */}
      {!connectedTo && (
        <Section title="Stream Settings">
          {/* App */}
          <div className="flex items-center justify-between text-xs mb-2">
            <span className="text-neutral-500">App</span>
            <input
              value={app}
              onChange={(e) => setApp(e.target.value)}
              className="bg-surface-3 text-neutral-200 text-[11px] rounded px-2 py-1 border-0 outline-none w-28 text-right"
              placeholder="Desktop"
            />
          </div>

          {/* Resolution */}
          <div className="flex items-center justify-between text-xs mb-2">
            <span className="text-neutral-500">Resolution</span>
            <div className="flex gap-1">
              {RESOLUTION_OPTIONS.map((r) => (
                <button
                  key={r.value}
                  onClick={() => setResolution(r.value)}
                  className={`px-2 py-0.5 rounded text-[11px] transition-colors ${
                    resolution === r.value
                      ? "bg-moonlight/20 text-moonlight border border-moonlight/30"
                      : "bg-surface-3 text-neutral-400 hover:text-neutral-200"
                  }`}
                >
                  {r.label}
                </button>
              ))}
            </div>
          </div>

          {/* Display mode */}
          <div className="flex items-center justify-between text-xs">
            <span className="text-neutral-500">Mode</span>
            <div className="flex gap-1">
              {(["Fullscreen", "Windowed"] as const).map((mode) => {
                const isW = mode === "Windowed";
                const active = windowed === isW;
                return (
                  <button
                    key={mode}
                    onClick={() => setWindowed(isW)}
                    className={`px-2 py-0.5 rounded text-[11px] transition-colors ${
                      active
                        ? "bg-moonlight/20 text-moonlight border border-moonlight/30"
                        : "bg-surface-3 text-neutral-400 hover:text-neutral-200"
                    }`}
                  >
                    {mode}
                  </button>
                );
              })}
            </div>
          </div>
        </Section>
      )}

      {/* Available Nodes */}
      <Section title={`Nodes (${onlineNodes.length})`}>
        {onlineNodes.length === 0 && !loading && (
          <p className="text-xs text-neutral-500 italic">No nodes discovered yet...</p>
        )}

        <div className="space-y-1.5">
          {onlineNodes.map((node) => (
            <NodeCard
              key={node.name}
              node={node}
              isSelected={selectedNode === node.address}
              isConnected={connectedTo === node.address}
              disabled={loading || !isInstalled || !!connectedTo}
              onSelect={() => setSelectedNode(node.address)}
            />
          ))}
        </div>

        {offlineNodes.length > 0 && (
          <div className="mt-3">
            <span className="text-[10px] text-neutral-600 uppercase tracking-wider">
              Offline ({offlineNodes.length})
            </span>
            <div className="space-y-1.5 mt-1 opacity-40">
              {offlineNodes.map((node) => (
                <NodeCard
                  key={node.name}
                  node={node}
                  isSelected={false}
                  isConnected={false}
                  disabled
                  onSelect={() => {}}
                />
              ))}
            </div>
          </div>
        )}
      </Section>

      {/* Error */}
      {error && (
        <div className="text-xs text-red-400 bg-red-500/10 rounded px-2 py-1.5 mb-3">{error}</div>
      )}

      {/* Connect button */}
      {!connectedTo && (
        <div className="mt-auto pt-4 space-y-2">
          {!isInstalled ? (
            <p className="text-xs text-neutral-500">Moonlight is not installed.</p>
          ) : (
            <>
              <Button
                variant="moonlight"
                onClick={handleConnect}
                disabled={loading || !selectedNode}
                className="w-full"
              >
                {loading
                  ? "Connecting..."
                  : selectedNode
                    ? `Connect to ${nodes.find((n) => n.address === selectedNode)?.name ?? selectedNode}`
                    : "Select a node"}
              </Button>
              {selectedNode && (
                <Button
                  variant="ghost"
                  onClick={() => {
                    const node = nodes.find((n) => n.address === selectedNode);
                    if (node) setPairTarget({ address: node.address, name: node.name });
                  }}
                  className="w-full"
                  size="sm"
                >
                  Pair with {nodes.find((n) => n.address === selectedNode)?.name ?? "node"}
                </Button>
              )}
            </>
          )}
        </div>
      )}

      {/* Pair dialog */}
      {pairTarget && (
        <PairInitiateDialog
          open={!!pairTarget}
          onClose={() => setPairTarget(null)}
          address={pairTarget.address}
          nodeName={pairTarget.name}
        />
      )}
    </div>
  );
}

function NodeCard({
  node,
  isSelected,
  isConnected,
  disabled,
  onSelect,
}: {
  node: MoonlightNode;
  isSelected: boolean;
  isConnected: boolean;
  disabled: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      onClick={onSelect}
      disabled={disabled}
      className={`w-full flex items-center justify-between rounded-lg px-3 py-2 text-left transition-colors ${
        isSelected
          ? "bg-moonlight/10 border border-moonlight/30"
          : isConnected
            ? "bg-moonlight/15 border border-moonlight/40"
            : "bg-surface-2 border border-transparent hover:bg-surface-3"
      } disabled:cursor-not-allowed`}
    >
      <div className="flex items-center gap-2">
        <StatusDot status={node.state as "online" | "offline" | "hosting" | "connected"} />
        <div>
          <div className="text-sm text-neutral-200 font-medium">{node.name}</div>
          <div className="flex items-center gap-2 text-[10px] text-neutral-500">
            {node.os && <span>{node.os}</span>}
            {node.encoder && <span>{node.encoder}</span>}
            <span>{node.source}</span>
          </div>
        </div>
      </div>

      {isConnected && <span className="text-xs text-moonlight-bright font-medium">Streaming</span>}
      {isSelected && !isConnected && node.sunshine_available && (
        <span className="text-[10px] text-moonlight">selected</span>
      )}
      {!node.sunshine_available && <span className="text-[10px] text-neutral-600">no host</span>}
    </button>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mb-4">
      <h3 className="text-xs font-medium text-neutral-400 uppercase tracking-wider mb-2">
        {title}
      </h3>
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
