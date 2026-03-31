"""Orrbeam popup — lightweight floating overlay for stream control.

Designed to appear on the HOST desktop during a Moonlight session.
The Moonlight client sees it through the stream and can interact
via mouse/keyboard passthrough.

Launch: orrbeam popup
Bind to a hotkey in your DE (e.g. Super+O in KDE).
"""

import tkinter as tk
from tkinter import ttk

from .api_client import api_get, api_post
from .config import Config


class OrrbeamPopup:
    def __init__(self, config: Config) -> None:
        self.config = config
        self.monitors: list[dict] = []
        self.nodes: list[dict] = []
        self.current_output = ""

        self.root = tk.Tk()
        self.root.title("orrbeam")
        self.root.attributes("-topmost", True)
        self.root.overrideredirect(False)
        self.root.resizable(False, False)

        # Dark theme
        self.bg = "#1e1e2e"
        self.fg = "#cdd6f4"
        self.accent = "#89b4fa"
        self.btn_bg = "#313244"
        self.btn_active = "#45475a"
        self.green = "#a6e3a1"
        self.red = "#f38ba8"
        self.dim = "#6c7086"

        self.root.configure(bg=self.bg)

        # Style
        style = ttk.Style()
        style.theme_use("clam")
        style.configure("Dark.TFrame", background=self.bg)
        style.configure("Dark.TLabel", background=self.bg, foreground=self.fg, font=("sans-serif", 11))
        style.configure("Title.TLabel", background=self.bg, foreground=self.accent, font=("sans-serif", 13, "bold"))
        style.configure("Dim.TLabel", background=self.bg, foreground=self.dim, font=("sans-serif", 10))
        style.configure("Dark.TButton", background=self.btn_bg, foreground=self.fg,
                         font=("sans-serif", 11), padding=(12, 6))
        style.map("Dark.TButton",
                  background=[("active", self.btn_active), ("pressed", self.accent)],
                  foreground=[("active", self.fg)])
        style.configure("Apply.TButton", background=self.accent, foreground="#1e1e2e",
                         font=("sans-serif", 11, "bold"), padding=(16, 8))
        style.map("Apply.TButton",
                  background=[("active", "#74c7ec"), ("pressed", "#74c7ec")])
        style.configure("Dark.TRadiobutton", background=self.bg, foreground=self.fg,
                         font=("sans-serif", 11), indicatorcolor=self.btn_bg)
        style.map("Dark.TRadiobutton",
                  indicatorcolor=[("selected", self.accent)],
                  background=[("active", self.bg)])

        self._build_ui()
        self._load_data()

        # Center on screen
        self.root.update_idletasks()
        w = self.root.winfo_width()
        h = self.root.winfo_height()
        sw = self.root.winfo_screenwidth()
        sh = self.root.winfo_screenheight()
        self.root.geometry(f"+{(sw - w) // 2}+{(sh - h) // 2}")

        # Escape to close
        self.root.bind("<Escape>", lambda e: self.root.destroy())

    def _build_ui(self) -> None:
        frame = ttk.Frame(self.root, style="Dark.TFrame", padding=16)
        frame.pack(fill="both", expand=True)

        # Title
        ttk.Label(frame, text="orrbeam", style="Title.TLabel").pack(anchor="w")
        ttk.Label(frame, text="stream control", style="Dim.TLabel").pack(anchor="w", pady=(0, 12))

        # Monitor section
        ttk.Label(frame, text="Monitor", style="Dark.TLabel").pack(anchor="w", pady=(0, 4))
        self.monitor_var = tk.StringVar()
        self.monitor_frame = ttk.Frame(frame, style="Dark.TFrame")
        self.monitor_frame.pack(fill="x", pady=(0, 12))

        # Rotation section
        ttk.Label(frame, text="Rotation", style="Dark.TLabel").pack(anchor="w", pady=(0, 4))
        self.rotation_var = tk.StringVar(value="normal")
        rot_frame = ttk.Frame(frame, style="Dark.TFrame")
        rot_frame.pack(fill="x", pady=(0, 12))
        for val, label in [("normal", "0\u00b0"), ("left", "90\u00b0"), ("inverted", "180\u00b0"), ("right", "270\u00b0")]:
            ttk.Radiobutton(rot_frame, text=label, variable=self.rotation_var,
                            value=val, style="Dark.TRadiobutton").pack(side="left", padx=(0, 12))

        # Status line
        self.status_var = tk.StringVar(value="")
        ttk.Label(frame, textvariable=self.status_var, style="Dim.TLabel").pack(anchor="w", pady=(0, 8))

        # Buttons
        btn_frame = ttk.Frame(frame, style="Dark.TFrame")
        btn_frame.pack(fill="x", pady=(4, 0))
        ttk.Button(btn_frame, text="Apply", style="Apply.TButton",
                   command=self._apply).pack(side="left", padx=(0, 8))
        ttk.Button(btn_frame, text="Close", style="Dark.TButton",
                   command=self.root.destroy).pack(side="right")

        # Node section (compact)
        ttk.Label(frame, text="Nodes", style="Dark.TLabel").pack(anchor="w", pady=(12, 4))
        self.node_frame = ttk.Frame(frame, style="Dark.TFrame")
        self.node_frame.pack(fill="x")

    def _load_data(self) -> None:
        # Monitors
        data = api_get(self.config, "/api/monitors")
        if data:
            self.monitors = data.get("monitors", [])
            self.current_output = data.get("current_output") or ""

        # Clear and rebuild monitor buttons
        for w in self.monitor_frame.winfo_children():
            w.destroy()

        for m in self.monitors:
            name = m["name"]
            res = f"{m['width']}x{m['height']}"
            rot = m.get("rotation", "normal")
            idx = m.get("index", 0)
            is_current = (str(idx) == str(self.current_output) or name == self.current_output)

            btn_text = f"{name}  {res}"
            if rot != "normal":
                btn_text += f"  ({rot})"

            btn = tk.Button(
                self.monitor_frame, text=btn_text,
                bg=self.accent if is_current else self.btn_bg,
                fg="#1e1e2e" if is_current else self.fg,
                activebackground=self.btn_active, activeforeground=self.fg,
                font=("sans-serif", 11), bd=0, padx=12, pady=6, cursor="hand2",
                command=lambda n=name: self._select_monitor(n),
            )
            btn.pack(side="left", padx=(0, 6), pady=2)

            if rot != "normal":
                self.rotation_var.set(rot)

        if self.current_output:
            self.monitor_var.set(self.current_output)
        elif self.monitors:
            self.monitor_var.set(self.monitors[0]["name"])

        # Nodes
        data = api_get(self.config, "/api/nodes")
        if data:
            self.nodes = data.get("nodes", [])

        for w in self.node_frame.winfo_children():
            w.destroy()

        indicators = {"online": "\u25cf", "offline": "\u25cb", "hosting": "\u25c6"}
        colors = {"online": self.green, "offline": self.red, "hosting": self.accent}
        for n in self.nodes:
            state = n.get("state", "offline")
            color = colors.get(state, self.dim)
            ind = indicators.get(state, "?")
            lbl = tk.Label(self.node_frame, text=f"{ind} {n['name']}",
                           bg=self.bg, fg=color, font=("sans-serif", 10))
            lbl.pack(side="left", padx=(0, 12))

    def _select_monitor(self, name: str) -> None:
        self.monitor_var.set(name)
        # Update button highlighting
        for w in self.monitor_frame.winfo_children():
            if isinstance(w, tk.Button):
                is_selected = name in w.cget("text").split()[0]
                w.configure(
                    bg=self.accent if is_selected else self.btn_bg,
                    fg="#1e1e2e" if is_selected else self.fg,
                )
        self.status_var.set(f"Selected: {name}")

    def _apply(self) -> None:
        output = self.monitor_var.get()
        rotation = self.rotation_var.get()
        if not output:
            self.status_var.set("Select a monitor first")
            return

        self.status_var.set(f"Applying {output} ({rotation})...")
        self.root.update()

        result = api_post(self.config, "/api/display", {
            "output_name": output,
            "rotation": rotation,
        })
        if result and result.get("applied"):
            self.status_var.set(f"Applied: {output} ({rotation})")
            # Close after short delay
            self.root.after(800, self.root.destroy)
        else:
            self.status_var.set("Failed to apply")

    def run(self) -> None:
        self.root.mainloop()


def main() -> None:
    config = Config.load()
    popup = OrrbeamPopup(config)
    popup.run()


if __name__ == "__main__":
    main()
