"""Node model and registry."""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from enum import Enum


class NodeState(str, Enum):
    ONLINE = "online"
    OFFLINE = "offline"
    HOSTING = "hosting"
    CONNECTED = "connected"


class DiscoverySource(str, Enum):
    MDNS = "mdns"
    TAILSCALE = "tailscale"
    STATIC = "static"


@dataclass
class Node:
    name: str
    address: str
    port: int = 47782
    fingerprint: str = ""
    state: NodeState = NodeState.OFFLINE
    source: DiscoverySource = DiscoverySource.STATIC
    sunshine_available: bool = False
    moonlight_available: bool = False
    last_seen: float = 0.0
    is_local: bool = False

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "address": self.address,
            "port": self.port,
            "fingerprint": self.fingerprint,
            "state": self.state.value,
            "source": self.source.value,
            "sunshine_available": self.sunshine_available,
            "moonlight_available": self.moonlight_available,
            "last_seen": self.last_seen,
            "is_local": self.is_local,
        }


class NodeRegistry:
    """Thread-safe registry of known nodes."""

    def __init__(self) -> None:
        self._nodes: dict[str, Node] = {}

    def upsert(self, node: Node) -> None:
        node.last_seen = time.time()
        self._nodes[node.name] = node

    def remove(self, name: str) -> None:
        self._nodes.pop(name, None)

    def get(self, name: str) -> Node | None:
        return self._nodes.get(name)

    def all(self) -> list[Node]:
        return sorted(self._nodes.values(), key=lambda n: n.name)

    def online(self) -> list[Node]:
        return [n for n in self.all() if n.state != NodeState.OFFLINE]

    def prune_stale(self, max_age: float = 120.0) -> list[str]:
        """Remove nodes not seen within max_age seconds. Returns removed names."""
        now = time.time()
        stale = [name for name, n in self._nodes.items()
                 if not n.is_local and n.source != DiscoverySource.STATIC
                 and now - n.last_seen > max_age]
        for name in stale:
            self._nodes[name].state = NodeState.OFFLINE
        return stale

    def to_list(self) -> list[dict]:
        return [n.to_dict() for n in self.all()]
