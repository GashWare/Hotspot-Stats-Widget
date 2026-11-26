use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoInfo {
    pub ip: String,
    pub country: String,
    pub country_code: String,
    pub region: String,
    pub city: String,
    pub company: String,
    pub isp: String,
    pub lat: f32,
    pub lon: f32,
    pub domain: String,
}

impl Default for GeoInfo {
    fn default() -> Self {
        Self {
            ip: String::new(),
            country: "-".to_string(),
            country_code: "-".to_string(),
            region: "-".to_string(),
            city: "-".to_string(),
            company: "-".to_string(),
            isp: "-".to_string(),
            lat: 0.0,
            lon: 0.0,
            domain: "-".to_string(),
        }
    }
}

#[derive(Deserialize)]
struct IpApiResponse {
    status: Option<String>,
    country: Option<String>,
    #[serde(rename = "countryCode")]
    country_code: Option<String>,
    #[serde(rename = "regionName")]
    region_name: Option<String>,
    city: Option<String>,
    lat: Option<f32>,
    lon: Option<f32>,
    isp: Option<String>,
    org: Option<String>,
    #[serde(rename = "as")]
    as_info: Option<String>,
}

#[derive(Clone)]
pub struct GeoResolver {
    cache: Arc<Mutex<HashMap<String, GeoInfo>>>,
    queue: Arc<Mutex<VecDeque<String>>>,
    pub local_origin: Arc<Mutex<(f32, f32, String)>>, // (lat, lon, label)
}

impl GeoResolver {
    pub fn new() -> Self {
        let resolver = Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            local_origin: Arc::new(Mutex::new((37.7749, -122.4194, "Local Hotspot".to_string()))),
        };

        resolver.spawn_worker();
        resolver.resolve_local_origin();
        resolver
    }

    /// Resolves local machine's public location once for the 3D globe origin point
    fn resolve_local_origin(&self) {
        let origin_arc = Arc::clone(&self.local_origin);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            if let Some(resp) = Self::http_get_ip_api("") {
                if let (Some(lat), Some(lon)) = (resp.lat, resp.lon) {
                    let city = resp.city.unwrap_or_else(|| "Local".to_string());
                    let cc = resp.country_code.unwrap_or_default();
                    let label = if !cc.is_empty() {
                        format!("{}, {}", city, cc)
                    } else {
                        city
                    };
                    *origin_arc.lock().unwrap() = (lat, lon, label);
                }
            }
        });
    }

    /// Fast lookup in cache or well-known prefixes
    pub fn get_or_queue(&self, ip: &str, known_domain: &str) -> GeoInfo {
        let ip_clean = ip.trim();
        if ip_clean.is_empty() || ip_clean == "-" {
            return GeoInfo::default();
        }

        // 1. Check in-memory cache
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(existing) = cache.get_mut(ip_clean) {
                if existing.domain == "-" && !known_domain.is_empty() && known_domain != "-" {
                    existing.domain = known_domain.to_string();
                }
                return existing.clone();
            }
        }

        // 2. Private LAN / Hotspot IP
        if Self::is_private_ip(ip_clean) {
            let info = GeoInfo {
                ip: ip_clean.to_string(),
                country: "Local Network".to_string(),
                country_code: "LAN".to_string(),
                region: "LAN".to_string(),
                city: "Hotspot Client".to_string(),
                company: "Local Network".to_string(),
                isp: "Wi-Fi Direct Hotspot".to_string(),
                lat: 0.0,
                lon: 0.0,
                domain: "-".to_string(),
            };
            self.cache.lock().unwrap().insert(ip_clean.to_string(), info.clone());
            return info;
        }

        // 3. Well-known provider heuristic for instant 0-latency tagging
        if let Some(mut info) = Self::well_known_lookup(ip_clean) {
            if !known_domain.is_empty() && known_domain != "-" {
                info.domain = known_domain.to_string();
            }
            self.cache.lock().unwrap().insert(ip_clean.to_string(), info.clone());
            // Still queue for exact city / coordinates if needed
            self.enqueue(ip_clean);
            return info;
        }

        // 4. Default placeholder while background worker resolves
        let placeholder = GeoInfo {
            ip: ip_clean.to_string(),
            domain: if known_domain.is_empty() { "-".to_string() } else { known_domain.to_string() },
            ..GeoInfo::default()
        };

        self.cache.lock().unwrap().insert(ip_clean.to_string(), placeholder.clone());
        self.enqueue(ip_clean);
        placeholder
    }

    pub fn set_domain(&self, ip: &str, domain: &str) {
        if ip.is_empty() || domain.is_empty() || domain == "-" || Self::is_private_ip(ip) {
            return;
        }
        let mut cache = self.cache.lock().unwrap();
        if let Some(info) = cache.get_mut(ip) {
            info.domain = domain.to_string();
        }
    }

    pub fn get_all_active(&self) -> Vec<GeoInfo> {
        let cache = self.cache.lock().unwrap();
        cache.values().cloned().collect()
    }

    fn enqueue(&self, ip: &str) {
        let mut q = self.queue.lock().unwrap();
        if !q.contains(&ip.to_string()) && q.len() < 250 {
            q.push_back(ip.to_string());
        }
    }

    fn spawn_worker(&self) {
        let cache_arc = Arc::clone(&self.cache);
        let queue_arc = Arc::clone(&self.queue);

        thread::spawn(move || loop {
            let next_ip = {
                let mut q = queue_arc.lock().unwrap();
                q.pop_front()
            };

            match next_ip {
                Some(ip) => {
                    // Reverse DNS lookup (PTR)
                    let rdns = Self::reverse_dns(&ip);

                    // HTTP GeoIP lookup
                    if let Some(resp) = Self::http_get_ip_api(&ip) {
                        let country = resp.country.unwrap_or_else(|| "Unknown".to_string());
                        let country_code = resp.country_code.unwrap_or_else(|| "--".to_string());
                        let region = resp.region_name.unwrap_or_else(|| "-".to_string());
                        let city = resp.city.unwrap_or_else(|| "Unknown".to_string());
                        let lat = resp.lat.unwrap_or(0.0);
                        let lon = resp.lon.unwrap_or(0.0);
                        let org = resp.org.or(resp.as_info).unwrap_or_else(|| "-".to_string());
                        let isp = resp.isp.unwrap_or_else(|| "-".to_string());

                        let company = Self::clean_company_name(&org, &isp);

                        let mut cache = cache_arc.lock().unwrap();
                        let domain = cache
                            .get(&ip)
                            .map(|i| i.domain.clone())
                            .unwrap_or_else(|| "-".to_string());

                        let final_domain = if domain != "-" {
                            domain
                        } else if let Some(r) = rdns {
                            r
                        } else {
                            "-".to_string()
                        };

                        cache.insert(
                            ip.clone(),
                            GeoInfo {
                                ip,
                                country,
                                country_code,
                                region,
                                city,
                                company,
                                isp,
                                lat,
                                lon,
                                domain: final_domain,
                            },
                        );
                    }

                    // Gentle rate limit for free tier
                    thread::sleep(Duration::from_millis(900));
                }
                None => {
                    thread::sleep(Duration::from_millis(300));
                }
            }
        });
    }

    fn reverse_dns(ip: &str) -> Option<String> {
        let addr_str = format!("{}:80", ip);
        if let Ok(mut addrs) = addr_str.to_socket_addrs() {
            if let Some(_addr) = addrs.next() {
                // Reverse lookup via socket address
                #[cfg(target_os = "windows")]
                {
                    let addr = _addr;
                    use std::ffi::CStr;
                    use std::os::raw::c_char;
                    #[link(name = "ws2_32")]
                    unsafe extern "system" {
                        fn getnameinfo(
                            sa: *const u8,
                            salen: i32,
                            host: *mut c_char,
                            hostlen: u32,
                            serv: *mut c_char,
                            servlen: u32,
                            flags: i32,
                        ) -> i32;
                    }

                    let mut host_buf = [0 as c_char; 256];
                    let (sa_bytes, sa_len) = match addr {
                        std::net::SocketAddr::V4(v4) => {
                            let raw = unsafe {
                                std::mem::transmute::<_, [u8; 16]>(std::mem::zeroed::<[u8; 16]>())
                            };
                            let mut buf = raw;
                            buf[0] = 2; // AF_INET
                            buf[2] = (v4.port() >> 8) as u8;
                            buf[3] = (v4.port() & 0xFF) as u8;
                            let oct = v4.ip().octets();
                            buf[4..8].copy_from_slice(&oct);
                            (buf, 16)
                        }
                        _ => return None,
                    };

                    let res = unsafe {
                        getnameinfo(
                            sa_bytes.as_ptr(),
                            sa_len,
                            host_buf.as_mut_ptr(),
                            256,
                            std::ptr::null_mut(),
                            0,
                            0,
                        )
                    };
                    if res == 0 {
                        let cstr = unsafe { CStr::from_ptr(host_buf.as_ptr()) };
                        if let Ok(s) = cstr.to_str() {
                            let trimmed = s.trim();
                            if !trimmed.is_empty() && trimmed != ip {
                                return Some(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn http_get_ip_api(ip: &str) -> Option<IpApiResponse> {
        let path = if ip.is_empty() {
            "/json/?fields=status,message,country,countryCode,regionName,city,lat,lon,isp,org,as,query".to_string()
        } else {
            format!("/json/{}?fields=status,message,country,countryCode,regionName,city,lat,lon,isp,org,as,query", ip)
        };

        let stream_res = TcpStream::connect_timeout(
            &("ip-api.com:80".to_socket_addrs().ok()?.next()?),
            Duration::from_millis(2500),
        );

        let mut stream = stream_res.ok()?;
        stream.set_read_timeout(Some(Duration::from_millis(3000))).ok()?;
        stream.set_write_timeout(Some(Duration::from_millis(2000))).ok()?;

        let req = format!(
            "GET {} HTTP/1.1\r\nHost: ip-api.com\r\nUser-Agent: NetMonitor-StatsWidget/1.0\r\nConnection: close\r\nAccept: application/json\r\n\r\n",
            path
        );

        stream.write_all(req.as_bytes()).ok()?;

        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);

        let text = String::from_utf8_lossy(&response);
        if let Some((_headers, body)) = text.split_once("\r\n\r\n") {
            let clean_body = body.trim();
            if let Ok(parsed) = serde_json::from_str::<IpApiResponse>(clean_body) {
                if parsed.status.as_deref() == Some("success") {
                    return Some(parsed);
                }
            }
        }
        None
    }

    fn clean_company_name(org: &str, isp: &str) -> String {
        let raw = if !org.is_empty() && org != "-" {
            org
        } else if !isp.is_empty() && isp != "-" {
            isp
        } else {
            return "-".to_string();
        };

        // Strip leading AS number e.g. "AS15169 Google LLC" -> "Google LLC"
        let trimmed = if raw.starts_with("AS") {
            if let Some((_as_num, rest)) = raw.split_once(' ') {
                rest.trim()
            } else {
                raw.trim()
            }
        } else {
            raw.trim()
        };

        trimmed.to_string()
    }

    pub fn is_private_ip(ip: &str) -> bool {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        if let (Ok(o1), Ok(o2)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
            if o1 == 10 || o1 == 127 {
                return true;
            }
            if o1 == 172 && (16..=31).contains(&o2) {
                return true;
            }
            if o1 == 192 && o2 == 168 {
                return true;
            }
        }
        false
    }

    fn well_known_lookup(ip: &str) -> Option<GeoInfo> {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return None;
        }
        let o1 = parts[0].parse::<u8>().ok()?;
        let o2 = parts[1].parse::<u8>().ok()?;

        // Google
        if o1 == 8 && (o2 == 8) {
            return Some(GeoInfo {
                ip: ip.to_string(),
                country: "United States".to_string(),
                country_code: "US".to_string(),
                region: "California".to_string(),
                city: "Mountain View".to_string(),
                company: "Google LLC".to_string(),
                isp: "Google Public DNS".to_string(),
                lat: 37.4225,
                lon: -122.085,
                domain: "dns.google".to_string(),
            });
        }
        if (o1 == 172 && (o2 == 217)) || (o1 == 142 && (o2 == 250 || o2 == 251)) || (o1 == 216 && o2 == 58) {
            return Some(GeoInfo {
                ip: ip.to_string(),
                country: "United States".to_string(),
                country_code: "US".to_string(),
                region: "California".to_string(),
                city: "Mountain View".to_string(),
                company: "Google LLC".to_string(),
                isp: "Google Services".to_string(),
                lat: 37.4225,
                lon: -122.085,
                domain: "1e100.net".to_string(),
            });
        }

        // Cloudflare
        if (o1 == 1 && o2 == 1) || (o1 == 104 && (16..=31).contains(&o2)) || (o1 == 172 && (64..=79).contains(&o2)) {
            return Some(GeoInfo {
                ip: ip.to_string(),
                country: "United States".to_string(),
                country_code: "US".to_string(),
                region: "California".to_string(),
                city: "San Francisco".to_string(),
                company: "Cloudflare, Inc.".to_string(),
                isp: "Cloudflare Network".to_string(),
                lat: 37.7749,
                lon: -122.4194,
                domain: "cloudflare.com".to_string(),
            });
        }

        // Microsoft
        if (o1 == 20 && (o2 == 190 || o2 == 20)) || (o1 == 40 && (76..=125).contains(&o2)) || (o1 == 52 && (96..=115).contains(&o2)) {
            return Some(GeoInfo {
                ip: ip.to_string(),
                country: "United States".to_string(),
                country_code: "US".to_string(),
                region: "Washington".to_string(),
                city: "Redmond".to_string(),
                company: "Microsoft Corporation".to_string(),
                isp: "Microsoft Azure".to_string(),
                lat: 47.6740,
                lon: -122.1215,
                domain: "microsoft.com".to_string(),
            });
        }

        // Amazon AWS
        if o1 == 54 || o1 == 52 || o1 == 3 || o1 == 18 {
            return Some(GeoInfo {
                ip: ip.to_string(),
                country: "United States".to_string(),
                country_code: "US".to_string(),
                region: "Washington".to_string(),
                city: "Seattle".to_string(),
                company: "Amazon.com, Inc.".to_string(),
                isp: "Amazon Web Services (AWS)".to_string(),
                lat: 47.6062,
                lon: -122.3321,
                domain: "amazonaws.com".to_string(),
            });
        }

        // Apple
        if o1 == 17 {
            return Some(GeoInfo {
                ip: ip.to_string(),
                country: "United States".to_string(),
                country_code: "US".to_string(),
                region: "California".to_string(),
                city: "Cupertino".to_string(),
                company: "Apple Inc.".to_string(),
                isp: "Apple Network".to_string(),
                lat: 37.3230,
                lon: -122.0322,
                domain: "apple.com".to_string(),
            });
        }

        None
    }
}
