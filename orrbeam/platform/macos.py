"""macOS platform implementation."""

import os
import shutil
import subprocess
import plistlib
from pathlib import Path

import json as _json

from .base import Platform, ServiceStatus, Monitor

LAUNCHD_PLIST_DIR = Path.home() / "Library" / "LaunchAgents"
LAUNCHD_LABEL = "com.orrbeam.daemon"
LAUNCHD_PLIST = LAUNCHD_PLIST_DIR / f"{LAUNCHD_LABEL}.plist"


def _run(cmd: list[str], check: bool = False) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(cmd, capture_output=True, text=True, check=check, timeout=30)
    except FileNotFoundError:
        return subprocess.CompletedProcess(cmd, returncode=127, stdout="", stderr="command not found")
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(cmd, returncode=124, stdout="", stderr="timeout")


def _which(name: str) -> str:
    return shutil.which(name) or ""


class MacOSPlatform(Platform):

    def detect_sunshine(self) -> ServiceStatus:
        # Sunshine on macOS: .app bundle, homebrew, or binary in PATH
        path = _which("sunshine")
        if not path:
            brew_path = "/opt/homebrew/bin/sunshine"
            if os.path.isfile(brew_path):
                path = brew_path
        # Check for .app bundle
        app_paths = [
            "/Applications/Sunshine.app",
            Path.home() / "Applications" / "Sunshine.app",
        ]
        app_path = ""
        for p in app_paths:
            if Path(p).exists():
                app_path = str(p)
                # Find the binary inside the bundle
                bundle_bin = Path(p) / "Contents" / "MacOS" / "sunshine"
                if bundle_bin.exists():
                    path = str(bundle_bin)
                elif not path:
                    path = str(p)
                break
        if not path and not app_path:
            return ServiceStatus()

        version = ""
        # Don't run --version on macOS (may require GUI context)

        pid = None
        running = False
        try:
            r = _run(["pgrep", "-f", "Sunshine"])
            if r.returncode == 0:
                pid = int(r.stdout.strip().split("\n")[0])
                running = True
        except Exception:
            pass

        return ServiceStatus(installed=True, running=running, pid=pid,
                             version=version, path=path or app_path)

    def detect_moonlight(self) -> ServiceStatus:
        # Moonlight on macOS is a .app bundle
        app_paths = [
            "/Applications/Moonlight.app",
            Path.home() / "Applications" / "Moonlight.app",
        ]
        path = ""
        for p in app_paths:
            if Path(p).exists():
                path = str(p)
                break

        # Also check for CLI
        cli_path = _which("moonlight") or _which("moonlight-qt")
        if cli_path:
            path = path or cli_path

        if not path:
            # Check homebrew cask
            r = _run(["brew", "list", "--cask"])
            if r.returncode == 0 and "moonlight" in r.stdout.lower():
                path = "/Applications/Moonlight.app"

        if not path:
            return ServiceStatus()

        return ServiceStatus(installed=True, path=path)

    def install_sunshine(self) -> bool:
        # Sunshine is not in Homebrew — must be installed from GitHub releases
        print("Sunshine must be installed manually on macOS:")
        print("  Download from: https://github.com/LizardByte/Sunshine/releases")
        print("  Get the macOS ARM64 DMG and drag Sunshine.app to /Applications")
        # Check if already installed
        if Path("/Applications/Sunshine.app").exists():
            print("  (Sunshine.app is already installed)")
            return True
        return False

    def install_moonlight(self) -> bool:
        if _which("brew"):
            r = _run(["brew", "install", "--cask", "moonlight"])
            return r.returncode == 0
        print("Install Homebrew first, then: brew install --cask moonlight")
        return False

    def start_sunshine(self) -> bool:
        # Try .app bundle first
        for app_path in ["/Applications/Sunshine.app",
                         str(Path.home() / "Applications" / "Sunshine.app")]:
            if Path(app_path).exists():
                _run(["open", "-a", app_path])
                return True
        # Try direct binary
        path = _which("sunshine") or "/opt/homebrew/bin/sunshine"
        if os.path.isfile(path):
            subprocess.Popen([path], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return True
        return False

    def stop_sunshine(self) -> bool:
        _run(["pkill", "-f", "Sunshine"])
        return True

    def moonlight_cli_path(self) -> str | None:
        # CLI in PATH first
        cli = _which("moonlight") or _which("moonlight-qt")
        if cli:
            return cli
        # Extract CLI from .app bundle (supports stream, pair, list, quit)
        for app_dir in ["/Applications/Moonlight.app",
                        str(Path.home() / "Applications" / "Moonlight.app")]:
            binary = Path(app_dir) / "Contents" / "MacOS" / "Moonlight"
            if binary.exists():
                return str(binary)
        return None

    def start_moonlight(self, address: str, app: str = "Desktop") -> bool:
        cli = self.moonlight_cli_path()
        if not cli:
            return False
        # Don't redirect stdout/stderr — Qt needs them for proper GUI initialization
        subprocess.Popen([cli, "stream", address, app], start_new_session=True)
        return True

    def pair_moonlight(self, address: str, pin: str) -> bool:
        cli = self.moonlight_cli_path()
        if not cli:
            return False
        # Launch pair as detached process — Qt needs to stay alive for the
        # pairing handshake while the daemon submits the PIN to remote Sunshine
        subprocess.Popen([cli, "pair", address, "--pin", pin],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                         start_new_session=True)
        return True

    def stop_moonlight(self) -> bool:
        _run(["pkill", "-f", "Moonlight"])
        return True

    def install_service(self) -> bool:
        exec_path = _which("orrbeamd")
        if not exec_path:
            import sys
            exec_path = sys.executable
            program_args = [exec_path, "-m", "orrbeam.daemon"]
        else:
            program_args = [exec_path]

        plist = {
            "Label": LAUNCHD_LABEL,
            "ProgramArguments": program_args,
            "RunAtLoad": True,
            "KeepAlive": True,
            "StandardOutPath": str(Path.home() / "Library" / "Logs" / "orrbeam.log"),
            "StandardErrorPath": str(Path.home() / "Library" / "Logs" / "orrbeam.err"),
            "EnvironmentVariables": {
                "ORRBEAM_DAEMON": "1",
            },
        }

        LAUNCHD_PLIST_DIR.mkdir(parents=True, exist_ok=True)
        with open(LAUNCHD_PLIST, "wb") as f:
            plistlib.dump(plist, f)

        _run(["launchctl", "load", str(LAUNCHD_PLIST)])
        return True

    def uninstall_service(self) -> bool:
        _run(["launchctl", "unload", str(LAUNCHD_PLIST)])
        if LAUNCHD_PLIST.exists():
            LAUNCHD_PLIST.unlink()
        return True

    def configure_firewall(self) -> bool:
        # macOS uses pf, but Application Firewall is the common one
        # Sunshine handles its own port setup; we just need to add to allowlist
        sun = _which("sunshine") or "/opt/homebrew/bin/sunshine"
        if os.path.isfile(sun):
            _run(["sudo", "/usr/libexec/ApplicationFirewall/socketfilterfw",
                   "--add", sun])
            _run(["sudo", "/usr/libexec/ApplicationFirewall/socketfilterfw",
                   "--unblockapp", sun])
        print("If prompted, allow Sunshine through the macOS firewall in System Settings > Privacy & Security.")
        return True

    def display_server(self) -> str:
        return "quartz"

    def gpu_info(self) -> dict:
        info: dict = {"gpus": [], "hw_encode": False, "encoder": "software"}
        r = _run(["system_profiler", "SPDisplaysDataType", "-json"])
        if r.returncode == 0:
            try:
                data = _json.loads(r.stdout)
                displays = data.get("SPDisplaysDataType", [])
                for gpu in displays:
                    name = gpu.get("sppci_model", "Unknown GPU")
                    info["gpus"].append({"name": name, "vendor": "apple"})
                if displays:
                    info["hw_encode"] = True
                    info["encoder"] = "videotoolbox"
            except (_json.JSONDecodeError, KeyError):
                pass
        return info

    def list_monitors(self) -> list[Monitor]:
        r = _run(["system_profiler", "SPDisplaysDataType", "-json"])
        if r.returncode != 0:
            return []
        monitors = []
        try:
            data = _json.loads(r.stdout)
            for gpu in data.get("SPDisplaysDataType", []):
                for disp in gpu.get("spdisplays_ndrvs", []):
                    name = disp.get("_name", "Display")
                    # Parse resolution like "3456 x 2234" or pixel string
                    res = disp.get("_spdisplays_resolution", "")
                    w, h = 0, 0
                    if " x " in res:
                        parts = res.split(" x ")
                        try:
                            w = int(parts[0].strip().split()[0])
                            h = int(parts[1].strip().split()[0])
                        except (ValueError, IndexError):
                            pass
                    is_main = disp.get("spdisplays_main") == "spdisplays_yes"
                    monitors.append(Monitor(
                        name=name, description=name,
                        width=w, height=h, refresh_rate=0.0,
                        rotation="normal", active=True, primary=is_main,
                    ))
        except (_json.JSONDecodeError, KeyError):
            pass
        return monitors

    def set_rotation(self, output: str, rotation: str) -> bool:
        # macOS display rotation requires displayplacer or system APIs
        return False
