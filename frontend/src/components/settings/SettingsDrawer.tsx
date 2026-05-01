import { useState } from "react";
import { GeneralTab } from "./GeneralTab";
import { PeersTab } from "./PeersTab";
import { AboutTab } from "./AboutTab";

type Tab = "general" | "peers" | "about";

const TABS: { id: Tab; label: string }[] = [
  { id: "general", label: "General" },
  { id: "peers", label: "Peers" },
  { id: "about", label: "About" },
];

interface Props {
  open: boolean;
  onClose: () => void;
}

export function SettingsDrawer({ open, onClose }: Props) {
  const [activeTab, setActiveTab] = useState<Tab>("general");

  return (
    <>
      {/* Backdrop */}
      <div
        className={`fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-300 ${
          open ? "opacity-100 pointer-events-auto" : "opacity-0 pointer-events-none"
        }`}
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Drawer panel */}
      <div
        className={`fixed top-0 right-0 z-[60] h-screen w-[420px] bg-surface-1 border-l border-surface-3 shadow-2xl flex flex-col transition-transform duration-300 ${
          open ? "translate-x-0" : "translate-x-full"
        }`}
        role="dialog"
        aria-modal="true"
        aria-label="Settings"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-surface-3">
          <h2 className="text-sm font-semibold text-neutral-200">Settings</h2>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-surface-3 text-neutral-500 hover:text-neutral-300 transition-colors text-base leading-none"
            title="Close"
            aria-label="Close settings"
          >
            &#10005;
          </button>
        </div>

        {/* Tab bar */}
        <div className="flex border-b border-surface-3 px-2 pt-2">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-3 py-1.5 text-xs font-medium rounded-t transition-colors ${
                activeTab === tab.id
                  ? "text-neutral-100 bg-surface-3 border border-b-0 border-surface-4"
                  : "text-neutral-500 hover:text-neutral-300 hover:bg-surface-2"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* Tab content */}
        <div className="flex-1 overflow-y-auto">
          {activeTab === "general" && <GeneralTab />}
          {activeTab === "peers" && <PeersTab />}
          {activeTab === "about" && <AboutTab />}
        </div>
      </div>
    </>
  );
}
