# File: backend.py
# Description: Optimized core logic for Hardware stats, Hotspot control, Network scanning (ARP), and an Nmap/Native Port Scanner.

import psutil
import subprocess
import platform
import socket
import threading
import time
import shutil
import asyncio
import re
import concurrent.futures
from utils import log_error

# Constants for silent subprocess execution
CREATE_NO_WINDOW = 0x08000000

# Winsdk Import
try:
    import winsdk.windows.networking.connectivity as network_info
    import winsdk.windows.networking.networkoperators as network_ops
    from winsdk.windows.networking.networkoperators import NetworkOperatorTetheringManager, TetheringOperationalState
except ImportError as e:
    log_error(f"Winsdk Error: {e}")
    # Defines dummy class to prevent crashes if winsdk is missing
    class TetheringOperationalState:
        ON = 2
        OFF = 1
        IN_TRANSITION = 3
        UNKNOWN = 0

class HardwareBackend:
    _cpu_name_cache = None
    _temp_cache = ""
    _last_temp_fetch = 0.0

    @staticmethod
    def get_cpu_name(callback):
        def _fetch():
            try:
                startupinfo = subprocess.STARTUPINFO()
                startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                result = subprocess.run(
                    ["powershell", "-Command", "Get-CimInstance Win32_Processor | Select-Object -ExpandProperty Name"],
                    capture_output=True, text=True, startupinfo=startupinfo, creationflags=CREATE_NO_WINDOW
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
        # Optimization: Spawning PowerShell every 1 sec is heavy. Cache result for 3 seconds.
        current_time = time.time()
        if current_time - HardwareBackend._last_temp_fetch < 3.0:
            return HardwareBackend._temp_cache

        script = "Get-CimInstance -Namespace 'root/wmi' -ClassName MSAcpi_ThermalZoneTemperature | Select-Object -ExpandProperty CurrentTemperature"
        try:
            startupinfo = subprocess.STARTUPINFO()
            startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            result = subprocess.run(
                ["powershell", "-Command", script],
                capture_output=True, text=True, startupinfo=startupinfo, creationflags=CREATE_NO_WINDOW
            )
            val = result.stdout.strip()
            if val and val.isdigit():
                temp_c = (int(val) - 2732) / 10.0
                HardwareBackend._temp_cache = f"{temp_c:.0f}\u00b0C"
                HardwareBackend._last_temp_fetch = current_time
                return HardwareBackend._temp_cache
        except: pass
        
        return HardwareBackend._temp_cache

class HotspotDirect:
    _clients_cache = [] # List of dicts: {'ip':, 'mac':, 'hostname':, 'down_bps':, 'up_bps':, 'total_bytes':}
    _last_scan_time = 0
    _is_scanning = False
    _usage_stats = {} # mac -> {'down_total':, 'up_total':, 'last_down':, 'last_up':, 'last_tick':}
    _monitor_thread = None
    _stop_monitor = False

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
        # Source of truth: Check if the Hotspot IP is actively bound to an adapter
        has_ip = False
        try:
            addrs = psutil.net_if_addrs()
            for name, snics in addrs.items():
                for snic in snics:
                    if snic.family == socket.AF_INET:
                        if snic.address == "192.168.137.1" or ("*" in name and snic.address.startswith("192.168.")):
                            has_ip = True
                            break
                if has_ip: break
        except: pass

        if has_ip: return TetheringOperationalState.ON

        # Fallback to Windows API
        manager, err = HotspotDirect.get_manager()
        if not manager: return 0
        return manager.tethering_operational_state

    @staticmethod
    def get_clients():
        # Start traffic monitor if not running (crucial for startup sync)
        if not HotspotDirect._monitor_thread or not HotspotDirect._monitor_thread.is_alive():
            HotspotDirect._stop_monitor = False
            HotspotDirect._monitor_thread = threading.Thread(target=HotspotDirect._monitor_traffic, daemon=True)
            HotspotDirect._monitor_thread.start()

        # Trigger background scan if cache is stale (>15s) or empty
        if not HotspotDirect._is_scanning and (time.time() - HotspotDirect._last_scan_time > 15 or not HotspotDirect._clients_cache):
            threading.Thread(target=HotspotDirect._refresh_clients, daemon=True).start()
        
        # Explicit status messages for the UI
        if HotspotDirect._is_scanning and not HotspotDirect._clients_cache:
             return [{"ip": "Scanning...", "mac": "00-00-00-00-00-00", "hostname": "Please wait", "down_bps": 0, "up_bps": 0}]
        
        return HotspotDirect._clients_cache

    @staticmethod
    def _refresh_clients():
        print("DEBUG: --- STARTING SCAN ---")
        HotspotDirect._is_scanning = True
        
        # 1. Subnet Detection
        hotspot_ip = "192.168.137.1"
        try:
            addrs = psutil.net_if_addrs()
            for name, snics in addrs.items():
                for snic in snics:
                    if snic.family == socket.AF_INET:
                        if snic.address == "192.168.137.1":
                            hotspot_ip = snic.address; break
                        if ("Local Area Connection" in name or "Microsoft" in name or "Wi-Fi Direct" in name) and "*" in name:
                             if not snic.address.startswith("169.254"):
                                 hotspot_ip = snic.address; break
        except: pass
        
        subnet_prefix = ".".join(hotspot_ip.split('.')[:3]) + "."
        print(f"DEBUG: Hotspot IP: {hotspot_ip}, Subnet: {subnet_prefix}x")

        # 2. Ping Sweep
        def ping_ip(ip_addr):
            startupinfo = subprocess.STARTUPINFO()
            startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            try:
                subprocess.call(['ping', '-n', '1', '-w', '150', ip_addr], 
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, 
                                creationflags=CREATE_NO_WINDOW, startupinfo=startupinfo)
            except: pass

        targets = [f"{subnet_prefix}{i}" for i in range(2, 255) if f"{subnet_prefix}{i}" != hotspot_ip]
        with concurrent.futures.ThreadPoolExecutor(max_workers=100) as executor:
            list(executor.map(ping_ip, targets))

        # 3. Discovery & Filtering
        discovered = {} # mac -> dict
        
        # WinSDK (Hostnames)
        try:
            manager, err = HotspotDirect.get_manager()
            if manager:
                clients = manager.get_tethering_clients()
                if clients:
                    for c in clients:
                        mac = str(c.mac_address).upper().replace(":", "-")
                        hostname = c.host_names[0].canonical_name if c.host_names else "Unknown"
                        discovered[mac] = {"ip": "Detecting...", "mac": mac, "hostname": hostname}
        except: pass

        # ARP (IPs)
        try:
            startupinfo = subprocess.STARTUPINFO()
            startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            output = subprocess.check_output(["arp", "-a"], startupinfo=startupinfo, creationflags=CREATE_NO_WINDOW).decode('cp437', errors='ignore')
            pattern = re.compile(fr"({re.escape(subnet_prefix)}\d+)\s+([a-fA-F0-9-:]+)\s+(dynamic|static)", re.IGNORECASE)
            for ip, mac, etype in pattern.findall(output):
                mac = mac.upper().replace(":", "-")
                if ip != hotspot_ip:
                    if mac not in discovered:
                        discovered[mac] = {"ip": ip, "mac": mac, "hostname": "Unknown"}
                    else:
                        discovered[mac]["ip"] = ip
        except: pass

        # PowerShell (Reliability)
        try:
            ps_cmd = "Get-NetNeighbor -AddressFamily IPv4 | Where-Object { $_.State -ne 'Unreachable' } | Select-Object IPAddress, LinkLayerAddress | ConvertTo-Json"
            startupinfo = subprocess.STARTUPINFO()
            startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            ps_out = subprocess.check_output(["powershell", "-Command", ps_cmd], startupinfo=startupinfo, creationflags=CREATE_NO_WINDOW).decode('utf-8', errors='ignore')
            if ps_out.strip():
                import json
                data = json.loads(ps_out)
                if isinstance(data, dict): data = [data]
                for n in data:
                    ip = n.get("IPAddress", "")
                    mac = (n.get("LinkLayerAddress") or "").upper().replace(":", "-")
                    if ip.startswith(subnet_prefix) and ip != hotspot_ip:
                        if mac not in discovered:
                            discovered[mac] = {"ip": ip, "mac": mac, "hostname": "Unknown"}
                        else:
                            discovered[mac]["ip"] = ip
        except: pass

        # 4. Final Filtering (Surgical)
        final_list = []
        for mac, info in discovered.items():
            ip = info["ip"]
            # Exclude only what the user requested + broadcast/unresolved
            if ip in ["Detecting...", hotspot_ip] or ip.endswith(".255"): continue
            if mac in ["00-00-00-00-00-00", "FF-FF-FF-FF-FF-FF"]: continue
            
            # Add usage
            stats = HotspotDirect._usage_stats.get(mac, {"down_bps": 0, "up_bps": 0, "total": 0})
            info.update(stats)
            final_list.append(info)

        HotspotDirect._clients_cache = final_list
        HotspotDirect._last_scan_time = time.time()
        HotspotDirect._is_scanning = False
        print(f"DEBUG: Scan Complete. Found {len(final_list)} clients.")


    @staticmethod
    def _monitor_traffic():
        """
        Background thread to monitor per-client traffic.
        Uses Scapy for accurate per-IP tracking.
        """
        try:
            from scapy.all import sniff, IP, conf
        except ImportError:
            print("DEBUG: Scapy not installed. Per-client traffic monitoring disabled.")
            return

        # 1. Re-run interface/subnet detection to match discovery
        hotspot_ip = None
        target_iface_name = None
        
        try:
            addrs = psutil.net_if_addrs()
            for name, snics in addrs.items():
                for snic in snics:
                    if snic.family == socket.AF_INET:
                        if snic.address == "192.168.137.1":
                            hotspot_ip = snic.address
                            target_iface_name = name
                            break
                        if ("Local Area Connection" in name or "Microsoft" in name or "Wi-Fi Direct" in name) and "*" in name:
                             if not snic.address.startswith("169.254"):
                                 hotspot_ip = snic.address
                                 target_iface_name = name
                                 break
                if hotspot_ip: break
        except: pass

        if not hotspot_ip: hotspot_ip = "192.168.137.1"
        subnet_prefix = ".".join(hotspot_ip.split('.')[:3]) + "."

        # 2. Find the correct Scapy interface
        scapy_iface = None
        try:
            from scapy.arch.windows import get_windows_if_list
            for iface in get_windows_if_list():
                # Match by IP or by the name from psutil
                for addr in iface.get('ips', []):
                    if addr == hotspot_ip:
                        scapy_iface = iface['name']
                        break
                if scapy_iface: break
                
                # Fallback: Match by name (psutil name is often part of scapy description/name)
                if target_iface_name and (target_iface_name in iface['name'] or target_iface_name in iface['description']):
                    scapy_iface = iface['name']
                    break
        except: pass

        if not scapy_iface:
            scapy_iface = conf.iface
            print(f"DEBUG: Could not find specific hotspot interface. Defaulting to {scapy_iface}")

        print(f"DEBUG: Starting Scapy sniffer on {scapy_iface} (Subnet: {subnet_prefix}x)...")
        
        temp_stats = {} # IP -> {'in':, 'out':}
        last_calc_time = time.time()

        def process_packet(pkt):
            if pkt.haslayer(IP):
                src = pkt[IP].src
                dst = pkt[IP].dst
                size = len(pkt)
                
                # Check if src or dst is in our hotspot subnet
                if src.startswith(subnet_prefix):
                    if src not in temp_stats: temp_stats[src] = {'in': 0, 'out': 0}
                    temp_stats[src]['out'] += size
                if dst.startswith(subnet_prefix):
                    if dst not in temp_stats: temp_stats[dst] = {'in': 0, 'out': 0}
                    temp_stats[dst]['in'] += size

        # Run sniffer in a loop
        while not HotspotDirect._stop_monitor:
            try:
                # Sniff for a short burst
                sniff(iface=scapy_iface, prn=process_packet, timeout=2, store=0)
                
                # Calculate rates
                now = time.time()
                dt = now - last_calc_time
                if dt <= 0: dt = 1.0
                
                # Update global stats
                for c in HotspotDirect._clients_cache:
                    ip = c.get("ip")
                    mac = c.get("mac")
                    if ip and ip in temp_stats:
                        down_bps = temp_stats[ip]['in'] / dt
                        up_bps = temp_stats[ip]['out'] / dt
                        
                        if mac not in HotspotDirect._usage_stats:
                            HotspotDirect._usage_stats[mac] = {"down_bps": 0, "up_bps": 0, "total": 0}
                        
                        HotspotDirect._usage_stats[mac]["down_bps"] = down_bps
                        HotspotDirect._usage_stats[mac]["up_bps"] = up_bps
                        HotspotDirect._usage_stats[mac]["total"] += (temp_stats[ip]['in'] + temp_stats[ip]['out'])
                
                temp_stats = {}
                last_calc_time = now
            except Exception as e:
                print(f"DEBUG: Sniffer Loop Error: {e}")
                time.sleep(2)


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
    COMMON_PORTS = [21, 22, 23, 25, 53, 80, 110, 135, 139, 443, 445, 3389, 8080, 8443]

    @staticmethod
    def scan_device(ip_str, callback_fn):
        ip = ip_str.split(' ')[0]
        
        def _scan():
            # If nmap is installed, use it for thoroughness
            if shutil.which("nmap"):
                try:
                    startupinfo = subprocess.STARTUPINFO()
                    startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                    cmd = ["nmap", "-F", "--open", ip]
                    result = subprocess.run(cmd, capture_output=True, text=True, startupinfo=startupinfo, creationflags=CREATE_NO_WINDOW)
                    
                    open_ports = []
                    for line in result.stdout.splitlines():
                        if "/tcp" in line and "open" in line:
                            open_ports.append(line.split("/")[0])
                    
                    if open_ports: callback_fn(ip_str, f"[{', '.join(open_ports)}]")
                    else: callback_fn(ip_str, "[No Open Ports]")
                    return
                except: pass

            # FALLBACK: Native Python Socket Scan (Concurrent/Fast)
            print(f"DEBUG: Nmap not found. Using high-speed native socket scanner for {ip}")
            
            def check_port(port):
                with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
                    sock.settimeout(0.15) # Very short timeout for local network
                    if sock.connect_ex((ip, port)) == 0:
                        return str(port)
                return None

            open_ports = []
            with concurrent.futures.ThreadPoolExecutor(max_workers=len(PortScanner.COMMON_PORTS)) as executor:
                results = executor.map(check_port, PortScanner.COMMON_PORTS)
                open_ports = [p for p in results if p]
            
            if open_ports: callback_fn(ip_str, f"[{', '.join(open_ports)}]")
            else: callback_fn(ip_str, "[No Open Ports]")

        threading.Thread(target=_scan, daemon=True).start()