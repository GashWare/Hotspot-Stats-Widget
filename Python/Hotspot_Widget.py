import customtkinter as ctk
import psutil
import subprocess
import tkinter as tk
import ctypes
from ctypes import windll, byref, Structure, c_long
import sys
import os
import json
import asyncio
import threading
import time
import platform
import math
import re
import shutil
import socket
import traceback
from collections import deque

# --- LOGGING SETUP ---
def log_error(msg):
    try:
        with open("debug_log.txt", "a") as f:
            f.write(f"{time.strftime('%Y-%m-%d %H:%M:%S')} - {msg}\n")
    except: pass
    print(msg)

# --- STEALTH MODE ---
# Hides the console window if running as a .py script instead of .pyw
try:
    if os.name == 'nt':
        hwnd = ctypes.windll.kernel32.GetConsoleWindow()
        if hwnd != 0:
            # Uncomment next line to hide console in production
            # ctypes.windll.user32.ShowWindow(hwnd, 0)
            pass
except Exception as e:
    log_error(f"Stealth mode error: {e}")

# --- DEPENDENCY CHECKS ---
try:
    import pystray
    from PIL import Image, ImageDraw
except ImportError as e:
    log_error(f"Import Error: {e}")
    ctypes.windll.user32.MessageBoxW(0, "Please run: pip install pystray Pillow", "Missing Library", 1)
    sys.exit()

try:
    import winsdk.windows.networking.connectivity as network_info
    import winsdk.windows.networking.networkoperators as network_ops
    from winsdk.windows.networking.networkoperators import NetworkOperatorTetheringManager, TetheringOperationalState
except ImportError as e:
    log_error(f"Winsdk Error: {e}")
    pass 

# --- CONFIGURATION ---
UPDATE_DELAY = 1000 
MAX_POINTS = 30
ACCENT_CYAN = "#00ffcc"
ACCENT_RED = "#ff0055" 
ACCENT_YELLOW = "#ffcc00"
ACCENT_PURPLE = "#cc00ff"
ACCENT_ORANGE = "#ff8800"
BG_COLOR = "#1a1a1a"
GRAPH_BG = "#0f0f0f"
TASK_NAME = "NetMonitorAutoStart"
CONFIG_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "widget_config.json")

DEFAULT_CONFIG = {
    "x": -1, "y": -1, "width": 350, 
    "opacity": 0.92, "topmost": False,
    "show_network": True,
    "show_cpu": False,
    "show_ram": False,
    "show_swap": False,
    "show_clients": True,
    "module_order": ["network", "cpu", "ram", "swap"],
    "ram_graph_style": "line",
    "swap_graph_style": "line" # New config for Swap
}

try: ctypes.windll.shcore.SetProcessDpiAwareness(1)
except: pass 

class RECT(Structure):
    _fields_ = [("left", c_long), ("top", c_long), ("right", c_long), ("bottom", c_long)]

def get_work_area():
    rect = RECT()
    windll.user32.SystemParametersInfoW(48, 0, byref(rect), 0)
    return rect.left, rect.top, rect.right, rect.bottom

# --- ADMIN HANDLER ---
class AdminHandler:
    @staticmethod
    def is_admin():
        try: return ctypes.windll.shell32.IsUserAnAdmin()
        except: return False

    @staticmethod
    def elevate():
        print("DEBUG: Attempting to elevate privileges...")
        exe = sys.executable 
        script = f'"{os.path.abspath(__file__)}"'
        ctypes.windll.shell32.ShellExecuteW(None, "runas", exe, script, None, 1)
        sys.exit()

    @staticmethod
    def check_task_exists():
        startupinfo = subprocess.STARTUPINFO()
        startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        cmd = f'schtasks /query /tn "{TASK_NAME}"'
        result = subprocess.run(cmd, capture_output=True, shell=True, startupinfo=startupinfo)
        return result.returncode == 0

    @staticmethod
    def create_startup_task():
        exe = sys.executable.replace("python.exe", "pythonw.exe") if "python.exe" in sys.executable else sys.executable
        script = os.path.abspath(__file__)
        startupinfo = subprocess.STARTUPINFO()
        startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        subprocess.run(f'schtasks /delete /tn "{TASK_NAME}" /f', shell=True, startupinfo=startupinfo)
        cmd = (f'schtasks /create /tn "{TASK_NAME}" /tr "\'{exe}\' \'{script}\'" /sc onlogon /rl highest /f')
        subprocess.run(cmd, shell=True, startupinfo=startupinfo)

# --- BACKENDS ---
class HardwareBackend:
    _cpu_name_cache = None

    @staticmethod
    def get_cpu_name(callback):
        def _fetch():
            try:
                startupinfo = subprocess.STARTUPINFO()
                startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                result = subprocess.run(
                    ["powershell", "-Command", "Get-CimInstance Win32_Processor | Select-Object -ExpandProperty Name"],
                    capture_output=True, text=True, startupinfo=startupinfo
                )
                raw = result.stdout.strip()
                if raw:
                    clean = raw.replace("(R)", "").replace("(TM)", "").replace("CPU", "").replace("Processor", "")
                    if "@" in clean: clean = clean.split("@")[0]
                    if "Intel" in clean: clean = clean.replace("Core", "").replace("Intel", "").strip()
                    if "AMD" in clean: clean = clean.replace("Ryzen", "").replace("AMD", "").strip()
                    HardwareBackend._cpu_name_cache = " ".join(clean.split())
                    callback(HardwareBackend._cpu_name_cache)
                    return
            except Exception as e:
                print(f"DEBUG: CPU Name Error: {e}")
            callback(platform.processor())
        threading.Thread(target=_fetch, daemon=True).start()

    @staticmethod
    def get_cpu_temp():
        script = "Get-WmiObject MSAcpi_ThermalZoneTemperature -Namespace \"root/wmi\" | Select-Object -ExpandProperty CurrentTemperature"
        try:
            startupinfo = subprocess.STARTUPINFO()
            startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            result = subprocess.run(
                ["powershell", "-Command", script],
                capture_output=True, text=True, startupinfo=startupinfo
            )
            val = result.stdout.strip()
            if val and val.isdigit():
                temp_c = (int(val) - 2732) / 10.0
                return f"{temp_c:.0f}°C"
        except: pass
        return ""

class HotspotDirect:
    _clients_cache = []
    _last_scan_time = 0
    _is_scanning = False
    
    @staticmethod
    def get_manager():
        try:
            profile = network_info.NetworkInformation.get_internet_connection_profile()
            if not profile: return None, "NO INTERNET"
            manager = NetworkOperatorTetheringManager.create_from_connection_profile(profile)
            return manager, None
        except Exception as e: return None, str(e)

    @staticmethod
    def get_status():
        manager, err = HotspotDirect.get_manager()
        if not manager: return 0
        return manager.tethering_operational_state

    @staticmethod
    def get_clients():
        if not HotspotDirect._is_scanning and (time.time() - HotspotDirect._last_scan_time > 15):
            threading.Thread(target=HotspotDirect._refresh_clients, daemon=True).start()
        return HotspotDirect._clients_cache

    @staticmethod
    def _refresh_clients():
        HotspotDirect._is_scanning = True
        clients_found = []
        
        try:
            # 1. Identify Hotspot IP
            hotspot_ip = None
            try:
                addrs = psutil.net_if_addrs()
                for name, snics in addrs.items():
                    is_virt = "Local Area Connection" in name and "*" in name
                    for snic in snics:
                        if snic.family == socket.AF_INET:
                            if snic.address.startswith("192.168.137."):
                                hotspot_ip = snic.address
                                break
                            if is_virt and not snic.address.startswith("169.254"):
                                hotspot_ip = snic.address
                                break
                    if hotspot_ip: break
            except: pass

            if not hotspot_ip: hotspot_ip = "192.168.137.1"
            
            subnet_base = hotspot_ip.rsplit('.', 1)[0]
            pool_sema = threading.BoundedSemaphore(value=30) 

            def ping_ip(ip_addr):
                with pool_sema:
                    try:
                        creationflags = 0x08000000 
                        subprocess.call(['ping', '-n', '1', '-w', '50', ip_addr], 
                                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, creationflags=creationflags)
                    except: pass

            threads = []
            # Scan common range 2-60 first, then some higher ones
            scan_list = list(range(2, 60)) + list(range(100, 130))
            for i in scan_list: 
                t = threading.Thread(target=ping_ip, args=(f"{subnet_base}.{i}",))
                t.start()
                threads.append(t)
            
            for t in threads: t.join(timeout=0.15)

            # 3. Parse ARP Table
            try:
                startupinfo = subprocess.STARTUPINFO()
                startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                output = subprocess.check_output(["arp", "-a"], startupinfo=startupinfo).decode('cp437', errors='ignore')
                
                lines = output.split('\n')
                capturing = False
                
                # Global Fallback List: Any 192.168.137.x entry regardless of interface
                fallback_list = []
                
                for line in lines:
                    line = line.strip()
                    if "Interface:" in line:
                        capturing = hotspot_ip in line
                        continue
                    
                    parts = line.split()
                    if len(parts) >= 3:
                        ip = parts[0]
                        mac = parts[1]
                        type_ = parts[2]
                        
                        # Add to fallback if it matches subnet pattern
                        if ip.startswith("192.168.137.") and type_.lower() == "dynamic" and not ip.endswith(".255") and ip != "192.168.137.1":
                             fallback_list.append(f"{ip} ({mac})")

                        # If correctly capturing under the right interface
                        if capturing:
                            if type_.lower() == "dynamic" and not ip.endswith(".255") and ip != hotspot_ip:
                                clients_found.append(f"{ip} ({mac})")
                
                # If specific interface capture found nothing, use the fallback list
                if not clients_found and fallback_list:
                    clients_found = fallback_list
                    
            except Exception as e:
                print(f"DEBUG: ARP Parse Error: {e}")
            
            # 4. Fallback WinSDK
            if not clients_found:
                try:
                    manager, err = HotspotDirect.get_manager()
                    if manager:
                        clients = manager.get_tethering_clients()
                        if clients:
                            for c in clients:
                                if c.host_names: clients_found.append(c.host_names[0].canonical_name)
                                else: clients_found.append(str(c.mac_address))
                except: pass

            HotspotDirect._clients_cache = list(set(clients_found))
            HotspotDirect._last_scan_time = time.time()
        
        finally:
            HotspotDirect._is_scanning = False

    @staticmethod
    async def toggle_async():
        manager, err = HotspotDirect.get_manager()
        if not manager: return f"ERR: {err}"
        try:
            if manager.tethering_operational_state == TetheringOperationalState.ON:
                result = await manager.stop_tethering_async()
            else:
                result = await manager.start_tethering_async()
            return "Success" if result.status == 0 else f"Fail Code: {result.status}"
        except Exception as e: return f"Ex: {e}"

    @staticmethod
    def toggle_sync():
        return asyncio.run(HotspotDirect.toggle_async())

class PortScanner:
    @staticmethod
    def scan_device(ip_str, callback_fn):
        ip = ip_str.split(' ')[0]
        def _scan():
            if not shutil.which("nmap"):
                callback_fn(ip_str, "Nmap missing")
                return
            try:
                startupinfo = subprocess.STARTUPINFO()
                startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                cmd = ["nmap", "-F", "--open", ip]
                result = subprocess.run(cmd, capture_output=True, text=True, startupinfo=startupinfo)
                
                open_ports = []
                for line in result.stdout.splitlines():
                    if "/tcp" in line and "open" in line:
                        open_ports.append(line.split("/")[0])
                if open_ports: callback_fn(ip_str, f"[{', '.join(open_ports)}]")
                else: callback_fn(ip_str, "[No Open Ports]")
            except: callback_fn(ip_str, "Scan Err")
        threading.Thread(target=_scan, daemon=True).start()

class TrayIconHandler:
    def __init__(self, exit_callback, restore_callback):
        self.icon = None
        self.exit_callback = exit_callback
        self.restore_callback = restore_callback
        self.last_down = 0
        self.is_active = False

    def create_image(self):
        width = 64
        height = 64
        image = Image.new('RGBA', (width, height), (0, 0, 0, 0))
        draw = ImageDraw.Draw(image)
        bg_color = (0, 255, 204, 255) if self.is_active else (50, 50, 50, 255)
        draw.ellipse((0, 0, width, height), outline=bg_color, width=4)
        if self.last_down > 0:
            draw.rectangle((24, 24, 40, 40), fill=(0, 255, 204, 255))
        return image

    def run(self):
        # Default action triggers on click/double-click
        menu = (
            pystray.MenuItem('Show NetMonitor', self.restore_callback, default=True),
            pystray.MenuItem('Exit', self.exit_callback),
        )
        self.icon = pystray.Icon("NetMonitor", self.create_image(), "NetMonitor", menu)
        self.icon.run()

    def update_stats(self, down, is_active):
        self.last_down = down
        self.is_active = is_active
        if self.icon: self.icon.icon = self.create_image()

    def stop(self):
        if self.icon: self.icon.stop()

class FuturisticGraph(ctk.CTkFrame):
    def __init__(self, master, line_colors, max_val=None, style="line", **kwargs):
        super().__init__(master, **kwargs)
        self.colors = line_colors
        self.max_val = max_val 
        self.style = style 
        self.lines = [deque([0]*MAX_POINTS, maxlen=MAX_POINTS) for _ in self.colors]
        self.block_ids = [] 
        
        self.canvas = tk.Canvas(self, bg=GRAPH_BG, highlightthickness=0, height=80)
        self.canvas.pack(fill="both", expand=True, padx=2, pady=2)
        
        self.canvas.bind("<ButtonPress-1>", master.on_left_down)
        self.canvas.bind("<B1-Motion>", master.on_left_drag)
        self.canvas.bind("<ButtonRelease-1>", master.on_left_up)
        self.canvas.bind("<Button-3>", master.on_right_click)
        self.canvas.bind("<Configure>", self._on_resize)

    def _on_resize(self, event):
        if self.style == "block":
            self._init_blocks(event.width, event.height)

    def set_style(self, style):
        self.style = style
        self.canvas.delete("all")
        self.block_ids = []
        self.draw()

    def update_graph(self, *values):
        try:
            if not self.winfo_exists(): return
            for i, val in enumerate(values):
                if i < len(self.lines): self.lines[i].append(val)
            self.draw()
        except: pass

    def draw(self):
        try:
            w = self.canvas.winfo_width()
            h = self.canvas.winfo_height()
            if w < 10: return

            if self.style == "block":
                if not self.block_ids: self._init_blocks(w, h)
                self._update_blocks()
            else:
                self.canvas.delete("all") 
                self._draw_lines(w, h)
        except: pass

    def _draw_lines(self, w, h):
        if self.max_val: current_max = self.max_val
        else:
            all_data = [val for line in self.lines for val in line]
            current_max = max(max(all_data), 1024) 
        if current_max == 0: current_max = 1
        step_x = w / (MAX_POINTS - 1)

        for i in range(0, int(h), 20):
            self.canvas.create_line(0, i, w, i, fill="#222", width=1)
        
        for idx, data in enumerate(self.lines):
            points = []
            for i, val in enumerate(data):
                x = i * step_x
                y = h - ((val / current_max) * (h - 10))
                points.extend([x, y])
            if len(points) >= 4:
                self.canvas.create_line(points, fill=self.colors[idx], width=2, smooth=True)

    def _init_blocks(self, w, h):
        self.canvas.delete("all")
        self.block_ids = []
        cols = 50
        rows = 4
        spacing = 1
        block_w = (w - (cols+1)*spacing) / cols
        block_h = (h - (rows+1)*spacing) / rows
        
        for r in range(rows - 1, -1, -1):
            for c in range(cols):
                x1 = spacing + c * (block_w + spacing)
                y1 = spacing + r * (block_h + spacing)
                x2 = x1 + block_w
                y2 = y1 + block_h
                bid = self.canvas.create_rectangle(x1, y1, x2, y2, fill="#1a1a1a", outline="")
                self.block_ids.append(bid)

    def _update_blocks(self):
        if not self.lines or not self.block_ids: return
        last_val = self.lines[0][-1] if len(self.lines[0]) > 0 else 0
        max_v = self.max_val if self.max_val else 100
        percent = min(100, max(0, (last_val / max_v) * 100))
        blocks_lit = int((percent / 100) * 200)
        active_c = self.colors[0]
        dim_c = "#1a1a1a"
        for i, bid in enumerate(self.block_ids):
            new_col = active_c if i < blocks_lit else dim_c
            self.canvas.itemconfig(bid, fill=new_col)

class DraggableModule(ctk.CTkFrame):
    def __init__(self, master, name, **kwargs):
        super().__init__(master, **kwargs)
        self.name = name
        self.master_app = master
        self.is_moving_mode = False
        
        self.bind("<ButtonPress-1>", self.on_left_down)
        self.bind("<B1-Motion>", self.on_left_drag)
        self.bind("<ButtonRelease-1>", self.on_left_up)
        self.bind("<Button-3>", self.on_right_click)

    def on_right_click(self, event):
        self.master_app.show_context_menu(event, target_module=self.name)

    def enable_move_mode(self):
        self.is_moving_mode = True
        self.configure(border_width=2, border_color=ACCENT_YELLOW)
        self.master_app.dragging_module = self.name
        self.update()

    def disable_move_mode(self):
        self.is_moving_mode = False
        self.master_app.dragging_module = None
        self.configure(border_width=0, border_color=BG_COLOR) 
        self.update_idletasks() 
        self.master_app.save_config()

    def on_left_down(self, event):
        if self.is_moving_mode: pass
        else: self.master_app.start_move(event)

    def on_left_drag(self, event):
        if self.is_moving_mode:
            self.master_app.check_module_swap(self.name, event.y_root)
        else:
            self.master_app.do_move(event)

    def on_left_up(self, event):
        if self.is_moving_mode:
            self.disable_move_mode()
        else:
            self.master_app.stop_move(event)

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
        self.config["swap_graph_style"] = self.swap_graph.style # Save Swap style
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
            # RAM Style Menu
            if target_module == "ram":
                menu.add_separator()
                menu.add_command(label="Switch to Block Graph", command=lambda: self.set_ram_style("block"))
                menu.add_command(label="Switch to Line Graph", command=lambda: self.set_ram_style("line"))
            # Swap Style Menu
            if target_module == "swap":
                menu.add_separator()
                menu.add_command(label="Switch to Block Graph", command=lambda: self.set_swap_style("block"))
                menu.add_command(label="Switch to Line Graph", command=lambda: self.set_swap_style("line"))

            menu.add_separator()

        # Vars are now guaranteed to exist from setup_ui
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
        menu = tk.Menu(self, tearoff=0, bg="#2b2b2b", fg=ACCENT_CYAN, font=("Consolas", 10))
        menu.add_command(label=f"Scan Open Ports (Nmap)", command=lambda: self.run_nmap_scan(ip_str, label_widget))
        try: menu.tk_popup(event.x_root, event.y_root)
        except: pass

    def run_nmap_scan(self, ip_str, label_widget):
        label_widget.configure(text=f"{ip_str} [Scanning...]")
        PortScanner.scan_device(ip_str, lambda i, res: self.update_client_label(label_widget, i, res))

    def update_client_label(self, label, ip, result):
        if label.winfo_exists(): label.configure(text=f"{ip} {result}")

    def update_clients_ui(self, clients):
        current_ips = set(clients)
        known_ips = set(self.clients_widgets.keys())
        
        for ip in known_ips - current_ips:
            if self.clients_widgets[ip].winfo_exists():
                self.clients_widgets[ip].destroy()
            del self.clients_widgets[ip]
            
        for ip in current_ips - known_ips:
            lbl = ctk.CTkLabel(self.clients_frame, text=ip, text_color="gray", font=("Consolas", 10), anchor="w")
            lbl.pack(fill="x", padx=15, pady=1)
            lbl.bind("<Button-3>", lambda e, i=ip, l=lbl: self.show_client_menu(e, i, l))
            self.clients_widgets[ip] = lbl
        
        # Always repack if clients changed (height change)
        if (len(known_ips) != len(current_ips)):
            self.repack_ui()
        # If list is empty but toggle is on, show "Searching..."
        if len(self.clients_widgets) == 0 and self.show_clients_var.get():
             if not hasattr(self, 'lbl_searching'):
                 self.lbl_searching = ctk.CTkLabel(self.clients_frame, text="Searching...", text_color="#555", font=("Consolas", 10, "italic"))
                 self.lbl_searching.pack(pady=5)
        elif hasattr(self, 'lbl_searching'):
             self.lbl_searching.destroy()
             del self.lbl_searching

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
        if self.config["x"] == -1: x, y = right - self.current_width - 10, bottom - 400
        else: x, y = self.config["x"], self.config["y"]
        self.geometry(f"+{x}+{y}")
        self.repack_ui() 

    def setup_ui(self):
        # Initialize variables early
        self.topmost_var = tk.BooleanVar(value=self.config["topmost"])
        self.show_net_var = tk.BooleanVar(value=self.config["show_network"])
        self.show_cpu_var = tk.BooleanVar(value=self.config["show_cpu"])
        self.show_ram_var = tk.BooleanVar(value=self.config["show_ram"])
        self.show_swap_var = tk.BooleanVar(value=self.config.get("show_swap", False))
        self.show_clients_var = tk.BooleanVar(value=self.config.get("show_clients", True))

        self.header = ctk.CTkFrame(self, height=30, fg_color="#2b2b2b", corner_radius=0)
        self.header.pack(fill="x")
        ctk.CTkLabel(self.header, text="CORE :: SYSTEM", text_color=ACCENT_CYAN, font=("Consolas", 12, "bold")).pack(side="left", padx=10)

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
        
        # Clients Container
        self.client_sep = ctk.CTkFrame(self.net_frame, height=1, fg_color="#333")
        self.lbl_clients_header = ctk.CTkLabel(self.net_frame, text="CONNECTED DEVICES:", text_color="#888", font=("Consolas", 9, "bold"), anchor="w")
        self.clients_frame = ctk.CTkFrame(self.net_frame, fg_color="transparent")
        self.client_sep.pack(fill="x", padx=15, pady=(5,2))
        self.lbl_clients_header.pack(fill="x", padx=15)
        self.clients_frame.pack(fill="x", pady=2)

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

    def monitor_system(self):
        try:
            if not self.running: return
            
            # --- Network ---
            if self.show_net_var.get():
                c_io = psutil.net_io_counters()
                ds = c_io.bytes_recv - self.last_net_io.bytes_recv
                us = c_io.bytes_sent - self.last_net_io.bytes_sent
                self.last_net_io = c_io
                
                self.lbl_net_down.configure(text=f"DL: {self.format_bytes(ds)}")
                self.lbl_net_up.configure(text=f"UL: {self.format_bytes(us)}")
                self.lbl_max_d.configure(text=f"Mx: {self.format_bytes(self.max_down)}")
                self.lbl_max_u.configure(text=f"Mx: {self.format_bytes(self.max_up)}")
                self.lbl_tot_d.configure(text=f"Tot: {self.format_bytes(self.total_down, False)}")
                self.lbl_tot_u.configure(text=f"Tot: {self.format_bytes(self.total_up, False)}")

                self.net_graph.update_graph(ds, us)
                self.tray.update_stats(ds, self.hotspot_status == TetheringOperationalState.ON)
                
                if self.hotspot_status == TetheringOperationalState.ON:
                    clients = HotspotDirect.get_clients()
                    self.update_clients_ui(clients)
                else:
                    self.update_clients_ui([])
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