"""Node discovery via mDNS and Tailscale."""

import asyncio
import json
import logging
import shutil
import socket

from zeroconf import IPVersion, Zeroconf, ServiceInfo
from zeroconf.asyncio import AsyncZeroconf, AsyncServiceBrowser, AsyncServiceInfo

from .node import Node, NodeRegistry, NodeState, DiscoverySource
from .identity import get_fingerprint
from .config import DEFAULT_API_PORT

log = logging.getLogger("orrbeam.discovery")

SERVICE_TYPE = "_orrbeam._tcp.local."


class MDNSDiscovery:
    """Broadcast and discover orrbeam nodes via mDNS."""

    def __init__(self, node_name: str, port: int, registry: NodeRegistry) -> None:
        self.node_name = node_name
        self.port = port
        self.registry = registry
        self._zc: AsyncZeroconf | None = None
        self._browser: AsyncServiceBrowser | None = None
        self._info: ServiceInfo | None = None

    async def start(self) -> None:
        self._zc = AsyncZeroconf(ip_version=IPVersion.V4Only)

        # Register our service
        fingerprint = get_fingerprint()
        addresses = self._get_local_ips()
        self._info = ServiceInfo(
            SERVICE_TYPE,
            f"{self.node_name}.{SERVICE_TYPE}",
            addresses=[socket.inet_aton(ip) for ip in addresses],
            port=self.port,
            properties={
                "fingerprint": fingerprint,
                "version": "0.1.0",
            },
        )
        await self._zc.async_register_service(self._info)
        log.info("mDNS: broadcasting as %s on %s", self.node_name, addresses)

        # Browse for other nodes
        self._browser = AsyncServiceBrowser(
            self._zc.zeroconf, SERVICE_TYPE, handlers=[self._on_change]
        )

    async def stop(self) -> None:
        if self._browser:
            self._browser.cancel()
        if self._zc and self._info:
            await self._zc.async_unregister_service(self._info)
        if self._zc:
            await self._zc.async_close()

    def _on_change(self, zeroconf: Zeroconf, service_type: str, name: str, state_change) -> None:
        asyncio.ensure_future(self._handle_change(zeroconf, service_type, name, state_change))

    async def _handle_change(self, zeroconf: Zeroconf, service_type: str, name: str, state_change) -> None:
        from zeroconf import ServiceStateChange

        node_name = name.replace(f".{SERVICE_TYPE}", "")
        if node_name == self.node_name:
            return

        if state_change in (ServiceStateChange.Added, ServiceStateChange.Updated):
            info = AsyncServiceInfo(service_type, name)
            await info.async_request(zeroconf, 3000)
            if info.addresses:
                address = socket.inet_ntoa(info.addresses[0])
                props = {k.decode(): v.decode() if isinstance(v, bytes) else v
                         for k, v in (info.properties or {}).items()}
                node = Node(
                    name=node_name,
                    address=address,
                    port=info.port or DEFAULT_API_PORT,
                    fingerprint=props.get("fingerprint", ""),
                    state=NodeState.ONLINE,
                    source=DiscoverySource.MDNS,
                )
                self.registry.upsert(node)
                log.info("mDNS: discovered %s at %s:%d", node_name, address, node.port)

        elif state_change == ServiceStateChange.Removed:
            existing = self.registry.get(node_name)
            if existing and existing.source == DiscoverySource.MDNS:
                existing.state = NodeState.OFFLINE
                log.info("mDNS: %s went offline", node_name)

    def _get_local_ips(self) -> list[str]:
        ips = []
        try:
            for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
                ip = info[4][0]
                if not ip.startswith("127."):
                    ips.append(ip)
        except socket.gaierror:
            pass
        if not ips:
            try:
                s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                s.connect(("8.8.8.8", 80))
                ips.append(s.getsockname()[0])
                s.close()
            except Exception:
                ips.append("127.0.0.1")
        return ips


class TailscaleDiscovery:
    """Discover orrbeam nodes on the Tailscale/Headscale network."""

    def __init__(self, registry: NodeRegistry, local_name: str) -> None:
        self.registry = registry
        self.local_name = local_name
        self._running = False

    async def start(self) -> None:
        self._running = True
        asyncio.ensure_future(self._poll_loop())

    async def stop(self) -> None:
        self._running = False

    async def _poll_loop(self) -> None:
        while self._running:
            try:
                await self._scan()
            except Exception as e:
                log.debug("Tailscale scan failed: %s", e)
            await asyncio.sleep(30)

    async def _scan(self) -> None:
        ts = shutil.which("tailscale")
        if not ts:
            return

        proc = await asyncio.create_subprocess_exec(
            ts, "status", "--json",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.DEVNULL,
        )
        stdout, _ = await proc.communicate()
        if proc.returncode != 0:
            return

        data = json.loads(stdout)
        peers = data.get("Peer", {})

        for _key, peer in peers.items():
            if not peer.get("Online", False):
                continue
            hostname = peer.get("HostName", "").lower()
            if not hostname or hostname == self.local_name:
                continue

            ts_ips = peer.get("TailscaleIPs", [])
            if not ts_ips:
                continue

            # Prefer IPv4
            address = ts_ips[0]
            for ip in ts_ips:
                if "." in ip:
                    address = ip
                    break

            # Probe for orrbeam daemon
            is_orrbeam = await self._probe(address, DEFAULT_API_PORT)
            if is_orrbeam:
                node = Node(
                    name=hostname,
                    address=address,
                    port=DEFAULT_API_PORT,
                    state=NodeState.ONLINE,
                    source=DiscoverySource.TAILSCALE,
                )
                self.registry.upsert(node)
                log.info("Tailscale: found orrbeam node %s at %s", hostname, address)

    async def _probe(self, address: str, port: int) -> bool:
        """Check if an orrbeam daemon is running at the given address."""
        try:
            reader, writer = await asyncio.wait_for(
                asyncio.open_connection(address, port), timeout=2.0
            )
            writer.write(b"GET /health HTTP/1.0\r\nHost: orrbeam\r\n\r\n")
            await writer.drain()
            data = await asyncio.wait_for(reader.read(256), timeout=2.0)
            writer.close()
            await writer.wait_closed()
            return b"orrbeam" in data
        except Exception:
            return False
