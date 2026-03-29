"""Linux platform implementation."""

import os
import shutil
import subprocess
from pathlib import Path

from .base import Platform, ServiceStatus

SYSTEMD_UNIT = """[Unit]
Description=Orrbeam mesh daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={exec_path}
Restart=on-failure
RestartSec=5
Environment=ORRBEAM_DAEMON=1

[Install]
WantedBy=multi-user.target
"""

SYSTEMD_USER_DIR = Path.home() / ".config" / "systemd" / "user"
SYSTEMD_UNIT_NAME = "orrbeamd.service"


def _run(cmd: list[str], check: bool = False, capture: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=capture, text=True, check=check, timeout=30)


def _which(name: str) -> str:
    return shutil.which(name) or ""


def _is_active(service: str, user: bool = True) -> bool:
    cmd = ["systemctl"]
    if user:
        cmd.append("--user")
    cmd.extend(["is-active", "--quiet", service])
    return _run(cmd).returncode == 0


def _is_enabled(service: str, user: bool = True) -> bool:
    cmd = ["systemctl"]
    if user:
        cmd.append("--user")
    cmd.extend(["is-enabled", "--quiet", service])
    return _run(cmd).returncode == 0


def _systemctl(action: str, service: str, user: bool = True) -> bool:
    cmd = ["systemctl"]
    if user:
        cmd.append("--user")
    cmd.extend([action, service])
    return _run(cmd).returncode == 0


class LinuxPlatform(Platform):

    def detect_sunshine(self) -> ServiceStatus:
        path = _which("sunshine")
        if not path:
            for p in ["/usr/bin/sunshine", "/usr/local/bin/sunshine"]:
                if os.path.isfile(p):
                    path = p
                    break
        if not path:
            return ServiceStatus()

        version = ""
        try:
            r = _run([path, "--version"])
            version = r.stdout.strip() or r.stderr.strip()
        except Exception:
            pass

        # Check both system and user service
        running = _is_active("sunshine", user=True) or _is_active("sunshine", user=False)
        enabled = _is_enabled("sunshine", user=True) or _is_enabled("sunshine", user=False)

        pid = None
        try:
            r = _run(["pgrep", "-x", "sunshine"])
            if r.returncode == 0:
                pid = int(r.stdout.strip().split("\n")[0])
        except Exception:
            pass

        return ServiceStatus(installed=True, running=running, enabled=enabled,
                             pid=pid, version=version, path=path)

    def detect_moonlight(self) -> ServiceStatus:
        # Moonlight-qt on Linux
        path = _which("moonlight") or _which("moonlight-qt")
        if not path:
            for p in ["/usr/bin/moonlight", "/usr/bin/moonlight-qt",
                      "/usr/local/bin/moonlight", "/var/lib/flatpak/exports/bin/com.moonlight_stream.Moonlight"]:
                if os.path.isfile(p):
                    path = p
                    break
        if not path:
            # Check flatpak
            r = _run(["flatpak", "list", "--app", "--columns=application"])
            if r.returncode == 0 and "com.moonlight_stream.Moonlight" in r.stdout:
                path = "flatpak:com.moonlight_stream.Moonlight"

        if not path:
            return ServiceStatus()

        version = ""
        if not path.startswith("flatpak:"):
            try:
                r = _run([path, "--version"])
                version = r.stdout.strip()
            except Exception:
                pass

        return ServiceStatus(installed=True, path=path, version=version)

    def install_sunshine(self) -> bool:
        # Try pacman (Arch/CachyOS), then apt, then suggest manual
        if _which("pacman"):
            r = _run(["sudo", "pacman", "-S", "--noconfirm", "sunshine"])
            return r.returncode == 0
        elif _which("apt"):
            # Sunshine needs PPA or manual .deb on Ubuntu
            print("Sunshine is not in default Ubuntu repos.")
            print("Install from: https://github.com/LizardByte/Sunshine/releases")
            return False
        print("Could not auto-install Sunshine. Install manually from:")
        print("  https://github.com/LizardByte/Sunshine/releases")
        return False

    def install_moonlight(self) -> bool:
        if _which("pacman"):
            r = _run(["sudo", "pacman", "-S", "--noconfirm", "moonlight-qt"])
            return r.returncode == 0
        elif _which("apt"):
            r = _run(["sudo", "apt", "install", "-y", "moonlight-qt"])
            return r.returncode == 0
        elif _which("flatpak"):
            r = _run(["flatpak", "install", "-y", "flathub", "com.moonlight_stream.Moonlight"])
            return r.returncode == 0
        print("Could not auto-install Moonlight. Install manually.")
        return False

    def start_sunshine(self) -> bool:
        # Prefer user service if it exists
        if _is_enabled("sunshine", user=True):
            return _systemctl("start", "sunshine", user=True)
        if _is_enabled("sunshine", user=False):
            return _systemctl("start", "sunshine", user=False)
        # Direct launch
        sun = _which("sunshine")
        if sun:
            subprocess.Popen([sun], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return True
        return False

    def stop_sunshine(self) -> bool:
        if _is_active("sunshine", user=True):
            return _systemctl("stop", "sunshine", user=True)
        if _is_active("sunshine", user=False):
            return _systemctl("stop", "sunshine", user=False)
        _run(["pkill", "-x", "sunshine"])
        return True

    def start_moonlight(self, address: str, app: str = "Desktop") -> bool:
        path = _which("moonlight") or _which("moonlight-qt")
        if not path:
            # Try flatpak
            r = _run(["flatpak", "run", "com.moonlight_stream.Moonlight", "stream", address, app])
            return r.returncode == 0
        subprocess.Popen([path, "stream", address, app],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return True

    def stop_moonlight(self) -> bool:
        _run(["pkill", "-f", "moonlight"])
        return True

    def install_service(self) -> bool:
        exec_path = _which("orrbeamd")
        if not exec_path:
            # Fall back to python -m
            import sys
            exec_path = f"{sys.executable} -m orrbeam.daemon"

        SYSTEMD_USER_DIR.mkdir(parents=True, exist_ok=True)
        unit_path = SYSTEMD_USER_DIR / SYSTEMD_UNIT_NAME
        unit_path.write_text(SYSTEMD_UNIT.format(exec_path=exec_path))

        _systemctl("daemon-reload", "", user=True)
        _systemctl("enable", SYSTEMD_UNIT_NAME, user=True)
        _systemctl("start", SYSTEMD_UNIT_NAME, user=True)
        return True

    def uninstall_service(self) -> bool:
        _systemctl("stop", SYSTEMD_UNIT_NAME, user=True)
        _systemctl("disable", SYSTEMD_UNIT_NAME, user=True)
        unit_path = SYSTEMD_USER_DIR / SYSTEMD_UNIT_NAME
        if unit_path.exists():
            unit_path.unlink()
        _systemctl("daemon-reload", "", user=True)
        return True

    def configure_firewall(self) -> bool:
        # Try firewall-cmd (firewalld) then ufw
        if _which("firewall-cmd"):
            for port in [47984, 47985, 47986, 47987, 47988, 47989, 47990, 48010, 47782]:
                _run(["sudo", "firewall-cmd", "--permanent", f"--add-port={port}/tcp"])
                _run(["sudo", "firewall-cmd", "--permanent", f"--add-port={port}/udp"])
            _run(["sudo", "firewall-cmd", "--reload"])
            return True
        elif _which("ufw"):
            for port in [47984, 47985, 47986, 47987, 47988, 47989, 47990, 48010, 47782]:
                _run(["sudo", "ufw", "allow", str(port)])
            return True
        print("No supported firewall manager found. Manually open ports 47984-47990, 48010, 47782.")
        return False

    def display_server(self) -> str:
        session = os.environ.get("XDG_SESSION_TYPE", "").lower()
        if session == "wayland":
            return "wayland"
        if session == "x11":
            return "x11"
        if os.environ.get("WAYLAND_DISPLAY"):
            return "wayland"
        if os.environ.get("DISPLAY"):
            return "x11"
        return "headless"

    def gpu_info(self) -> dict:
        info: dict = {"gpus": [], "hw_encode": False, "encoder": "software"}
        # NVIDIA
        r = _run(["nvidia-smi", "--query-gpu=name", "--format=csv,noheader,nounits"])
        if r.returncode == 0:
            for line in r.stdout.strip().split("\n"):
                if line.strip():
                    info["gpus"].append({"name": line.strip(), "vendor": "nvidia"})
            info["hw_encode"] = True
            info["encoder"] = "nvenc"
            return info
        # VAAPI
        if _which("vainfo"):
            r = _run(["vainfo"])
            if r.returncode == 0 and "VAEntrypointEncSlice" in r.stdout:
                info["hw_encode"] = True
                info["encoder"] = "vaapi"
                info["gpus"].append({"name": "VAAPI device", "vendor": "intel/amd"})
                return info
        return info
