"""Sunshine REST API client for pairing automation."""

import asyncio
import logging
import ssl

import aiohttp

log = logging.getLogger("orrbeam.sunshine_api")


def _ssl_context() -> ssl.SSLContext:
    """SSL context that accepts self-signed certs (standard for Sunshine)."""
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return ctx


async def submit_pin(host: str, port: int, username: str, password: str,
                     pin: str, client_name: str = "orrbeam") -> bool:
    """Submit a pairing PIN to Sunshine's web API.

    Retries because the PIN endpoint only succeeds when a pairing request
    is actively pending (timing window after moonlight pair starts).
    """
    url = f"https://{host}:{port}/api/pin"
    auth = aiohttp.BasicAuth(username, password)

    for attempt in range(15):
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    url,
                    json={"pin": pin, "name": client_name},
                    auth=auth,
                    ssl=_ssl_context(),
                    timeout=aiohttp.ClientTimeout(total=5),
                ) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        if data.get("status"):
                            log.info("PIN accepted by Sunshine at %s", host)
                            return True
                        log.debug("PIN attempt %d: status=false (no pending request yet)", attempt + 1)
                    else:
                        log.debug("PIN attempt %d: HTTP %d", attempt + 1, resp.status)
        except Exception as e:
            log.debug("PIN attempt %d: %s", attempt + 1, e)
        await asyncio.sleep(1.0)

    log.warning("PIN submission failed after 15 attempts for %s", host)
    return False
