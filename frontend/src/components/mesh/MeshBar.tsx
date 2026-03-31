import { useMoonlightStore } from "@/stores/moonlight";
import { usePlatformStore } from "@/stores/platform";
import { useSunshineStore } from "@/stores/sunshine";
import { StatusDot } from "@/components/ui/StatusDot";

export function MeshBar() {
  const nodes = useMoonlightStore((s) => s.nodes);
  const connectedTo = useMoonlightStore((s) => s.connectedTo);
  const info = usePlatformStore((s) => s.info);
  const sunshineStatus = useSunshineStore((s) => s.status);

  const onlineCount = nodes.filter((n) => n.state !== "offline").length;
  const isHosting = sunshineStatus?.status === "running";

  return (
    <footer className="flex items-center gap-4 px-4 py-1.5 bg-surface-2 border-t border-surface-3 text-xs text-neutral-500">
      {/* Local node */}
      <div className="flex items-center gap-1.5">
        <StatusDot status={isHosting ? "hosting" : "online"} />
        <span className="text-neutral-300">{info?.hostname ?? "local"}</span>
      </div>

      {/* Connections */}
      {connectedTo && (
        <div className="flex items-center gap-1">
          <span className="text-neutral-600">&rarr;</span>
          <span className="text-moonlight-bright">{connectedTo}</span>
        </div>
      )}

      {/* Mesh summary */}
      <div className="ml-auto flex items-center gap-3">
        {nodes
          .filter((n) => n.state !== "offline")
          .map((node) => (
            <div key={node.name} className="flex items-center gap-1">
              <StatusDot status={node.state as "online" | "hosting"} />
              <span>{node.name}</span>
            </div>
          ))}
        <span className="text-neutral-600">|</span>
        <span>
          {onlineCount + 1} node{onlineCount !== 0 ? "s" : ""} in mesh
        </span>
      </div>
    </footer>
  );
}
