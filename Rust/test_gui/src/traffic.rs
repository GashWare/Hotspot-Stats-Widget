use chrono::Local;
use std::collections::{HashSet, VecDeque};
use std::net::ToSocketAddrs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::geo::GeoResolver;
use crate::logger::{log_info, log_success, log_warn};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TrafficRecord {
    pub timestamp: String,
    pub client_ip: String,
    pub domain: String,
    pub remote_ip: String,
    pub remote_port: u16,
    pub proto: String,
    pub size: usize,
    pub blocked: bool,
    pub info: String,
    pub payload: Vec<u8>,
    pub location: String,
    pub company: String,
    pub lat: f32,
    pub lon: f32,
}

impl TrafficRecord {
    pub fn format_hex_dump(&self) -> String {
        if self.payload.is_empty() {
            return "[No raw payload captured for this frame]".to_string();
        }
        let mut out = String::new();
        for (i, chunk) in self.payload.chunks(16).enumerate() {
            let offset = i * 16;
            out.push_str(&format!("{:04x}  ", offset));

            for (j, b) in chunk.iter().enumerate() {
                out.push_str(&format!("{:02x} ", b));
                if j == 7 {
                    out.push(' ');
                }
            }
            if chunk.len() < 16 {
                let missing = 16 - chunk.len();
                for j in 0..missing {
                    out.push_str("   ");
                    if chunk.len() + j == 7 {
                        out.push(' ');
                    }
                }
            }
            out.push_str(" |");
            for b in chunk {
                if *b >= 32 && *b <= 126 {
                    out.push(*b as char);
                } else {
                    out.push('.');
                }
            }
            out.push_str("|\n");
        }
        out
    }

    pub fn format_ascii_text(&self) -> String {
        if self.payload.is_empty() {
            return "[No payload data captured]".to_string();
        }
        let mut out = String::new();
        for &b in &self.payload {
            if b == b'\n' || b == b'\r' || b == b'\t' {
                out.push(b as char);
            } else if b >= 32 && b <= 126 {
                out.push(b as char);
            } else {
                out.push('.');
            }
        }
        out
    }
}

pub struct TrafficInspector {
    pub records: Arc<Mutex<VecDeque<TrafficRecord>>>,
    pub blocked_domains: Arc<Mutex<HashSet<String>>>,
    pub blocked_ips: Arc<Mutex<HashSet<String>>>,
    pub blocker_enabled: Arc<Mutex<bool>>,
    pub is_sniffing: Arc<Mutex<bool>>,
    pub hotspot_ip: Arc<Mutex<String>>,
    pub geo_resolver: GeoResolver,
}

// C-compatible structures for dynamic Npcap / libpcap loading
#[repr(C)]
struct PcapIf {
    next: *mut PcapIf,
    name: *const std::ffi::c_char,
    description: *const std::ffi::c_char,
    addresses: *mut std::ffi::c_void,
    flags: u32,
}

#[repr(C)]
struct TimeVal {
    tv_sec: i32,
    tv_usec: i32,
}

#[repr(C)]
struct PcapPkthdr {
    ts: TimeVal,
    caplen: u32,
    len: u32,
}

struct PcapLib {
    handle: *mut std::ffi::c_void,
    findalldevs: unsafe extern "C" fn(*mut *mut PcapIf, *mut std::ffi::c_char) -> i32,
    freealldevs: unsafe extern "C" fn(*mut PcapIf),
    open_live: unsafe extern "C" fn(*const std::ffi::c_char, i32, i32, i32, *mut std::ffi::c_char) -> *mut std::ffi::c_void,
    next_ex: unsafe extern "C" fn(*mut std::ffi::c_void, *mut *mut PcapPkthdr, *mut *const u8) -> i32,
    close: unsafe extern "C" fn(*mut std::ffi::c_void),
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn LoadLibraryA(lpLibFileName: *const u8) -> *mut std::ffi::c_void;
    fn GetProcAddress(hModule: *mut std::ffi::c_void, lpProcName: *const u8) -> *mut std::ffi::c_void;
    fn FreeLibrary(hModule: *mut std::ffi::c_void) -> i32;
}

#[cfg(not(target_os = "windows"))]
unsafe extern "C" {
    fn dlopen(filename: *const u8, flags: i32) -> *mut std::ffi::c_void;
    fn dlsym(handle: *mut std::ffi::c_void, symbol: *const u8) -> *mut std::ffi::c_void;
    fn dlclose(handle: *mut std::ffi::c_void) -> i32;
}

#[allow(dead_code, unsafe_op_in_unsafe_fn)]
impl PcapLib {
    unsafe fn load() -> Option<Self> {
        #[cfg(target_os = "windows")]
        {
            let mut h = LoadLibraryA(b"wpcap.dll\0".as_ptr());
            if h.is_null() {
                h = LoadLibraryA(b"C:\\Windows\\System32\\Npcap\\wpcap.dll\0".as_ptr());
            }
            if h.is_null() {
                return None;
            }

            let f_findalldevs = GetProcAddress(h, b"pcap_findalldevs\0".as_ptr());
            let f_freealldevs = GetProcAddress(h, b"pcap_freealldevs\0".as_ptr());
            let f_open_live = GetProcAddress(h, b"pcap_open_live\0".as_ptr());
            let f_next_ex = GetProcAddress(h, b"pcap_next_ex\0".as_ptr());
            let f_close = GetProcAddress(h, b"pcap_close\0".as_ptr());

            if f_findalldevs.is_null()
                || f_freealldevs.is_null()
                || f_open_live.is_null()
                || f_next_ex.is_null()
                || f_close.is_null()
            {
                FreeLibrary(h);
                return None;
            }

            Some(PcapLib {
                handle: h,
                findalldevs: std::mem::transmute(f_findalldevs),
                freealldevs: std::mem::transmute(f_freealldevs),
                open_live: std::mem::transmute(f_open_live),
                next_ex: std::mem::transmute(f_next_ex),
                close: std::mem::transmute(f_close),
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut h = dlopen(b"libpcap.so.1\0".as_ptr(), 1);
            if h.is_null() {
                h = dlopen(b"libpcap.so\0".as_ptr(), 1);
            }
            if h.is_null() {
                return None;
            }

            let f_findalldevs = dlsym(h, b"pcap_findalldevs\0".as_ptr());
            let f_freealldevs = dlsym(h, b"pcap_freealldevs\0".as_ptr());
            let f_open_live = dlsym(h, b"pcap_open_live\0".as_ptr());
            let f_next_ex = dlsym(h, b"pcap_next_ex\0".as_ptr());
            let f_close = dlsym(h, b"pcap_close\0".as_ptr());

            if f_findalldevs.is_null()
                || f_freealldevs.is_null()
                || f_open_live.is_null()
                || f_next_ex.is_null()
                || f_close.is_null()
            {
                dlclose(h);
                return None;
            }

            Some(PcapLib {
                handle: h,
                findalldevs: std::mem::transmute(f_findalldevs),
                freealldevs: std::mem::transmute(f_freealldevs),
                open_live: std::mem::transmute(f_open_live),
                next_ex: std::mem::transmute(f_next_ex),
                close: std::mem::transmute(f_close),
            })
        }
    }
}

impl Drop for PcapLib {
    fn drop(&mut self) {
        unsafe {
            #[cfg(target_os = "windows")]
            FreeLibrary(self.handle);
            #[cfg(not(target_os = "windows"))]
            dlclose(self.handle);
        }
    }
}

#[allow(dead_code)]
impl TrafficInspector {
    pub fn new(initial_domains: Vec<String>, initial_ips: Vec<String>, blocker_enabled: bool) -> Self {
        let records = Arc::new(Mutex::new(VecDeque::with_capacity(2000)));
        let blocked_domains = Arc::new(Mutex::new(initial_domains.into_iter().collect()));
        let blocked_ips = Arc::new(Mutex::new(initial_ips.into_iter().collect()));
        let blocker_enabled = Arc::new(Mutex::new(blocker_enabled));
        let is_sniffing = Arc::new(Mutex::new(false));
        let hotspot_ip = Arc::new(Mutex::new("192.168.137.1".to_string()));
        let geo_resolver = GeoResolver::new();

        Self {
            records,
            blocked_domains,
            blocked_ips,
            blocker_enabled,
            is_sniffing,
            hotspot_ip,
            geo_resolver,
        }
    }

    pub fn set_hotspot_ip(&self, ip: String) {
        *self.hotspot_ip.lock().unwrap() = ip;
    }

    pub fn trigger_probe(&self, target_ip: &str) {
        let ip_str = target_ip.to_string();
        log_info("TRAFFIC", &format!("Triggering live probe packets for device: {}", ip_str));
        thread::spawn(move || {
            // Send lightweight DNS probe and ping to generate network traffic
            let _ = std::net::UdpSocket::bind("0.0.0.0:0").and_then(|s| {
                let _ = s.set_read_timeout(Some(std::time::Duration::from_millis(100)));
                s.send_to(b"\x00\x00\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00", format!("{}:53", ip_str))
            });
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                let _ = Command::new("ping")
                    .args(["-n", "1", "-w", "250", &ip_str])
                    .creation_flags(0x08000000)
                    .output();
            }
        });
    }

    pub fn start(&self) {
        let mut sniffing_lock = self.is_sniffing.lock().unwrap();
        if *sniffing_lock {
            return;
        }
        *sniffing_lock = true;

        let records_clone = Arc::clone(&self.records);
        let domains_clone = Arc::clone(&self.blocked_domains);
        let ips_clone = Arc::clone(&self.blocked_ips);
        let blocker_clone = Arc::clone(&self.blocker_enabled);
        let sniffing_flag = Arc::clone(&self.is_sniffing);
        let hs_ip_clone = Arc::clone(&self.hotspot_ip);
        let geo_clone = self.geo_resolver.clone();

        thread::spawn(move || {
            let pcap_opt = unsafe { PcapLib::load() };
            let pcap = match pcap_opt {
                Some(p) => {
                    log_success("TRAFFIC", "Npcap / libpcap dynamic library loaded successfully.");
                    p
                }
                None => {
                    log_warn("TRAFFIC", "Npcap packet capture library not found. Install Npcap for live packet sniffing.");
                    let mut recs = records_clone.lock().unwrap();
                    recs.push_back(TrafficRecord {
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        client_ip: "-".to_string(),
                        domain: "Notice: Npcap packet capture library not found".to_string(),
                        remote_ip: "-".to_string(),
                        remote_port: 0,
                        proto: "SYSTEM".to_string(),
                        size: 0,
                        blocked: false,
                        info: "Install Npcap or run as Administrator for live packet sniffing".to_string(),
                        payload: Vec::new(),
                        location: "-".to_string(),
                        company: "-".to_string(),
                        lat: 0.0,
                        lon: 0.0,
                    });
                    *sniffing_flag.lock().unwrap() = false;
                    return;
                }
            };

            while *sniffing_flag.lock().unwrap() {
                let hs_ip = hs_ip_clone.lock().unwrap().clone();
                let prefix = format!(
                    "{}.",
                    hs_ip.rsplit_once('.').map(|(p, _)| p).unwrap_or("192.168.137")
                );

                // Enumerate devices via pcap_findalldevs
                let mut alldevs: *mut PcapIf = std::ptr::null_mut();
                let mut errbuf = [0 as std::ffi::c_char; 256];
                let ret = unsafe { (pcap.findalldevs)(&mut alldevs, errbuf.as_mut_ptr()) };

                if ret != 0 || alldevs.is_null() {
                    log_warn("TRAFFIC", "pcap_findalldevs returned no interfaces or error. Retrying in 3s...");
                    thread::sleep(std::time::Duration::from_secs(3));
                    continue;
                }

                let mut chosen_dev = String::new();
                let mut chosen_desc = String::new();
                let mut fallback_dev = String::new();
                let mut fallback_desc = String::new();
                let mut curr = alldevs;

                unsafe {
                    while !curr.is_null() {
                        let dev_name = if !(*curr).name.is_null() {
                            std::ffi::CStr::from_ptr((*curr).name)
                                .to_string_lossy()
                                .into_owned()
                        } else {
                            String::new()
                        };

                        let dev_desc = if !(*curr).description.is_null() {
                            std::ffi::CStr::from_ptr((*curr).description)
                                .to_string_lossy()
                                .into_owned()
                        } else {
                            String::new()
                        };

                        if fallback_dev.is_empty() && !dev_name.is_empty() && !dev_desc.contains("Loopback") {
                            fallback_dev = dev_name.clone();
                            fallback_desc = dev_desc.clone();
                        }

                        // Prioritize Hotspot Virtual Adapter / Wi-Fi Direct
                        let desc_lower = dev_desc.to_lowercase();
                        let name_lower = dev_name.to_lowercase();
                        if desc_lower.contains("wi-fi direct")
                            || desc_lower.contains("virtual adapter")
                            || desc_lower.contains("local area connection")
                            || name_lower.contains("direct")
                        {
                            chosen_dev = dev_name;
                            chosen_desc = dev_desc;
                            break;
                        }

                        curr = (*curr).next;
                    }
                }

                if chosen_dev.is_empty() {
                    chosen_dev = fallback_dev;
                    chosen_desc = fallback_desc;
                }

                if chosen_dev.is_empty() {
                    unsafe { (pcap.freealldevs)(alldevs) };
                    log_warn("TRAFFIC", "No suitable network capture adapter found yet. Waiting...");
                    thread::sleep(std::time::Duration::from_secs(3));
                    continue;
                }

                log_info("TRAFFIC", &format!("Opening capture on: '{}' ('{}')...", chosen_dev, chosen_desc));

                let c_dev = std::ffi::CString::new(chosen_dev.clone()).unwrap_or_default();
                let handle = unsafe { (pcap.open_live)(c_dev.as_ptr(), 65535, 1, 200, errbuf.as_mut_ptr()) };
                unsafe { (pcap.freealldevs)(alldevs) };

                if handle.is_null() {
                    let err = unsafe { std::ffi::CStr::from_ptr(errbuf.as_ptr()).to_string_lossy() };
                    log_warn("TRAFFIC", &format!("pcap_open_live failed on {}: {}. Retrying in 3s...", chosen_dev, err));
                    thread::sleep(std::time::Duration::from_secs(3));
                    continue;
                }

                log_success("TRAFFIC", &format!("Packet capture active on {}", chosen_desc));

                let mut loop_count = 0u32;
                while *sniffing_flag.lock().unwrap() {
                    let mut pkt_hdr: *mut PcapPkthdr = std::ptr::null_mut();
                    let mut pkt_data: *const u8 = std::ptr::null();
                    let next_ret = unsafe { (pcap.next_ex)(handle, &mut pkt_hdr, &mut pkt_data) };

                    if next_ret == 1 && !pkt_hdr.is_null() && !pkt_data.is_null() {
                        let caplen = unsafe { (*pkt_hdr).caplen as usize };
                        let packet = unsafe { std::slice::from_raw_parts(pkt_data, caplen) };

                        Self::process_packet(
                            packet,
                            &hs_ip,
                            &prefix,
                            &records_clone,
                            &domains_clone,
                            &ips_clone,
                            &blocker_clone,
                            &geo_clone,
                        );
                    } else if next_ret == 0 {
                        // Timeout - sleep briefly to eliminate CPU spinning while idle
                        thread::sleep(std::time::Duration::from_millis(15));
                    } else {
                        // Adapter reported disconnect or reset
                        log_info("TRAFFIC", "Capture interface closed or reset. Re-evaluating adapters...");
                        break;
                    }

                    loop_count += 1;
                    // Periodically (every ~10s) check if current hotspot IP changed
                    if loop_count % 500 == 0 {
                        let cur_ip = hs_ip_clone.lock().unwrap().clone();
                        if cur_ip != hs_ip {
                            log_info("TRAFFIC", &format!("Hotspot IP changed from {} to {}. Rebinding...", hs_ip, cur_ip));
                            break;
                        }
                    }
                }

                unsafe { (pcap.close)(handle) };
                thread::sleep(std::time::Duration::from_millis(500));
            }
        });
    }

    fn process_packet(
        packet: &[u8],
        hs_ip: &str,
        prefix: &str,
        records: &Arc<Mutex<VecDeque<TrafficRecord>>>,
        domains: &Arc<Mutex<HashSet<String>>>,
        ips: &Arc<Mutex<HashSet<String>>>,
        blocker_enabled: &Arc<Mutex<bool>>,
        geo_resolver: &GeoResolver,
    ) {
        // Parse Ethernet -> IPv4 or raw IPv4
        let ip_data = if packet.len() >= 14 && packet[12] == 0x08 && packet[13] == 0x00 {
            &packet[14..]
        } else if packet.len() >= 20 && (packet[0] >> 4) == 4 {
            packet
        } else {
            return;
        };

        if ip_data.len() < 20 || (ip_data[0] >> 4) != 4 {
            return;
        }

        let ihl = ((ip_data[0] & 0x0F) as usize) * 4;
        if ihl < 20 || ip_data.len() < ihl {
            return;
        }

        let protocol = ip_data[9];
        let src = format!("{}.{}.{}.{}", ip_data[12], ip_data[13], ip_data[14], ip_data[15]);
        let dst = format!("{}.{}.{}.{}", ip_data[16], ip_data[17], ip_data[18], ip_data[19]);
        let total_size = packet.len();

        let is_src_hs = src.starts_with(prefix) && src != hs_ip;
        let is_dst_hs = dst.starts_with(prefix) && dst != hs_ip;

        if !is_src_hs && !is_dst_hs {
            return;
        }

        let client_ip = if is_src_hs { src.clone() } else { dst.clone() };
        let remote_ip = if is_src_hs { dst.clone() } else { src.clone() };

        let payload = &ip_data[ihl..];
        let mut domain = "-".to_string();
        let mut proto_name = "IP".to_string();
        let mut remote_port = 0u16;

        if protocol == 17 {
            // UDP
            if payload.len() >= 8 {
                let src_port = u16::from_be_bytes([payload[0], payload[1]]);
                let dst_port = u16::from_be_bytes([payload[2], payload[3]]);
                remote_port = if is_src_hs { dst_port } else { src_port };

                if src_port == 53 || dst_port == 53 {
                    proto_name = "DNS".to_string();
                    if payload.len() > 8 {
                        if let Some(d) = Self::parse_dns_query(&payload[8..]) {
                            domain = d;
                        }
                    }
                } else {
                    proto_name = format!("UDP/{}", remote_port);
                }
            }
        } else if protocol == 6 {
            // TCP
            if payload.len() >= 20 {
                let src_port = u16::from_be_bytes([payload[0], payload[1]]);
                let dst_port = u16::from_be_bytes([payload[2], payload[3]]);
                let data_offset = ((payload[12] >> 4) as usize) * 4;
                remote_port = if is_src_hs { dst_port } else { src_port };

                let tcp_data = if payload.len() >= data_offset {
                    &payload[data_offset..]
                } else {
                    &[]
                };

                if dst_port == 443 || src_port == 443 {
                    proto_name = "HTTPS".to_string();
                    if let Some(sni) = Self::parse_tls_sni(tcp_data) {
                        domain = sni;
                    }
                } else if dst_port == 80 || src_port == 80 {
                    proto_name = "HTTP".to_string();
                    if let Some(h) = Self::parse_http_host(tcp_data) {
                        domain = h;
                    }
                } else {
                    proto_name = format!("TCP/{}", remote_port);
                }
            }
        } else if protocol == 1 {
            proto_name = "ICMP".to_string();
        }

        // Determine if blocked
        let mut is_blocked = false;
        let blocker_on = *blocker_enabled.lock().unwrap();

        if blocker_on {
            let ip_set = ips.lock().unwrap();
            if ip_set.contains(&client_ip) || ip_set.contains(&remote_ip) {
                is_blocked = true;
            }

            if !is_blocked && domain != "-" {
                let dom_set = domains.lock().unwrap();
                let dom_lower = domain.to_lowercase();
                for b in dom_set.iter() {
                    let b_low = b.to_lowercase();
                    if dom_lower == b_low || dom_lower.ends_with(&format!(".{}", b_low)) {
                        is_blocked = true;
                        break;
                    }
                }
            }
        }

        let geo = geo_resolver.get_or_queue(&remote_ip, &domain);
        let final_domain = if domain != "-" {
            domain
        } else if !crate::geo::GeoResolver::is_private_ip(&remote_ip) && geo.domain != "-" {
            geo.domain.clone()
        } else {
            "-".to_string()
        };

        let location = if geo.city != "-" && !geo.city.is_empty() {
            if geo.country_code != "-" && !geo.country_code.is_empty() {
                format!("{}, {}", geo.city, geo.country_code)
            } else {
                geo.city.clone()
            }
        } else if geo.country != "-" && !geo.country.is_empty() {
            geo.country.clone()
        } else {
            "-".to_string()
        };

        let company = if geo.company != "-" && !geo.company.is_empty() {
            geo.company.clone()
        } else {
            "-".to_string()
        };

        let info = if proto_name == "DNS" {
            format!("DNS Query: {}", final_domain)
        } else if proto_name == "HTTPS" {
            if final_domain != "-" { format!("TLS SNI: {}", final_domain) } else { "TLS Encrypted Traffic".to_string() }
        } else if proto_name == "HTTP" {
            format!("HTTP Host: {}", final_domain)
        } else {
            format!("{} Flow ({} bytes)", proto_name, total_size)
        };

        let preview_cap = packet.len().min(512);
        let payload = packet[..preview_cap].to_vec();

        let record = TrafficRecord {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            client_ip,
            domain: final_domain,
            remote_ip,
            remote_port,
            proto: proto_name,
            size: total_size,
            blocked: is_blocked,
            info,
            payload,
            location,
            company,
            lat: geo.lat,
            lon: geo.lon,
        };

        let mut rec_lock = records.lock().unwrap();
        if rec_lock.len() >= 1500 {
            rec_lock.pop_front();
        }
        rec_lock.push_back(record);
    }

    pub fn stop(&self) {
        *self.is_sniffing.lock().unwrap() = false;
    }

    pub fn add_blocked_domain(&self, domain: String) {
        let clean = domain.trim().to_lowercase();
        if !clean.is_empty() {
            self.blocked_domains.lock().unwrap().insert(clean.clone());
            Self::apply_domain_firewall_rule(&clean);
        }
    }

    pub fn remove_blocked_domain(&self, domain: &str) {
        let clean = domain.trim().to_lowercase();
        self.blocked_domains.lock().unwrap().remove(&clean);
        Self::remove_domain_firewall_rule(&clean);
    }

    pub fn add_blocked_ip(&self, ip: String) {
        let clean = ip.trim().to_string();
        if !clean.is_empty() {
            self.blocked_ips.lock().unwrap().insert(clean.clone());
            Self::apply_ip_firewall_rule(&clean);
        }
    }

    pub fn remove_blocked_ip(&self, ip: &str) {
        let clean = ip.trim();
        self.blocked_ips.lock().unwrap().remove(clean);
        Self::remove_ip_firewall_rule(clean);
    }

    pub fn set_blocker_enabled(&self, enabled: bool) {
        *self.blocker_enabled.lock().unwrap() = enabled;
    }

    pub fn get_records(&self) -> Vec<TrafficRecord> {
        self.records.lock().unwrap().iter().cloned().collect()
    }

    pub fn clear_records(&self) {
        self.records.lock().unwrap().clear();
    }

    pub fn get_client_totals(&self) -> std::collections::HashMap<String, (u64, usize)> {
        let mut map = std::collections::HashMap::new();
        if let Ok(recs) = self.records.lock() {
            for r in recs.iter() {
                if !r.client_ip.is_empty() && r.client_ip != "-" {
                    let entry = map.entry(r.client_ip.clone()).or_insert((0u64, 0usize));
                    entry.0 += r.size as u64;
                    entry.1 += 1;
                }
            }
        }
        map
    }

    pub fn get_globe_connections(&self) -> Vec<crate::globe::GlobeConnection> {
        let recs = self.records.lock().unwrap();
        let mut map: std::collections::HashMap<String, crate::globe::GlobeConnection> = std::collections::HashMap::new();

        for r in recs.iter().rev().take(400) {
            if r.remote_ip == "-" || r.remote_ip.is_empty() || (r.lat.abs() < 0.1 && r.lon.abs() < 0.1) {
                continue;
            }

            let entry = map.entry(r.remote_ip.clone()).or_insert_with(|| crate::globe::GlobeConnection {
                client_ip: r.client_ip.clone(),
                remote_ip: r.remote_ip.clone(),
                domain: r.domain.clone(),
                company: r.company.clone(),
                location: r.location.clone(),
                proto: r.proto.clone(),
                remote_lat: r.lat,
                remote_lon: r.lon,
                packet_count: 0,
                bytes: 0,
                is_blocked: r.blocked,
                last_seen_secs_ago: 0.0,
            });

            entry.packet_count += 1;
            entry.bytes += r.size;
            if r.blocked {
                entry.is_blocked = true;
            }
            if (entry.domain == "-" || entry.domain.is_empty()) && r.domain != "-" {
                entry.domain = r.domain.clone();
            }
            if (entry.company == "-" || entry.company.is_empty()) && r.company != "-" {
                entry.company = r.company.clone();
            }
            if (entry.location == "-" || entry.location.is_empty()) && r.location != "-" {
                entry.location = r.location.clone();
            }
        }

        let mut list: Vec<crate::globe::GlobeConnection> = map.into_values().collect();
        list.sort_by(|a, b| b.packet_count.cmp(&a.packet_count));
        list
    }

    pub fn export_csv(&self) -> Option<String> {
        let recs = self.get_records();
        let mut csv = String::from("Timestamp,Client_IP,Domain_SNI,Remote_IP,Remote_Port,Location,Company,Protocol,Bytes,Blocked,Info\n");
        for r in recs {
            let line = format!(
                "{},{},\"{}\",{},{},\"{}\",\"{}\",{},{},{},\"{}\"\n",
                r.timestamp,
                r.client_ip,
                r.domain.replace('"', "'"),
                r.remote_ip,
                r.remote_port,
                r.location.replace('"', "'"),
                r.company.replace('"', "'"),
                r.proto,
                r.size,
                r.blocked,
                r.info.replace('"', "'")
            );
            csv.push_str(&line);
        }
        let filename = format!("traffic_export_{}.csv", Local::now().format("%Y%m%d_%H%M%S"));
        let path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(&filename)))
            .unwrap_or_else(|| std::path::PathBuf::from(&filename));
        if std::fs::write(&path, csv).is_ok() {
            Some(path.to_string_lossy().to_string())
        } else {
            None
        }
    }

    // Windows Firewall Integration
    fn apply_domain_firewall_rule(domain: &str) {
        let dom = domain.to_string();
        thread::spawn(move || {
            let addr_str = format!("{}:80", dom);
            if let Ok(iter) = addr_str.to_socket_addrs() {
                for sa in iter {
                    let ip = sa.ip().to_string();
                    Self::apply_ip_firewall_rule(&ip);
                }
            }
        });
    }

    fn remove_domain_firewall_rule(_domain: &str) {
        // Automatically handled or removed via IP rules
    }

    fn apply_ip_firewall_rule(ip: &str) {
        #[cfg(target_os = "windows")]
        {
            let rule_name = format!("NetMon_Block_{}", ip);
            let cmd = format!(
                "netsh advfirewall firewall add rule name=\"{}\" dir=out action=block remoteip={} profile=any",
                rule_name, ip
            );
            let _ = Command::new("cmd").args(["/c", &cmd]).output();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("iptables").args(["-I", "FORWARD", "-d", ip, "-j", "DROP"]).output();
            let _ = Command::new("iptables").args(["-I", "OUTPUT", "-d", ip, "-j", "DROP"]).output();
        }
    }

    fn remove_ip_firewall_rule(ip: &str) {
        #[cfg(target_os = "windows")]
        {
            let rule_name = format!("NetMon_Block_{}", ip);
            let cmd = format!("netsh advfirewall firewall delete rule name=\"{}\"", rule_name);
            let _ = Command::new("cmd").args(["/c", &cmd]).output();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("iptables").args(["-D", "FORWARD", "-d", ip, "-j", "DROP"]).output();
            let _ = Command::new("iptables").args(["-D", "OUTPUT", "-d", ip, "-j", "DROP"]).output();
        }
    }

    // Pure Rust Protocol Parsers
    fn parse_dns_query(payload: &[u8]) -> Option<String> {
        if payload.len() < 13 {
            return None;
        }
        let qdcount = u16::from_be_bytes([payload[4], payload[5]]);
        if qdcount == 0 {
            return None;
        }

        let mut offset = 12;
        let mut labels = Vec::new();

        while offset < payload.len() {
            let len = payload[offset] as usize;
            if len == 0 {
                break;
            }
            offset += 1;
            if offset + len > payload.len() {
                return None;
            }
            if let Ok(label) = std::str::from_utf8(&payload[offset..offset + len]) {
                labels.push(label);
            }
            offset += len;
        }

        if !labels.is_empty() {
            Some(labels.join("."))
        } else {
            None
        }
    }

    fn parse_tls_sni(payload: &[u8]) -> Option<String> {
        if payload.len() < 43 || payload[0] != 0x16 || payload[5] != 0x01 {
            return None;
        }
        let mut offset = 43; // skip header + random
        if payload.len() <= offset {
            return None;
        }

        // Session ID
        let sess_len = payload[offset] as usize;
        offset += 1 + sess_len;
        if payload.len() < offset + 2 {
            return None;
        }

        // Cipher suites
        let cipher_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
        offset += 2 + cipher_len;
        if payload.len() < offset + 1 {
            return None;
        }

        // Compression methods
        let comp_len = payload[offset] as usize;
        offset += 1 + comp_len;
        if payload.len() < offset + 2 {
            return None;
        }

        // Extensions
        let ext_total = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
        offset += 2;
        let ext_end = (offset + ext_total).min(payload.len());

        while offset + 4 <= ext_end {
            let ext_type = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
            let ext_len = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]) as usize;
            offset += 4;

            if ext_type == 0 {
                // SNI
                if offset + ext_len <= payload.len() && ext_len >= 5 {
                    let sn_type = payload[offset + 2];
                    if sn_type == 0 {
                        // host_name
                        let sn_len =
                            u16::from_be_bytes([payload[offset + 3], payload[offset + 4]]) as usize;
                        if offset + 5 + sn_len <= payload.len() {
                            if let Ok(host) =
                                std::str::from_utf8(&payload[offset + 5..offset + 5 + sn_len])
                            {
                                return Some(host.to_string());
                            }
                        }
                    }
                }
                break;
            }
            offset += ext_len;
        }

        None
    }

    fn parse_http_host(payload: &[u8]) -> Option<String> {
        if let Ok(text) = std::str::from_utf8(payload) {
            if text.starts_with("GET ")
                || text.starts_with("POST ")
                || text.starts_with("HEAD ")
                || text.starts_with("CONNECT ")
            {
                for line in text.lines() {
                    if let Some(host_part) = line.strip_prefix("Host: ") {
                        return Some(host_part.trim().to_string());
                    }
                }
            }
        }
        None
    }
}
