import customtkinter as ctk
import tkinter as tk
import time
import math
import pystray
from PIL import Image, ImageDraw
from collections import deque
from config import *
import ctypes
import sys

# Check for dependencies
try:
    import pystray
    from PIL import Image, ImageDraw
except ImportError:
    pass # Handled in main

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
    def __init__(self, master, line_colors, max_val=None, style="line", fluid=True, interval_ms=250, **kwargs):
        super().__init__(master, **kwargs)
        self.colors = line_colors
        self.max_val = max_val 
        self.style = style 
        self.fluid = fluid
        self.interval_ms = max(50, interval_ms)
        self.lines = [deque([0]*MAX_POINTS, maxlen=MAX_POINTS) for _ in self.colors]
        self.block_ids = [] 
        self.current_percent = 0.0
        self.target_percent = 0.0
        self.last_update_time = time.time()
        
        self.canvas = tk.Canvas(self, bg=GRAPH_BG, highlightthickness=0, height=80)
        self.canvas.pack(fill="both", expand=True, padx=2, pady=2)
        
        self.canvas.bind("<ButtonPress-1>", master.on_left_down)
        self.canvas.bind("<B1-Motion>", master.on_left_drag)
        self.canvas.bind("<ButtonRelease-1>", master.on_left_up)
        self.canvas.bind("<Button-3>", master.on_right_click)
        self.canvas.bind("<Configure>", self._on_resize)

        # Start animation ticker
        self.after(33, self._animate)

    def _on_resize(self, event):
        if self.style == "block":
            self._init_blocks(event.width, event.height)

    def set_style(self, style):
        self.style = style
        self.canvas.delete("all")
        self.block_ids = []
        self.draw()

    def set_fluid_mode(self, fluid_enabled, interval_ms=None):
        self.fluid = fluid_enabled
        if interval_ms:
            self.interval_ms = max(50, interval_ms)
        self.draw()

    def _animate(self):
        try:
            if not self.winfo_exists():
                return
            if self.fluid:
                if self.style == "block":
                    diff = self.target_percent - self.current_percent
                    if abs(diff) > 0.5:
                        self.current_percent += diff * 0.25
                        self._update_blocks_val(self.current_percent)
                else:
                    self.draw()
                self.after(40, self._animate)
            else:
                self.after(500, self._animate)
        except:
            pass

    def update_graph(self, *values):
        try:
            if not self.winfo_exists(): return
            for i, val in enumerate(values):
                if i < len(self.lines): self.lines[i].append(val)
            self.last_update_time = time.time()
            if self.style == "block" and self.lines and len(self.lines[0]) > 0:
                last_val = self.lines[0][-1]
                max_v = self.max_val if self.max_val else 100
                self.target_percent = min(100.0, max(0.0, (last_val / max_v) * 100.0))
                if not self.fluid:
                    self.current_percent = self.target_percent
            self.draw()
        except: pass

    def draw(self):
        try:
            w = self.canvas.winfo_width()
            h = self.canvas.winfo_height()
            if w < 10: return

            if self.style == "block":
                if not self.block_ids: self._init_blocks(w, h)
                self._update_blocks_val(self.current_percent)
            else:
                self.canvas.delete("all") 
                # Calculate sub-tick fluid shift (0.0 to 1.0)
                shift_frac = 0.0
                if self.fluid and self.interval_ms > 0:
                    elapsed = (time.time() - self.last_update_time) * 1000.0
                    shift_frac = min(1.0, max(0.0, elapsed / self.interval_ms))
                self._draw_lines(w, h, shift_frac)
        except: pass

    def _draw_lines(self, w, h, shift_frac=0.0):
        if self.max_val: current_max = self.max_val
        else:
            all_data = [val for line in self.lines for val in line]
            current_max = max(max(all_data), 1024) 
        if current_max == 0: current_max = 1
        step_x = w / (MAX_POINTS - 1)

        for i in range(0, int(h), 20):
            self.canvas.create_line(0, i, w, i, fill="#222", width=1)
        
        offset_x = shift_frac * step_x if self.fluid else 0.0

        for idx, data in enumerate(self.lines):
            points = []
            for i, val in enumerate(data):
                x = (i * step_x) - offset_x
                y = h - ((val / current_max) * (h - 10))
                points.extend([x, y])
            if len(points) >= 4:
                # Add terminal point at rightmost edge so line extends to border
                if self.fluid and len(data) >= 1:
                    last_val = data[-1]
                    last_y = h - ((last_val / current_max) * (h - 10))
                    points.extend([w, last_y])
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

    def _update_blocks_val(self, percent):
        if not self.block_ids: return
        blocks_lit = int((percent / 100.0) * len(self.block_ids))
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