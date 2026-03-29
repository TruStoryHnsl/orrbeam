"""Orrbeam daemon — manages Sunshine/Moonlight and serves the REST API."""

import asyncio
import logging
import random
import signal
import socket
import sys

import aiohttp
from aiohttp import web

from .config import Config
from .node import Node, NodeRegistry, NodeState, DiscoverySource
from .identity import load_identity, get_fingerprint, default_node_name
from .discovery import MDNSDiscovery, OrrtelliteDiscovery
from .platform import get_platform
from . import sunshine_api
from . import sunshine_config

log = logging.getLogger("orrbeam")


def _get_local_address() -> str:
    """Get this machine's non-loopback IP address."""
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        addr = s.getsockname()[0]
        s.close()
        return addr
    except Exception:
        return "127.0.0.1"


class OrrbeamDaemon:
    def __init__(self, config: Config) -> None:
        self.config = config
        self.platform = get_platform()
        self.registry = NodeRegistry()
        self._mdns: MDNSDiscovery | None = None
        self._orrtellite: OrrtelliteDiscovery | None = None
        self._app = web.Application()
        self._runner: web.AppRunner | None = None
        self._prune_task: asyncio.Task | None = None

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
        self._app.router.add_post("/api/pair", self._pair)
        self._app.router.add_post("/api/pair/accept", self._pair_accept)
        self._app.router.add_post("/api/connect-back", self._connect_back)
        self._app.router.add_post("/api/loop", self._loop)
        self._app.router.add_get("/api/monitors", self._list_monitors)
        self._app.router.add_post("/api/display", self._set_display)

    # ── Status handlers ──

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

    async def _platform_info(self, _req: web.Request) -> web.Response:
        return web.json_response({
            "display": self.platform.display_server(),
            "gpu": self.platform.gpu_info(),
            "sunshine": self.platform.detect_sunshine().__dict__,
            "moonlight": self.platform.detect_moonlight().__dict__,
        })

    # ── Connect/disconnect ──

    async def _connect(self, req: web.Request) -> web.Response:
        data = await req.json()
        target = data.get("node") or data.get("address")
        app = data.get("app", "Desktop")
        if not target:
            return web.json_response({"error": "missing 'node' or 'address'"}, status=400)
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

    # ── Pairing ──

    async def _pair(self, req: web.Request) -> web.Response:
        """Orchestrate Moonlight-Sunshine pairing with a remote node."""
        data = await req.json()
        target = data.get("node") or data.get("address")
        if not target:
            return web.json_response({"error": "missing 'node' or 'address'"}, status=400)

        address = target
        node = self.registry.get(target)
        if node:
            address = node.address

        pin = f"{random.randint(0, 9999):04d}"
        log.info("Pairing with %s using PIN %s", address, pin)

        # Start moonlight pair (spawns process, may or may not block)
        started = await asyncio.get_event_loop().run_in_executor(
            None, self.platform.pair_moonlight, address, pin)
        if not started:
            return web.json_response({"error": "moonlight CLI not found"}, status=500)

        # Wait for the pairing handshake to initiate (Moonlight needs time to
        # discover the host and start the TLS handshake with Sunshine)
        await asyncio.sleep(5)

        # Ask remote daemon to submit PIN to its local Sunshine
        pin_accepted = False
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"http://{address}:47782/api/pair/accept",
                    json={"pin": pin, "name": self.config.node_name},
                    timeout=aiohttp.ClientTimeout(total=20),
                ) as resp:
                    result = await resp.json()
                    pin_accepted = result.get("accepted", False)
        except Exception as e:
            log.warning("Failed to submit PIN to remote daemon: %s", e)

        if pin_accepted:
            log.info("Successfully paired with %s", address)
        else:
            log.warning("Pairing with %s: pin_accepted=%s", address, pin_accepted)

        return web.json_response({"paired": pin_accepted, "target": address})

    async def _pair_accept(self, req: web.Request) -> web.Response:
        """Accept a pairing PIN on local Sunshine (called by remote daemon)."""
        data = await req.json()
        pin = data.get("pin", "")
        client_name = data.get("name", "remote")

        if not pin:
            return web.json_response({"error": "missing 'pin'"}, status=400)

        if not self.config.sunshine_username or not self.config.sunshine_password:
            return web.json_response(
                {"error": "sunshine credentials not configured"}, status=500)

        log.info("Accepting pairing PIN from %s", client_name)
        accepted = await sunshine_api.submit_pin(
            host="127.0.0.1", port=47990,
            username=self.config.sunshine_username,
            password=self.config.sunshine_password,
            pin=pin, client_name=client_name,
        )
        return web.json_response({"accepted": accepted})

    # ── Display management ──

    async def _list_monitors(self, _req: web.Request) -> web.Response:
        monitors = await asyncio.get_event_loop().run_in_executor(
            None, self.platform.list_monitors)
        current = sunshine_config.get_output_name()
        return web.json_response({
            "monitors": [m.to_dict() for m in monitors],
            "current_output": current,
        })

    async def _set_display(self, req: web.Request) -> web.Response:
        data = await req.json()
        output_name = data.get("output_name")
        rotation = data.get("rotation")

        if output_name:
            await asyncio.get_event_loop().run_in_executor(
                None, sunshine_config.set_output_name, output_name)

        if rotation:
            target = output_name or sunshine_config.get_output_name() or ""
            await asyncio.get_event_loop().run_in_executor(
                None, self.platform.set_rotation, target, rotation)

        ok = await asyncio.get_event_loop().run_in_executor(
            None, sunshine_config.restart_sunshine)
        log.info("Display config: output=%s rotation=%s restart=%s", output_name, rotation, ok)
        return web.json_response({"applied": ok, "output_name": output_name, "rotation": rotation})

    # ── Bidirectional loop ──

    async def _connect_back(self, req: web.Request) -> web.Response:
        """Start Moonlight streaming back to caller (for bidirectional loop)."""
        data = await req.json()
        caller_address = data.get("caller_address", "")
        app = data.get("app", "Desktop")
        if not caller_address:
            return web.json_response({"error": "missing 'caller_address'"}, status=400)

        log.info("Connect-back: streaming to %s (%s)", caller_address, app)
        ok = self.platform.start_moonlight(caller_address, app)
        return web.json_response({"connected": ok, "target": caller_address})

    async def _loop(self, req: web.Request) -> web.Response:
        """Start bidirectional streaming loop with a remote node."""
        data = await req.json()
        target = data.get("node") or data.get("address")
        app = data.get("app", "Desktop")
        if not target:
            return web.json_response({"error": "missing 'node' or 'address'"}, status=400)

        address = target
        node = self.registry.get(target)
        if node:
            address = node.address

        local_address = _get_local_address()
        log.info("Loop: %s <-> %s (%s)", local_address, address, app)

        # Start local Moonlight -> remote Sunshine
        ok = self.platform.start_moonlight(address, app)
        if not ok:
            return web.json_response({"error": "failed to start local Moonlight"}, status=500)

        # Ask remote daemon to connect back
        await asyncio.sleep(1)
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"http://{address}:47782/api/connect-back",
                    json={"app": app, "caller_address": local_address},
                    timeout=aiohttp.ClientTimeout(total=10),
                ) as resp:
                    result = await resp.json()
                    if not result.get("connected"):
                        return web.json_response(
                            {"error": "remote failed to connect back"}, status=502)
        except Exception as e:
            return web.json_response({"error": f"remote unreachable: {e}"}, status=502)

        log.info("Loop active: %s <-> %s", self.config.node_name, target)
        return web.json_response({"looping": True, "target": address, "local": local_address})

    # ── Lifecycle ──

    async def start(self) -> None:
        load_identity()
        log.info("Node: %s (fingerprint: %s)", self.config.node_name, get_fingerprint())

        self.registry.upsert(Node(
            name=self.config.node_name, address="127.0.0.1",
            port=self.config.api_port, fingerprint=get_fingerprint(),
            state=NodeState.HOSTING, source=DiscoverySource.STATIC,
            sunshine_available=self.platform.detect_sunshine().installed,
            moonlight_available=self.platform.detect_moonlight().installed,
            is_local=True,
        ))

        for sn in self.config.static_nodes:
            self.registry.upsert(Node(
                name=sn.name, address=sn.address, port=sn.port,
                fingerprint=sn.fingerprint, state=NodeState.OFFLINE,
                source=DiscoverySource.STATIC,
            ))

        sun_installed = self.platform.detect_sunshine().installed
        moon_installed = self.platform.detect_moonlight().installed
        if self.config.discovery_enabled and self.config.mdns_enabled:
            try:
                self._mdns = MDNSDiscovery(
                    self.config.node_name, self.config.api_port, self.registry,
                    has_sunshine=sun_installed, has_moonlight=moon_installed,
                )
                await self._mdns.start()
            except Exception as e:
                log.warning("mDNS failed: %s (continuing without)", e)
                self._mdns = None

        if self.config.discovery_enabled and self.config.orrtellite_enabled:
            try:
                self._orrtellite = OrrtelliteDiscovery(
                    self.registry, self.config.node_name,
                    headscale_url=self.config.orrtellite_url,
                    api_key=self.config.orrtellite_api_key,
                )
                await self._orrtellite.start()
            except Exception as e:
                log.warning("orrtellite failed: %s (continuing without)", e)
                self._orrtellite = None

        self._prune_task = asyncio.create_task(self._prune_loop())

        self._runner = web.AppRunner(self._app, access_log=None)
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
        if self._orrtellite:
            await self._orrtellite.stop()
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
