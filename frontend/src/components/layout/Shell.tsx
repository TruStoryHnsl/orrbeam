import { type ReactNode, useEffect, useState } from "react";
import { usePlatformStore } from "@/stores/platform";

const TRAY_NOTICE_KEY = "orrbeam-tray-notice-shown";

export function Shell({ children }: { children: ReactNode }) {
  const identity = usePlatformStore((s) => s.identity);
  const info = usePlatformStore((s) => s.info);
  const [showTrayNotice, setShowTrayNotice] = useState(false);

  useEffect(() => {
    // Listen for the window close event to show a one-time tray notice
    let unlisten: (() => void) | undefined;

    (async () => {
      // Only register in Tauri, not in browser mock mode
      if (!("__TAURI_INTERNALS__" in window) && !("isTauri" in window)) return;

      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      unlisten = await getCurrentWindow().onCloseRequested(() => {
        if (localStorage.getItem(TRAY_NOTICE_KEY)) return;
        localStorage.setItem(TRAY_NOTICE_KEY, "1");
        setShowTrayNotice(true);
        setTimeout(() => setShowTrayNotice(false), 3000);
      });
    })();

    return () => {
      unlisten?.();
    };
  }, []);

  return (
    <div className="flex flex-col h-screen bg-surface-0">
      {/* Tray notification banner */}
      {showTrayNotice && (
        <div className="px-4 py-2 bg-surface-2 border-b border-surface-3 text-sm text-neutral-300 text-center animate-pulse">
          Orrbeam is still running in the system tray.
        </div>
      )}

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
