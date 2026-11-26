import os

# --- CONFIGURATION ---
UPDATE_DELAY = 1000  # ms
MAX_POINTS = 30 
ACCENT_CYAN = "#00ffcc"
ACCENT_RED = "#ff0055" 
ACCENT_YELLOW = "#ffcc00"
ACCENT_PURPLE = "#cc00ff"
ACCENT_ORANGE = "#ff8800"
ACCENT_GREEN = "#00ff88"
BG_COLOR = "#1a1a1a"
GRAPH_BG = "#0f0f0f"
TASK_NAME = "NetMonitorAutoStart"
CONFIG_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "widget_config.json")

DEFAULT_CONFIG = {
    "x": -1, "y": -1, "width": 420, 
    "opacity": 0.92, "topmost": False,
    "show_network": True,
    "show_cpu": False,
    "show_ram": False,
    "show_swap": False,
    "show_clients": True,
    "module_order": ["network", "cpu", "ram", "swap"],
    "ram_graph_style": "line",
    "swap_graph_style": "line",
    "fluid_graphs": True,
    "graph_interval_ms": 250,
    "selected_wifi_adapter": "",
    "blocked_domains": [],
    "blocked_ips": [],
    "blocker_enabled": True,
    "traffic_inspector_enabled": True
}