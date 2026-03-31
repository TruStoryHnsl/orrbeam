import { useEffect } from "react";
import { usePlatformStore } from "@/stores/platform";
import { useSunshineStore } from "@/stores/sunshine";
import { useMoonlightStore } from "@/stores/moonlight";
import { Shell } from "@/components/layout/Shell";
import { SunshinePanel } from "@/components/sunshine/SunshinePanel";
import { MoonlightPanel } from "@/components/moonlight/MoonlightPanel";
import { MeshBar } from "@/components/mesh/MeshBar";

const POLL_INTERVAL = 5000;

function App() {
  const fetchPlatformInfo = usePlatformStore((s) => s.fetchInfo);
  const fetchIdentity = usePlatformStore((s) => s.fetchIdentity);
  const fetchSunshineStatus = useSunshineStore((s) => s.fetchStatus);
  const fetchGpu = useSunshineStore((s) => s.fetchGpu);
  const fetchMonitors = useSunshineStore((s) => s.fetchMonitors);
  const fetchMoonlightStatus = useMoonlightStore((s) => s.fetchStatus);
  const fetchNodes = useMoonlightStore((s) => s.fetchNodes);

  // Initial fetch
  useEffect(() => {
    fetchPlatformInfo();
    fetchIdentity();
    fetchSunshineStatus();
    fetchGpu();
    fetchMonitors();
    fetchMoonlightStatus();
    fetchNodes();
  }, []);

  // Poll for status updates
  useEffect(() => {
    const interval = setInterval(() => {
      fetchSunshineStatus();
      fetchMoonlightStatus();
      fetchNodes();
    }, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, []);

  return (
    <Shell>
      <div className="flex flex-1 gap-px overflow-hidden">
        <SunshinePanel />
        <MoonlightPanel />
      </div>
      <MeshBar />
    </Shell>
  );
}

export default App;
