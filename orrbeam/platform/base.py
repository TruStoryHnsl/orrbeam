"""Abstract platform interface."""

from abc import ABC, abstractmethod
from dataclasses import dataclass


@dataclass
class ServiceStatus:
    installed: bool = False
    running: bool = False
    enabled: bool = False
    pid: int | None = None
    version: str = ""
    path: str = ""


class Platform(ABC):
    """Abstract platform operations."""

    @abstractmethod
    def detect_sunshine(self) -> ServiceStatus:
        """Detect Sunshine installation and status."""

    @abstractmethod
    def detect_moonlight(self) -> ServiceStatus:
        """Detect Moonlight installation and status."""

    @abstractmethod
    def install_sunshine(self) -> bool:
        """Install Sunshine. Returns True on success."""

    @abstractmethod
    def install_moonlight(self) -> bool:
        """Install Moonlight. Returns True on success."""

    @abstractmethod
    def start_sunshine(self) -> bool:
        """Start Sunshine service."""

    @abstractmethod
    def stop_sunshine(self) -> bool:
        """Stop Sunshine service."""

    @abstractmethod
    def start_moonlight(self, address: str, app: str = "Desktop") -> bool:
        """Launch Moonlight to connect to a remote host."""

    @abstractmethod
    def stop_moonlight(self) -> bool:
        """Stop Moonlight client."""

    @abstractmethod
    def moonlight_cli_path(self) -> str | None:
        """Return path to moonlight CLI binary that supports 'stream' and 'pair' subcommands."""

    @abstractmethod
    def pair_moonlight(self, address: str, pin: str) -> bool:
        """Initiate Moonlight pairing with a predetermined PIN. Returns True if pairing started."""

    @abstractmethod
    def install_service(self) -> bool:
        """Install orrbeamd as a system service."""

    @abstractmethod
    def uninstall_service(self) -> bool:
        """Remove orrbeamd system service."""

    @abstractmethod
    def configure_firewall(self) -> bool:
        """Open required ports for Sunshine."""

    @abstractmethod
    def display_server(self) -> str:
        """Return display server type: 'x11', 'wayland', 'quartz', 'headless'."""

    @abstractmethod
    def gpu_info(self) -> dict:
        """Return GPU info relevant to encoding capabilities."""
