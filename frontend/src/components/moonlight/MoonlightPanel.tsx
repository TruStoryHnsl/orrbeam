import { useMoonlightStore } from "@/stores/moonlight";
import { StatusDot } from "@/components/ui/StatusDot";
import { Button } from "@/components/ui/Button";

export function MoonlightPanel() {
  const { status, nodes, connectedTo, loading, error, connect, disconnect } =
    useMoonlightStore();

  const isInstalled = status?.status !== "not_installed";
  const onlineNodes = nodes.filter((n) => n.state !== "offline");
  const offlineNodes = nodes.filter((n) => n.state === "offline");

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
      <div className="space-y-3 mb-4">
        <InfoRow
          label="Status"
          value={
            connectedTo
              ? `Connected to ${connectedTo}`
              : status?.status ?? "checking..."
          }
        />
        {status?.version && (
          <InfoRow label="Version" value={status.version} />
        )}
      </div>

      {/* Connected — show disconnect */}
      {connectedTo && (
        <div className="mb-4">
          <Button
            variant="danger"
            onClick={disconnect}
            disabled={loading}
            className="w-full"
          >
            {loading ? "Disconnecting..." : `Disconnect from ${connectedTo}`}
          </Button>
        </div>
      )}

      {/* Available Nodes */}
      <div className="flex-1">
        <h3 className="text-xs font-medium text-neutral-400 uppercase tracking-wider mb-2">
          Available Nodes ({onlineNodes.length})
        </h3>

        {onlineNodes.length === 0 && !loading && (
          <p className="text-xs text-neutral-500 italic">
            No nodes discovered yet...
          </p>
        )}

        <div className="space-y-1.5">
          {onlineNodes.map((node) => (
            <NodeCard
              key={node.name}
              node={node}
              onConnect={() => connect(node.address)}
              isConnected={connectedTo === node.address}
              disabled={loading || !isInstalled}
            />
          ))}
        </div>

        {offlineNodes.length > 0 && (
          <>
            <h3 className="text-xs font-medium text-neutral-500 uppercase tracking-wider mt-4 mb-2">
              Offline ({offlineNodes.length})
            </h3>
            <div className="space-y-1.5 opacity-50">
              {offlineNodes.map((node) => (
                <NodeCard
                  key={node.name}
                  node={node}
                  onConnect={() => {}}
                  isConnected={false}
                  disabled
                />
              ))}
            </div>
          </>
        )}
      </div>

      {/* Error */}
      {error && (
        <div className="text-xs text-red-400 bg-red-500/10 rounded px-2 py-1.5 mt-4">
          {error}
        </div>
      )}

      {!isInstalled && (
        <p className="text-xs text-neutral-500 mt-auto pt-4">
          Moonlight is not installed.
        </p>
      )}
    </div>
  );
}

function NodeCard({
  node,
  onConnect,
  isConnected,
  disabled,
}: {
  node: {
    name: string;
    address: string;
    state: string;
    os: string | null;
    encoder: string | null;
    source: string;
    sunshine_available: boolean;
  };
  onConnect: () => void;
  isConnected: boolean;
  disabled: boolean;
}) {
  return (
    <div className="flex items-center justify-between bg-surface-2 rounded-lg px-3 py-2">
      <div className="flex items-center gap-2">
        <StatusDot
          status={node.state as "online" | "offline" | "hosting" | "connected"}
        />
        <div>
          <div className="text-sm text-neutral-200 font-medium">
            {node.name}
          </div>
          <div className="flex items-center gap-2 text-[10px] text-neutral-500">
            {node.os && <span>{node.os}</span>}
            {node.encoder && <span>{node.encoder}</span>}
            <span>{node.source}</span>
          </div>
        </div>
      </div>

      {!isConnected && node.sunshine_available && (
        <Button
          variant="moonlight"
          size="sm"
          onClick={onConnect}
          disabled={disabled}
        >
          Connect
        </Button>
      )}
      {isConnected && (
        <span className="text-xs text-moonlight-bright font-medium">
          Streaming
        </span>
      )}
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
