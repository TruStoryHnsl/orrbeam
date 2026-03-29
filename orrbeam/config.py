"""Configuration management for orrbeam."""

import os
import sys
from pathlib import Path
from dataclasses import dataclass, field

import yaml


def _config_dir() -> Path:
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "orrbeam"
    return Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "orrbeam"


def _data_dir() -> Path:
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "orrbeam"
    return Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share")) / "orrbeam"


CONFIG_DIR = _config_dir()
DATA_DIR = _data_dir()
CONFIG_FILE = CONFIG_DIR / "config.yaml"
NODES_FILE = CONFIG_DIR / "nodes.yaml"
IDENTITY_DIR = DATA_DIR / "identity"

DEFAULT_API_PORT = 47782
SUNSHINE_PORTS = range(47984, 47991)  # 47984-47990
SUNSHINE_WEB_PORT = 47990
MOONLIGHT_DEFAULT_PORT = 47989


@dataclass
class NodeEntry:
    name: str
    address: str
    port: int = DEFAULT_API_PORT
    fingerprint: str = ""
    trusted: bool = False


@dataclass
class Config:
    node_name: str = ""
    api_port: int = DEFAULT_API_PORT
    api_bind: str = "127.0.0.1"
    discovery_enabled: bool = True
    tailscale_enabled: bool = True
    mdns_enabled: bool = True
    sunshine_path: str = ""
    moonlight_path: str = ""
    static_nodes: list[NodeEntry] = field(default_factory=list)

    @classmethod
    def load(cls) -> "Config":
        if CONFIG_FILE.exists():
            with open(CONFIG_FILE) as f:
                data = yaml.safe_load(f) or {}
            nodes_raw = data.pop("static_nodes", [])
            nodes = [NodeEntry(**n) for n in nodes_raw]
            return cls(**data, static_nodes=nodes)
        return cls()

    def save(self) -> None:
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        data = {
            "node_name": self.node_name,
            "api_port": self.api_port,
            "api_bind": self.api_bind,
            "discovery_enabled": self.discovery_enabled,
            "tailscale_enabled": self.tailscale_enabled,
            "mdns_enabled": self.mdns_enabled,
            "sunshine_path": self.sunshine_path,
            "moonlight_path": self.moonlight_path,
            "static_nodes": [
                {"name": n.name, "address": n.address, "port": n.port,
                 "fingerprint": n.fingerprint, "trusted": n.trusted}
                for n in self.static_nodes
            ],
        }
        with open(CONFIG_FILE, "w") as f:
            yaml.dump(data, f, default_flow_style=False)
