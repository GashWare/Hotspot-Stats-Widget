#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct ModelQuotaData {
    pub is_connected: bool,
    pub last_update: Instant,
    pub gemini_5h_pct: f32,          // Remaining fuel % (0.0 .. 100.0)
    pub gemini_5h_reset: String,     // e.g. "4h 34m"
    pub gemini_weekly_pct: f32,      // Remaining fuel % (0.0 .. 100.0)
    pub gemini_weekly_reset: String,  // e.g. "4d 21h"
    pub claude_5h_pct: f32,
    pub claude_5h_reset: String,
    pub claude_weekly_pct: f32,
    pub claude_weekly_reset: String,
    pub status_text: String,
}

impl Default for ModelQuotaData {
    fn default() -> Self {
        Self {
            is_connected: false,
            last_update: Instant::now(),
            gemini_5h_pct: 100.0,
            gemini_5h_reset: "-".to_string(),
            gemini_weekly_pct: 100.0,
            gemini_weekly_reset: "-".to_string(),
            claude_5h_pct: 100.0,
            claude_5h_reset: "-".to_string(),
            claude_weekly_pct: 100.0,
            claude_weekly_reset: "-".to_string(),
            status_text: "Connecting...".to_string(),
        }
    }
}

pub struct QuotaCollector {
    pub data: Arc<Mutex<ModelQuotaData>>,
}

impl QuotaCollector {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(ModelQuotaData::default()));
        let data_thread = Arc::clone(&data);

        thread::spawn(move || {
            // Initial poll immediately
            Self::poll_once(&data_thread);

            loop {
                thread::sleep(Duration::from_secs(15));
                Self::poll_once(&data_thread);
            }
        });

        Self { data }
    }

    pub fn get_data(&self) -> ModelQuotaData {
        self.data.lock().unwrap().clone()
    }

    pub fn refresh(&self) {
        let data_clone = Arc::clone(&self.data);
        thread::spawn(move || {
            Self::poll_once(&data_clone);
        });
    }

    pub fn poll_once(shared_data: &Arc<Mutex<ModelQuotaData>>) {
        let creds = Self::discover_credentials();
        if let Some((addr, token)) = creds {
            match Self::fetch_quota(&addr, &token) {
                Ok(quota) => {
                    *shared_data.lock().unwrap() = quota;
                }
                Err(err) => {
                    let mut lock = shared_data.lock().unwrap();
                    lock.is_connected = false;
                    lock.status_text = format!("API Error: {}", err);
                }
            }
        } else {
            let mut lock = shared_data.lock().unwrap();
            lock.is_connected = false;
            lock.status_text = "Language Server Offline".to_string();
        }
    }

    /// Auto-discovers the active Antigravity Language Server address and CSRF token:
    /// 1. Environment variables ANTIGRAVITY_LS_ADDRESS and ANTIGRAVITY_CSRF_TOKEN.
    /// 2. Newest hub_*.json in ~/.gemini/antigravity/daemon/.
    /// 3. antigravity_bridge.json in the current working directory / exe dir.
    fn discover_credentials() -> Option<(String, String)> {
        // 1. Environment variables
        if let (Ok(addr), Ok(token)) = (
            std::env::var("ANTIGRAVITY_LS_ADDRESS"),
            std::env::var("ANTIGRAVITY_CSRF_TOKEN"),
        ) {
            if !addr.is_empty() && !token.is_empty() {
                let clean_addr = addr.trim().trim_start_matches("http://").to_string();
                Self::save_bridge_file(&clean_addr, &token);
                return Some((clean_addr, token));
            }
        }

        // 2. Scan ~/.gemini/antigravity/daemon/ for the latest hub_*.json
        if let Some(hub_dir) = Self::get_daemon_dir() {
            if let Ok(entries) = fs::read_dir(hub_dir) {
                let mut hub_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("hub_") && name.ends_with(".json") {
                            let mtime = entry.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                            hub_files.push((path, mtime));
                        }
                    }
                }

                // Sort newest first
                hub_files.sort_by(|a, b| b.1.cmp(&a.1));

                for (hub_path, _) in hub_files {
                    if let Ok(contents) = fs::read_to_string(&hub_path) {
                        if let Ok(v) = serde_json::from_str::<Value>(&contents) {
                            let http_port = v.get("httpPort").and_then(|p| p.as_u64());
                            let csrf_token = v.get("csrfToken").and_then(|t| t.as_str());

                            if let (Some(port), Some(token)) = (http_port, csrf_token) {
                                let addr = format!("127.0.0.1:{}", port);
                                Self::save_bridge_file(&addr, token);
                                return Some((addr, token.to_string()));
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback to bridge file
        for p in &["antigravity_bridge.json", "Rust/antigravity_bridge.json"] {
            if let Ok(contents) = fs::read_to_string(p) {
                if let Ok(v) = serde_json::from_str::<Value>(&contents) {
                    let addr = v.get("address").and_then(|a| a.as_str());
                    let token = v.get("token").and_then(|t| t.as_str());
                    if let (Some(a), Some(t)) = (addr, token) {
                        return Some((a.to_string(), t.to_string()));
                    }
                }
            }
        }

        None
    }

    fn get_daemon_dir() -> Option<PathBuf> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()?;
        let p = Path::new(&home).join(".gemini").join("antigravity").join("daemon");
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }

    fn save_bridge_file(addr: &str, token: &str) {
        let json = format!(
            "{{\n  \"address\": \"{}\",\n  \"token\": \"{}\"\n}}",
            addr, token
        );
        let _ = fs::write("antigravity_bridge.json", &json);
        let _ = fs::write("Rust/antigravity_bridge.json", &json);
    }

    /// Fetches RetrieveUserQuotaSummary over HTTP/1.1 POST using native TcpStream
    fn fetch_quota(addr_str: &str, token: &str) -> Result<ModelQuotaData, String> {
        // Resolve host & port
        let mut addrs_iter = addr_str
            .to_socket_addrs()
            .map_err(|e| format!("Address resolve error: {}", e))?;
        let socket_addr = addrs_iter
            .next()
            .ok_or_else(|| "Could not resolve address".to_string())?;

        let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2))
            .map_err(|e| format!("Connect failed: {}", e))?;

        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| e.to_string())?;

        let request = format!(
            "POST /exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/json\r\n\
             Connect-Protocol-Version: 1\r\n\
             x-antigravity-csrf-token: {}\r\n\
             Content-Length: 2\r\n\
             Connection: close\r\n\
             \r\n\
             {{}}",
            addr_str, token
        );

        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("Write failed: {}", e))?;

        let mut response_bytes = Vec::new();
        stream
            .read_to_end(&mut response_bytes)
            .map_err(|e| format!("Read failed: {}", e))?;

        let response_str = String::from_utf8_lossy(&response_bytes);

        // Split HTTP headers from body
        let body = if let Some(pos) = response_str.find("\r\n\r\n") {
            &response_str[pos + 4..]
        } else {
            &response_str
        };

        let json_val: Value = serde_json::from_str(body)
            .map_err(|e| format!("JSON parse failed: {} (body: {})", e, &body[..body.len().min(80)]))?;

        let mut quota = ModelQuotaData {
            is_connected: true,
            last_update: Instant::now(),
            status_text: "Online".to_string(),
            ..Default::default()
        };

        let now = Utc::now();

        if let Some(groups) = json_val.pointer("/response/groups").and_then(|g| g.as_array()) {
            for group in groups {
                let disp_name = group.get("displayName").and_then(|d| d.as_str()).unwrap_or("");
                let is_gemini = disp_name.contains("Gemini");
                let is_claude = disp_name.contains("Claude");

                if let Some(buckets) = group.get("buckets").and_then(|b| b.as_array()) {
                    for bucket in buckets {
                        let window = bucket.get("window").and_then(|w| w.as_str()).unwrap_or("");
                        let rem_frac = bucket.get("remainingFraction").and_then(|r| r.as_f64()).unwrap_or(1.0) as f32;
                        let pct = (rem_frac * 100.0).clamp(0.0, 100.0);

                        let reset_str = if let Some(reset_time_str) = bucket.get("resetTime").and_then(|rt| rt.as_str()) {
                            Self::format_reset_time(reset_time_str, now)
                        } else {
                            "Full".to_string()
                        };

                        if is_gemini {
                            if window == "5h" {
                                quota.gemini_5h_pct = pct;
                                quota.gemini_5h_reset = reset_str;
                            } else if window == "weekly" {
                                quota.gemini_weekly_pct = pct;
                                quota.gemini_weekly_reset = reset_str;
                            }
                        } else if is_claude {
                            if window == "5h" {
                                quota.claude_5h_pct = pct;
                                quota.claude_5h_reset = reset_str;
                            } else if window == "weekly" {
                                quota.claude_weekly_pct = pct;
                                quota.claude_weekly_reset = reset_str;
                            }
                        }
                    }
                }
            }
        }

        Ok(quota)
    }

    fn format_reset_time(iso_str: &str, now: DateTime<Utc>) -> String {
        if let Ok(target) = DateTime::parse_from_rfc3339(iso_str) {
            let target_utc = target.with_timezone(&Utc);
            let diff = target_utc.signed_duration_since(now);

            if diff.num_seconds() <= 0 {
                return "0m".to_string();
            }

            let days = diff.num_days();
            let hours = diff.num_hours() % 24;
            let mins = diff.num_minutes() % 60;

            if days > 0 {
                format!("{}d {}h", days, hours)
            } else if hours > 0 {
                format!("{}h {}m", hours, mins)
            } else {
                format!("{}m", mins.max(1))
            }
        } else {
            "-".to_string()
        }
    }
}
