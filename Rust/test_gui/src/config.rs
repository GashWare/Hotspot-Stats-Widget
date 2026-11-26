use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(0, 255, 204);
pub const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(255, 0, 85);
pub const ACCENT_YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 204, 0);
pub const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(204, 0, 255);
pub const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 136, 0);
pub const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(0, 255, 136);
pub const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(60, 160, 255);
pub const BG_COLOR: egui::Color32 = egui::Color32::from_rgb(26, 26, 26);
pub const HEADER_BG: egui::Color32 = egui::Color32::from_rgb(43, 43, 43);
pub const GRAPH_BG: egui::Color32 = egui::Color32::from_rgb(15, 15, 15);

#[derive(Clone, Debug)]
pub struct ThemePalette {
    pub id: String,
    pub name: String,
    pub bg: egui::Color32,
    pub header_bg: egui::Color32,
    pub card_bg: egui::Color32,
    pub graph_bg: egui::Color32,
    pub primary: egui::Color32,
    pub secondary: egui::Color32,
    pub tertiary: egui::Color32,
    pub warning: egui::Color32,
    pub redline: egui::Color32,
    pub text_bright: egui::Color32,
    pub text_dim: egui::Color32,
    pub border: egui::Color32,
    pub grid: egui::Color32,
}

impl ThemePalette {
    pub fn for_theme(id: &str) -> Self {
        match id {
            "retro_amber" => Self {
                id: "retro_amber".to_string(),
                name: "⚡ 80s Retro Digital Dash (Amber/Gold)".to_string(),
                bg: egui::Color32::from_rgb(14, 12, 10),
                header_bg: egui::Color32::from_rgb(32, 24, 16),
                card_bg: egui::Color32::from_rgb(20, 16, 12),
                graph_bg: egui::Color32::from_rgb(8, 7, 5),
                primary: egui::Color32::from_rgb(255, 176, 0),    // Vibrant Amber
                secondary: egui::Color32::from_rgb(255, 105, 0),  // Tach Orange
                tertiary: egui::Color32::from_rgb(255, 215, 0),   // Gold
                warning: egui::Color32::from_rgb(255, 75, 15),    // Warning Red-Orange
                redline: egui::Color32::from_rgb(255, 30, 30),    // Redline Red
                text_bright: egui::Color32::from_rgb(255, 225, 140),
                text_dim: egui::Color32::from_rgb(160, 125, 70),
                border: egui::Color32::from_rgb(60, 44, 26),
                grid: egui::Color32::from_rgb(32, 24, 14),
            },
            "vfd_green" => Self {
                id: "vfd_green".to_string(),
                name: "📟 80s VFD Phosphor (Subaru XT / Mint)".to_string(),
                bg: egui::Color32::from_rgb(8, 16, 12),
                header_bg: egui::Color32::from_rgb(16, 34, 24),
                card_bg: egui::Color32::from_rgb(12, 24, 18),
                graph_bg: egui::Color32::from_rgb(5, 12, 8),
                primary: egui::Color32::from_rgb(0, 255, 170),    // VFD Mint Phosphor
                secondary: egui::Color32::from_rgb(40, 220, 110), // Emerald Green
                tertiary: egui::Color32::from_rgb(90, 255, 220),  // Ice Cyan
                warning: egui::Color32::from_rgb(255, 180, 0),    // Amber Warning
                redline: egui::Color32::from_rgb(255, 45, 45),    // Redline Red
                text_bright: egui::Color32::from_rgb(200, 255, 230),
                text_dim: egui::Color32::from_rgb(75, 145, 105),
                border: egui::Color32::from_rgb(26, 58, 40),
                grid: egui::Color32::from_rgb(14, 34, 22),
            },
            "synthwave" => Self {
                id: "synthwave".to_string(),
                name: "🌆 Synthwave 1984 (Neon Pink & Sky)".to_string(),
                bg: egui::Color32::from_rgb(18, 12, 26),
                header_bg: egui::Color32::from_rgb(34, 20, 50),
                card_bg: egui::Color32::from_rgb(24, 15, 36),
                graph_bg: egui::Color32::from_rgb(12, 7, 18),
                primary: egui::Color32::from_rgb(255, 42, 133),   // Neon Hot Pink
                secondary: egui::Color32::from_rgb(0, 238, 255),  // Electric Sky Blue
                tertiary: egui::Color32::from_rgb(180, 70, 255),  // Laser Purple
                warning: egui::Color32::from_rgb(255, 195, 0),    // Sunset Yellow
                redline: egui::Color32::from_rgb(255, 25, 75),    // Neon Red
                text_bright: egui::Color32::from_rgb(250, 230, 255),
                text_dim: egui::Color32::from_rgb(145, 100, 165),
                border: egui::Color32::from_rgb(65, 32, 90),
                grid: egui::Color32::from_rgb(34, 16, 48),
            },
            "corvette_red" => Self {
                id: "corvette_red".to_string(),
                name: "🏎 C4 Corvette Cockpit (Cockpit Redline)".to_string(),
                bg: egui::Color32::from_rgb(14, 8, 8),
                header_bg: egui::Color32::from_rgb(32, 14, 14),
                card_bg: egui::Color32::from_rgb(22, 10, 10),
                graph_bg: egui::Color32::from_rgb(8, 4, 4),
                primary: egui::Color32::from_rgb(255, 36, 36),    // Corvette Red
                secondary: egui::Color32::from_rgb(255, 100, 20), // High Heat Orange
                tertiary: egui::Color32::from_rgb(255, 190, 40),  // Tach Gold
                warning: egui::Color32::from_rgb(255, 135, 0),    // Warning Orange
                redline: egui::Color32::from_rgb(255, 15, 15),    // Pure Redline
                text_bright: egui::Color32::from_rgb(255, 215, 215),
                text_dim: egui::Color32::from_rgb(165, 80, 80),
                border: egui::Color32::from_rgb(60, 22, 22),
                grid: egui::Color32::from_rgb(32, 12, 12),
            },
            "stealth_monochrome" => Self {
                id: "stealth_monochrome".to_string(),
                name: "🕶 Stealth OLED (Titanium & Ice White)".to_string(),
                bg: egui::Color32::from_rgb(14, 14, 14),
                header_bg: egui::Color32::from_rgb(28, 28, 28),
                card_bg: egui::Color32::from_rgb(20, 20, 20),
                graph_bg: egui::Color32::from_rgb(8, 8, 8),
                primary: egui::Color32::from_rgb(240, 245, 250),  // Ice White
                secondary: egui::Color32::from_rgb(150, 165, 180),// Titanium Slate
                tertiary: egui::Color32::from_rgb(105, 120, 135), // Gunmetal
                warning: egui::Color32::from_rgb(255, 185, 55),   // Amber
                redline: egui::Color32::from_rgb(255, 60, 60),    // Crisp Red
                text_bright: egui::Color32::from_rgb(245, 245, 245),
                text_dim: egui::Color32::from_rgb(125, 130, 135),
                border: egui::Color32::from_rgb(48, 48, 48),
                grid: egui::Color32::from_rgb(26, 26, 26),
            },
            "ferrari_dash" => Self {
                id: "ferrari_dash".to_string(),
                name: "🏎 Ferrari Digital Dash (Rosso Corsa & Giallo)".to_string(),
                bg: egui::Color32::from_rgb(12, 12, 14),
                header_bg: egui::Color32::from_rgb(28, 20, 20),
                card_bg: egui::Color32::from_rgb(20, 16, 18),
                graph_bg: egui::Color32::from_rgb(6, 6, 8),
                primary: egui::Color32::from_rgb(255, 40, 0),     // Rosso Corsa Red
                secondary: egui::Color32::from_rgb(255, 215, 0),  // Giallo Modena Yellow
                tertiary: egui::Color32::from_rgb(255, 125, 0),   // Modena Orange
                warning: egui::Color32::from_rgb(255, 175, 0),    // Warning Amber
                redline: egui::Color32::from_rgb(255, 0, 0),      // Pure 9000 RPM Redline
                text_bright: egui::Color32::from_rgb(255, 240, 230),
                text_dim: egui::Color32::from_rgb(160, 130, 120),
                border: egui::Color32::from_rgb(58, 28, 24),
                grid: egui::Color32::from_rgb(28, 16, 16),
            },
            "nissan_300zx" => Self {
                id: "nissan_300zx".to_string(),
                name: "🏎 Nissan 300ZX Digital Dash (Z31 Phosphor)".to_string(),
                bg: egui::Color32::from_rgb(6, 12, 9),
                header_bg: egui::Color32::from_rgb(12, 26, 18),
                card_bg: egui::Color32::from_rgb(10, 20, 14),
                graph_bg: egui::Color32::from_rgb(4, 9, 6),
                primary: egui::Color32::from_rgb(0, 255, 136),    // Z31 Emerald VFD Phosphor
                secondary: egui::Color32::from_rgb(50, 220, 150), // Mint Green
                tertiary: egui::Color32::from_rgb(130, 255, 195), // High Beam Mint White
                warning: egui::Color32::from_rgb(255, 176, 0),    // Warning Amber
                redline: egui::Color32::from_rgb(255, 40, 50),    // Tach Redline
                text_bright: egui::Color32::from_rgb(215, 255, 235),
                text_dim: egui::Color32::from_rgb(55, 135, 90),
                border: egui::Color32::from_rgb(20, 56, 36),
                grid: egui::Color32::from_rgb(10, 28, 18),
            },
            "matrix" => Self {
                id: "matrix".to_string(),
                name: "🟢 The Matrix Phosphor (Terminal Code & Obsidian)".to_string(),
                bg: egui::Color32::from_rgb(5, 10, 6),
                header_bg: egui::Color32::from_rgb(10, 24, 12),
                card_bg: egui::Color32::from_rgb(8, 18, 10),
                graph_bg: egui::Color32::from_rgb(3, 7, 4),
                primary: egui::Color32::from_rgb(0, 255, 65),     // Matrix Phosphor Green
                secondary: egui::Color32::from_rgb(0, 143, 17),   // Dark Digital Green
                tertiary: egui::Color32::from_rgb(140, 255, 150), // Glitch White-Green
                warning: egui::Color32::from_rgb(255, 176, 0),    // Operator Amber
                redline: egui::Color32::from_rgb(255, 40, 40),    // Agent Red
                text_bright: egui::Color32::from_rgb(170, 255, 180),
                text_dim: egui::Color32::from_rgb(45, 125, 60),
                border: egui::Color32::from_rgb(18, 55, 24),
                grid: egui::Color32::from_rgb(8, 30, 12),
            },
            "hackers" => Self {
                id: "hackers".to_string(),
                name: "⚡ Hackers 1995 (Acid Green & Gibson Purple)".to_string(),
                bg: egui::Color32::from_rgb(12, 10, 18),
                header_bg: egui::Color32::from_rgb(26, 18, 42),
                card_bg: egui::Color32::from_rgb(18, 14, 28),
                graph_bg: egui::Color32::from_rgb(7, 5, 12),
                primary: egui::Color32::from_rgb(57, 255, 20),    // Acid Green
                secondary: egui::Color32::from_rgb(0, 255, 255),  // Gibson Cyan
                tertiary: egui::Color32::from_rgb(255, 0, 128),   // Glitch Magenta
                warning: egui::Color32::from_rgb(255, 185, 0),    // CRT Amber
                redline: egui::Color32::from_rgb(255, 20, 80),    // Elite Red
                text_bright: egui::Color32::from_rgb(230, 255, 240),
                text_dim: egui::Color32::from_rgb(120, 100, 160),
                border: egui::Color32::from_rgb(60, 30, 95),
                grid: egui::Color32::from_rgb(28, 15, 48),
            },
            _ => Self {
                // "cyberpunk" Default
                id: "cyberpunk".to_string(),
                name: "🌐 Cyberpunk Neon (Cyan & Pink)".to_string(),
                bg: egui::Color32::from_rgb(22, 24, 28),
                header_bg: egui::Color32::from_rgb(36, 40, 48),
                card_bg: egui::Color32::from_rgb(28, 31, 38),
                graph_bg: egui::Color32::from_rgb(14, 15, 18),
                primary: egui::Color32::from_rgb(0, 255, 204),    // Neon Cyan
                secondary: egui::Color32::from_rgb(255, 0, 85),   // Neon Pink/Red
                tertiary: egui::Color32::from_rgb(255, 204, 0),   // Neon Yellow
                warning: egui::Color32::from_rgb(255, 136, 0),    // Orange
                redline: egui::Color32::from_rgb(255, 30, 80),    // Redline Pink
                text_bright: egui::Color32::from_rgb(230, 240, 245),
                text_dim: egui::Color32::from_rgb(115, 135, 150),
                border: egui::Color32::from_rgb(46, 56, 70),
                grid: egui::Color32::from_rgb(28, 35, 44),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct WidgetConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
    pub topmost: bool,
    pub lock_position: bool,
    pub mini_mode: bool,
    pub show_network: bool,
    pub show_cpu: bool,
    pub show_ram: bool,
    pub show_gpu: bool,
    pub show_vram: bool,
    pub show_swap: bool,
    pub show_disk: bool,
    pub show_bluetooth: bool,
    pub show_clients: bool,
    #[serde(default = "default_true")]
    pub show_model_quota: bool,
    pub module_order: Vec<String>,
    pub module_heights: HashMap<String, f32>,
    pub theme: String,
    pub net_graph_style: String,
    #[serde(default = "default_dash_style")]
    pub quota_graph_style: String,
    pub cpu_graph_style: String,
    pub ram_graph_style: String,
    pub gpu_graph_style: String,
    pub vram_graph_style: String,
    pub swap_graph_style: String,
    pub disk_graph_style: String,
    pub hotspot_traffic_only: bool,
    pub fluid_graphs: bool,
    pub graph_interval_ms: u64,
    pub selected_wifi_adapter: String,
    #[serde(default)]
    pub bluetooth_favorites: Vec<String>,
    #[serde(default)]
    pub hidden_bluetooth_devices: Vec<String>,
    #[serde(default)]
    pub blocked_domains: Vec<String>,
    #[serde(default)]
    pub blocked_ips: Vec<String>,
    #[serde(default)]
    pub blocker_enabled: bool,
    #[serde(default)]
    pub autostart_hotspot: bool,
    #[serde(default)]
    pub show_taskbar_gauges: bool,
    #[serde(default)]
    pub taskbar_gauge_x: Option<f32>,
    #[serde(default)]
    pub taskbar_gauge_width: Option<f32>,
    #[serde(default = "default_menu_show_delay")]
    pub menu_show_delay_ms: u32,
    #[serde(default)]
    pub screen_dim_percent: u8,
    #[serde(default)]
    pub auto_clean_ram_interval_enabled: bool,
    #[serde(default = "default_ram_clean_interval_mins")]
    pub auto_clean_ram_interval_mins: u32,
    #[serde(default)]
    pub auto_clean_ram_threshold_enabled: bool,
    #[serde(default = "default_ram_clean_threshold_pct")]
    pub auto_clean_ram_threshold_pct: u8,
    #[serde(default = "default_hotspot_band")]
    pub hotspot_band: String,
    #[serde(default = "default_true")]
    pub hotspot_no_timeout: bool,
    #[serde(default = "default_true")]
    pub hotspot_max_throughput: bool,
}

fn default_hotspot_band() -> String {
    "5ghz".to_string()
}

fn default_true() -> bool {
    true
}

fn default_dash_style() -> String {
    "dash".to_string()
}

fn default_ram_clean_interval_mins() -> u32 {
    15
}

fn default_ram_clean_threshold_pct() -> u8 {
    80
}

fn default_menu_show_delay() -> u32 {
    250
}

impl Default for WidgetConfig {
    fn default() -> Self {
        let mut heights = HashMap::new();
        heights.insert("network".to_string(), 68.0);
        heights.insert("bluetooth".to_string(), 90.0);
        heights.insert("quota".to_string(), 62.0);
        heights.insert("cpu".to_string(), 56.0);
        heights.insert("ram".to_string(), 56.0);
        heights.insert("gpu".to_string(), 56.0);
        heights.insert("vram".to_string(), 56.0);
        heights.insert("swap".to_string(), 56.0);
        heights.insert("disk".to_string(), 56.0);

        Self {
            x: -1.0,
            y: -1.0,
            width: 350.0,
            height: 560.0,
            opacity: 0.95,
            topmost: false,
            lock_position: false,
            mini_mode: false,
            show_network: true,
            show_cpu: true,
            show_ram: true,
            show_gpu: true,
            show_vram: true,
            show_swap: false,
            show_disk: false,
            show_bluetooth: true,
            show_clients: true,
            show_model_quota: true,
            module_order: vec![
                "network".to_string(),
                "bluetooth".to_string(),
                "quota".to_string(),
                "cpu".to_string(),
                "ram".to_string(),
                "gpu".to_string(),
                "vram".to_string(),
                "swap".to_string(),
                "disk".to_string(),
            ],
            module_heights: heights,
            theme: "cyberpunk".to_string(),
            net_graph_style: "line".to_string(),
            quota_graph_style: "dash".to_string(),
            cpu_graph_style: "line".to_string(),
            ram_graph_style: "line".to_string(),
            gpu_graph_style: "line".to_string(),
            vram_graph_style: "line".to_string(),
            swap_graph_style: "line".to_string(),
            disk_graph_style: "block".to_string(),
            hotspot_traffic_only: false,
            fluid_graphs: true,
            graph_interval_ms: 250,
            selected_wifi_adapter: String::new(),
            bluetooth_favorites: Vec::new(),
            hidden_bluetooth_devices: Vec::new(),
            blocked_domains: vec![
                "telemetry.microsoft.com".to_string(),
                "doubleclick.net".to_string(),
            ],
            blocked_ips: Vec::new(),
            blocker_enabled: true,
            autostart_hotspot: false,
            show_taskbar_gauges: true,
            taskbar_gauge_x: None,
            taskbar_gauge_width: Some(520.0),
            menu_show_delay_ms: 250,
            screen_dim_percent: 0,
            auto_clean_ram_interval_enabled: false,
            auto_clean_ram_interval_mins: 15,
            auto_clean_ram_threshold_enabled: false,
            auto_clean_ram_threshold_pct: 80,
            hotspot_band: "5ghz".to_string(),
            hotspot_no_timeout: true,
            hotspot_max_throughput: true,
        }
    }
}

impl WidgetConfig {
    pub fn config_path() -> PathBuf {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        let dir = exe.parent().unwrap_or_else(|| Path::new("."));
        dir.join("widget_config.json")
    }

    pub fn load() -> Self {
        let mut cfg = Self::default();
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(parsed) = serde_json::from_str::<WidgetConfig>(&data) {
                    cfg = parsed;
                }
            }
        } else {
            let local = Path::new("widget_config.json");
            if local.exists() {
                if let Ok(data) = fs::read_to_string(local) {
                    if let Ok(parsed) = serde_json::from_str::<WidgetConfig>(&data) {
                        cfg = parsed;
                    }
                }
            }
        }
        for m in ["network", "bluetooth", "quota", "cpu", "ram", "gpu", "vram", "swap", "disk"] {
            if !cfg.module_order.iter().any(|x| x == m) {
                if m == "quota" {
                    if let Some(pos) = cfg.module_order.iter().position(|x| x == "bluetooth") {
                        cfg.module_order.insert(pos + 1, m.to_string());
                    } else if let Some(pos) = cfg.module_order.iter().position(|x| x == "cpu") {
                        cfg.module_order.insert(pos, m.to_string());
                    } else {
                        cfg.module_order.push(m.to_string());
                    }
                } else if m == "gpu" {
                    if let Some(pos) = cfg.module_order.iter().position(|x| x == "ram") {
                        cfg.module_order.insert(pos + 1, m.to_string());
                    } else {
                        cfg.module_order.push(m.to_string());
                    }
                } else if m == "vram" {
                    if let Some(pos) = cfg.module_order.iter().position(|x| x == "gpu") {
                        cfg.module_order.insert(pos + 1, m.to_string());
                    } else if let Some(pos) = cfg.module_order.iter().position(|x| x == "ram") {
                        cfg.module_order.insert(pos + 1, m.to_string());
                    } else {
                        cfg.module_order.push(m.to_string());
                    }
                } else {
                    cfg.module_order.push(m.to_string());
                }
            }
            if !cfg.module_heights.contains_key(m) {
                let default_h = match m {
                    "network" => 68.0,
                    "bluetooth" => 90.0,
                    "quota" => 62.0,
                    _ => 56.0,
                };
                cfg.module_heights.insert(m.to_string(), default_h);
            }
        }
        if cfg.cpu_graph_style == "dash" {
            if cfg.gpu_graph_style == "line" {
                cfg.gpu_graph_style = "dash".to_string();
            }
            if cfg.vram_graph_style == "line" {
                cfg.vram_graph_style = "dash".to_string();
            }
            if cfg.quota_graph_style == "line" {
                cfg.quota_graph_style = "dash".to_string();
            }
        }
        if let Some(w) = cfg.taskbar_gauge_width {
            if w < 500.0 {
                cfg.taskbar_gauge_width = Some(520.0);
            }
        } else {
            cfg.taskbar_gauge_width = Some(520.0);
        }
        cfg
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, &json);
            let local = Path::new("widget_config.json");
            if local.exists() || !path.exists() {
                let _ = fs::write(local, &json);
            }
        }
    }

    pub fn get_module_height(&self, module: &str) -> f32 {
        self.module_heights.get(module).copied().unwrap_or_else(|| {
            match module {
                "network" => 68.0,
                "bluetooth" => 90.0,
                "quota" => 62.0,
                _ => 56.0,
            }
        })
    }

    pub fn set_module_height(&mut self, module: &str, height: f32) {
        self.module_heights.insert(module.to_string(), height.clamp(32.0, 260.0));
    }

    pub fn get_theme_palette(&self) -> ThemePalette {
        ThemePalette::for_theme(&self.theme)
    }

    pub fn apply_theme_preset(&mut self, theme_id: &str) {
        self.theme = theme_id.to_string();
        if matches!(
            theme_id,
            "retro_amber" | "vfd_green" | "corvette_red" | "ferrari_dash" | "nissan_300zx" | "matrix" | "hackers" | "synthwave"
        ) {
            // Automatically switch graphs to automotive digital dash style!
            self.net_graph_style = "dash".to_string();
            self.quota_graph_style = "dash".to_string();
            self.cpu_graph_style = "dash".to_string();
            self.ram_graph_style = "dash".to_string();
            self.gpu_graph_style = "dash".to_string();
            self.vram_graph_style = "dash".to_string();
            self.swap_graph_style = "dash".to_string();
            self.disk_graph_style = "dash".to_string();
        }
    }
}
