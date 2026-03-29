"""Node identity — keypair generation and management."""

import hashlib
import socket
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization

from .config import IDENTITY_DIR


def _key_path() -> Path:
    return IDENTITY_DIR / "node_key"


def _pub_path() -> Path:
    return IDENTITY_DIR / "node_key.pub"


def generate_identity() -> tuple[bytes, bytes]:
    """Generate a new Ed25519 keypair. Returns (private_pem, public_pem)."""
    IDENTITY_DIR.mkdir(parents=True, exist_ok=True)
    private_key = ed25519.Ed25519PrivateKey.generate()
    private_pem = private_key.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    )
    public_pem = private_key.public_key().public_bytes(
        serialization.Encoding.PEM,
        serialization.PublicFormat.SubjectPublicKeyInfo,
    )
    _key_path().write_bytes(private_pem)
    _key_path().chmod(0o600)
    _pub_path().write_bytes(public_pem)
    return private_pem, public_pem


def load_identity() -> tuple[bytes, bytes]:
    """Load existing keypair or generate new one."""
    if _key_path().exists() and _pub_path().exists():
        return _key_path().read_bytes(), _pub_path().read_bytes()
    return generate_identity()


def get_fingerprint(public_pem: bytes | None = None) -> str:
    """Get the SHA256 fingerprint of the node's public key."""
    if public_pem is None:
        _, public_pem = load_identity()
    return hashlib.sha256(public_pem).hexdigest()[:16]


def default_node_name() -> str:
    """Generate a default node name from hostname."""
    return socket.gethostname().split(".")[0].lower()
