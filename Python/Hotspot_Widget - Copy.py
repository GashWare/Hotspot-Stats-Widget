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
from collections import deque

# --- NEW IMPORTS ---
try:
    import pystray
    from PIL import Image, ImageDraw
except ImportError:
    ctypes.windll.user32.MessageBoxW(0, "Please run: pip install pystray Pillow", "Missing Library", 1)
    sys.exit()

# --- STEALTH MODE ---
try:
    if os.name == 'nt':
        hwnd = ctypes.windll.kernel32.GetConsoleWindow()
        if hwnd != 0:
            ctypes.windll.user32.ShowWindow(hwnd, 0)
            sys.stdout = open(os.devnull, 'w')
            sys.stderr = open(os.devnull, 'w')
except Exception:
    pass

# --- INSTALL CHECK ---
try:
    import winsdk.windows.networking.connectivity as network_info
    import winsdk.windows.networking.networkoperators as network_ops
    from winsdk.windows.networking.networkoperators import NetworkOperatorTetheringManager, TetheringOperationalState
except ImportError:
    ctypes.windll.user32.MessageBoxW(0, "Please run: pip install winsdk", "Missing Library", 1)
    sys.exit()

# --- Configuration Constants ---
UPDATE_DELAY = 1000  # ms
MAX_POINTS = 50      
ACCENT_COLOR = "#00ffcc" # Cyan
WARNING_COLOR = "#ff0055" # Red
BG_COLOR = "#1a1a1a"
GRAPH_BG = "#0f0f0f"
TASK_NAME = "NetMonitorAutoStart"
CONFIG_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "widget_config.json")

DEFAULT_CONFIG = {
    "x": -1, "y": -1, "width": 350, "height": 300, "opacity": 0.92, "topmost": False
}

# --- DPI Handling ---
try: ctypes.windll.shcore.SetProcessDpiAwareness(1)
except: pass 

class RECT(Structure):
    _fields_ = [("left", c_long), ("top", c_long), ("right", c_long), ("bottom", c_long)]

def get_work_area():
    rect = RECT()
    windll.user32.SystemParametersInfoW(48, 0, byref(rect), 0)
    return rect.left, rect.top, rect.right, rect.bottom

class AdminHandler:
    @staticmethod
    def is_admin():
        try: return ctypes.windll.shell32.IsUserAnAdmin()
        except: return False

    @staticmethod
    def elevate():
        exe = sys.executable.replace("python.exe", "pythonw.exe") if "python.exe" in sys.executable else sys.executable
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

class HotspotDirect:
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

class TrayIconHandler:
    def __init__(self, exit_callback):
        self.icon = None
        self.exit_callback = exit_callback
        self.last_down = 0
        self.last_up = 0
        self.is_active = False

    def create_image(self):
        # Create a 64x64 icon surface
        width = 64
        height = 64
        image = Image.new('RGBA', (width, height), (0, 0, 0, 0)) # Transparent
        draw = ImageDraw.Draw(image)

        # Draw Background Circle (Color coded by status)
        bg_color = (0, 255, 204, 255) if self.is_active else (50, 50, 50, 255) # Cyan vs Gray
        draw.ellipse((0, 0, width, height), outline=bg_color, width=4)

        # Scale bars (Logarithmic-ish scaling for visual clarity)
        # Max scale assumes roughly 10MB/s for visual fullness
        max_scale = 10 * 1024 * 1024 
        
        h_down = min(int((self.last_down / max_scale) * 40), 40)
        h_up = min(int((self.last_up / max_scale) * 40), 40)
        
        # Ensure at least 1px if there is any traffic
        if self.last_down > 0: h_down = max(h_down, 4)
        if self.last_up > 0: h_up = max(h_up, 4)

        # Draw Download Bar (Cyan)
        # x1, y1, x2, y2
        draw.rectangle((16, 52 - h_down, 28, 52), fill=(0, 255, 204, 255))
        
        # Draw Upload Bar (Red)
        draw.rectangle((36, 52 - h_up, 48, 52), fill=(255, 0, 85, 255))

        return image

    def run(self):
        menu = (pystray.MenuItem('Exit', self.exit_callback),)
        self.icon = pystray.Icon("NetMonitor", self.create_image(), "NetMonitor", menu)
        self.icon.run()

    def update_stats(self, down, up, is_active):
        self.last_down = down
        self.last_up = up
        self.is_active = is_active
        if self.icon:
            self.icon.icon = self.create_image()

    def stop(self):
        if self.icon:
            self.icon.stop()

class FuturisticGraph(ctk.CTkFrame):
    def __init__(self, master, **kwargs):
        super().__init__(master, **kwargs)
        self.canvas = tk.Canvas(self, bg=GRAPH_BG, highlightthickness=0, height=100)
        self.canvas.pack(fill="both", expand=True, padx=2, pady=2)
        
        self.canvas.bind("<ButtonPress-1>", master.start_move)
        self.canvas.bind("<ButtonRelease-1>", master.stop_move)
        self.canvas.bind("<B1-Motion>", master.do_move)
        self.canvas.bind("<Button-3>", master.show_context_menu)

        self.download_data = deque([0]*MAX_POINTS, maxlen=MAX_POINTS)
        self.upload_data = deque([0]*MAX_POINTS, maxlen=MAX_POINTS)

    def update_graph(self, down_val, up_val):
        try:
            if not self.winfo_exists(): return
            self.download_data.append(down_val)
            self.upload_data.append(up_val)
            self.draw()
        except: pass

    def draw(self):
        try:
            self.canvas.delete("all")
            w = self.canvas.winfo_width()
            h = self.canvas.winfo_height()
            if w < 10: return

            current_max = max(max(self.download_data), max(self.upload_data), 1024)
            step_x = w / (MAX_POINTS - 1)

            for i in range(0, int(h), 20):
                color = "#222222" if i % 40 != 0 else "#333333"
                self.canvas.create_line(0, i, w, i, fill=color, width=1)
            
            def plot_line(data, color):
                points = []
                for i, val in enumerate(data):
                    x = i * step_x
                    y = h - ((val / current_max) * (h - 10))
                    points.extend([x, y])
                if len(points) >= 4:
                    self.canvas.create_line(points, fill=color, width=2, smooth=True)

            plot_line(self.upload_data, WARNING_COLOR) 
            plot_line(self.download_data, ACCENT_COLOR)
        except: pass 

class HotspotWidget(ctk.CTk):
    def __init__(self):
        super().__init__()
        
        self.title("NetMonitor")
        self.overrideredirect(True)
        self.configure(fg_color=BG_COLOR)
        
        # Load Config
        self.config = self.load_config()
        self.current_width = self.config["width"]
        self.current_height = self.config["height"]
        
        self.geometry(f"{self.current_width}x{self.current_height}")
        self.attributes("-alpha", self.config["opacity"])
        self.attributes("-topmost", self.config["topmost"])

        # Data Tracking
        self.last_net_io = psutil.net_io_counters()
        self.hotspot_status = 0
        self.running = True 
        
        self.max_down = 0
        self.max_up = 0
        self.min_down = None
        self.min_up = None
        self.total_down = 0
        self.total_up = 0
        
        # Initialize Tray Icon
        self.tray = TrayIconHandler(self.safe_destroy)
        threading.Thread(target=self.tray.run, daemon=True).start()

        self.create_context_menu()
        self.setup_ui()
        self.setup_bindings()
        
        # Apply Window Hacks to remove from Taskbar
        self.after(10, self.hide_from_taskbar)
        self.after(100, self.apply_saved_position)
        
        self.monitor_network()
        threading.Thread(target=self.background_status_check, daemon=True).start()

    def hide_from_taskbar(self):
        # Uses Windows API to set the window as a "ToolWindow" which hides it from the taskbar
        GWL_EXSTYLE = -20
        WS_EX_APPWINDOW = 0x00040000
        WS_EX_TOOLWINDOW = 0x00000080
        
        hwnd = windll.user32.GetParent(self.winfo_id())
        style = windll.user32.GetWindowLongW(hwnd, GWL_EXSTYLE)
        style = style & ~WS_EX_APPWINDOW
        style = style | WS_EX_TOOLWINDOW
        windll.user32.SetWindowLongW(hwnd, GWL_EXSTYLE, style)
        
        # Force redraw
        self.withdraw()
        self.after(10, self.deiconify)

    def create_context_menu(self):
        self.context_menu = tk.Menu(self, tearoff=0, bg="#2b2b2b", fg=ACCENT_COLOR, font=("Consolas", 10))
        self.topmost_var = tk.BooleanVar(value=self.config["topmost"])
        self.context_menu.add_checkbutton(label="Always On Top", onvalue=True, offvalue=False, variable=self.topmost_var, command=self.toggle_topmost)
        self.context_menu.add_separator()
        
        self.opacity_menu = tk.Menu(self.context_menu, tearoff=0, bg="#2b2b2b", fg=ACCENT_COLOR)
        for val, label in [(1.0, "100%"), (0.9, "90%"), (0.75, "75%"), (0.5, "50%"), (0.4, "40%")]:
            self.opacity_menu.add_command(label=label, command=lambda v=val: self.set_opacity(v))
        self.context_menu.add_cascade(label="Transparency", menu=self.opacity_menu)

        self.size_menu = tk.Menu(self.context_menu, tearoff=0, bg="#2b2b2b", fg=ACCENT_COLOR)
        self.size_menu.add_command(label="Compact (300x200)", command=lambda: self.set_size(300, 200))
        self.size_menu.add_command(label="Normal  (350x300)", command=lambda: self.set_size(350, 300))
        self.size_menu.add_command(label="Large   (500x400)", command=lambda: self.set_size(500, 400))
        self.context_menu.add_cascade(label="Resize Window", menu=self.size_menu)

        self.context_menu.add_separator()
        self.context_menu.add_command(label="Reset Stats", command=self.reset_stats)
        self.context_menu.add_command(label="Exit", command=self.safe_destroy)

    def load_config(self):
        try:
            with open(CONFIG_FILE, 'r') as f: return {**DEFAULT_CONFIG, **json.load(f)}
        except: return DEFAULT_CONFIG

    def save_config(self):
        self.config["x"] = self.winfo_x()
        self.config["y"] = self.winfo_y()
        self.config["width"] = self.current_width
        self.config["height"] = self.current_height
        self.config["topmost"] = self.topmost_var.get()
        try:
            with open(CONFIG_FILE, 'w') as f: json.dump(self.config, f)
        except: pass

    def toggle_topmost(self):
        val = self.topmost_var.get()
        self.attributes("-topmost", val)
        self.save_config()

    def set_opacity(self, val):
        self.attributes("-alpha", val)
        self.config["opacity"] = val
        self.save_config()

    def set_size(self, w, h):
        self.current_width = w
        self.current_height = h
        self.geometry(f"{w}x{h}")
        self.save_config()

    def apply_saved_position(self):
        left, top, right, bottom = get_work_area()
        if self.config["x"] == -1:
            x = right - self.current_width - 10
            y = bottom - self.current_height - 10
        else:
            x = self.config["x"]
            y = self.config["y"]
            if x > right - 50: x = right - self.current_width - 10
            if y > bottom - 50: y = bottom - self.current_height - 10
        self.geometry(f"+{x}+{y}")

    def safe_destroy(self):
        self.running = False
        self.tray.stop()
        try: self.destroy()
        except: pass

    def reset_stats(self):
        self.max_down = 0
        self.max_up = 0
        self.min_down = None
        self.min_up = None
        self.total_down = 0
        self.total_up = 0

    def setup_ui(self):
        self.stats_frame = ctk.CTkFrame(self, fg_color="transparent")
        self.stats_frame.pack(fill="x", padx=10, pady=(15, 5))
        self.stats_frame.grid_columnconfigure(0, weight=1)
        self.stats_frame.grid_columnconfigure(1, weight=1)
        
        self.lbl_down = ctk.CTkLabel(self.stats_frame, text="DL: 0.0 B/s", text_color=ACCENT_COLOR, font=("Consolas", 12, "bold"), anchor="w")
        self.lbl_down.grid(row=0, column=0, sticky="w")
        self.lbl_up = ctk.CTkLabel(self.stats_frame, text="UL: 0.0 B/s", text_color=WARNING_COLOR, font=("Consolas", 12, "bold"), anchor="e")
        self.lbl_up.grid(row=0, column=1, sticky="e")

        self.lbl_max_down = ctk.CTkLabel(self.stats_frame, text="Mx: 0.0 B/s", text_color="#008f72", font=("Consolas", 10), anchor="w")
        self.lbl_max_down.grid(row=1, column=0, sticky="w")
        self.lbl_max_up = ctk.CTkLabel(self.stats_frame, text="Mx: 0.0 B/s", text_color="#8f0030", font=("Consolas", 10), anchor="e")
        self.lbl_max_up.grid(row=1, column=1, sticky="e")

        self.lbl_min_down = ctk.CTkLabel(self.stats_frame, text="Mn: 0.0 B/s", text_color="#005c49", font=("Consolas", 10), anchor="w")
        self.lbl_min_down.grid(row=2, column=0, sticky="w")
        self.lbl_min_up = ctk.CTkLabel(self.stats_frame, text="Mn: 0.0 B/s", text_color="#5c001f", font=("Consolas", 10), anchor="e")
        self.lbl_min_up.grid(row=2, column=1, sticky="e")
        
        self.lbl_tot_down = ctk.CTkLabel(self.stats_frame, text="Tot: 0.0 B", text_color="gray", font=("Consolas", 10), anchor="w")
        self.lbl_tot_down.grid(row=3, column=0, sticky="w")
        self.lbl_tot_up = ctk.CTkLabel(self.stats_frame, text="Tot: 0.0 B", text_color="gray", font=("Consolas", 10), anchor="e")
        self.lbl_tot_up.grid(row=3, column=1, sticky="e")

        self.graph = FuturisticGraph(self, fg_color="transparent")
        self.graph.pack(fill="both", expand=True, padx=10, pady=5)

        self.btn_hotspot = ctk.CTkButton(self, text="INITIALIZING...", fg_color="#222", 
                                         border_color="#444", border_width=1, hover_color="#333", 
                                         text_color="gray", font=("Consolas", 11, "bold"), height=35,
                                         command=self.start_toggle_thread)
        self.btn_hotspot.pack(fill="x", padx=10, pady=(10, 5))

        if not AdminHandler.check_task_exists():
            self.btn_autostart = ctk.CTkButton(self, text="ENABLE AUTO-START (ADMIN)", 
                                               fg_color="transparent", border_color="#333", 
                                               border_width=1, text_color="#888", 
                                               font=("Consolas", 9), height=20,
                                               command=self.enable_autostart)
            self.btn_autostart.pack(fill="x", padx=10, pady=(0, 15))
        else:
            ctk.CTkLabel(self, text="", height=5).pack()

    def setup_bindings(self):
        for widget in [self, self.stats_frame, self.lbl_down, self.lbl_up, 
                       self.lbl_max_down, self.lbl_max_up, self.lbl_min_down, self.lbl_min_up,
                       self.lbl_tot_down, self.lbl_tot_up]:
            try:
                widget.bind("<ButtonPress-1>", self.start_move)
                widget.bind("<ButtonRelease-1>", self.stop_move)
                widget.bind("<B1-Motion>", self.do_move)
                widget.bind("<Button-3>", self.show_context_menu)
            except: pass

    def show_context_menu(self, event):
        try: self.context_menu.tk_popup(event.x_root, event.y_root)
        except: pass

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
            left, top, right, bottom = get_work_area()
            if new_x < left: new_x = left
            if new_x + self.current_width > right: new_x = right - self.current_width
            if new_y < top: new_y = top
            if new_y + self.current_height > bottom: new_y = bottom - self.current_height
            self.geometry(f"+{new_x}+{new_y}")
        except: pass

    def enable_autostart(self):
        AdminHandler.create_startup_task()
        try: self.btn_autostart.configure(text="AUTO-START ENABLED", state="disabled", text_color=ACCENT_COLOR)
        except: pass

    def start_toggle_thread(self):
        threading.Thread(target=self.toggle_hotspot, daemon=True).start()

    def toggle_hotspot(self):
        if not self.running: return
        try: self.btn_hotspot.configure(state="disabled", text="PROCESSING...", text_color="gray")
        except: return 
        result = HotspotDirect.toggle_sync()
        if result == "Success":
            for _ in range(10):
                if not self.running: return
                self.background_status_check(loop=False)
                time.sleep(0.5)
            if self.running: self.background_status_check(loop=False)
        else:
            self.after(0, lambda: self.show_error(result))

    def background_status_check(self, loop=True):
        while self.running:
            status = HotspotDirect.get_status()
            if self.running: self.after(0, lambda s=status: self.update_status_ui(s))
            if not loop: break
            time.sleep(3) 

    def update_status_ui(self, status):
        try:
            if not self.running: return
            if status == TetheringOperationalState.ON:
                self.btn_hotspot.configure(text="HOTSPOT: ACTIVE", fg_color=ACCENT_COLOR, border_color=ACCENT_COLOR, state="normal", text_color="black")
                self.tray.update_stats(0, 0, True) # Just update status flag, data comes later
            elif status == TetheringOperationalState.OFF:
                self.btn_hotspot.configure(text="HOTSPOT: INACTIVE", fg_color="transparent", border_color=ACCENT_COLOR, state="normal", text_color=ACCENT_COLOR)
                self.tray.update_stats(0, 0, False)
            elif status == TetheringOperationalState.IN_TRANSITION:
                self.btn_hotspot.configure(text="SWITCHING...", fg_color="#333", border_color="#555", state="disabled", text_color="gray")
            else:
                self.btn_hotspot.configure(text="NO CONNECTION", fg_color="#333", border_color="#444", state="normal", text_color="gray")
        except: pass

    def show_error(self, msg):
        try:
            self.btn_hotspot.configure(text=f"FAIL: {msg[:15]}", fg_color="#330000", border_color=WARNING_COLOR, text_color=WARNING_COLOR, state="normal")
            self.after(3000, lambda: self.background_status_check(loop=False))
        except: pass

    def format_bytes(self, size, is_rate=True):
        power, n = 2**10, 0
        labels = {0:'', 1:'K', 2:'M', 3:'G', 4:'T'}
        while size > power: size /= power; n += 1
        suffix = "B/s" if is_rate else "B"
        return f"{size:.1f} {labels.get(n, 'T')}{suffix}"

    def monitor_network(self):
        try:
            if not self.running: return
            c_io = psutil.net_io_counters()
            ds = c_io.bytes_recv - self.last_net_io.bytes_recv
            us = c_io.bytes_sent - self.last_net_io.bytes_sent
            self.last_net_io = c_io
            
            self.total_down += ds
            self.total_up += us
            
            if ds > self.max_down: self.max_down = ds
            if us > self.max_up: self.max_up = us
            
            if self.min_down is None: self.min_down = ds
            else: self.min_down = min(self.min_down, ds)
            if self.min_up is None: self.min_up = us
            else: self.min_up = min(self.min_up, us)
            
            self.lbl_down.configure(text=f"DL: {self.format_bytes(ds)}")
            self.lbl_up.configure(text=f"UL: {self.format_bytes(us)}")
            self.lbl_max_down.configure(text=f"Mx: {self.format_bytes(self.max_down)}")
            self.lbl_max_up.configure(text=f"Mx: {self.format_bytes(self.max_up)}")
            self.lbl_min_down.configure(text=f"Mn: {self.format_bytes(self.min_down)}")
            self.lbl_min_up.configure(text=f"Mn: {self.format_bytes(self.min_up)}")
            self.lbl_tot_down.configure(text=f"Tot: {self.format_bytes(self.total_down, False)}")
            self.lbl_tot_up.configure(text=f"Tot: {self.format_bytes(self.total_up, False)}")

            self.graph.update_graph(ds, us)
            
            # Update Tray Icon Graph
            is_active = (self.hotspot_status == TetheringOperationalState.ON)
            self.tray.update_stats(ds, us, is_active)
            
            self.after(UPDATE_DELAY, self.monitor_network)
        except (tk.TclError, RuntimeError): pass

if __name__ == "__main__":
    if not AdminHandler.is_admin():
        AdminHandler.elevate()
    else:
        ctk.set_appearance_mode("Dark")
        app = HotspotWidget()
        app.mainloop()