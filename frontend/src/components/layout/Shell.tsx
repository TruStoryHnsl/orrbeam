import type { ReactNode } from "react";
import { usePlatformStore } from "@/stores/platform";

export function Shell({ children }: { children: ReactNode }) {
  const identity = usePlatformStore((s) => s.identity);
  const info = usePlatformStore((s) => s.info);

  return (
    <div className="flex flex-col h-screen bg-surface-0">
      {/* Title bar */}
      <header className="flex items-center justify-between px-4 py-2 bg-surface-1 border-b border-surface-3">
        <div className="flex items-center gap-3">
          <h1 className="text-lg font-semibold tracking-tight">
            <span className="text-sunshine">Orr</span>
            <span className="text-white">beam</span>
          </h1>
          {info && (
            <span className="text-xs text-neutral-500">
              {info.hostname}
            </span>
          )}
        </div>
        <div className="flex items-center gap-3">
          {identity && (
            <span className="text-xs text-neutral-500 font-mono">
              {identity.fingerprint}
            </span>
          )}
        </div>
      </header>

      {/* Main content */}
      <main className="flex flex-col flex-1 overflow-hidden">
        {children}
      </main>
    </div>
  );
}
