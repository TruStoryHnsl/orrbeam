"""Orrbeam TUI — interactive mesh control panel."""

from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.reactive import reactive
from textual.widgets import (
    Button, Footer, Header, Label, ListItem, ListView,
    RadioButton, RadioSet, Select, Static,
)
from textual import work

from ..api_client import api_get, api_post
from ..config import Config


class OrrbeamApp(App):
    TITLE = "orrbeam"
    SUB_TITLE = "mesh control"

    CSS = """
    Screen {
        layout: horizontal;
    }
    #node-panel {
        width: 1fr;
        min-width: 22;
        max-width: 34;
        border-right: solid $primary;
        padding: 1;
    }
    #config-panel {
        width: 3fr;
        padding: 1 2;
    }
    .panel-title {
        text-style: bold;
        color: $primary;
        margin-bottom: 1;
    }
    #node-list {
        height: 1fr;
    }
    #monitor-select {
        width: 100%;
        margin: 1 0;
    }
    #rotation-set {
        layout: horizontal;
        margin: 1 0;
        height: auto;
    }
    #rotation-set RadioButton {
        width: auto;
        margin-right: 1;
    }
    #apply-btn {
        margin: 1 0;
        width: auto;
    }
    #status-line {
        margin-top: 1;
        color: $text-muted;
    }
    #resolution-display {
        margin: 0 0 1 0;
        color: $text;
    }
    .label {
        margin-top: 1;
    }
    """

    BINDINGS = [
        Binding("c", "connect", "Connect"),
        Binding("d", "disconnect", "Disconnect"),
        Binding("l", "loop", "Loop"),
        Binding("p", "pair", "Pair"),
        Binding("r", "refresh", "Refresh", show=False),
        Binding("q", "quit", "Quit"),
    ]

    nodes: reactive[list[dict]] = reactive(list, recompose=False)
    monitors: reactive[list[dict]] = reactive(list, recompose=False)
    current_output: reactive[str] = reactive("")
    selected_node: str | None = None

    def __init__(self, config: Config, **kwargs) -> None:
        super().__init__(**kwargs)
        self.config = config

    def compose(self) -> ComposeResult:
        yield Header()
        with Horizontal():
            with Vertical(id="node-panel"):
                yield Label("NODES", classes="panel-title")
                yield ListView(id="node-list")
            with VerticalScroll(id="config-panel"):
                yield Label("STREAM CONFIG", classes="panel-title")
                yield Label("Monitor:", classes="label")
                yield Select([], id="monitor-select", prompt="Select monitor...")
                yield Static("", id="resolution-display")
                yield Label("Rotation:", classes="label")
                yield RadioSet(
                    RadioButton("0\u00b0", value=True, id="rot-0"),
                    RadioButton("90\u00b0", id="rot-90"),
                    RadioButton("180\u00b0", id="rot-180"),
                    RadioButton("270\u00b0", id="rot-270"),
                    id="rotation-set",
                )
                yield Button("Apply", id="apply-btn", variant="primary")
                yield Static("Ready", id="status-line")
        yield Footer()

    def on_mount(self) -> None:
        self.refresh_data()
        self.set_interval(5.0, self.refresh_data)

    @work(exclusive=True, thread=True)
    def refresh_data(self) -> None:
        # Nodes
        data = api_get(self.config, "/api/nodes")
        if data:
            self.nodes = data.get("nodes", [])

        # Monitors
        data = api_get(self.config, "/api/monitors")
        if data:
            self.monitors = data.get("monitors", [])
            self.current_output = data.get("current_output") or ""

    def watch_nodes(self, nodes: list[dict]) -> None:
        try:
            lv = self.query_one("#node-list", ListView)
        except Exception:
            return
        lv.clear()
        indicators = {"online": "\u25cf", "offline": "\u25cb", "hosting": "\u25c6", "connected": "\u25c8"}
        for n in nodes:
            state = n.get("state", "offline")
            ind = indicators.get(state, "?")
            name = n.get("name", "?")
            label_text = f"{ind} {name} [{state}]"
            item = ListItem(Label(label_text))
            item._node_name = name
            lv.append(item)

    def watch_monitors(self, monitors: list[dict]) -> None:
        try:
            sel = self.query_one("#monitor-select", Select)
        except Exception:
            return
        options = [
            (f"{m['name']} \u2014 {m['width']}x{m['height']}", m["name"])
            for m in monitors
        ]
        sel.set_options(options)
        if self.current_output:
            try:
                sel.value = self.current_output
            except Exception:
                pass
        elif options:
            sel.value = options[0][1]

    def on_list_view_selected(self, event: ListView.Selected) -> None:
        self.selected_node = getattr(event.item, "_node_name", None)
        self._set_status(f"Selected: {self.selected_node}")

    def on_select_changed(self, event: Select.Changed) -> None:
        if event.select.id == "monitor-select" and event.value != Select.BLANK:
            for m in self.monitors:
                if m["name"] == event.value:
                    res = self.query_one("#resolution-display", Static)
                    rot = m.get("rotation", "normal")
                    res.update(
                        f"{m['width']}x{m['height']} @ {m['refresh_rate']:.1f}Hz  "
                        f"rotation: {rot}"
                    )
                    break

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "apply-btn":
            self._apply_display()

    @work(thread=True)
    def _apply_display(self) -> None:
        sel = self.query_one("#monitor-select", Select)
        output = sel.value if sel.value != Select.BLANK else None
        if not output:
            self.notify("Select a monitor first", severity="warning")
            return

        rotation_map = {0: "normal", 1: "left", 2: "inverted", 3: "right"}
        rot_set = self.query_one("#rotation-set", RadioSet)
        rotation = rotation_map.get(rot_set.pressed_index, "normal")

        self._set_status("Applying...")
        self.notify(f"Switching to {output} ({rotation}), restarting Sunshine...")
        result = api_post(self.config, "/api/display", {
            "output_name": output,
            "rotation": rotation,
        })
        if result and result.get("applied"):
            self._set_status(f"Active: {output} ({rotation})")
            self.notify("Display config applied")
        else:
            self._set_status("Failed to apply")
            self.notify("Failed to apply display config", severity="error")

    # ── Actions (keybindings) ──

    def action_connect(self) -> None:
        if not self.selected_node:
            self.notify("Select a node first", severity="warning")
            return
        self._do_connect(self.selected_node)

    @work(thread=True)
    def _do_connect(self, target: str) -> None:
        self.notify(f"Connecting to {target}...")
        result = api_post(self.config, "/api/connect", {"node": target, "app": "Desktop"})
        if result and result.get("connected"):
            self.notify(f"Connected to {target}")
            self._set_status(f"Streaming: {target}")
        else:
            self.notify(f"Failed to connect to {target}", severity="error")

    def action_disconnect(self) -> None:
        self._do_disconnect()

    @work(thread=True)
    def _do_disconnect(self) -> None:
        result = api_post(self.config, "/api/disconnect")
        if result and result.get("disconnected"):
            self.notify("Disconnected")
            self._set_status("Ready")

    def action_loop(self) -> None:
        if not self.selected_node:
            self.notify("Select a node first", severity="warning")
            return
        self._do_loop(self.selected_node)

    @work(thread=True)
    def _do_loop(self, target: str) -> None:
        self.notify(f"Starting loop with {target}...")
        result = api_post(self.config, "/api/loop", {"node": target, "app": "Desktop"})
        if result and result.get("looping"):
            self.notify(f"Loop active with {target}")
            self._set_status(f"Loop: {target}")
        else:
            self.notify(f"Loop failed", severity="error")

    def action_pair(self) -> None:
        if not self.selected_node:
            self.notify("Select a node first", severity="warning")
            return
        self._do_pair(self.selected_node)

    @work(thread=True)
    def _do_pair(self, target: str) -> None:
        self.notify(f"Pairing with {target}... (up to 30s)")
        result = api_post(self.config, "/api/pair", {"node": target})
        if result and result.get("paired"):
            self.notify(f"Paired with {target}")
        else:
            self.notify(f"Pairing failed", severity="error")

    def action_refresh(self) -> None:
        self.refresh_data()
        self.notify("Refreshing...")

    def _set_status(self, msg: str) -> None:
        try:
            self.query_one("#status-line", Static).update(msg)
        except Exception:
            pass
