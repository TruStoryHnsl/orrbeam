"""Orrbeam CLI — unified interface for the mesh."""

import json
import sys

import click
from rich.console import Console
from rich.table import Table

from .config import Config, DEFAULT_API_PORT
from .identity import default_node_name, generate_identity, get_fingerprint
from .platform import get_platform

console = Console()


def _api_url(config: Config, path: str) -> str:
    return f"http://{config.api_bind}:{config.api_port}{path}"


def _api_get(config: Config, path: str) -> dict | None:
    """Simple sync HTTP GET to the daemon API."""
    import urllib.request
    try:
        url = _api_url(config, path)
        with urllib.request.urlopen(url, timeout=3) as resp:
            return json.loads(resp.read())
    except Exception:
        return None


def _api_post(config: Config, path: str, data: dict | None = None) -> dict | None:
    """Simple sync HTTP POST to the daemon API."""
    import urllib.request
    try:
        url = _api_url(config, path)
        body = json.dumps(data or {}).encode()
        req = urllib.request.Request(url, data=body, method="POST",
                                     headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read())
    except Exception:
        return None


def _check_daemon(config: Config) -> bool:
    result = _api_get(config, "/health")
    if not result:
        console.print("[red]Daemon not running.[/red] Start it with: [bold]orrbeamd[/bold]")
        return False
    return True


@click.group()
@click.pass_context
def main(ctx: click.Context) -> None:
    """Orrbeam — unified Sunshine/Moonlight mesh manager."""
    ctx.ensure_object(dict)
    ctx.obj["config"] = Config.load()


@main.command()
@click.pass_context
def status(ctx: click.Context) -> None:
    """Show local node status."""
    config = ctx.obj["config"]
    if not _check_daemon(config):
        # Fallback: show local detection without daemon
        platform = get_platform()
        sun = platform.detect_sunshine()
        moon = platform.detect_moonlight()

        table = Table(title="Orrbeam Local Status (daemon offline)")
        table.add_column("Component", style="cyan")
        table.add_column("Status", style="bold")
        table.add_column("Details")

        table.add_row("Daemon", "[red]stopped[/red]", f"Port {config.api_port}")
        table.add_row("Sunshine",
                       "[green]installed[/green]" if sun.installed else "[red]not found[/red]",
                       f"{'running' if sun.running else 'stopped'} {sun.version}".strip())
        table.add_row("Moonlight",
                       "[green]installed[/green]" if moon.installed else "[red]not found[/red]",
                       moon.path or "")
        table.add_row("Display", platform.display_server(), "")
        gpu = platform.gpu_info()
        table.add_row("GPU", gpu.get("encoder", "unknown"),
                       ", ".join(g["name"] for g in gpu.get("gpus", [])) or "none detected")

        console.print(table)
        return

    data = _api_get(config, "/api/status")
    if not data:
        console.print("[red]Failed to get status[/red]")
        return

    table = Table(title=f"Orrbeam Node: {data['node_name']}")
    table.add_column("Component", style="cyan")
    table.add_column("Status", style="bold")
    table.add_column("Details")

    table.add_row("Fingerprint", data.get("fingerprint", "?"), "")
    sun = data.get("sunshine", {})
    table.add_row("Sunshine",
                   "[green]running[/green]" if sun.get("running") else
                   "[yellow]installed[/yellow]" if sun.get("installed") else "[red]missing[/red]",
                   sun.get("version", ""))
    moon = data.get("moonlight", {})
    table.add_row("Moonlight",
                   "[green]installed[/green]" if moon.get("installed") else "[red]missing[/red]",
                   moon.get("path", ""))
    table.add_row("Display", data.get("display", "?"), "")
    gpu = data.get("gpu", {})
    table.add_row("Encoder", gpu.get("encoder", "?"),
                   ", ".join(g["name"] for g in gpu.get("gpus", [])))
    table.add_row("Peers", str(data.get("peers", 0)), "discovered nodes")

    console.print(table)


@main.command("list")
@click.pass_context
def list_nodes(ctx: click.Context) -> None:
    """List all discovered mesh nodes."""
    config = ctx.obj["config"]
    if not _check_daemon(config):
        return

    data = _api_get(config, "/api/nodes")
    if not data:
        console.print("[red]Failed to list nodes[/red]")
        return

    nodes = data.get("nodes", [])
    if not nodes:
        console.print("[dim]No nodes discovered yet.[/dim]")
        return

    table = Table(title="Orrbeam Mesh Nodes")
    table.add_column("Name", style="cyan bold")
    table.add_column("Address")
    table.add_column("State", style="bold")
    table.add_column("Source")
    table.add_column("Sunshine")
    table.add_column("Moonlight")

    state_colors = {
        "online": "green", "offline": "red",
        "hosting": "blue", "connected": "magenta",
    }

    for n in nodes:
        state = n.get("state", "offline")
        color = state_colors.get(state, "white")
        table.add_row(
            n["name"],
            f"{n['address']}:{n['port']}",
            f"[{color}]{state}[/{color}]",
            n.get("source", "?"),
            "[green]yes[/green]" if n.get("sunshine_available") else "[dim]no[/dim]",
            "[green]yes[/green]" if n.get("moonlight_available") else "[dim]no[/dim]",
        )

    console.print(table)


@main.command()
@click.argument("target")
@click.option("--app", default="Desktop", help="App to stream (default: Desktop)")
@click.pass_context
def connect(ctx: click.Context, target: str, app: str) -> None:
    """Connect to a remote node via Moonlight."""
    config = ctx.obj["config"]
    if not _check_daemon(config):
        return

    console.print(f"Connecting to [cyan]{target}[/cyan] ({app})...")
    result = _api_post(config, "/api/connect", {"node": target, "app": app})
    if result and result.get("connected"):
        console.print(f"[green]Connected to {result.get('target')}[/green]")
    else:
        console.print(f"[red]Failed to connect to {target}[/red]")


@main.command()
@click.pass_context
def disconnect(ctx: click.Context) -> None:
    """Disconnect active Moonlight session."""
    config = ctx.obj["config"]
    result = _api_post(config, "/api/disconnect")
    if result and result.get("disconnected"):
        console.print("[green]Disconnected[/green]")
    else:
        console.print("[red]Failed to disconnect[/red]")


@main.command()
@click.pass_context
def setup(ctx: click.Context) -> None:
    """Interactive setup — install Sunshine, Moonlight, configure firewall, register service."""
    config = ctx.obj["config"]
    platform = get_platform()

    console.print("[bold]Orrbeam Setup[/bold]\n")

    # Node name
    name = config.node_name or default_node_name()
    console.print(f"Node name: [cyan]{name}[/cyan]")
    config.node_name = name

    # Identity
    console.print("Generating node identity...")
    generate_identity()
    console.print(f"Fingerprint: [cyan]{get_fingerprint()}[/cyan]")

    # Platform info
    console.print(f"Display server: [cyan]{platform.display_server()}[/cyan]")
    gpu = platform.gpu_info()
    encoder = gpu.get("encoder", "software")
    gpus = ", ".join(g["name"] for g in gpu.get("gpus", []))
    console.print(f"GPU: [cyan]{gpus or 'none'}[/cyan] (encoder: {encoder})")

    # Sunshine
    sun = platform.detect_sunshine()
    if sun.installed:
        console.print(f"[green]Sunshine already installed[/green] at {sun.path}")
    else:
        console.print("Installing Sunshine...")
        if platform.install_sunshine():
            console.print("[green]Sunshine installed[/green]")
        else:
            console.print("[yellow]Sunshine installation needs manual steps (see above)[/yellow]")

    # Moonlight
    moon = platform.detect_moonlight()
    if moon.installed:
        console.print(f"[green]Moonlight already installed[/green] at {moon.path}")
    else:
        console.print("Installing Moonlight...")
        if platform.install_moonlight():
            console.print("[green]Moonlight installed[/green]")
        else:
            console.print("[yellow]Moonlight installation needs manual steps[/yellow]")

    # Firewall
    console.print("Configuring firewall...")
    platform.configure_firewall()

    # Service
    console.print("Installing orrbeamd service...")
    if platform.install_service():
        console.print("[green]Service installed and started[/green]")
    else:
        console.print("[yellow]Service installation failed — run orrbeamd manually[/yellow]")

    # Save config
    config.save()
    console.print(f"\n[bold green]Setup complete![/bold green] Config saved to {config.CONFIG_FILE if hasattr(config, 'CONFIG_FILE') else '~/.config/orrbeam/config.yaml'}")
    console.print("Start daemon: [bold]orrbeamd[/bold]")
    console.print("Check status: [bold]orrbeam status[/bold]")


@main.command()
@click.pass_context
def sunshine(ctx: click.Context) -> None:
    """Start local Sunshine host."""
    config = ctx.obj["config"]
    result = _api_post(config, "/api/sunshine/start")
    if result and result.get("started"):
        console.print("[green]Sunshine started[/green]")
    else:
        # Try directly
        platform = get_platform()
        if platform.start_sunshine():
            console.print("[green]Sunshine started (direct)[/green]")
        else:
            console.print("[red]Failed to start Sunshine[/red]")


@main.command("uninstall")
@click.pass_context
def uninstall(ctx: click.Context) -> None:
    """Remove orrbeamd service."""
    platform = get_platform()
    if platform.uninstall_service():
        console.print("[green]Service removed[/green]")
    else:
        console.print("[red]Failed to remove service[/red]")


if __name__ == "__main__":
    main()
