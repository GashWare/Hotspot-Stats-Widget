# File: traffic_inspector_ui.py
# Description: Interactive UI for inspecting Hotspot traffic, viewing packet payloads (Wireshark-style), and managing domain/IP block rules.

import re
import tkinter as tk
from tkinter import ttk
import customtkinter as ctk
from config import ACCENT_CYAN, ACCENT_RED, ACCENT_YELLOW, ACCENT_GREEN, ACCENT_ORANGE, BG_COLOR, GRAPH_BG
from traffic_inspector import TrafficInspector

class PacketPayloadViewerDialog(ctk.CTkToplevel):
    """Wireshark-style individual packet payload inspector dialog."""
    def __init__(self, master, record):
        super().__init__(master)
        self.record = record
        self.title("Packet Payload Inspector [Wireshark Mode]")
        self.geometry("740x460")
        self.minsize(580, 340)
        self.configure(fg_color=BG_COLOR)

        # Header summary
        header_frame = ctk.CTkFrame(self, fg_color="#18242c", corner_radius=6)
        header_frame.pack(fill="x", padx=10, pady=(10, 6))

        title_lbl = ctk.CTkLabel(
            header_frame,
            text="PACKET DETAILS & PAYLOAD",
            text_color=ACCENT_CYAN,
            font=("Consolas", 12, "bold"),
        )
        title_lbl.pack(anchor="w", padx=10, pady=(6, 2))

        client = record.get("client_ip", "-")
        remote = record.get("remote_ip", "-")
        port = record.get("remote_port", 0)
        proto = record.get("proto", "-")
        domain = record.get("domain", "-")
        size = record.get("size", 0)
        payload_bytes = record.get("payload", b"")

        summary_text = (
            f"Time: {record.get('timestamp')} | Length: {size} B (captured: {len(payload_bytes)} B) | "
            f"Proto: {proto} | Flow: {client} -> {remote}:{port}"
        )
        if domain != "-":
            summary_text += f"\nDomain/SNI: {domain}"

        info_lbl = ctk.CTkLabel(
            header_frame,
            text=summary_text,
            text_color="#cccccc",
            font=("Consolas", 10),
            justify="left",
        )
        info_lbl.pack(anchor="w", padx=10, pady=(0, 6))

        # View Tabs: Hex Dump + ASCII vs Plaintext View
        tabview = ctk.CTkTabview(
            self,
            fg_color=BG_COLOR,
            segmented_button_fg_color="#222",
            segmented_button_selected_color="#333",
            segmented_button_selected_hover_color="#444",
            text_color=ACCENT_CYAN,
        )
        tabview.pack(fill="both", expand=True, padx=10, pady=4)

        tab_hex = tabview.add("Hex Dump + ASCII (Wireshark)")
        tab_text = tabview.add("Decoded Text")

        # Hex Dump View
        hex_text = TrafficInspector.format_hex_dump(payload_bytes)
        hex_box = ctk.CTkTextbox(
            tab_hex,
            font=("Consolas", 10),
            text_color="#00ffcc",
            fg_color="#0a1218",
            wrap="none",
        )
        hex_box.pack(fill="both", expand=True, padx=4, pady=4)
        hex_box.insert("1.0", hex_text)
        hex_box.configure(state="disabled")

        # Plaintext View
        ascii_text = TrafficInspector.format_ascii_text(payload_bytes)
        text_box = ctk.CTkTextbox(
            tab_text,
            font=("Consolas", 10),
            text_color="#b0ffb0",
            fg_color="#0a1218",
            wrap="char",
        )
        text_box.pack(fill="both", expand=True, padx=4, pady=4)
        text_box.insert("1.0", ascii_text)
        text_box.configure(state="disabled")

        # Bottom Button Bar
        btn_bar = ctk.CTkFrame(self, fg_color="transparent")
        btn_bar.pack(fill="x", padx=10, pady=(4, 10))

        btn_copy = ctk.CTkButton(
            btn_bar,
            text="Copy Hex Dump",
            width=130,
            height=28,
            font=("Consolas", 10),
            fg_color="#333",
            hover_color="#444",
            command=lambda: self._copy_to_clipboard(hex_text),
        )
        btn_copy.pack(side="left", padx=4)

        btn_close = ctk.CTkButton(
            btn_bar,
            text="Close",
            width=80,
            height=28,
            font=("Consolas", 10),
            fg_color="#444",
            hover_color="#555",
            command=self.destroy,
        )
        btn_close.pack(side="right", padx=4)

    def _copy_to_clipboard(self, text):
        self.clipboard_clear()
        self.clipboard_append(text)


class TrafficInspectorWindow(ctk.CTkToplevel):
    def __init__(self, master, inspector: TrafficInspector):
        super().__init__(master)
        self.inspector = inspector
        self.title("NetMonitor :: Hotspot Traffic Inspector & Packet Analyzer")
        self.geometry("900x560")
        self.minsize(720, 420)
        self.configure(fg_color=BG_COLOR)

        self.paused = False
        self.filter_text = ""
        self.filter_device = "All Devices"
        self.filter_blocked_only = False
        self.last_rendered_index = 0
        self.rendered_records_map = {} # item_id -> record

        self._setup_ui()
        self._setup_tree_style()
        self._refresh_blocklist_ui()

        # Start periodic UI refresh
        self.after(350, self._periodic_update)

    def _setup_ui(self):
        # 1. Top Header Bar
        top_bar = ctk.CTkFrame(self, height=40, fg_color="#242424", corner_radius=0)
        top_bar.pack(fill="x", padx=0, pady=0)

        lbl_title = ctk.CTkLabel(
            top_bar,
            text="HOTSPOT TRAFFIC INSPECTOR & PACKET ANALYZER",
            text_color=ACCENT_CYAN,
            font=("Consolas", 12, "bold"),
        )
        lbl_title.pack(side="left", padx=12, pady=6)

        self.lbl_status = ctk.CTkLabel(
            top_bar,
            text="STATUS: ACTIVE",
            text_color=ACCENT_GREEN,
            font=("Consolas", 10, "bold"),
        )
        self.lbl_status.pack(side="left", padx=10, pady=6)

        self.blocker_toggle_var = tk.BooleanVar(value=self.inspector.blocker_enabled)
        chk_blocker = ctk.CTkCheckBox(
            top_bar,
            text="Blocker Enabled",
            variable=self.blocker_toggle_var,
            command=self._on_blocker_toggle,
            text_color=ACCENT_RED,
            font=("Consolas", 10),
            fg_color=ACCENT_RED,
            hover_color="#990033",
            width=110,
        )
        chk_blocker.pack(side="right", padx=12, pady=6)

        # 2. Control & Search Strip (Device filter + search + pause)
        ctrl_strip = ctk.CTkFrame(self, height=36, fg_color="#1e1e1e", corner_radius=0)
        ctrl_strip.pack(fill="x", padx=0, pady=0)

        # Device Selection Filter
        lbl_dev = ctk.CTkLabel(
            ctrl_strip,
            text="Device:",
            text_color=ACCENT_ORANGE,
            font=("Consolas", 10, "bold"),
        )
        lbl_dev.pack(side="left", padx=(12, 4), pady=4)

        self.opt_devices = ctk.CTkOptionMenu(
            ctrl_strip,
            values=["All Devices"],
            command=self._on_device_changed,
            width=150,
            height=26,
            font=("Consolas", 10),
            fg_color="#333",
            button_color="#444",
        )
        self.opt_devices.pack(side="left", padx=(0, 10), pady=4)

        # Search Filter
        lbl_search = ctk.CTkLabel(ctrl_strip, text="Filter:", text_color="#aaa", font=("Consolas", 10))
        lbl_search.pack(side="left", padx=(4, 4), pady=4)

        self.entry_search = ctk.CTkEntry(
            ctrl_strip,
            width=160,
            height=26,
            font=("Consolas", 10),
            placeholder_text="Search domain/IP...",
        )
        self.entry_search.pack(side="left", padx=4, pady=4)
        self.entry_search.bind("<KeyRelease>", self._on_filter_changed)

        self.chk_blocked_only_var = tk.BooleanVar(value=False)
        chk_blocked_only = ctk.CTkCheckBox(
            ctrl_strip,
            text="Blocked Only",
            variable=self.chk_blocked_only_var,
            command=self._on_filter_changed,
            font=("Consolas", 10),
            width=90,
        )
        chk_blocked_only.pack(side="left", padx=8, pady=4)

        btn_pause = ctk.CTkButton(
            ctrl_strip,
            text="Pause",
            width=65,
            height=24,
            font=("Consolas", 10),
            fg_color="#333",
            hover_color="#444",
            command=self._toggle_pause,
        )
        btn_pause.pack(side="right", padx=6, pady=4)
        self.btn_pause = btn_pause

        btn_clear = ctk.CTkButton(
            ctrl_strip,
            text="Clear",
            width=60,
            height=24,
            font=("Consolas", 10),
            fg_color="#333",
            hover_color="#444",
            command=self._clear_table,
        )
        btn_clear.pack(side="right", padx=4, pady=4)

        # 3. Notebook / Tabs: Live Traffic vs Blocklist Manager
        self.tabview = ctk.CTkTabview(
            self,
            fg_color=BG_COLOR,
            segmented_button_fg_color="#222",
            segmented_button_selected_color="#333",
            segmented_button_selected_hover_color="#444",
            text_color=ACCENT_CYAN,
        )
        self.tabview.pack(fill="both", expand=True, padx=8, pady=(0, 6))

        self.tab_traffic = self.tabview.add("Live Traffic Feed")
        self.tab_rules = self.tabview.add("Blocklist Rules")

        self._setup_traffic_tab()
        self._setup_rules_tab()

    def _setup_tree_style(self):
        style = ttk.Style()
        style.theme_use("clam")
        style.configure(
            "Cyber.Treeview",
            background="#111111",
            foreground="#dddddd",
            fieldbackground="#111111",
            font=("Consolas", 9),
            rowheight=22,
            borderwidth=0,
        )
        style.configure(
            "Cyber.Treeview.Heading",
            background="#262626",
            foreground=ACCENT_CYAN,
            font=("Consolas", 9, "bold"),
            borderwidth=1,
            relief="flat",
        )
        style.map(
            "Cyber.Treeview",
            background=[("selected", "#004444")],
            foreground=[("selected", "#00ffff")],
        )

    def _setup_traffic_tab(self):
        tree_frame = ctk.CTkFrame(self.tab_traffic, fg_color="transparent")
        tree_frame.pack(fill="both", expand=True, padx=4, pady=4)

        cols = ("Time", "Client Device", "Domain / Host", "Target IP:Port", "Proto", "Size", "Status")
        self.tree = ttk.Treeview(
            tree_frame,
            columns=cols,
            show="headings",
            style="Cyber.Treeview",
            selectmode="browse",
        )

        col_widths = {
            "Time": 75,
            "Client Device": 115,
            "Domain / Host": 240,
            "Target IP:Port": 150,
            "Proto": 70,
            "Size": 65,
            "Status": 85,
        }
        for c in cols:
            self.tree.heading(c, text=c)
            self.tree.column(c, width=col_widths.get(c, 100), anchor="w")
        self.tree.column("Status", anchor="center")
        self.tree.column("Size", anchor="e")

        scrollbar = ttk.Scrollbar(tree_frame, orient="vertical", command=self.tree.yview)
        self.tree.configure(yscrollcommand=scrollbar.set)

        self.tree.pack(side="left", fill="both", expand=True)
        scrollbar.pack(side="right", fill="y")

        # Row Tags
        self.tree.tag_configure("blocked", foreground=ACCENT_RED, background="#22050b")
        self.tree.tag_configure("dns", foreground=ACCENT_GREEN)
        self.tree.tag_configure("https", foreground=ACCENT_CYAN)
        self.tree.tag_configure("http", foreground=ACCENT_YELLOW)
        self.tree.tag_configure("allowed", foreground="#eeeeee")

        # Double-click to inspect payload
        self.tree.bind("<Double-1>", self._on_tree_double_click)

        # Context Menu
        self.menu = tk.Menu(self, tearoff=0, bg="#2b2b2b", fg=ACCENT_CYAN, font=("Consolas", 9))
        self.menu.add_command(label="Inspect Packet Payload (Wireshark View)", command=self._inspect_selected_packet)
        self.menu.add_separator()
        self.menu.add_command(label="Block Selected Domain", command=self._block_selected_domain)
        self.menu.add_command(label="Block Selected Client IP", command=self._block_selected_client)
        self.menu.add_command(label="Block Target IP", command=self._block_selected_remote)
        self.menu.add_separator()
        self.menu.add_command(label="Copy Row Details", command=self._copy_selected_details)

        self.tree.bind("<Button-3>", self._on_tree_right_click)

    def _setup_rules_tab(self):
        frame = self.tab_rules

        top_ctrl = ctk.CTkFrame(frame, fg_color="transparent")
        top_ctrl.pack(fill="x", padx=10, pady=8)

        lbl_add = ctk.CTkLabel(top_ctrl, text="Add Domain/IP to Block:", font=("Consolas", 10), text_color=ACCENT_YELLOW)
        lbl_add.pack(side="left", padx=4)

        self.entry_block_rule = ctk.CTkEntry(
            top_ctrl,
            width=240,
            placeholder_text="e.g. ads.example.com or 1.2.3.4",
            font=("Consolas", 10),
        )
        self.entry_block_rule.pack(side="left", padx=6)

        btn_add = ctk.CTkButton(
            top_ctrl,
            text="+ Add Block Rule",
            width=120,
            height=28,
            fg_color="#440011",
            hover_color="#660022",
            text_color=ACCENT_RED,
            font=("Consolas", 10, "bold"),
            command=self._on_add_rule_click,
        )
        btn_add.pack(side="left", padx=6)

        btn_del = ctk.CTkButton(
            top_ctrl,
            text="Remove Selected",
            width=110,
            height=28,
            fg_color="#333",
            hover_color="#444",
            text_color="#ccc",
            font=("Consolas", 10),
            command=self._on_remove_rule_click,
        )
        btn_del.pack(side="right", padx=6)

        list_frame = ctk.CTkFrame(frame, fg_color="transparent")
        list_frame.pack(fill="both", expand=True, padx=10, pady=4)

        lbl_header = ctk.CTkLabel(
            list_frame,
            text="CURRENT ACTIVE BLOCK RULES:",
            font=("Consolas", 10, "bold"),
            text_color=ACCENT_RED,
            anchor="w",
        )
        lbl_header.pack(fill="x", pady=(2, 4))

        self.rules_listbox = tk.Listbox(
            list_frame,
            bg="#111",
            fg=ACCENT_RED,
            selectbackground="#440011",
            font=("Consolas", 10),
            relief="flat",
            highlightthickness=1,
            highlightcolor="#333",
        )
        self.rules_listbox.pack(fill="both", expand=True)

    def _on_blocker_toggle(self):
        val = self.blocker_toggle_var.get()
        self.inspector.set_blocker_enabled(val)
        txt = "STATUS: ACTIVE" if val else "STATUS: BLOCKER OFF"
        col = ACCENT_GREEN if val else "gray"
        self.lbl_status.configure(text=txt, text_color=col)

    def _toggle_pause(self):
        self.paused = not self.paused
        self.btn_pause.configure(text="Resume" if self.paused else "Pause")

    def _clear_table(self):
        for item in self.tree.get_children():
            self.tree.delete(item)
        self.rendered_records_map.clear()
        self.last_rendered_index = len(self.inspector.records)

    def _on_device_changed(self, choice):
        self.filter_device = choice
        self._on_filter_changed()

    def _on_filter_changed(self, event=None):
        self.filter_text = self.entry_search.get().strip().lower()
        self.filter_blocked_only = self.chk_blocked_only_var.get()
        for item in self.tree.get_children():
            self.tree.delete(item)
        self.rendered_records_map.clear()
        self.last_rendered_index = 0
        self._render_records(full_refresh=True)

    def _periodic_update(self):
        if not self.winfo_exists():
            return
        if not self.paused:
            self._render_records()
        self.after(400, self._periodic_update)

    def _render_records(self, full_refresh=False):
        records = list(self.inspector.records)
        total = len(records)

        # Update detected device options in dropdown if new ones appear
        current_devices = set(self.opt_devices.cget("values"))
        all_clients = {r["client_ip"] for r in records if r["client_ip"] and r["client_ip"] != "-"}
        combined = ["All Devices"] + sorted(all_clients)
        if set(combined) != current_devices:
            self.opt_devices.configure(values=combined)

        start_idx = 0 if full_refresh else self.last_rendered_index
        new_items = records[start_idx:]
        filter_str = self.filter_text
        blocked_only = self.filter_blocked_only
        target_dev = self.filter_device

        for r in new_items:
            domain = r["domain"]
            client = r["client_ip"]
            remote = r["remote_ip"]
            is_blocked = r["blocked"]

            if blocked_only and not is_blocked:
                continue
            if target_dev != "All Devices" and client != target_dev:
                continue
            if filter_str:
                if filter_str not in domain.lower() and filter_str not in client and filter_str not in remote:
                    continue

            status_str = "[BLOCKED]" if is_blocked else "ALLOWED"
            if is_blocked:
                tag = "blocked"
            elif r["proto"] == "DNS":
                tag = "dns"
            elif r["proto"] == "HTTPS":
                tag = "https"
            elif r["proto"] == "HTTP":
                tag = "http"
            else:
                tag = "allowed"

            size_str = f"{r['size']} B" if r['size'] < 1024 else f"{r['size']/1024:.1f} KB"
            target = f"{remote}:{r['remote_port']}"

            item_id = self.tree.insert(
                "",
                0,
                values=(r["timestamp"], client, domain, target, r["proto"], size_str, status_str),
                tags=(tag,),
            )
            self.rendered_records_map[item_id] = r

        self.last_rendered_index = total

        children = self.tree.get_children()
        if len(children) > 800:
            for item in children[700:]:
                self.tree.delete(item)
                self.rendered_records_map.pop(item, None)

    def _on_tree_double_click(self, event):
        item_id = self.tree.identify_row(event.y)
        if item_id:
            record = self.rendered_records_map.get(item_id)
            if record:
                PacketPayloadViewerDialog(self, record)

    def _inspect_selected_packet(self):
        selected = self.tree.selection()
        if selected:
            record = self.rendered_records_map.get(selected[0])
            if record:
                PacketPayloadViewerDialog(self, record)

    def _on_tree_right_click(self, event):
        row_id = self.tree.identify_row(event.y)
        if row_id:
            self.tree.selection_set(row_id)
            self.menu.tk_popup(event.x_root, event.y_root)

    def _get_selected_row(self):
        selected = self.tree.selection()
        if not selected:
            return None
        return self.tree.item(selected[0])["values"]

    def _block_selected_domain(self):
        row = self._get_selected_row()
        if row and row[2] and row[2] != "-":
            domain = row[2]
            self.inspector.add_blocked_domain(domain)
            self._refresh_blocklist_ui()

    def _block_selected_client(self):
        row = self._get_selected_row()
        if row and row[1]:
            client = row[1]
            self.inspector.add_blocked_ip(client)
            self._refresh_blocklist_ui()

    def _block_selected_remote(self):
        row = self._get_selected_row()
        if row and row[3]:
            ip = row[3].split(':')[0]
            self.inspector.add_blocked_ip(ip)
            self._refresh_blocklist_ui()

    def _copy_selected_details(self):
        row = self._get_selected_row()
        if row:
            text = " | ".join(str(x) for x in row)
            self.clipboard_clear()
            self.clipboard_append(text)

    def _on_add_rule_click(self):
        rule = self.entry_block_rule.get().strip()
        if not rule:
            return
        if re.match(r'^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$', rule):
            self.inspector.add_blocked_ip(rule)
        else:
            self.inspector.add_blocked_domain(rule)
        self.entry_block_rule.delete(0, 'end')
        self._refresh_blocklist_ui()

    def _on_remove_rule_click(self):
        selected = self.rules_listbox.curselection()
        if not selected:
            return
        item_text = self.rules_listbox.get(selected[0])
        if item_text.startswith("[DOMAIN] "):
            d = item_text.replace("[DOMAIN] ", "")
            self.inspector.remove_blocked_domain(d)
        elif item_text.startswith("[IP] "):
            ip = item_text.replace("[IP] ", "")
            self.inspector.remove_blocked_ip(ip)
        self._refresh_blocklist_ui()

    def _refresh_blocklist_ui(self):
        self.rules_listbox.delete(0, 'end')
        for d in sorted(self.inspector.blocked_domains):
            self.rules_listbox.insert('end', f"[DOMAIN] {d}")
        for ip in sorted(self.inspector.blocked_ips):
            self.rules_listbox.insert('end', f"[IP] {ip}")
