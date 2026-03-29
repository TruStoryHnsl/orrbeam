"""Node discovery via mDNS and orrtellite (Headscale mesh)."""

import asyncio
import json
import logging
import socket
import ssl

from zeroconf import IPVersion, Zeroconf, ServiceInfo
from zeroconf.asyncio import AsyncZeroconf, AsyncServiceBrowser, AsyncServiceInfo

from .node import Node, NodeRegistry, NodeState, DiscoverySource
from .identity import get_fingerprint
from .config import DEFAULT_API_PORT

log = logging.getLogger("orrbeam.discovery")

SERVICE_TYPE = "_orrbeam._tcp.local."


class MDNSDiscovery:
    """Broadcast and discover orrbeam nodes via mDNS."""

    def __init__(self, node_name: str, port: int, registry: NodeRegistry,
                 has_sunshine: bool = False, has_moonlight: bool = False) -> None:
        self.node_name = node_name
        self.port = port
        self.registry = registry
        self._has_sunshine = has_sunshine
        self._has_moonlight = has_moonlight
        self._zc: AsyncZeroconf | None = None
        self._browser: AsyncServiceBrowser | None = None
        self._info: ServiceInfo | None = None

    async def start(self) -> None:
        self._zc = AsyncZeroconf(ip_version=IPVersion.V4Only)

        # Register our service with capabilities
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
                "sunshine": "1" if self._has_sunshine else "0",
                "moonlight": "1" if self._has_moonlight else "0",
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
                    sunshine_available=props.get("sunshine") == "1",
                    moonlight_available=props.get("moonlight") == "1",
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


class OrrtelliteDiscovery:
    """Discover orrbeam nodes on the orrtellite mesh (Headscale API)."""

    def __init__(self, registry: NodeRegistry, local_name: str,
                 headscale_url: str = "", api_key: str = "") -> None:
        self.registry = registry
        self.local_name = local_name
        self.headscale_url = headscale_url.rstrip("/")
        self.api_key = api_key
        self._running = False

    async def start(self) -> None:
        if not self.headscale_url or not self.api_key:
            log.info("orrtellite: no URL or API key configured, skipping mesh discovery")
            return
        self._running = True
        asyncio.ensure_future(self._poll_loop())
        log.info("orrtellite: polling %s every 30s", self.headscale_url)

    async def stop(self) -> None:
        self._running = False

    async def _poll_loop(self) -> None:
        # Initial scan immediately
        await self._scan()
        while self._running:
            await asyncio.sleep(30)
            try:
                await self._scan()
            except Exception as e:
                log.debug("orrtellite scan failed: %s", e)

    async def _scan(self) -> None:
        """Query Headscale API for all mesh nodes."""
        nodes = await self._api_get_nodes()
        if nodes is None:
            return

        for hs_node in nodes:
            name = hs_node.get("givenName") or hs_node.get("name", "")
            name = name.lower()
            if not name or name == self.local_name:
                continue

            online = hs_node.get("online", False)
            if not online:
                continue

            # Get mesh IPs (prefer IPv4 in 100.64.x.x range)
            ip_addrs = hs_node.get("ipAddresses", [])
            if not ip_addrs:
                continue

            address = ip_addrs[0]
            for ip in ip_addrs:
                if ip.startswith("100."):
                    address = ip
                    break

            # Probe for orrbeam daemon
            probe_result = await self._probe(address, DEFAULT_API_PORT)
            if probe_result:
                real_name = probe_result.get("node", name)
                # Skip if already known via mDNS (mDNS has fingerprint + capabilities)
                existing = self.registry.get(real_name)
                if existing and existing.source == DiscoverySource.MDNS:
                    continue
                node = Node(
                    name=real_name,
                    address=address,
                    port=DEFAULT_API_PORT,
                    state=NodeState.ONLINE,
                    source=DiscoverySource.ORRTELLITE,
                )
                self.registry.upsert(node)
                log.info("orrtellite: found orrbeam node %s at %s", real_name, address)

    async def _api_get_nodes(self) -> list[dict] | None:
        """Fetch node list from Headscale REST API."""
        import aiohttp

        url = f"{self.headscale_url}/api/v1/node"
        headers = {"Authorization": f"Bearer {self.api_key}"}

        # Allow self-signed certs (common in self-hosted setups)
        ssl_ctx = ssl.create_default_context()
        ssl_ctx.check_hostname = False
        ssl_ctx.verify_mode = ssl.CERT_NONE

        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(url, headers=headers, ssl=ssl_ctx,
                                       timeout=aiohttp.ClientTimeout(total=10)) as resp:
                    if resp.status != 200:
                        log.debug("orrtellite API returned %d", resp.status)
                        return None
                    data = await resp.json()
                    return data.get("nodes", [])
        except Exception as e:
            log.debug("orrtellite API error: %s", e)
            return None

    async def _probe(self, address: str, port: int) -> dict | None:
        """Check if an orrbeam daemon is running. Returns health JSON or None."""
        try:
            reader, writer = await asyncio.wait_for(
                asyncio.open_connection(address, port), timeout=2.0
            )
            writer.write(b"GET /health HTTP/1.0\r\nHost: orrbeam\r\n\r\n")
            await writer.drain()
            data = await asyncio.wait_for(reader.read(512), timeout=2.0)
            writer.close()
            await writer.wait_closed()
            body = data.split(b"\r\n\r\n", 1)[-1]
            if b"orrbeam" in body:
                return json.loads(body)
            return None
        except Exception:
            return None
