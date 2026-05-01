import { useEffect, useState } from "react";
import { invoke } from "@/api/tauri";
import { usePlatformStore } from "@/stores/platform";
import type { TlsFingerprint } from "@/types/peers";

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard not available in some contexts
    }
  };

  return (
    <button
      onClick={handleCopy}
      className="ml-2 px-1.5 py-0.5 rounded text-xs text-neutral-500 hover:text-neutral-300 hover:bg-surface-3 transition-colors"
      title="Copy to clipboard"
    >
      {copied ? "&#10003;" : "copy"}
    </button>
  );
}

function InfoRow({
  label,
  value,
  mono = false,
  copyable = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
  copyable?: boolean;
}) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-xs text-neutral-500 uppercase tracking-wide">{label}</span>
      <div className="flex items-center">
        <span className={`text-sm text-neutral-200 break-all ${mono ? "font-mono" : ""}`}>
          {value}
        </span>
        {copyable && <CopyButton text={value} />}
      </div>
    </div>
  );
}

/** Format a hex string into space-separated groups of 4 chars. */
function formatHex(hex: string, groupSize = 4): string {
  return hex
    .replace(new RegExp(`.{1,${groupSize}}`, "g"), (m) => m)
    .split("")
    .reduce((acc, ch, i) => {
      if (i > 0 && i % groupSize === 0) acc += " ";
      return acc + ch;
    }, "");
}

export function AboutTab() {
  const info = usePlatformStore((s) => s.info);
  const identity = usePlatformStore((s) => s.identity);
  const [tls, setTls] = useState<TlsFingerprint | null>(null);
  const [tlsError, setTlsError] = useState<string | null>(null);

  useEffect(() => {
    invoke("get_tls_fingerprint")
      .then((data) => setTls(data as TlsFingerprint))
      .catch((e) => setTlsError(String(e)));
  }, []);

  const nodeName = info?.hostname ?? "—";
  const ed25519Fp = identity?.fingerprint ?? tls?.ed25519_fingerprint ?? "—";
  const certSha256Raw = tls?.cert_sha256 ?? null;
  const certSha256Fmt = certSha256Raw ? formatHex(certSha256Raw, 4) : "—";
  const controlPort = tls?.control_port ?? "—";

  return (
    <div className="space-y-5 p-4">
      <InfoRow label="Node name" value={nodeName} />
      <InfoRow label="Ed25519 fingerprint" value={ed25519Fp} mono copyable={ed25519Fp !== "—"} />
      {tlsError ? (
        <div className="flex flex-col gap-0.5">
          <span className="text-xs text-neutral-500 uppercase tracking-wide">TLS cert SHA-256</span>
          <span className="text-xs text-red-400">{tlsError}</span>
        </div>
      ) : (
        <div className="flex flex-col gap-0.5">
          <span className="text-xs text-neutral-500 uppercase tracking-wide">TLS cert SHA-256</span>
          <div className="flex items-start">
            <span className="text-sm text-neutral-200 font-mono break-all leading-relaxed">
              {certSha256Fmt}
            </span>
            {certSha256Raw && <CopyButton text={certSha256Raw} />}
          </div>
        </div>
      )}
      <InfoRow label="Control port" value={String(controlPort)} />
      <InfoRow label="Protocol version" value="orrbeam/1" />
    </div>
  );
}
