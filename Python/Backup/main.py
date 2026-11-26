# File: main.py
# Description: Main entry point, UI assembly, Event Loop, dynamic layout management, and Auto-Bootloader.

import sys
import os
import subprocess
import importlib.util

# --- BOOTLOADER: Dependency Check & Auto-Install ---
def check_and_install_dependencies():
    """
    Checks for all required third-party libraries and Npcap.
    If missing, automatically installs them and restarts the application.
    """
    packages = {
        "customtkinter": "customtkinter",
        "psutil": "psutil",
        "pystray": "pystray",
        "PIL": "Pillow",
        "winsdk": "winsdk",
        "scapy": "scapy"
    }
    
    needs_install = []
    for module, pip_name in packages.items():
        if importlib.util.find_spec(module) is None:
            needs_install.append(pip_name)
            
    # Check for Npcap (required for Scapy on Windows)
    has_npcap = os.path.exists(os.path.join(os.environ.get("SystemRoot", "C:\\Windows"), "System32", "wpcap.dll"))
    
    if needs_install or not has_npcap:
        print("\n" + "="*40)
        print("--- NETMONITOR BOOTLOADER ---")
        if needs_install: print(f"Missing libraries: {', '.join(needs_install)}")
        if not has_npcap: print("Missing dependency: Npcap (Network Driver)")
        print("Installing now. Please wait...")
        print("="*40 + "\n")
        
        for pkg in needs_install:
            print(f"-> Installing {pkg}...")
            try:
                subprocess.check_call([sys.executable, "-m", "pip", "install", pkg])
            except Exception as e:
                print(f"CRITICAL: Failed to install {pkg}. Error: {e}")
                
        if not has_npcap:
            print("-> Downloading and Installing Npcap...")
            try:
                # Download Npcap installer (v1.75 is a stable version)
                import urllib.request
                npcap_url = "https://nmap.org/npcap/dist/npcap-1.75.exe"
                installer_path = os.path.join(os.environ.get("TEMP", "."), "npcap_installer.exe")
                urllib.request.urlretrieve(npcap_url, installer_path)
                # Run silently (as much as possible)
                print("   Executing Npcap installer... (May require user confirmation)")
                subprocess.check_call([installer_path, "/S", "/dot11=0", "/loopback=0"])
                print("   Npcap installation triggered.")
            except Exception as e:
                print(f"CRITICAL: Failed to install Npcap. Error: {e}")
                print("Please download and install Npcap manually from https://npcap.com/")
                input("Press Enter to continue anyway (Scapy might fail)...")
                
        print("\nInitialization complete! Restarting application...\n")
        os.execl(sys.executable, sys.executable, *sys.argv)

# Run the bootloader BEFORE any 3rd-party imports happen
check_and_install_dependencies()

# --- STANDARD IMPORTS ---
import customtkinter as ctk
import tkinter as tk
import threading
import time
import psutil
import json
import traceback
import socket
import collections

from config import *
from utils import AdminHandler, get_work_area, log_error
from backend import HardwareBackend, HotspotDirect, PortScanner, TetheringOperationalState
from ui_components import TrayIconHandler, FuturisticGraph, DraggableModule

class HotspotWidget(ctk.CTk):
    def __init__(self):
        super().__init__()
        
        print("DEBUG: Initializing Main Window...")
        self.title("NetMonitor")
        self.overrideredirect(True)
        self.configure(fg_color=BG_COLOR)
        
        self.config = self.load_config()
        self.module_order = self.config.get("module_order", ["network", "cpu", "ram", "swap"])
        if "swap" not in self.module_order: self.module_order.append("swap")
        
        self.current_width = self.config["width"]
        self.attributes("-alpha", self.config["opacity"])
        self.attributes("-topmost", self.config["topmost"])
        
        self.hotspot_traffic_only_var = tk.BooleanVar(value=self.config.get("hotspot_traffic_only", False))

        self.last_net_io = psutil.net_io_counters()
        self.hotspot_status = 0
        self.running = True 
        self.dragging_module = None
        
        self.max_down = 0; self.max_up = 0
        self.min_down = None; self.min_up = None
        self.total_down = 0; self.total_up = 0
        self.tick_count = 0
        
        self.proc_cache = {}
        self.cpu_name = "Loading..."
        HardwareBackend.get_cpu_name(self.set_cpu_name) 
        
        self.clients_widgets = {} 
        self.top_cpu = "-"
        self.top_ram = "-"
        
        self.tray = TrayIconHandler(self.safe_destroy, self.restore_window)
        threading.Thread(target=self.tray.run, daemon=True).start()

        self.setup_ui()
        self.setup_bindings()
        self.after(100, self.apply_saved_position)
        
        self.monitor_system()
        threading.Thread(target=self.background_status_check, daemon=True).start()

    # --- METHODS ---
    def safe_destroy(self):
        self.running = False
        self.tray.stop()
        try: self.destroy()
        except: pass

    def restore_window(self):
        self.deiconify()
        self.lift()
        self.focus_force()
    
    def set_cpu_name(self, name):
        self.cpu_name = name

    def load_config(self):
        try:
            with open(CONFIG_FILE, 'r') as f: return {**DEFAULT_CONFIG, **json.load(f)}
        except: return DEFAULT_CONFIG

    def save_config(self):
        self.config["x"] = self.winfo_x()
        self.config["y"] = self.winfo_y()
        self.config["width"] = self.current_width
        self.config["topmost"] = self.topmost_var.get()
        self.config["show_network"] = self.show_net_var.get()
        self.config["show_cpu"] = self.show_cpu_var.get()
        self.config["show_ram"] = self.show_ram_var.get()
        self.config["show_swap"] = self.show_swap_var.get()
        self.config["show_clients"] = self.show_clients_var.get()
        self.config["module_order"] = self.module_order
        self.config["ram_graph_style"] = self.ram_graph.style
        self.config["swap_graph_style"] = self.swap_graph.style
        self.config["hotspot_traffic_only"] = self.hotspot_traffic_only_var.get()
        try:
            with open(CONFIG_FILE, 'w') as f: json.dump(self.config, f)
        except: pass

    def show_context_menu(self, event, target_module=None):
        menu = tk.Menu(self, tearoff=0, bg="#2b2b2b", fg=ACCENT_CYAN, font=("Consolas", 10))
        if target_module:
            pretty_name = target_module.upper()
            menu.add_command(label=f"Unlock & Move {pretty_name}", 
                             command=lambda: self.modules[target_module].enable_move_mode(),
                             background="#443300", foreground=ACCENT_YELLOW)
            if target_module == "network":
                menu.add_separator()
                menu.add_checkbutton(label="Monitor Hotspot Traffic Only", variable=self.hotspot_traffic_only_var, command=self.on_hotspot_traffic_toggle)
            if target_module == "ram":
                menu.add_separator()
                menu.add_command(label="Switch to Block Graph", command=lambda: self.set_ram_style("block"))
                menu.add_command(label="Switch to Line Graph", command=lambda: self.set_ram_style("line"))
            if target_module == "swap":
                menu.add_separator()
                menu.add_command(label="Switch to Block Graph", command=lambda: self.set_swap_style("block"))
                menu.add_command(label="Switch to Line Graph", command=lambda: self.set_swap_style("line"))
            menu.add_separator()

        self.topmost_var.set(self.attributes("-topmost"))
        menu.add_checkbutton(label="Always On Top", variable=self.topmost_var, command=self.toggle_topmost)
        menu.add_separator()
        
        size_menu = tk.Menu(menu, tearoff=0, bg="#2b2b2b", fg=ACCENT_CYAN)
        size_menu.add_command(label="Compact Width (300px)", command=lambda: self.set_width(300))
        size_menu.add_command(label="Normal Width (350px)", command=lambda: self.set_width(350))
        size_menu.add_command(label="Wide Width (500px)", command=lambda: self.set_width(500))
        menu.add_cascade(label="Window Width", menu=size_menu)

        graph_menu = tk.Menu(menu, tearoff=0, bg="#2b2b2b", fg=ACCENT_CYAN)
        graph_menu.add_checkbutton(label="Network", variable=self.show_net_var, command=self.repack_ui)
        graph_menu.add_checkbutton(label="Connected Devices", variable=self.show_clients_var, command=self.repack_ui)
        graph_menu.add_checkbutton(label="CPU Usage", variable=self.show_cpu_var, command=self.repack_ui)
        graph_menu.add_checkbutton(label="Memory Usage", variable=self.show_ram_var, command=self.repack_ui)
        graph_menu.add_checkbutton(label="Swap Usage", variable=self.show_swap_var, command=self.repack_ui)
        menu.add_cascade(label="Toggle Graphs", menu=graph_menu)
        
        opacity_menu = tk.Menu(menu, tearoff=0, bg="#2b2b2b", fg=ACCENT_CYAN)
        for val, label in [(1.0, "100%"), (0.9, "90%"), (0.75, "75%"), (0.5, "50%")]:
            opacity_menu.add_command(label=label, command=lambda v=val: self.set_opacity(v))
        menu.add_cascade(label="Transparency", menu=opacity_menu)

        menu.add_separator()
        menu.add_command(label="Reset Stats", command=self.reset_stats)
        menu.add_command(label="Exit", command=self.safe_destroy)

        try:
            if event: menu.tk_popup(event.x_root, event.y_root)
            else: menu.tk_popup(self.winfo_rootx() + 50, self.winfo_rooty() + 50)
        except: pass

    def show_client_menu(self, event, ip_str, label_widget):
        if "Scanning" in ip_str or "No Devices" in ip_str or "Inactive" in ip_str: return
        
        menu = tk.Menu(self, tearoff=0, bg="#2b2b2b", fg=ACCENT_CYAN, font=("Consolas", 10))
        menu.add_command(label=f"Scan Open Ports", command=lambda: self.run_nmap_scan(ip_str, label_widget))
        try: menu.tk_popup(event.x_root, event.y_root)
        except: pass

    def run_nmap_scan(self, ip_str, label_widget):
        label_widget.configure(text=f"{ip_str} [Scanning Ports...]")
        PortScanner.scan_device(ip_str, lambda i, res: self.update_client_label(label_widget, i, res))

    def update_client_label(self, label, ip, result):
        if label.winfo_exists(): label.configure(text=f"{ip} {result}")

    def update_clients_ui(self, clients):
        # clients is now a list of dicts: {"ip":, "mac":, "hostname":, "down_bps":, "up_bps":, "total":}
        
        current_ids = []
        status_msg = None
        
        if clients and isinstance(clients[0], str):
            status_msg = clients[0]
            current_ids = [status_msg]
        else:
            current_ids = [c["mac"] for c in clients if c.get("mac")]

        known_ids = set(self.clients_widgets.keys())
        
        # Remove old widgets
        for cid in known_ids - set(current_ids):
            if self.clients_widgets[cid].winfo_exists():
                self.clients_widgets[cid].destroy()
            del self.clients_widgets[cid]
            
        # Add/Update widgets
        if status_msg:
             if status_msg not in self.clients_widgets:
                 lbl = ctk.CTkLabel(self.clients_frame, text=status_msg, text_color="#555", font=("Consolas", 10, "italic"))
                 lbl.pack(pady=5, fill="x")
                 self.clients_widgets[status_msg] = lbl
        else:
            for c in clients:
                cid = c["mac"]
                ip = c["ip"]
                host = c["hostname"]
                # Truncate hostname to prevent clipping
                if len(host) > 18: host = host[:15] + "..."
                
                # Format usage
                usage = f"{self.format_bytes(c.get('down_bps', 0))} \u2193 {self.format_bytes(c.get('up_bps', 0))} \u2191"
                
                # Multi-line text with explicit padding
                text = f"{ip} ({cid})\n{host} | {usage}"
                
                if cid not in self.clients_widgets:
                    lbl = ctk.CTkLabel(self.clients_frame, text=text, text_color=ACCENT_CYAN, 
                                       font=("Consolas", 10), anchor="w", justify="left")
                    lbl.pack(fill="x", padx=15, pady=4)
                    lbl.bind("<Button-3>", lambda e, i=ip, l=lbl: self.show_client_menu(e, i, l))
                    self.clients_widgets[cid] = lbl
                else:
                    self.clients_widgets[cid].configure(text=text)
        
        if len(self.clients_widgets) == 0 and self.show_clients_var.get():
             if not hasattr(self, 'lbl_searching'):
                 self.lbl_searching = ctk.CTkLabel(self.clients_frame, text="Searching...", text_color="#555", font=("Consolas", 10, "italic"))
                 self.lbl_searching.pack(pady=5, fill="x")
        elif hasattr(self, 'lbl_searching'):
             self.lbl_searching.destroy()
             del self.lbl_searching

        if len(known_ids) != len(current_ids):
            self.repack_ui()

    def set_ram_style(self, style):
        self.ram_graph.set_style(style)
        self.save_config()

    def set_swap_style(self, style):
        self.swap_graph.set_style(style)
        self.save_config()

    def reset_stats(self):
        self.max_down = 0; self.max_up = 0
        self.min_down = None; self.min_up = None
        self.total_down = 0; self.total_up = 0
        # Prevent initial spike logic after reset
        self.last_net_io = psutil.net_io_counters() 

    def on_hotspot_traffic_toggle(self):
        self.save_config()
        self.reset_stats()
        # Establish a new baseline for the network traffic tracking
        if self.hotspot_traffic_only_var.get():
            iface = self.get_hotspot_interface()
            if iface:
                self.last_net_io = psutil.net_io_counters(pernic=True).get(iface, collections.namedtuple('snetio', ['bytes_recv', 'bytes_sent'])(0, 0))
            else:
                self.last_net_io = collections.namedtuple('snetio', ['bytes_recv', 'bytes_sent'])(0, 0)
        else:
            self.last_net_io = psutil.net_io_counters()

    def toggle_topmost(self):
        self.attributes("-topmost", self.topmost_var.get())
        self.save_config()

    def set_opacity(self, val):
        self.attributes("-alpha", val)
        self.config["opacity"] = val
        self.save_config()

    def set_width(self, w):
        self.current_width = w
        self.repack_ui()

    def apply_saved_position(self):
        left, top, right, bottom = get_work_area()
        if self.config["x"] == -1: 
            # Place it in the lower right by default
            x = right - self.current_width - 10
            y = bottom - 450 # Initial guess, repack_ui will refine this
            self.geometry(f"+{x}+{y}")
        else: 
            x, y = self.config["x"], self.config["y"]
            self.geometry(f"+{x}+{y}")
        
        # Initial pack to set correct height and flush with bottom
        self.repack_ui() 

    def check_module_swap(self, active_name, y_root):
        try: curr_idx = self.module_order.index(active_name)
        except: return
        if curr_idx > 0:
            neighbor = self.module_order[curr_idx - 1]
            if neighbor in self.modules and self.modules[neighbor].winfo_ismapped():
                nb_y = self.modules[neighbor].winfo_rooty() + (self.modules[neighbor].winfo_height() // 2)
                if y_root < nb_y:
                    self.module_order[curr_idx], self.module_order[curr_idx-1] = self.module_order[curr_idx-1], self.module_order[curr_idx]
                    self.repack_ui()
                    return
        if curr_idx < len(self.module_order) - 1:
            neighbor = self.module_order[curr_idx + 1]
            if neighbor in self.modules and self.modules[neighbor].winfo_ismapped():
                nb_y = self.modules[neighbor].winfo_rooty() + (self.modules[neighbor].winfo_height() // 2)
                if y_root > nb_y:
                    self.module_order[curr_idx], self.module_order[curr_idx+1] = self.module_order[curr_idx+1], self.module_order[curr_idx]
                    self.repack_ui()
                    return

    def repack_ui(self):
        # Prevent jumping by anchoring to the bottom edge if the app is already drawn
        current_bottom = None
        if self.winfo_ismapped():
            current_bottom = self.winfo_y() + self.winfo_height()

        self.controls_frame.pack_forget() 
        for m in self.modules.values(): m.pack_forget()

        self.controls_frame.pack(fill="x", pady=(10, 5), side="bottom", anchor="s")

        base_h = 45 
        graph_h = 130 
        footer_h = 90 if hasattr(self, 'btn_autostart') else 60

        total_h = base_h + footer_h
        
        for mod_name in self.module_order:
            if mod_name == "network" and self.show_net_var.get():
                self.modules["network"].pack(fill="x", pady=2)
                net_base_h = 145
                clients_h = 0
                
                if self.show_clients_var.get():
                    self.client_sep.pack(fill="x", padx=15, pady=(8,4))
                    self.lbl_clients_header.pack(fill="x", padx=15)
                    self.clients_frame.pack(fill="x", pady=2)
                    c_count = len(self.clients_widgets)
                    if c_count == 0 and hasattr(self, 'lbl_searching'): c_count = 1
                    
                    # Target height: Header (40) + each client label (~55px for 2 lines + padding)
                    clients_h = 45 + (c_count * 55)
                else:
                    self.client_sep.pack_forget()
                    self.lbl_clients_header.pack_forget()
                    self.clients_frame.pack_forget()
                
                total_h += (net_base_h + clients_h)
                
            elif mod_name == "cpu" and self.show_cpu_var.get():
                self.modules["cpu"].pack(fill="x", pady=2)
                total_h += graph_h
            elif mod_name == "ram" and self.show_ram_var.get():
                self.modules["ram"].pack(fill="x", pady=2)
                total_h += graph_h
            elif mod_name == "swap" and self.show_swap_var.get():
                self.modules["swap"].pack(fill="x", pady=2)
                total_h += graph_h

        # Add global padding buffer to prevent vertical clipping
        total_h += 15

        left, top, right, bottom = get_work_area()
        new_x = self.winfo_x()
        
        # Determine Y placement (Bottom anchored if previously placed, else default to lower right)
        if current_bottom and current_bottom > top:
            new_y = current_bottom - total_h
        else:
            # Default to flush with taskbar (bottom)
            new_y = bottom - total_h
            new_x = right - self.current_width - 10

        # Clamping to Work Area (Excluding Taskbar)
        if new_y < top: new_y = top
        if new_x < left: new_x = left
        if new_x + self.current_width > right: new_x = right - self.current_width
        if new_y + total_h > bottom: new_y = bottom - total_h

        self.geometry(f"{self.current_width}x{total_h}+{new_x}+{new_y}")
        self.save_config()

    def setup_ui(self):
        self.topmost_var = tk.BooleanVar(value=self.config["topmost"])
        self.show_net_var = tk.BooleanVar(value=self.config["show_network"])
        self.show_cpu_var = tk.BooleanVar(value=self.config["show_cpu"])
        self.show_ram_var = tk.BooleanVar(value=self.config["show_ram"])
        self.show_swap_var = tk.BooleanVar(value=self.config.get("show_swap", False))
        self.show_clients_var = tk.BooleanVar(value=self.config.get("show_clients", True))

        self.header = ctk.CTkFrame(self, height=30, fg_color="#2b2b2b", corner_radius=0)
        self.header.pack(fill="x")
        ctk.CTkLabel(self.header, text="CORE :: SYSTEM", text_color=ACCENT_CYAN, font=("Consolas", 12, "bold")).pack(side="left", padx=10)
        
        # Admin indicator
        is_admin = AdminHandler.is_admin()
        admin_col = ACCENT_CYAN if is_admin else ACCENT_RED
        admin_txt = "ROOT" if is_admin else "USER"
        self.lbl_admin = ctk.CTkLabel(self.header, text=admin_txt, text_color=admin_col, font=("Consolas", 10, "bold"))
        self.lbl_admin.pack(side="right", padx=10)

        self.modules = {}

        # 1. NETWORK
        self.net_frame = DraggableModule(self, "network", fg_color="transparent")
        self.net_stats_grid = ctk.CTkFrame(self.net_frame, fg_color="transparent")
        self.net_stats_grid.pack(fill="x", padx=10)
        self.net_stats_grid.grid_columnconfigure(0, weight=1)
        self.net_stats_grid.grid_columnconfigure(1, weight=1)
        
        self.lbl_net_down = ctk.CTkLabel(self.net_stats_grid, text="DL: 0 B/s", text_color=ACCENT_CYAN, font=("Consolas", 11, "bold"), anchor="w")
        self.lbl_net_down.grid(row=0, column=0, sticky="w")
        self.lbl_net_up = ctk.CTkLabel(self.net_stats_grid, text="UL: 0 B/s", text_color=ACCENT_RED, font=("Consolas", 11, "bold"), anchor="e")
        self.lbl_net_up.grid(row=0, column=1, sticky="e")
        
        self.lbl_max_d = ctk.CTkLabel(self.net_stats_grid, text="Mx: 0 B/s", text_color="#008f72", font=("Consolas", 9), anchor="w")
        self.lbl_max_d.grid(row=1, column=0, sticky="w")
        self.lbl_max_u = ctk.CTkLabel(self.net_stats_grid, text="Mx: 0 B/s", text_color="#8f0030", font=("Consolas", 9), anchor="e")
        self.lbl_max_u.grid(row=1, column=1, sticky="e")

        self.lbl_tot_d = ctk.CTkLabel(self.net_stats_grid, text="Tot: 0 B", text_color="gray", font=("Consolas", 9), anchor="w")
        self.lbl_tot_d.grid(row=2, column=0, sticky="w")
        self.lbl_tot_u = ctk.CTkLabel(self.net_stats_grid, text="Tot: 0 B", text_color="gray", font=("Consolas", 9), anchor="e")
        self.lbl_tot_u.grid(row=2, column=1, sticky="e")

        self.net_graph = FuturisticGraph(self.net_frame, line_colors=[ACCENT_CYAN, ACCENT_RED], max_val=None, fg_color="transparent")
        self.net_graph.pack(fill="both", expand=True, padx=10, pady=2)
        
        self.client_sep = ctk.CTkFrame(self.net_frame, height=1, fg_color="#333")
        self.lbl_clients_header = ctk.CTkLabel(self.net_frame, text="CONNECTED DEVICES:", text_color="#888", font=("Consolas", 9, "bold"), anchor="w")
        
        # Standard frame for absolute visibility
        self.clients_frame = ctk.CTkFrame(self.net_frame, fg_color="transparent")
        
        for w in [self.net_stats_grid, self.lbl_net_down, self.lbl_net_up, self.lbl_max_d, self.lbl_max_u, self.lbl_tot_d, self.lbl_tot_u, self.clients_frame, self.lbl_clients_header]:
             w.bind("<ButtonPress-1>", self.net_frame.on_left_down)
             w.bind("<B1-Motion>", self.net_frame.on_left_drag)
             w.bind("<ButtonRelease-1>", self.net_frame.on_left_up)
             w.bind("<Button-3>", self.net_frame.on_right_click)
        self.modules["network"] = self.net_frame

        # 2. CPU
        self.cpu_frame = DraggableModule(self, "cpu", fg_color="transparent")
        self.lbl_cpu_stats = ctk.CTkLabel(self.cpu_frame, text="CPU Init...", text_color=ACCENT_YELLOW, font=("Consolas", 10), anchor="w")
        self.lbl_cpu_stats.pack(fill="x", padx=10)
        self.cpu_graph = FuturisticGraph(self.cpu_frame, line_colors=[ACCENT_YELLOW], max_val=100, fg_color="transparent")
        self.cpu_graph.pack(fill="both", expand=True, padx=10, pady=2)
        
        self.lbl_cpu_stats.bind("<ButtonPress-1>", self.cpu_frame.on_left_down)
        self.lbl_cpu_stats.bind("<B1-Motion>", self.cpu_frame.on_left_drag)
        self.lbl_cpu_stats.bind("<ButtonRelease-1>", self.cpu_frame.on_left_up)
        self.lbl_cpu_stats.bind("<Button-3>", self.cpu_frame.on_right_click)
        self.modules["cpu"] = self.cpu_frame

        # 3. RAM
        self.ram_frame = DraggableModule(self, "ram", fg_color="transparent")
        self.lbl_ram_stats = ctk.CTkLabel(self.ram_frame, text="RAM Init...", text_color=ACCENT_PURPLE, font=("Consolas", 10), anchor="w")
        self.lbl_ram_stats.pack(fill="x", padx=10)
        
        ram_style = self.config.get("ram_graph_style", "line")
        self.ram_graph = FuturisticGraph(self.ram_frame, line_colors=[ACCENT_PURPLE], max_val=100, style=ram_style, fg_color="transparent")
        self.ram_graph.pack(fill="both", expand=True, padx=10, pady=2)
        
        self.lbl_ram_stats.bind("<ButtonPress-1>", self.ram_frame.on_left_down)
        self.lbl_ram_stats.bind("<B1-Motion>", self.ram_frame.on_left_drag)
        self.lbl_ram_stats.bind("<ButtonRelease-1>", self.ram_frame.on_left_up)
        self.lbl_ram_stats.bind("<Button-3>", self.ram_frame.on_right_click)
        self.modules["ram"] = self.ram_frame

        # 4. SWAP
        self.swap_frame = DraggableModule(self, "swap", fg_color="transparent")
        self.lbl_swap_stats = ctk.CTkLabel(self.swap_frame, text="SWAP Init...", text_color=ACCENT_ORANGE, font=("Consolas", 10), anchor="w")
        self.lbl_swap_stats.pack(fill="x", padx=10)
        
        swap_style = self.config.get("swap_graph_style", "line")
        self.swap_graph = FuturisticGraph(self.swap_frame, line_colors=[ACCENT_ORANGE], max_val=100, style=swap_style, fg_color="transparent")
        self.swap_graph.pack(fill="both", expand=True, padx=10, pady=2)
        
        self.lbl_swap_stats.bind("<ButtonPress-1>", self.swap_frame.on_left_down)
        self.lbl_swap_stats.bind("<B1-Motion>", self.swap_frame.on_left_drag)
        self.lbl_swap_stats.bind("<ButtonRelease-1>", self.swap_frame.on_left_up)
        self.lbl_swap_stats.bind("<Button-3>", self.swap_frame.on_right_click)
        self.modules["swap"] = self.swap_frame

        # CONTROLS
        self.controls_frame = ctk.CTkFrame(self, fg_color="transparent")
        ctk.CTkFrame(self.controls_frame, height=2, fg_color="#333").pack(fill="x", padx=5, pady=5)
        
        self.btn_hotspot = ctk.CTkButton(self.controls_frame, text="INIT...", fg_color="#222", 
                                         border_color="#444", border_width=1, hover_color="#333", 
                                         text_color="gray", font=("Consolas", 11, "bold"), height=30,
                                         command=self.toggle_hotspot)
        self.btn_hotspot.pack(fill="x", padx=10, pady=5)
        
        if not AdminHandler.check_task_exists():
            self.btn_autostart = ctk.CTkButton(self.controls_frame, text="ENABLE AUTO-START", 
                                               fg_color="transparent", border_color="#333", 
                                               border_width=1, text_color="#888", 
                                               font=("Consolas", 9), height=20,
                                               command=self.enable_autostart)
            self.btn_autostart.pack(fill="x", padx=10, pady=(0, 5))

    def setup_bindings(self):
        self.header.bind("<ButtonPress-1>", self.start_move)
        self.header.bind("<ButtonRelease-1>", self.stop_move)
        self.header.bind("<B1-Motion>", self.do_move)
        self.header.bind("<Button-3>", lambda e: self.show_context_menu(e))
        self.bind_all("<ButtonRelease-1>", self.on_global_release)

    def on_global_release(self, event):
        if self.dragging_module:
            if self.dragging_module in self.modules:
                self.modules[self.dragging_module].disable_move_mode()
            self.dragging_module = None
        self.stop_move(event)

    def start_move(self, event): self.x, self.y = event.x, event.y
    def stop_move(self, event): 
        self.x, self.y = None, None
        self.save_config()

    def do_move(self, event):
        try:
            deltax = event.x - self.x
            deltay = event.y - self.y
            new_x = self.winfo_x() + deltax
            new_y = self.winfo_y() + deltay
            
            # --- SNAP TO CORNERS LOGIC ---
            left, top, right, bottom = get_work_area()
            snap_dist = 20
            w = self.winfo_width()
            h = self.winfo_height()
            
            # Snap X
            if abs(new_x - left) < snap_dist: new_x = left
            elif abs(new_x + w - right) < snap_dist: new_x = right - w
            
            # Snap Y
            if abs(new_y - top) < snap_dist: new_y = top
            elif abs(new_y + h - bottom) < snap_dist: new_y = bottom - h
            
            # Clamping (Prevent clipping off screen)
            if new_x < left: new_x = left
            if new_x + w > right: new_x = right - w
            if new_y < top: new_y = top
            if new_y + h > bottom: new_y = bottom - h

            self.geometry(f"+{new_x}+{new_y}")
        except: pass

    # --- LOGIC ---
    def safe_destroy(self):
        self.running = False
        self.tray.stop()
        try: self.destroy()
        except: pass

    def enable_autostart(self):
        AdminHandler.create_startup_task()
        try: self.btn_autostart.configure(text="AUTO-START ENABLED", state="disabled", text_color=ACCENT_CYAN)
        except: pass

    def toggle_hotspot(self):
        if not self.running: return
        try: self.btn_hotspot.configure(state="disabled", text="PROCESSING...", text_color="gray")
        except: return 
        threading.Thread(target=lambda: HotspotDirect.toggle_sync(), daemon=True).start()
        self.after(2000, lambda: self.background_status_check(loop=False))

    def background_status_check(self, loop=True):
        while self.running:
            status = HotspotDirect.get_status()
            if self.running: self.after(0, lambda s=status: self.update_hotspot_ui(s))
            if not loop: break
            time.sleep(3)

    def update_hotspot_ui(self, status):
        try:
            if not self.running: return
            self.hotspot_status = status
            if status == TetheringOperationalState.ON:
                self.btn_hotspot.configure(text="HOTSPOT: ACTIVE", fg_color=ACCENT_CYAN, border_color=ACCENT_CYAN, state="normal", text_color="black")
                self.tray.update_stats(0, True)
            elif status == TetheringOperationalState.OFF:
                self.btn_hotspot.configure(text="HOTSPOT: INACTIVE", fg_color="transparent", border_color=ACCENT_CYAN, state="normal", text_color=ACCENT_CYAN)
                self.tray.update_stats(0, False)
            else:
                self.btn_hotspot.configure(text="NO CONNECTION", fg_color="#333", border_color="#444", state="normal", text_color="gray")
        except: pass

    def show_error(self, msg):
        try:
            self.btn_hotspot.configure(text=f"FAIL: {msg[:15]}", fg_color="#330000", border_color=ACCENT_RED, text_color=ACCENT_RED, state="normal")
            self.after(3000, lambda: self.background_status_check(loop=False))
        except: pass

    def format_bytes(self, size, is_rate=True):
        power, n = 2**10, 0
        labels = {0:'', 1:'K', 2:'M', 3:'G', 4:'T'}
        while size > power: size /= power; n += 1
        suffix = "B/s" if is_rate else "B"
        return f"{size:.1f} {labels.get(n, 'T')}{suffix}"

    def find_top_processes(self):
        max_cpu = -1
        max_ram = -1
        cpu_name = "-"
        ram_name = "-"
        
        try:
            for p in psutil.process_iter(['pid', 'name']):
                try:
                    with p.oneshot():
                        if p.name() == "System Idle Process": continue
                        
                        c_pct = p.cpu_percent()
                        if c_pct > max_cpu:
                            max_cpu = c_pct
                            cpu_name = p.name()
                        
                        r_bytes = p.memory_info().rss
                        if r_bytes > max_ram:
                            max_ram = r_bytes
                            ram_name = p.name()
                except: pass
        except: pass
        
        return cpu_name, ram_name

    def get_hotspot_interface(self):
        try:
            addrs = psutil.net_if_addrs()
            for name, snics in addrs.items():
                for snic in snics:
                    if snic.family == socket.AF_INET:
                        if snic.address == "192.168.137.1":
                            return name
                        if ("Local Area Connection" in name or "Microsoft" in name) and "*" in name:
                             if not snic.address.startswith("169.254"):
                                 return name
        except: pass
        return None

    def monitor_system(self):
        try:
            if not self.running: return
            
            # Force status sync for reliable client discovery
            self.hotspot_status = HotspotDirect.get_status()

            # --- Network ---
            if self.show_net_var.get():
                is_hotspot_only = self.hotspot_traffic_only_var.get()
                c_io_current = None
                
                if is_hotspot_only:
                    iface = self.get_hotspot_interface()
                    if iface:
                        c_io_current = psutil.net_io_counters(pernic=True).get(iface)
                    if not c_io_current:
                        c_io_current = collections.namedtuple('snetio', ['bytes_recv', 'bytes_sent'])(0, 0)
                else:
                    c_io_current = psutil.net_io_counters()

                # Calculate traffic since last tick (safeguard against None/type mismatch)
                try:
                    ds = c_io_current.bytes_recv - self.last_net_io.bytes_recv
                    us = c_io_current.bytes_sent - self.last_net_io.bytes_sent
                except AttributeError:
                    ds, us = 0, 0

                if ds < 0 or us < 0:
                    ds, us = 0, 0
                
                # Accumulate the totals and max speeds correctly
                self.total_down += ds
                self.total_up += us
                if ds > self.max_down: self.max_down = ds
                if us > self.max_up: self.max_up = us

                self.last_net_io = c_io_current
                
                prefix = "HS " if is_hotspot_only else ""
                self.lbl_net_down.configure(text=f"{prefix}DL: {self.format_bytes(ds)}")
                self.lbl_net_up.configure(text=f"{prefix}UL: {self.format_bytes(us)}")
                self.lbl_max_d.configure(text=f"Mx: {self.format_bytes(self.max_down)}")
                self.lbl_max_u.configure(text=f"Mx: {self.format_bytes(self.max_up)}")
                self.lbl_tot_d.configure(text=f"Tot: {self.format_bytes(self.total_down, False)}")
                self.lbl_tot_u.configure(text=f"Tot: {self.format_bytes(self.total_up, False)}")

                self.net_graph.update_graph(ds, us)
                self.tray.update_stats(ds, self.hotspot_status == TetheringOperationalState.ON)
                
                # Check clients
                if self.hotspot_status == TetheringOperationalState.ON:
                    clients = HotspotDirect.get_clients()
                    self.update_clients_ui(clients)
                else:
                    self.update_clients_ui(["Hotspot Inactive"])
            else:
                # If network graph hidden, still track baseline so it doesn't spike when re-enabled
                is_hotspot_only = self.hotspot_traffic_only_var.get()
                if is_hotspot_only:
                    iface = self.get_hotspot_interface()
                    if iface:
                        self.last_net_io = psutil.net_io_counters(pernic=True).get(iface, collections.namedtuple('snetio', ['bytes_recv', 'bytes_sent'])(0, 0))
                    else:
                        self.last_net_io = collections.namedtuple('snetio', ['bytes_recv', 'bytes_sent'])(0, 0)
                else:
                    self.last_net_io = psutil.net_io_counters()

            # --- Process Scanning Throttling ---
            self.tick_count += 1
            if self.tick_count % 3 == 0:
                if self.show_cpu_var.get() or self.show_ram_var.get():
                    self.top_cpu, self.top_ram = self.find_top_processes()

            # --- CPU ---
            if self.show_cpu_var.get():
                try:
                    cpu = psutil.cpu_percent()
                    freq = psutil.cpu_freq().current / 1000.0 if psutil.cpu_freq() else 0.0
                    temp = HardwareBackend.get_cpu_temp()
                    cores = psutil.cpu_count(logical=False)
                    
                    brand = "CPU"
                    if "Intel" in self.cpu_name: brand = "Intel"
                    elif "AMD" in self.cpu_name: brand = "AMD"
                    
                    model = self.cpu_name.replace("Intel", "").replace("Core", "").replace("AMD", "").replace("Ryzen", "").replace("(R)", "").replace("(TM)", "").replace("CPU", "").replace("Processor", "").strip()
                    if "@" in model: model = model.split("@")[0].strip()
                    
                    if len(self.top_cpu) > 8: t_cpu = self.top_cpu[:8] + ".."
                    else: t_cpu = self.top_cpu
                    if len(model) > 10: model = model[:10]
                    
                    stats_str = f"{cpu}% {temp} {freq:.1f}GHz ({t_cpu}) {brand} {model} {cores}C"
                    self.lbl_cpu_stats.configure(text=stats_str)
                    self.cpu_graph.update_graph(cpu)
                except: pass
            
            # --- RAM ---
            if self.show_ram_var.get():
                try:
                    mem = psutil.virtual_memory()
                    ram_pct = mem.percent
                    used_gb = mem.used / (1024**3)
                    total_gb = mem.total / (1024**3)
                    
                    if len(self.top_ram) > 12: t_ram = self.top_ram[:12] + ".."
                    else: t_ram = self.top_ram
                    stats_str = f"{ram_pct}% {used_gb:.1f}/{total_gb:.1f}GB ({t_ram})"
                    self.lbl_ram_stats.configure(text=stats_str)
                    self.ram_graph.update_graph(ram_pct)
                except: pass
                
            # --- SWAP ---
            if self.show_swap_var.get():
                try:
                    swap = psutil.swap_memory()
                    swap_pct = swap.percent
                    used_swap = swap.used / (1024**3)
                    total_swap = swap.total / (1024**3)
                    
                    stats_str = f"SWAP: {swap_pct}%  {used_swap:.1f}GB / {total_swap:.1f}GB"
                    self.lbl_swap_stats.configure(text=stats_str)
                    self.swap_graph.update_graph(swap_pct)
                except: pass

            self.after(UPDATE_DELAY, self.monitor_system)
        except Exception as e:
            log_error(f"Monitor Error: {e}")
            self.after(UPDATE_DELAY, self.monitor_system)

if __name__ == "__main__":
    try:
        # NOTE: Admin checks happen here, ensuring dependencies 
        # were checked/installed by the top-level bootloader first.
        if not AdminHandler.is_admin():
            AdminHandler.elevate()
        else:
            ctk.set_appearance_mode("Dark")
            app = HotspotWidget()
            app.mainloop()
    except Exception as e:
        log_error(f"CRITICAL ERROR: {e}")
        traceback.print_exc()
        input("Press Enter to exit...")
