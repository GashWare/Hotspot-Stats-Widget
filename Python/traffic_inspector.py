# File: traffic_inspector.py
# Description: Real-time network traffic inspector, DNS/TLS SNI parser, and domain/traffic blocker for Hotspot.

import threading
import time
import socket
import struct
import re
import subprocess
import collections
import platform
from datetime import datetime
from config import ACCENT_CYAN, ACCENT_RED, ACCENT_YELLOW, ACCENT_GREEN, BG_COLOR, GRAPH_BG

CREATE_NO_WINDOW = 0x08000000 if platform.system() == "Windows" else 0

class TrafficInspector:
    _instance = None
    _lock = threading.Lock()

    def __init__(self, config=None, save_config_callback=None):
        self.config = config or {}
        self.save_config_callback = save_config_callback
        
        self.running = False
        self.sniff_thread = None
        self.records = collections.deque(maxlen=2000) # Ring buffer of last 2000 packets/flows
        self.blocked_domains = set(self.config.get("blocked_domains", []))
        self.blocked_ips = set(self.config.get("blocked_ips", []))
        self.blocked_ports = set(self.config.get("blocked_ports", []))
        self.blocker_enabled = self.config.get("blocker_enabled", True)
        
        self.hotspot_ip = "192.168.137.1"
        self.subnet_prefix = "192.168.137."
        self.target_iface_name = None
        
        # Cache of domain -> resolved IPs
        self.domain_ip_cache = {}
        # Rate tracking per domain
        self.domain_stats = collections.defaultdict(lambda: {"packets": 0, "bytes": 0, "last_seen": 0})
        # Listeners for real-time UI updates
        self.ui_listeners = []

    @classmethod
    def get_instance(cls, config=None, save_callback=None):
        with cls._lock:
            if cls._instance is None:
                cls._instance = cls(config, save_callback)
            return cls._instance

    def set_hotspot_info(self, hotspot_ip, iface_name):
        self.hotspot_ip = hotspot_ip
        self.subnet_prefix = ".".join(hotspot_ip.split('.')[:3]) + "."
        self.target_iface_name = iface_name

    def register_listener(self, callback):
        if callback not in self.ui_listeners:
            self.ui_listeners.append(callback)

    def unregister_listener(self, callback):
        if callback in self.ui_listeners:
            self.ui_listeners.remove(callback)

    def is_domain_blocked(self, domain):
        if not self.blocker_enabled or not domain:
            return False
        domain_lower = domain.lower().strip('.')
        for b in self.blocked_domains:
            b_lower = b.lower().strip('.')
            if domain_lower == b_lower or domain_lower.endswith("." + b_lower):
                return True
        return False

    def is_ip_blocked(self, ip):
        if not self.blocker_enabled or not ip:
            return False
        return ip in self.blocked_ips

    def is_port_blocked(self, port):
        if not self.blocker_enabled or not port:
            return False
        return port in self.blocked_ports

    def add_blocked_domain(self, domain):
        clean = domain.strip().lower()
        if clean and clean not in self.blocked_domains:
            self.blocked_domains.add(clean)
            self._save_rules()
            self._apply_firewall_rule_for_domain(clean)
            self._notify_listeners()

    def remove_blocked_domain(self, domain):
        clean = domain.strip().lower()
        if clean in self.blocked_domains:
            self.blocked_domains.remove(clean)
            self._save_rules()
            self._remove_firewall_rule_for_domain(clean)
            self._notify_listeners()

    def add_blocked_ip(self, ip):
        clean = ip.strip()
        if clean and clean not in self.blocked_ips:
            self.blocked_ips.add(clean)
            self._save_rules()
            self._apply_firewall_rule_for_ip(clean)
            self._notify_listeners()

    def remove_blocked_ip(self, ip):
        clean = ip.strip()
        if clean in self.blocked_ips:
            self.blocked_ips.remove(clean)
            self._save_rules()
            self._remove_firewall_rule_for_ip(clean)
            self._notify_listeners()

    def set_blocker_enabled(self, enabled):
        self.blocker_enabled = enabled
        self._save_rules()
        self._notify_listeners()

    def _save_rules(self):
        if self.config is not None:
            self.config["blocked_domains"] = list(self.blocked_domains)
            self.config["blocked_ips"] = list(self.blocked_ips)
            self.config["blocked_ports"] = list(self.blocked_ports)
            self.config["blocker_enabled"] = self.blocker_enabled
            if self.save_config_callback:
                try:
                    self.save_config_callback()
                except Exception as e:
                    print(f"Error saving config in inspector: {e}")

    def _notify_listeners(self):
        for cb in list(self.ui_listeners):
            try:
                cb()
            except:
                pass

    def _apply_firewall_rule_for_domain(self, domain):
        # Resolve domain to IPs and add firewall block
        def _bg():
            try:
                ips = socket.gethostbyname_ex(domain)[2]
                for ip in ips:
                    self._apply_firewall_rule_for_ip(ip, f"NetMon_D_{domain}_{ip}")
            except:
                pass
        threading.Thread(target=_bg, daemon=True).start()

    def _remove_firewall_rule_for_domain(self, domain):
        def _bg():
            try:
                cmd = f'netsh advfirewall firewall delete rule name="all" dir=out profile=any description="NetMonitor:{domain}"'
                subprocess.run(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, creationflags=CREATE_NO_WINDOW)
            except:
                pass
        threading.Thread(target=_bg, daemon=True).start()

    def _apply_firewall_rule_for_ip(self, ip, rule_name=None):
        if not rule_name:
            rule_name = f"NetMon_IP_{ip}"
        def _bg():
            try:
                if platform.system() == "Windows":
                    cmd = f'netsh advfirewall firewall add rule name="{rule_name}" dir=out action=block remoteip={ip} profile=any'
                    subprocess.run(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, creationflags=CREATE_NO_WINDOW)
                elif platform.system() == "Linux":
                    subprocess.run(["iptables", "-I", "FORWARD", "-d", ip, "-j", "DROP"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    subprocess.run(["iptables", "-I", "OUTPUT", "-d", ip, "-j", "DROP"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except:
                pass
        threading.Thread(target=_bg, daemon=True).start()

    def _remove_firewall_rule_for_ip(self, ip, rule_name=None):
        if not rule_name:
            rule_name = f"NetMon_IP_{ip}"
        def _bg():
            try:
                if platform.system() == "Windows":
                    cmd = f'netsh advfirewall firewall delete rule name="{rule_name}"'
                    subprocess.run(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, creationflags=CREATE_NO_WINDOW)
                elif platform.system() == "Linux":
                    subprocess.run(["iptables", "-D", "FORWARD", "-d", ip, "-j", "DROP"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    subprocess.run(["iptables", "-D", "OUTPUT", "-d", ip, "-j", "DROP"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except:
                pass
        threading.Thread(target=_bg, daemon=True).start()

    def start(self):
        if self.running:
            return
        self.running = True
        self.sniff_thread = threading.Thread(target=self._sniff_loop, daemon=True)
        self.sniff_thread.start()

    def stop(self):
        self.running = False

    def _extract_tls_sni(self, payload):
        """
        Parses TLS ClientHello and extracts Server Name Indication (SNI).
        Works on raw TCP payload for port 443 without needing Scapy TLS parser.
        """
        try:
            if len(payload) < 43:
                return None
            # Check for TLS Handshake (0x16) and Client Hello (0x01)
            content_type = payload[0]
            if content_type != 0x16:
                return None
            
            # Version check (TLS 1.0 - 1.3: 0x0301 - 0x0303)
            handshake_type = payload[5]
            if handshake_type != 0x01: # ClientHello
                return None
            
            # Skip session ID
            offset = 43 # 5 header + 4 handshake header + 2 version + 32 random
            if len(payload) < offset + 1:
                return None
            session_id_len = payload[offset]
            offset += 1 + session_id_len
            
            # Skip cipher suites
            if len(payload) < offset + 2:
                return None
            cipher_suites_len = struct.unpack("!H", payload[offset:offset+2])[0]
            offset += 2 + cipher_suites_len
            
            # Skip compression methods
            if len(payload) < offset + 1:
                return None
            comp_methods_len = payload[offset]
            offset += 1 + comp_methods_len
            
            # Extensions
            if len(payload) < offset + 2:
                return None
            ext_total_len = struct.unpack("!H", payload[offset:offset+2])[0]
            offset += 2
            
            end_offset = offset + ext_total_len
            while offset + 4 <= end_offset and offset + 4 <= len(payload):
                ext_type = struct.unpack("!H", payload[offset:offset+2])[0]
                ext_len = struct.unpack("!H", payload[offset+2:offset+4])[0]
                offset += 4
                if ext_type == 0: # server_name (SNI)
                    if offset + ext_len <= len(payload):
                        # server_name_list_length
                        sn_list_len = struct.unpack("!H", payload[offset:offset+2])[0]
                        # server_name type (0 = host_name)
                        sn_type = payload[offset+2]
                        if sn_type == 0:
                            sn_len = struct.unpack("!H", payload[offset+3:offset+5])[0]
                            host = payload[offset+5:offset+5+sn_len].decode('utf-8', errors='ignore')
                            return host
                    break
                offset += ext_len
        except:
            pass
        return None

    def _extract_http_host(self, payload):
        """Extracts HTTP Host header from TCP payload."""
        try:
            if payload.startswith(b"GET ") or payload.startswith(b"POST ") or payload.startswith(b"HEAD ") or payload.startswith(b"CONNECT "):
                text = payload[:1024].decode('utf-8', errors='ignore')
                match = re.search(r'(?i)Host:\s*([^\r\n]+)', text)
                if match:
                    return match.group(1).strip()
        except:
            pass
        return None

    def _extract_dns_query(self, payload):
        """Extracts queried domain from standard DNS query packet."""
        try:
            # DNS header is 12 bytes
            if len(payload) < 13:
                return None
            # Questions count
            qdcount = struct.unpack("!H", payload[4:6])[0]
            if qdcount == 0:
                return None
            offset = 12
            labels = []
            while offset < len(payload):
                length = payload[offset]
                if length == 0:
                    break
                offset += 1
                if offset + length > len(payload):
                    return None
                labels.append(payload[offset:offset+length].decode('utf-8', errors='ignore'))
                offset += length
            if labels:
                return ".".join(labels)
        except:
            pass
        return None

    def _sniff_loop(self):
        try:
            from scapy.all import sniff, IP, TCP, UDP, DNS, DNSQR, conf
            from scapy.arch.windows import get_windows_if_list
        except ImportError:
            print("TrafficInspector: Scapy not installed. Traffic inspector inactive.")
            return

        # Find interface
        scapy_iface = None
        try:
            for iface in get_windows_if_list():
                for addr in iface.get('ips', []):
                    if addr == self.hotspot_ip:
                        scapy_iface = iface['name']
                        break
                if scapy_iface:
                    break
                if self.target_iface_name and (self.target_iface_name in iface['name'] or self.target_iface_name in iface.get('description', '')):
                    scapy_iface = iface['name']
                    break
        except:
            pass

        if not scapy_iface:
            scapy_iface = conf.iface

        print(f"TrafficInspector: Listening on {scapy_iface} for hotspot {self.subnet_prefix}*")

        def _packet_handler(pkt):
            if not pkt.haslayer(IP):
                return
            
            ip_layer = pkt[IP]
            src_ip = ip_layer.src
            dst_ip = ip_layer.dst
            pkt_len = len(pkt)
            
            # Check if this traffic belongs to the hotspot
            is_client_src = src_ip.startswith(self.subnet_prefix) and src_ip != self.hotspot_ip
            is_client_dst = dst_ip.startswith(self.subnet_prefix) and dst_ip != self.hotspot_ip
            
            if not (is_client_src or is_client_dst):
                return

            client_ip = src_ip if is_client_src else dst_ip
            remote_ip = dst_ip if is_client_src else src_ip
            
            proto_name = "IP"
            remote_port = 0
            domain = None
            is_blocked = False

            # Check if remote or client IP is explicitly blocked
            if self.is_ip_blocked(client_ip) or self.is_ip_blocked(remote_ip):
                is_blocked = True

            # DNS Inspection
            if pkt.haslayer(UDP) and (pkt[UDP].dport == 53 or pkt[UDP].sport == 53):
                proto_name = "DNS"
                remote_port = 53
                if pkt.haslayer(DNS) and pkt.haslayer(DNSQR):
                    qname = pkt[DNSQR].qname
                    if isinstance(qname, bytes):
                        domain = qname.decode('utf-8', errors='ignore').rstrip('.')
                    elif isinstance(qname, str):
                        domain = qname.rstrip('.')
                else:
                    try:
                        raw_payload = bytes(pkt[UDP].payload)
                        domain = self._extract_dns_query(raw_payload)
                    except:
                        pass
                
                if domain and self.is_domain_blocked(domain):
                    is_blocked = True

            # TCP Inspection (HTTP / HTTPS / TLS SNI)
            elif pkt.haslayer(TCP):
                tcp = pkt[TCP]
                remote_port = tcp.dport if is_client_src else tcp.sport
                if self.is_port_blocked(remote_port):
                    is_blocked = True
                
                try:
                    payload = bytes(tcp.payload)
                    if remote_port == 443:
                        proto_name = "HTTPS"
                        sni = self._extract_tls_sni(payload)
                        if sni:
                            domain = sni
                            if self.is_domain_blocked(domain):
                                is_blocked = True
                    elif remote_port == 80:
                        proto_name = "HTTP"
                        h = self._extract_http_host(payload)
                        if h:
                            domain = h
                            if self.is_domain_blocked(domain):
                                is_blocked = True
                    else:
                        proto_name = f"TCP/{remote_port}"
                except:
                    proto_name = f"TCP/{remote_port}"

            elif pkt.haslayer(UDP):
                udp = pkt[UDP]
                remote_port = udp.dport if is_client_src else udp.sport
                proto_name = f"UDP/{remote_port}"
                if self.is_port_blocked(remote_port):
                    is_blocked = True

            # Capture raw frame payload preview (capped at 512 bytes)
            raw_payload = bytes(pkt)[:512]

            # Record entry
            timestamp = datetime.now().strftime("%H:%M:%S")
            record = {
                "timestamp": timestamp,
                "client_ip": client_ip,
                "domain": domain or "-",
                "remote_ip": remote_ip,
                "remote_port": remote_port,
                "proto": proto_name,
                "size": pkt_len,
                "blocked": is_blocked,
                "payload": raw_payload
            }
            self.records.append(record)
            
            if domain:
                self.domain_stats[domain]["packets"] += 1
                self.domain_stats[domain]["bytes"] += pkt_len
                self.domain_stats[domain]["last_seen"] = time.time()

        while self.running:
            try:
                sniff(iface=scapy_iface, prn=_packet_handler, timeout=1, store=0)
            except Exception as e:
                time.sleep(1)

    @staticmethod
    def format_hex_dump(payload):
        if not payload:
            return "[No raw payload data captured for this frame]"
        lines = []
        for i in range(0, len(payload), 16):
            chunk = payload[i:i+16]
            hex_part = " ".join(f"{b:02x}" for b in chunk)
            if len(chunk) < 16:
                hex_part = hex_part.ljust(16 * 3)
            if len(hex_part) > 24:
                hex_part = hex_part[:24] + " " + hex_part[24:]
            ascii_part = "".join(chr(b) if 32 <= b <= 126 else "." for b in chunk)
            lines.append(f"{i:04x}  {hex_part:<49} |{ascii_part}|")
        return "\n".join(lines)

    @staticmethod
    def format_ascii_text(payload):
        if not payload:
            return "[No raw payload data captured for this frame]"
        chars = []
        for b in payload:
            if b in (ord('\n'), ord('\r'), ord('\t')):
                chars.append(chr(b))
            elif 32 <= b <= 126:
                chars.append(chr(b))
            else:
                chars.append('.')
        return "".join(chars)
