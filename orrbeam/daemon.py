"""Orrbeam daemon — manages Sunshine/Moonlight and serves the REST API."""

import asyncio
import logging
import signal
import sys

from aiohttp import web

from .config import Config
from .node import Node, NodeRegistry, NodeState, DiscoverySource
from .identity import load_identity, get_fingerprint, default_node_name
from .discovery import MDNSDiscovery, TailscaleDiscovery
from .platform import get_platform

log = logging.getLogger("orrbeam")


class OrrbeamDaemon:
    def __init__(self, config: Config) -> None:
        self.config = config
        self.platform = get_platform()
        self.registry = NodeRegistry()
        self._mdns: MDNSDiscovery | None = None
        self._tailscale: TailscaleDiscovery | None = None
        self._app = web.Application()
        self._runner: web.AppRunner | None = None
        self._prune_task: asyncio.Task | None = None

        # Ensure node name
        if not self.config.node_name:
            self.config.node_name = default_node_name()
            self.config.save()

        self._setup_routes()

    def _setup_routes(self) -> None:
        self._app.router.add_get("/health", self._health)
        self._app.router.add_get("/api/status", self._status)
        self._app.router.add_get("/api/nodes", self._list_nodes)
        self._app.router.add_post("/api/connect", self._connect)
        self._app.router.add_post("/api/disconnect", self._disconnect)
        self._app.router.add_post("/api/sunshine/start", self._sunshine_start)
        self._app.router.add_post("/api/sunshine/stop", self._sunshine_stop)
        self._app.router.add_get("/api/platform", self._platform_info)

    # ── API handlers ──

    async def _health(self, _req: web.Request) -> web.Response:
        return web.json_response({"status": "ok", "service": "orrbeam",
                                  "node": self.config.node_name})

    async def _status(self, _req: web.Request) -> web.Response:
        sun = self.platform.detect_sunshine()
        moon = self.platform.detect_moonlight()
        return web.json_response({
            "node_name": self.config.node_name,
            "fingerprint": get_fingerprint(),
            "sunshine": {
                "installed": sun.installed, "running": sun.running,
                "version": sun.version, "path": sun.path,
            },
            "moonlight": {
                "installed": moon.installed, "path": moon.path,
                "version": moon.version,
            },
            "display": self.platform.display_server(),
            "gpu": self.platform.gpu_info(),
            "peers": len(self.registry.online()),
        })

    async def _list_nodes(self, _req: web.Request) -> web.Response:
        return web.json_response({"nodes": self.registry.to_list()})

    async def _connect(self, req: web.Request) -> web.Response:
        data = await req.json()
        target = data.get("node") or data.get("address")
        app = data.get("app", "Desktop")
        if not target:
            return web.json_response({"error": "missing 'node' or 'address'"}, status=400)

        # Resolve node name to address
        address = target
        node = self.registry.get(target)
        if node:
            address = node.address

        ok = self.platform.start_moonlight(address, app)
        return web.json_response({"connected": ok, "target": address, "app": app})

    async def _disconnect(self, _req: web.Request) -> web.Response:
        ok = self.platform.stop_moonlight()
        return web.json_response({"disconnected": ok})

    async def _sunshine_start(self, _req: web.Request) -> web.Response:
        ok = self.platform.start_sunshine()
        return web.json_response({"started": ok})

    async def _sunshine_stop(self, _req: web.Request) -> web.Response:
        ok = self.platform.stop_sunshine()
        return web.json_response({"stopped": ok})

    async def _platform_info(self, _req: web.Request) -> web.Response:
        return web.json_response({
            "display": self.platform.display_server(),
            "gpu": self.platform.gpu_info(),
            "sunshine": self.platform.detect_sunshine().__dict__,
            "moonlight": self.platform.detect_moonlight().__dict__,
        })

    # ── Lifecycle ──

    async def start(self) -> None:
        load_identity()
        log.info("Node: %s (fingerprint: %s)", self.config.node_name, get_fingerprint())

        # Register local node
        self.registry.upsert(Node(
            name=self.config.node_name,
            address="127.0.0.1",
            port=self.config.api_port,
            fingerprint=get_fingerprint(),
            state=NodeState.HOSTING,
            source=DiscoverySource.STATIC,
            sunshine_available=self.platform.detect_sunshine().installed,
            moonlight_available=self.platform.detect_moonlight().installed,
            is_local=True,
        ))

        # Load static nodes
        for sn in self.config.static_nodes:
            self.registry.upsert(Node(
                name=sn.name, address=sn.address, port=sn.port,
                fingerprint=sn.fingerprint,
                state=NodeState.OFFLINE,
                source=DiscoverySource.STATIC,
            ))

        # Start discovery
        if self.config.discovery_enabled and self.config.mdns_enabled:
            self._mdns = MDNSDiscovery(self.config.node_name, self.config.api_port, self.registry)
            await self._mdns.start()

        if self.config.discovery_enabled and self.config.tailscale_enabled:
            self._tailscale = TailscaleDiscovery(self.registry, self.config.node_name)
            await self._tailscale.start()

        # Stale node pruning
        self._prune_task = asyncio.create_task(self._prune_loop())

        # Start API server
        self._runner = web.AppRunner(self._app)
        await self._runner.setup()
        site = web.TCPSite(self._runner, self.config.api_bind, self.config.api_port)
        await site.start()
        log.info("API listening on %s:%d", self.config.api_bind, self.config.api_port)

    async def stop(self) -> None:
        log.info("Shutting down...")
        if self._prune_task:
            self._prune_task.cancel()
        if self._mdns:
            await self._mdns.stop()
        if self._tailscale:
            await self._tailscale.stop()
        if self._runner:
            await self._runner.cleanup()

    async def _prune_loop(self) -> None:
        while True:
            await asyncio.sleep(60)
            stale = self.registry.prune_stale()
            if stale:
                log.debug("Pruned stale nodes: %s", stale)


async def _run_daemon() -> None:
    config = Config.load()

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )

    daemon = OrrbeamDaemon(config)
    loop = asyncio.get_event_loop()
    stop_event = asyncio.Event()

    def _signal_handler():
        stop_event.set()

    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, _signal_handler)

    await daemon.start()
    log.info("Orrbeam daemon running. Press Ctrl+C to stop.")
    await stop_event.wait()
    await daemon.stop()


def main() -> None:
    try:
        asyncio.run(_run_daemon())
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
