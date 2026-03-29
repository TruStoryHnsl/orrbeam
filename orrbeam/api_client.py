"""Shared HTTP client for talking to the orrbeam daemon."""

import json
import urllib.request

from .config import Config


def api_url(config: Config, path: str) -> str:
    return f"http://127.0.0.1:{config.api_port}{path}"


def api_get(config: Config, path: str) -> dict | None:
    try:
        with urllib.request.urlopen(api_url(config, path), timeout=3) as resp:
            return json.loads(resp.read())
    except Exception:
        return None


def api_post(config: Config, path: str, data: dict | None = None) -> dict | None:
    try:
        body = json.dumps(data or {}).encode()
        req = urllib.request.Request(api_url(config, path), data=body, method="POST",
                                     headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=60) as resp:
            return json.loads(resp.read())
    except Exception:
        return None
