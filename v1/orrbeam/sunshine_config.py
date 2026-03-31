"""Read/write Sunshine configuration and restart the service."""

import os
import sys
import shutil
import subprocess
from pathlib import Path


def _conf_path() -> Path:
    if sys.platform == "darwin":
        # macOS Sunshine stores config in various locations
        for p in [Path.home() / "Library" / "Application Support" / "Sunshine" / "sunshine.conf",
                  Path.home() / ".config" / "sunshine" / "sunshine.conf"]:
            if p.exists():
                return p
        # Default to the first path
        return Path.home() / "Library" / "Application Support" / "Sunshine" / "sunshine.conf"
    return Path.home() / ".config" / "sunshine" / "sunshine.conf"


def read_conf() -> dict[str, str]:
    """Parse sunshine.conf into key=value pairs."""
    path = _conf_path()
    if not path.exists():
        return {}
    config = {}
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" in line:
            key, _, value = line.partition("=")
            config[key.strip()] = value.strip()
    return config


def write_conf(config: dict[str, str]) -> None:
    """Write config dict to sunshine.conf, preserving comments."""
    path = _conf_path()
    path.parent.mkdir(parents=True, exist_ok=True)

    existing_lines = []
    if path.exists():
        existing_lines = path.read_text().splitlines()

    written_keys: set[str] = set()
    new_lines = []
    for line in existing_lines:
        stripped = line.strip()
        if stripped and not stripped.startswith("#") and "=" in stripped:
            key = stripped.split("=", 1)[0].strip()
            if key in config:
                new_lines.append(f"{key} = {config[key]}")
                written_keys.add(key)
                continue
        new_lines.append(line)

    # Append any new keys
    for key, value in config.items():
        if key not in written_keys:
            new_lines.append(f"{key} = {value}")

    path.write_text("\n".join(new_lines) + "\n")


def get_output_name() -> str | None:
    return read_conf().get("output_name")


def set_output_name(name: str) -> None:
    config = read_conf()
    config["output_name"] = name
    write_conf(config)


def restart_sunshine() -> bool:
    """Restart Sunshine to pick up config changes."""
    if sys.platform == "darwin":
        subprocess.run(["pkill", "-f", "Sunshine"], capture_output=True)
        import time
        time.sleep(2)
        for app_path in ["/Applications/Sunshine.app",
                         str(Path.home() / "Applications" / "Sunshine.app")]:
            if Path(app_path).exists():
                subprocess.run(["open", "-a", app_path], capture_output=True)
                return True
        return False
    else:
        # Linux: try systemd user service first
        r = subprocess.run(["systemctl", "--user", "restart", "sunshine"],
                           capture_output=True, text=True)
        if r.returncode == 0:
            return True
        # Direct restart
        subprocess.run(["pkill", "-x", "sunshine"], capture_output=True)
        sun = shutil.which("sunshine")
        if sun:
            subprocess.Popen([sun], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return True
        return False
