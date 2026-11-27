#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod bluetooth;
mod config;
mod dummy;
mod geo;
mod globe;
mod globe_data;
mod graph;
mod hotspot;
mod inspector_ui;
mod log_viewer;
mod logger;
mod memclean;
mod quota;
mod skins;
mod stats;
mod traffic;
mod tray;
mod window_fx;

use bluetooth::{BluetoothDevice, BluetoothManager};
use config::{WidgetConfig, ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, ACCENT_YELLOW};
use eframe::egui::{
    self, epaint::PathShape, pos2, Align, Color32, CursorIcon, Layout, Popup, PopupCloseBehavior, Pos2, Rect, RectAlign,
    ResizeDirection, RichText, Stroke, Vec2, ViewportCommand, WidgetText,
};
use graph::FluidGraph;
use hotspot::{HotspotManager, HotspotState};
use inspector_ui::InspectorUI;
use log_viewer::LogViewerUI;
use logger::log_info;
use stats::StatsCollector;
use std::time::{Duration, Instant};
use traffic::TrafficInspector;
use tray::{create_cyberpunk_icon, SystemTray, TrayAction};

fn format_bytes(bytes: f64, is_rate: bool) -> String {
    let power = 1024.0;
    let mut size = bytes;
    let mut n = 0;
    let labels = ["", "K", "M", "G", "T"];
    while size >= power && n < labels.len() - 1 {
        size /= power;
        n += 1;
    }
    let suffix = if is_rate { "B/s" } else { "B" };
    format!("{:.1} {}{}", size, labels[n], suffix)
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d {:02}h {:02}m", days, hours, mins)
    } else {
        format!("{:02}h {:02}m", hours, mins)
    }
}

#[derive(Clone, Copy)]
struct TaskbarSmoothState {
    cpu: f32,
    ram: f32,
    gpu: f32,
    vram: f32,
    dl: f32,
    ul: f32,
    fuel: f32,
    pk_cpu: f32,
    pk_ram: f32,
    pk_gpu: f32,
    pk_vram: f32,
    pk_dl: f32,
    pk_ul: f32,
    last_frame: Instant,
}

impl Default for TaskbarSmoothState {
    fn default() -> Self {
        Self {
            cpu: 0.0,
            ram: 0.0,
            gpu: 0.0,
            vram: 0.0,
            dl: 0.0,
            ul: 0.0,
            fuel: 100.0,
            pk_cpu: 0.0,
            pk_ram: 0.0,
            pk_gpu: 0.0,
            pk_vram: 0.0,
            pk_dl: 0.0,
            pk_ul: 0.0,
            last_frame: Instant::now(),
        }
    }
}

struct NetMonitorApp {
    config: WidgetConfig,
    stats: StatsCollector,
    hotspot: HotspotManager,
    bluetooth: BluetoothManager,
    traffic: TrafficInspector,
    inspector_ui: InspectorUI,
    log_viewer: LogViewerUI,
    tray: SystemTray,
    quota: quota::QuotaCollector,

    // Graphs
    net_graph: FluidGraph,
    cpu_graph: FluidGraph,
    ram_graph: FluidGraph,
    gpu_graph: FluidGraph,
    vram_graph: FluidGraph,
    swap_graph: FluidGraph,
    disk_graph: FluidGraph,

    last_graph_sample: Instant,
    last_position_save: Instant,
    show_settings_modal: bool,
    show_bluetooth_settings_modal: bool,
    show_admin_elevation_modal: bool,
    show_hotspot_password: bool,
    hotspot_opt_status: Option<(Instant, String)>,
    hotspot_edit_ssid: String,
    hotspot_edit_password: String,
    hotspot_cred_status: Option<(Instant, String)>,
    gpu_info: String,
    is_admin: bool,
    autostart_enabled: bool,
    opacity_applied_value: f32,
    unlocked_module: Option<String>,
    module_rects: Vec<(String, Rect)>,
    pre_mini_height: f32,
    is_window_dragging: bool,
    drag_pos: Pos2,
    show_taskbar_context_menu: bool,
    mem_cleaner: memclean::MemoryCleaner,
    last_auto_clean_interval: Instant,
    last_auto_clean_threshold: Instant,
    tb_smooth: TaskbarSmoothState,
}

#[derive(Clone, Copy, Default)]
pub struct SubmenuHoverTracker {
    pub hovered_id: Option<egui::Id>,
    pub hover_start: f64,
    pub open_id: Option<egui::Id>,
    pub last_pass: u64,
}

impl NetMonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Enable true native multi-viewports so Taskbar Micro-Gauges can render as an independent OS window
        cc.egui_ctx.set_embed_viewports(false);

        let mut cfg = WidgetConfig::load();
        let theme = cfg.get_theme_palette();

        // Theme visuals setup with rich hover and button-press tactile feedback
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = theme.bg;
        visuals.window_fill = theme.bg;
        visuals.window_stroke = Stroke::new(1.0, theme.border);
        visuals.override_text_color = Some(theme.text_bright);

        visuals.widgets.inactive.bg_fill = theme.card_bg;
        visuals.widgets.inactive.weak_bg_fill = theme.card_bg;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, theme.border);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, theme.text_bright);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(3);

        let hover_bg = Color32::from_rgba_unmultiplied(
            (theme.primary.r() as u16 / 2 + 40).min(255) as u8,
            (theme.primary.g() as u16 / 2 + 40).min(255) as u8,
            (theme.primary.b() as u16 / 2 + 40).min(255) as u8,
            180,
        );
        visuals.widgets.hovered.bg_fill = hover_bg;
        visuals.widgets.hovered.weak_bg_fill = hover_bg;
        visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, theme.primary);
        visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, Color32::WHITE);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(3);
        visuals.widgets.hovered.expansion = 1.0;

        visuals.widgets.active.bg_fill = theme.primary;
        visuals.widgets.active.weak_bg_fill = theme.primary;
        visuals.widgets.active.bg_stroke = Stroke::new(2.0, theme.secondary);
        visuals.widgets.active.fg_stroke = Stroke::new(2.0, Color32::BLACK);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(3);
        visuals.widgets.active.expansion = -0.5;

        cc.egui_ctx.set_visuals(visuals);

        let mut gpu_info = "Hardware GPU (OpenGL)".to_string();
        if let Some(gl) = &cc.gl {
            use eframe::glow::HasContext;
            let renderer = unsafe { gl.get_parameter_string(eframe::glow::RENDERER) };
            gpu_info = format!("{} (OpenGL HW)", renderer);
        }

        let stats = StatsCollector::new();
        stats.set_hotspot_only(cfg.hotspot_traffic_only);

        let hotspot = HotspotManager::new();

        // Smart adapter selection: Detect if system Wi-Fi connection is currently in use and a secondary wireless adapter is available
        let initial_adapters = hotspot.get_adapters();
        let smart_choice = HotspotManager::select_smart_hotspot_adapter(&initial_adapters);
        if let Some(ref smart) = smart_choice {
            let in_use_active = initial_adapters.iter().any(|a| !a.is_virtual && a.status.eq_ignore_ascii_case("Up") && a.name == cfg.selected_wifi_adapter);
            let is_secondary = smart != &cfg.selected_wifi_adapter;
            if cfg.selected_wifi_adapter.is_empty() || (in_use_active && is_secondary) {
                log_info(
                    "HOTSPOT",
                    &format!("Smart detection: Active Wi-Fi connection in use. Defaulting Mobile Hotspot to secondary wireless adapter '{}' with max settings automatically applied.", smart),
                );
                cfg.selected_wifi_adapter = smart.clone();
                cfg.hotspot_max_throughput = true;
                cfg.hotspot_no_timeout = true;
                cfg.save();
            }
        }

        if !cfg.selected_wifi_adapter.is_empty() {
            hotspot.set_selected_adapter(cfg.selected_wifi_adapter.clone());
        }
        if !cfg.hotspot_band.is_empty() {
            hotspot.set_preferred_band(cfg.hotspot_band.clone());
        }
        if cfg.hotspot_max_throughput {
            hotspot.apply_throughput_optimizations(&cfg.selected_wifi_adapter);
        }
        if cfg.autostart_hotspot {
            log_info("HOTSPOT", "Auto-starting Hotspot on application startup as requested...");
            hotspot.start_hotspot();
        }

        let bluetooth = BluetoothManager::new(cfg.bluetooth_favorites.clone());

        let traffic = TrafficInspector::new(
            cfg.blocked_domains.clone(),
            cfg.blocked_ips.clone(),
            cfg.blocker_enabled,
        );
        traffic.start();

        let net_graph = FluidGraph::new(
            vec![theme.primary, theme.secondary],
            None,
            &cfg.net_graph_style,
            cfg.fluid_graphs,
            cfg.graph_interval_ms,
        );
        let cpu_graph = FluidGraph::new(
            vec![theme.primary],
            Some(100.0),
            &cfg.cpu_graph_style,
            cfg.fluid_graphs,
            cfg.graph_interval_ms,
        );
        let ram_graph = FluidGraph::new(
            vec![theme.secondary],
            Some(100.0),
            &cfg.ram_graph_style,
            cfg.fluid_graphs,
            cfg.graph_interval_ms,
        );
        let gpu_graph = FluidGraph::new(
            vec![theme.secondary],
            Some(100.0),
            &cfg.gpu_graph_style,
            cfg.fluid_graphs,
            cfg.graph_interval_ms,
        );
        let vram_graph = FluidGraph::new(
            vec![theme.tertiary],
            Some(100.0),
            &cfg.vram_graph_style,
            cfg.fluid_graphs,
            cfg.graph_interval_ms,
        );
        let swap_graph = FluidGraph::new(
            vec![theme.warning],
            Some(100.0),
            &cfg.swap_graph_style,
            cfg.fluid_graphs,
            cfg.graph_interval_ms,
        );
        let disk_graph = FluidGraph::new(
            vec![theme.tertiary],
            Some(100.0),
            &cfg.disk_graph_style,
            cfg.fluid_graphs,
            cfg.graph_interval_ms,
        );

        let is_admin = window_fx::is_admin();
        let autostart_enabled = window_fx::check_autostart_enabled();
        let pre_mini_height = cfg.height.max(460.0);
        let initial_pos = Pos2::new(cfg.x, cfg.y);

        log_info("APP", &format!("NetMonitor initialized. Admin: {}, GPU: {}, Theme: {}", is_admin, gpu_info, theme.name));

        let mut app = Self {
            config: cfg,
            stats,
            hotspot,
            bluetooth,
            traffic,
            inspector_ui: InspectorUI::new(),
            log_viewer: LogViewerUI::new(),
            tray: SystemTray::new(),
            quota: quota::QuotaCollector::new(),
            net_graph,
            cpu_graph,
            ram_graph,
            gpu_graph,
            vram_graph,
            swap_graph,
            disk_graph,
            last_graph_sample: Instant::now(),
            last_position_save: Instant::now(),
            show_settings_modal: false,
            show_bluetooth_settings_modal: false,
            show_admin_elevation_modal: false,
            show_hotspot_password: false,
            hotspot_opt_status: None,
            hotspot_edit_ssid: String::new(),
            hotspot_edit_password: String::new(),
            hotspot_cred_status: None,
            gpu_info,
            is_admin,
            autostart_enabled,
            opacity_applied_value: -1.0,
            unlocked_module: None,
            module_rects: Vec::new(),
            pre_mini_height,
            is_window_dragging: false,
            drag_pos: initial_pos,
            show_taskbar_context_menu: false,
            mem_cleaner: memclean::MemoryCleaner::new(),
            last_auto_clean_interval: Instant::now(),
            last_auto_clean_threshold: Instant::now() - Duration::from_secs(300),
            tb_smooth: TaskbarSmoothState::default(),
        };
        app.sync_graph_settings();
        if app.config.screen_dim_percent > 0 {
            window_fx::set_screen_dimmer(app.config.screen_dim_percent);
        }
        app
    }

    fn sample_graphs(&mut self) {
        let elapsed = self.last_graph_sample.elapsed().as_millis() as u64;
        if elapsed >= self.config.graph_interval_ms.max(50) {
            let data = self.stats.get();
            self.net_graph
                .push_values(&[data.net_dl_bps as f32, data.net_ul_bps as f32]);
            self.cpu_graph.push_values(&[data.cpu_percent]);
            self.ram_graph.push_values(&[data.ram_percent]);
            self.gpu_graph.push_values(&[data.gpu_percent]);
            self.vram_graph.push_values(&[data.vram_percent]);
            self.swap_graph.push_values(&[data.swap_percent]);
            self.disk_graph.push_values(&[data.disk_percent]);
            self.last_graph_sample = Instant::now();

            // Periodic auto-clean memory check
            self.check_auto_ram_clean(data.ram_percent);
        }
    }

    fn check_auto_ram_clean(&mut self, current_ram_pct: f32) {
        if self.mem_cleaner.is_running() {
            return;
        }

        // 1. Time-based interval auto-clean (every X minutes)
        if self.config.auto_clean_ram_interval_enabled {
            let interval_secs = (self.config.auto_clean_ram_interval_mins.max(1) as u64) * 60;
            if self.last_auto_clean_interval.elapsed().as_secs() >= interval_secs {
                logger::log_info(
                    "RAM-AUTO",
                    &format!(
                        "Auto-clean triggered by time schedule (every {}m)",
                        self.config.auto_clean_ram_interval_mins
                    ),
                );
                self.last_auto_clean_interval = Instant::now();
                self.mem_cleaner.start();
                return;
            }
        }

        // 2. Threshold-based auto-clean (when RAM hits >= Y%) with a 90s cooldown to prevent thrashing
        if self.config.auto_clean_ram_threshold_enabled {
            let target_thresh = self.config.auto_clean_ram_threshold_pct as f32;
            if current_ram_pct >= target_thresh && self.last_auto_clean_threshold.elapsed().as_secs() >= 90 {
                logger::log_info(
                    "RAM-AUTO",
                    &format!(
                        "Auto-clean triggered: RAM usage ({:.1}%) exceeded threshold ({}%)",
                        current_ram_pct, self.config.auto_clean_ram_threshold_pct
                    ),
                );
                self.last_auto_clean_threshold = Instant::now();
                self.mem_cleaner.start();
            }
        }
    }

    fn move_module(&mut self, module_name: &str, dir: isize) {
        if let Some(idx) = self.config.module_order.iter().position(|m| m == module_name) {
            let new_idx = (idx as isize + dir).clamp(0, (self.config.module_order.len() - 1) as isize) as usize;
            if idx != new_idx {
                self.config.module_order.swap(idx, new_idx);
                self.config.save();
            }
        }
    }

    fn sync_graph_settings(&mut self) {
        let f = self.config.fluid_graphs;
        let ms = self.config.graph_interval_ms;
        let theme = self.config.get_theme_palette();
        for g in [
            &mut self.net_graph,
            &mut self.cpu_graph,
            &mut self.ram_graph,
            &mut self.gpu_graph,
            &mut self.vram_graph,
            &mut self.swap_graph,
            &mut self.disk_graph,
        ] {
            g.fluid = f;
            g.interval_ms = ms;
        }
        self.net_graph.style = self.config.net_graph_style.clone();
        self.net_graph.colors = vec![theme.primary, theme.secondary];

        self.cpu_graph.style = self.config.cpu_graph_style.clone();
        self.cpu_graph.colors = vec![theme.primary];

        self.ram_graph.style = self.config.ram_graph_style.clone();
        self.ram_graph.colors = vec![theme.secondary];

        self.gpu_graph.style = self.config.gpu_graph_style.clone();
        self.gpu_graph.colors = vec![theme.secondary];

        self.vram_graph.style = self.config.vram_graph_style.clone();
        self.vram_graph.colors = vec![theme.tertiary];

        self.swap_graph.style = self.config.swap_graph_style.clone();
        self.swap_graph.colors = vec![theme.warning];

        self.disk_graph.style = self.config.disk_graph_style.clone();
        self.disk_graph.colors = vec![theme.tertiary];

        self.net_graph.label = "NET".to_string();
        self.cpu_graph.label = "CPU".to_string();
        self.ram_graph.label = "RAM".to_string();
        self.gpu_graph.label = "GPU".to_string();
        self.vram_graph.label = "VRAM".to_string();
        self.swap_graph.label = "SWAP".to_string();
        self.disk_graph.label = "DISK".to_string();
    }

    fn toggle_mini_mode(&mut self, ctx: &egui::Context) {
        self.config.mini_mode = !self.config.mini_mode;
        if self.config.mini_mode {
            if self.config.height > 100.0 {
                self.pre_mini_height = self.config.height;
            }
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(
                self.config.width.max(260.0),
                34.0,
            )));
        } else {
            let restore_h = self.pre_mini_height.max(460.0);
            self.config.height = restore_h;
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(
                self.config.width.max(260.0),
                restore_h,
            )));
        }
        self.config.save();
    }

    pub fn delayed_menu_button<R>(
        ui: &mut egui::Ui,
        title: impl Into<WidgetText>,
        delay_ms: u64,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<egui::InnerResponse<R>> {
        let title_text: WidgetText = title.into();
        let id = ui.make_persistent_id(title_text.text());
        let tracker_id = egui::Id::new("netmonitor_submenu_tracker");
        let pass = ui.ctx().cumulative_pass_nr();

        let mut tracker = ui.ctx().data_mut(|d| {
            let mut t = d.get_temp::<SubmenuHoverTracker>(tracker_id).unwrap_or_default();
            if pass > t.last_pass + 2 {
                t.open_id = None;
                t.hovered_id = None;
            }
            t.last_pass = pass;
            t
        });

        let now = ui.input(|i| i.time);
        let hover_pos = ui.ctx().pointer_hover_pos();
        let delay_sec = (delay_ms as f64) / 1000.0;

        let button_resp = ui.add(egui::Button::new(title_text).right_text("⏵"));

        let is_btn_hovered = button_resp.hovered();
        let is_btn_clicked = button_resp.clicked();

        if is_btn_clicked {
            if tracker.open_id == Some(id) {
                tracker.open_id = None;
            } else {
                tracker.open_id = Some(id);
            }
            tracker.hovered_id = Some(id);
        } else if is_btn_hovered {
            if delay_ms == 0 {
                tracker.open_id = Some(id);
                tracker.hovered_id = Some(id);
            } else if tracker.hovered_id != Some(id) {
                tracker.hovered_id = Some(id);
                tracker.hover_start = now;
                ui.ctx().request_repaint_after(Duration::from_millis(delay_ms));
            } else {
                if now - tracker.hover_start >= delay_sec {
                    tracker.open_id = Some(id);
                } else {
                    ui.ctx().request_repaint_after(Duration::from_millis(30));
                }
            }
        } else if tracker.hovered_id == Some(id) {
            tracker.hovered_id = None;
        }

        let is_open = tracker.open_id == Some(id);

        let popup_res = Popup::from_response(&button_resp)
            .id(id.with("delayed_popup"))
            .open(is_open)
            .align(RectAlign::RIGHT_START)
            .layout(Layout::top_down_justified(Align::Min))
            .frame(egui::Frame::menu(ui.style()))
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                add_contents(ui)
            });

        if let Some(ref pr) = popup_res {
            if pr.response.should_close() {
                tracker.open_id = None;
            } else {
                let in_popup = hover_pos.is_some_and(|p| pr.response.rect.contains(p));
                let moving_toward = ui.input(|i| i.pointer.is_moving_towards_rect(&pr.response.rect));
                if in_popup || moving_toward {
                    // Stay open while cursor is inside popup or moving towards it
                } else if !is_btn_hovered && tracker.open_id == Some(id) {
                    if tracker.hovered_id.is_some() && tracker.hovered_id != Some(id) {
                        let other_hover_time = now - tracker.hover_start;
                        if delay_ms == 0 || other_hover_time >= delay_sec {
                            tracker.open_id = None;
                        }
                    }
                }
            }
        } else if tracker.open_id == Some(id) && !is_btn_hovered {
            tracker.open_id = None;
        }

        ui.ctx().data_mut(|d| {
            d.insert_temp(tracker_id, tracker);
        });

        popup_res
    }

    fn render_global_context_menu(&mut self, ui: &mut egui::Ui, target_module: Option<&str>) {
        let theme = self.config.get_theme_palette();
        let delay_ms = self.config.menu_show_delay_ms as u64;
        ui.style_mut().visuals.override_text_color = Some(theme.text_bright);

        // Per-widget focused context menu when right-clicking a specific module/gauge
        if let Some(mod_name) = target_module {
            let (header_title, label_upper) = match mod_name {
                "quota" => ("AI FUEL TANK", "AI FUEL"),
                "network" => ("NETWORK THROUGHPUT", "NETWORK"),
                "bluetooth" => ("BLUETOOTH HUB", "BLUETOOTH"),
                "cpu" => ("CPU TELEMETRY", "CPU"),
                "ram" => ("RAM MEMORY", "RAM"),
                "gpu" => ("GPU TELEMETRY", "GPU"),
                "vram" => ("VRAM MEMORY", "VRAM"),
                "swap" => ("SWAP MEMORY", "SWAP"),
                "disk" => ("DISK & UPTIME", "DISK"),
                _ => ("MODULE", "MODULE"),
            };
            ui.label(RichText::new(header_title).color(theme.primary).strong().size(11.0));
            ui.separator();

            // 1. Widget-specific primary options
            if mod_name == "quota" {
                Self::delayed_menu_button(ui, "◈ AI Fuel Gauge Style", delay_ms, |ui| {
                    let style_ref = &mut self.config.quota_graph_style;
                    let mut changed = false;
                    changed |= ui.selectable_value(style_ref, "dash".to_string(), "✦ Theme-Matched Dash (Auto)").clicked();
                    changed |= ui.selectable_value(style_ref, "twin_dial".to_string(), "◐ Twin Analog Fuel Dials").clicked();
                    changed |= ui.selectable_value(style_ref, "digital_bar".to_string(), "▤ Digital Segmented LCD Bars").clicked();
                    changed |= ui.selectable_value(style_ref, "vertical_tubes".to_string(), "▮ Twin Vertical Fuel Cells").clicked();
                    changed |= ui.selectable_value(style_ref, "arc_rings".to_string(), "◯ Minimalist Ring Gauges").clicked();
                    changed |= ui.selectable_value(style_ref, "hud_telemetry".to_string(), "◈ Cockpit HUD Telemetry").clicked();
                    if changed {
                        self.config.save();
                        ui.close();
                    }
                });
                if ui.button("↻ Refresh Quota Now").clicked() {
                    self.quota.refresh();
                    ui.close();
                }
                let q_info = self.quota.get_data();
                ui.label(RichText::new(format!("Status: {}", q_info.status_text)).size(9.5).color(theme.text_dim));
            } else if mod_name == "bluetooth" {
                if ui.button("⚙ Bluetooth Settings & Adapter...").clicked() {
                    self.show_bluetooth_settings_modal = true;
                    ui.close();
                }
            } else if mod_name == "cpu" || mod_name == "ram" || mod_name == "gpu" || mod_name == "vram" || mod_name == "swap" || mod_name == "disk" || mod_name == "network" {
                Self::delayed_menu_button(ui, format!("◈ {} Graph Style", label_upper), delay_ms, |ui| {
                    let style_ref = match mod_name {
                        "network" => &mut self.config.net_graph_style,
                        "cpu" => &mut self.config.cpu_graph_style,
                        "ram" => &mut self.config.ram_graph_style,
                        "gpu" => &mut self.config.gpu_graph_style,
                        "vram" => &mut self.config.vram_graph_style,
                        "swap" => &mut self.config.swap_graph_style,
                        _ => &mut self.config.disk_graph_style,
                    };
                    let c1 = ui
                        .selectable_value(style_ref, "dash".to_string(), "✦ Retro Automotive Digital Dash")
                        .clicked();
                    let c2 = ui
                        .selectable_value(style_ref, "line".to_string(), "╱ Smooth Line Graph")
                        .clicked();
                    let c3 = ui
                        .selectable_value(style_ref, "block".to_string(), "▤ Segmented Block Bar")
                        .clicked();
                    if c1 || c2 || c3 {
                        self.sync_graph_settings();
                        self.config.save();
                        ui.close();
                    }
                });
            }

            if mod_name == "network" {
                if ui.button("↻ Reset Peak & Total Counters").clicked() {
                    self.stats.reset_stats();
                    ui.close();
                }
                if ui.checkbox(&mut self.config.hotspot_traffic_only, "Hotspot Traffic Only").changed() {
                    self.stats.set_hotspot_only(self.config.hotspot_traffic_only);
                    self.config.save();
                }
                if ui.checkbox(&mut self.config.show_clients, "Show Connected Hotspot Devices").changed() {
                    self.config.save();
                }
                if ui.button("◈ Open Traffic Inspector & Blocker").clicked() {
                    self.inspector_ui.is_open = true;
                    ui.close();
                }
            }

            if mod_name == "ram" {
                if ui.button(RichText::new("✦ Clean Working Set RAM Now").color(theme.primary).strong()).clicked() {
                    self.mem_cleaner.start();
                    ui.close();
                }

                Self::delayed_menu_button(ui, "⚙ Automated Memory Cleaning", delay_ms, |ui| {
                    ui.label(RichText::new("Automatic RAM Optimization:").strong().color(theme.primary).size(10.5));
                    ui.separator();

                    let int_label = format!("Clean every {} minutes", self.config.auto_clean_ram_interval_mins);
                    if ui.checkbox(&mut self.config.auto_clean_ram_interval_enabled, int_label).changed() {
                        self.last_auto_clean_interval = Instant::now();
                        self.config.save();
                    }
                    if self.config.auto_clean_ram_interval_enabled {
                        ui.horizontal(|ui| {
                            ui.label("Interval:");
                            for &m in &[5u32, 10, 15, 30, 60] {
                                if ui.selectable_label(self.config.auto_clean_ram_interval_mins == m, format!("{}m", m)).clicked() {
                                    self.config.auto_clean_ram_interval_mins = m;
                                    self.last_auto_clean_interval = Instant::now();
                                    self.config.save();
                                }
                            }
                        });
                        ui.add(egui::Slider::new(&mut self.config.auto_clean_ram_interval_mins, 1..=120).text("mins"));
                    }

                    ui.separator();

                    let thresh_label = format!("Clean when RAM reaches {}%", self.config.auto_clean_ram_threshold_pct);
                    if ui.checkbox(&mut self.config.auto_clean_ram_threshold_enabled, thresh_label).changed() {
                        self.config.save();
                    }
                    if self.config.auto_clean_ram_threshold_enabled {
                        ui.horizontal(|ui| {
                            ui.label("Threshold:");
                            for &pct in &[65u8, 75, 80, 85, 90, 95] {
                                if ui.selectable_label(self.config.auto_clean_ram_threshold_pct == pct, format!("{}%", pct)).clicked() {
                                    self.config.auto_clean_ram_threshold_pct = pct;
                                    self.config.save();
                                }
                            }
                        });
                        ui.add(egui::Slider::new(&mut self.config.auto_clean_ram_threshold_pct, 40..=98).text("%"));
                    }
                });
            }

            ui.separator();

            // 2. Module sizing & layout controls (specific to this widget)
            let cur_h = self.config.get_module_height(mod_name);
            Self::delayed_menu_button(ui, format!("↕ Module Height ({:.0}px)", cur_h), delay_ms, |ui| {
                let mut h = cur_h;
                if ui.add(egui::Slider::new(&mut h, 32.0..=260.0).text("px")).changed() {
                    self.config.set_module_height(mod_name, h);
                    self.config.save();
                }
                ui.separator();
                for &(preset_label, val) in &[
                    ("Compact (40 px)", 40.0f32),
                    ("Normal (60 px)", 60.0),
                    ("Tall (90 px)", 90.0),
                    ("Expanded (130 px)", 130.0),
                    ("Giant Dash (190 px)", 190.0),
                ] {
                    if ui.selectable_label((h - val).abs() < 5.0, preset_label).clicked() {
                        self.config.set_module_height(mod_name, val);
                        self.config.save();
                        ui.close();
                    }
                }
            });

            let is_unlocked = self.unlocked_module.as_deref() == Some(mod_name);
            let unlock_label = if is_unlocked {
                format!("Lock {} Position", label_upper)
            } else {
                format!("Unlock {} to Drag/Move", label_upper)
            };
            if ui.button(unlock_label).clicked() {
                if is_unlocked {
                    self.unlocked_module = None;
                } else {
                    self.unlocked_module = Some(mod_name.to_string());
                }
                ui.close();
            }

            ui.horizontal(|ui| {
                if ui.button("▲ Move Up").clicked() {
                    self.move_module(mod_name, -1);
                    ui.close();
                }
                if ui.button("▼ Move Down").clicked() {
                    self.move_module(mod_name, 1);
                    ui.close();
                }
            });

            ui.separator();

            if ui.button(RichText::new(format!("✕ Hide {}", label_upper)).color(theme.warning)).clicked() {
                match mod_name {
                    "network" => self.config.show_network = false,
                    "bluetooth" => self.config.show_bluetooth = false,
                    "quota" => self.config.show_model_quota = false,
                    "cpu" => self.config.show_cpu = false,
                    "ram" => self.config.show_ram = false,
                    "gpu" => self.config.show_gpu = false,
                    "vram" => self.config.show_vram = false,
                    "swap" => self.config.show_swap = false,
                    "disk" => self.config.show_disk = false,
                    _ => {}
                }
                self.config.save();
                ui.close();
            }

            return;
        }

        // Global NetMonitor Menu (when right-clicking header bar or empty widget background)
        ui.label(RichText::new("⚡ NETMONITOR TELEMETRY").color(theme.primary).strong().size(11.0));
        ui.separator();

        // 1. Color & Dash Themes Preset Submenu
        Self::delayed_menu_button(ui, "🎨 Color & Dash Themes", delay_ms, |ui| {
            ui.label(RichText::new("Preset Themes:").color(theme.primary).size(10.0).strong());
            for (id, label) in &[
                ("ferrari_dash", "🏎 Ferrari Digital Dash (Rosso Corsa & Giallo)"),
                ("nissan_300zx", "🏎 Nissan 300ZX Digital Dash (Z31 Phosphor)"),
                ("matrix", "🟢 The Matrix Phosphor (Terminal Code & Obsidian)"),
                ("hackers", "⚡ Hackers 1995 (Acid Green & Gibson Purple)"),
                ("retro_amber", "⚡ 80s Retro Digital Dash (Amber & Gold)"),
                ("vfd_green", "📟 80s VFD Phosphor (Subaru XT / Mint)"),
                ("cyberpunk", "🌐 Cyberpunk Neon (Cyan & Pink)"),
                ("synthwave", "🌆 Synthwave 1984 (Neon Pink & Sky)"),
                ("corvette_red", "🏎 C4 Corvette Night Cockpit (Cockpit Redline)"),
                ("stealth_monochrome", "🕶 Stealth OLED (Titanium & Ice White)"),
            ] {
                let is_selected = self.config.theme == *id;
                if ui.selectable_label(is_selected, *label).clicked() {
                    self.config.apply_theme_preset(id);
                    self.sync_graph_settings();
                    self.config.save();
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("🏎 Apply Retro Dash Style to All Graphs").clicked() {
                self.config.net_graph_style = "dash".to_string();
                self.config.cpu_graph_style = "dash".to_string();
                self.config.ram_graph_style = "dash".to_string();
                self.config.swap_graph_style = "dash".to_string();
                self.config.disk_graph_style = "dash".to_string();
                self.sync_graph_settings();
                self.config.save();
                ui.close();
            }
        });

        ui.separator();

        // Visible Modules Submenu
        Self::delayed_menu_button(ui, "📊 Visible Graphs & Modules", delay_ms, |ui| {
            if ui.checkbox(&mut self.config.show_network, "Network Throughput Graph").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_bluetooth, "Bluetooth Controller & Devices").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_model_quota, "AI Model Quota (Antigravity Fuel)").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_clients, "Connected Hotspot Devices").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_cpu, "CPU Usage & Top Process").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_ram, "RAM Usage & Top Process").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_gpu, "GPU Usage & Temperature").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_vram, "VRAM Video Memory Usage").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_swap, "SWAP Memory Usage").changed() {
                self.config.save();
            }
            if ui.checkbox(&mut self.config.show_disk, "Disk & System Uptime").changed() {
                self.config.save();
            }
            ui.separator();
            if ui.checkbox(&mut self.config.show_hotspot_controls, "Hotspot Toggle & Controls Footer").changed() {
                self.config.save();
            }
        });

        // Reorder Modules Submenu
        Self::delayed_menu_button(ui, "↕ Reorder Graphs", delay_ms, |ui| {
            let order = self.config.module_order.clone();
            for (idx, m) in order.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}. {}", idx + 1, m.to_uppercase())).monospace().size(11.0));
                    if ui.small_button("▲").clicked() {
                        self.move_module(m, -1);
                    }
                    if ui.small_button("▼").clicked() {
                        self.move_module(m, 1);
                    }
                });
            }
        });

        // Whole-Window Transparency Submenu
        Self::delayed_menu_button(ui, format!("👻 Transparency ({:.0}%)", self.config.opacity * 100.0), delay_ms, |ui| {
            ui.label(RichText::new("Whole-Window Opacity:").size(10.0).color(ACCENT_CYAN));
            if ui
                .add(egui::Slider::new(&mut self.config.opacity, 0.25..=1.0).text("Alpha"))
                .changed()
            {
                window_fx::apply_window_opacity(self.config.opacity);
                self.opacity_applied_value = self.config.opacity;
                self.config.save();
            }
            ui.separator();
            for &(label, val) in &[
                ("100% (Solid Opaque)", 1.0f32),
                ("95% (Cyberpunk Default)", 0.95),
                ("85% (Subtle Glass)", 0.85),
                ("75% (Translucent)", 0.75),
                ("60% (Ghost Mode)", 0.60),
                ("45% (Ultra Stealth)", 0.45),
            ] {
                if ui
                    .selectable_label((self.config.opacity - val).abs() < 0.02, label)
                    .clicked()
                {
                    self.config.opacity = val;
                    window_fx::apply_window_opacity(val);
                    self.opacity_applied_value = val;
                    self.config.save();
                    ui.close();
                }
            }
        });

        // Screen Dimmer Submenu (Darken entire monitor beyond minimum backlight)
        Self::delayed_menu_button(
            ui,
            format!("🌙 Screen Dimmer ({:.0}%)", self.config.screen_dim_percent),
            delay_ms,
            |ui| {
                ui.label(RichText::new("Darken entire display beyond minimum backlight:").size(10.0).color(theme.secondary));
                if ui.add(egui::Slider::new(&mut self.config.screen_dim_percent, 0..=90).text("% Dim")).changed() {
                    window_fx::set_screen_dimmer(self.config.screen_dim_percent);
                    self.config.save();
                }
                ui.separator();
                for &(label, val) in &[
                    ("Off (0% Normal)", 0u8),
                    ("Mild Dim (20%)", 20),
                    ("Night Reading (40%)", 40),
                    ("Deep Night (60%)", 60),
                    ("Ultra Dark (75%)", 75),
                    ("Midnight Stealth (85%)", 85),
                ] {
                    if ui.selectable_label(self.config.screen_dim_percent == val, label).clicked() {
                        self.config.screen_dim_percent = val;
                        window_fx::set_screen_dimmer(val);
                        self.config.save();
                        ui.close();
                    }
                }
            },
        );

        // Window Size Presets Submenu
        Self::delayed_menu_button(
            ui,
            format!("📐 Window Size ({:.0}×{:.0})", self.config.width, self.config.height),
            delay_ms,
            |ui| {
                ui.label(RichText::new("Drag any window edge/corner or pick preset:").size(10.0).color(Color32::GRAY));
                for &(label, w, h) in &[
                    ("Compact Narrow (300 × 480)", 300.0f32, 480.0f32),
                    ("Standard Default (350 × 520)", 350.0, 520.0),
                    ("Wide Dashboard (480 × 560)", 480.0, 560.0),
                    ("Ultra-Wide Command (620 × 620)", 620.0, 620.0),
                ] {
                    if ui
                        .selectable_label((self.config.width - w).abs() < 10.0, label)
                        .clicked()
                    {
                        self.config.width = w;
                        self.config.height = h;
                        self.config.mini_mode = false;
                        ui.ctx()
                            .send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(w, h)));
                        self.config.save();
                        ui.close();
                    }
                }
            },
        );

        // Fluid Graph Speed Submenu
        Self::delayed_menu_button(ui, "🌊 Graph Fluidity & Speed", delay_ms, |ui| {
            if ui
                .checkbox(&mut self.config.fluid_graphs, "Fluid Smooth Scrolling")
                .changed()
            {
                self.sync_graph_settings();
                self.config.save();
            }
            ui.separator();
            for &(label, ms) in &[
                ("Ultra-Fast (100 ms)", 100u64),
                ("Fast Fluid (250 ms)", 250),
                ("Balanced (500 ms)", 500),
                ("Low-Power (1000 ms)", 1000),
            ] {
                if ui
                    .selectable_label(self.config.graph_interval_ms == ms, label)
                    .clicked()
                {
                    self.config.graph_interval_ms = ms;
                    self.sync_graph_settings();
                    self.config.save();
                    ui.close();
                }
            }
        });

        // Menu Show Delay Setting Submenu
        Self::delayed_menu_button(
            ui,
            format!("⏱ Menu Hover Delay ({}ms)", delay_ms),
            delay_ms,
            |ui| {
                ui.label(RichText::new("Submenu show delay on mouse hover:").size(10.0).color(theme.secondary));
                for &(label, val) in &[
                    ("Instant (0 ms)", 0u32),
                    ("Fast (150 ms)", 150),
                    ("Balanced (250 ms)", 250),
                    ("Relaxed (350 ms)", 350),
                    ("Deliberate (500 ms)", 500),
                    ("Click Only (999 ms)", 999),
                ] {
                    if ui.selectable_label(self.config.menu_show_delay_ms == val, label).clicked() {
                        self.config.menu_show_delay_ms = val;
                        self.config.save();
                        ui.close();
                    }
                }
            },
        );

        ui.separator();

        // Window Behavior & Startup Toggles
        if ui
            .checkbox(&mut self.config.topmost, "📌 Always on Top")
            .changed()
        {
            let level = if self.config.topmost {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            ui.ctx().send_viewport_cmd(ViewportCommand::WindowLevel(level));
            self.config.save();
        }

        if ui
            .checkbox(&mut self.config.lock_position, "🔒 Lock Window Position")
            .changed()
        {
            self.config.save();
        }

        if ui
            .checkbox(&mut self.config.show_taskbar_gauges, "📊 Show Taskbar Micro-Gauges")
            .changed()
        {
            self.config.save();
        }

        let mut mini = self.config.mini_mode;
        if ui.checkbox(&mut mini, "➖ Compact Mini-Bar Mode").changed() {
            let ctx = ui.ctx().clone();
            self.toggle_mini_mode(&ctx);
            ui.close();
        }

        let mut auto_start = self.autostart_enabled;
        if ui
            .checkbox(&mut auto_start, "🚀 Start on System Startup")
            .changed()
        {
            if window_fx::set_autostart_enabled(auto_start) {
                self.autostart_enabled = auto_start;
            }
        }

        if !self.is_admin {
            if ui
                .button(RichText::new("🛡 Restart as Administrator (UAC)").color(ACCENT_YELLOW))
                .clicked()
            {
                window_fx::elevate_self();
                ui.close();
            }
        }

        ui.separator();

        if ui.button("🔍 Open Traffic Inspector & Blocker").clicked() {
            self.inspector_ui.is_open = true;
            ui.close();
        }

        if ui.button("📜 View Troubleshooting & System Logs").clicked() {
            self.log_viewer.is_open = true;
            ui.close();
        }

        if ui.button("⚙ Open Full Settings Window...").clicked() {
            self.show_settings_modal = true;
            ui.close();
        }

        if ui.button("🗕 Hide to System Tray").clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Visible(false));
            ui.close();
        }

        if ui
            .button(RichText::new("✖ Exit NetMonitor").color(ACCENT_RED))
            .clicked()
        {
            self.config.save();
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }
}

impl eframe::App for NetMonitorApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let theme = self.config.get_theme_palette();

        // Ensure theme visuals are live with tactile hover & press responsiveness
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = theme.bg;
        visuals.window_fill = theme.bg;
        visuals.window_stroke = Stroke::new(1.0, theme.border);
        visuals.override_text_color = Some(theme.text_bright);

        visuals.widgets.inactive.bg_fill = theme.card_bg;
        visuals.widgets.inactive.weak_bg_fill = theme.card_bg;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, theme.border);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, theme.text_bright);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(3);

        let hover_bg = Color32::from_rgba_premultiplied(
            (theme.card_bg.r() as u16 + theme.primary.r() as u16 / 5).min(255) as u8,
            (theme.card_bg.g() as u16 + theme.primary.g() as u16 / 5).min(255) as u8,
            (theme.card_bg.b() as u16 + theme.primary.b() as u16 / 5).min(255) as u8,
            245,
        );
        visuals.widgets.hovered.bg_fill = hover_bg;
        visuals.widgets.hovered.weak_bg_fill = hover_bg;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, theme.primary);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, theme.primary);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(3);
        visuals.widgets.hovered.expansion = 1.0;

        visuals.widgets.active.bg_fill = theme.primary;
        visuals.widgets.active.weak_bg_fill = theme.primary;
        visuals.widgets.active.bg_stroke = Stroke::new(2.0, theme.secondary);
        visuals.widgets.active.fg_stroke = Stroke::new(2.0, Color32::BLACK);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(3);
        visuals.widgets.active.expansion = -0.5;

        ctx.set_visuals(visuals);

        // Ensure OS-level whole-window opacity is applied on startup and whenever changed
        if (self.opacity_applied_value - self.config.opacity).abs() > 0.005 {
            window_fx::apply_window_opacity(self.config.opacity);
            self.opacity_applied_value = self.config.opacity;
        }

        // Track window position & size changes, strictly preventing glitching during active dragging
        let (outer_rect_opt, inner_rect_opt) = ctx.input(|i| {
            (
                i.viewport().outer_rect,
                i.viewport().inner_rect,
            )
        });

        // Reset dragging flag when mouse button is released
        if !ctx.input(|i| i.pointer.any_down()) && self.is_window_dragging {
            self.is_window_dragging = false;
            self.config.save();
        }

        if !self.is_window_dragging && !ctx.input(|i| i.pointer.any_down()) {
            if let Some(outer) = outer_rect_opt {
                let (wa_l, wa_t, wa_r, wa_b) = window_fx::get_work_area();
                let w = outer.width();
                let h = outer.height();

                // Non-dragging safety clamp: keep window completely on-screen
                let clamped_x = outer.min.x.clamp(wa_l, (wa_r - w).max(wa_l));
                let clamped_y = outer.min.y.clamp(wa_t, (wa_b - h).max(wa_t));
                if (clamped_x - outer.min.x).abs() > 1.0 || (clamped_y - outer.min.y).abs() > 1.0 {
                    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(clamped_x, clamped_y)));
                    self.config.x = clamped_x;
                    self.config.y = clamped_y;
                } else {
                    let pos_changed = (self.config.x - outer.min.x).abs() > 1.0 || (self.config.y - outer.min.y).abs() > 1.0;
                    if pos_changed {
                        self.config.x = outer.min.x;
                        self.config.y = outer.min.y;
                        if self.last_position_save.elapsed() > Duration::from_millis(300) {
                            self.config.save();
                            self.last_position_save = Instant::now();
                        }
                    }
                }
            }
        }

        if let Some(inner) = inner_rect_opt {
            let w = inner.width();
            let h = inner.height();
            let size_changed = (self.config.width - w).abs() > 1.5
                || (!self.config.mini_mode && (self.config.height - h).abs() > 1.5);
            if size_changed {
                self.config.width = w;
                if !self.config.mini_mode && h > 120.0 {
                    self.config.height = h;
                    self.pre_mini_height = h;
                }
                if self.last_position_save.elapsed() > Duration::from_millis(400) {
                    self.config.save();
                    self.last_position_save = Instant::now();
                }
            }
        }

        // Continuous fluid repaint vs interval-based repaint
        if self.config.fluid_graphs {
            // Smooth ~45-60fps animation for fluid graphs & damped taskbar needles
            let ms = if self.config.show_taskbar_gauges { 22 } else { 33 };
            ctx.request_repaint_after(Duration::from_millis(ms));
        } else {
            ctx.request_repaint_after(Duration::from_millis(self.config.graph_interval_ms.max(250)));
        }
        // Skinned themes with animation (Matrix rain, Synthwave grid, ...) need a smoother tick
        if skins::is_animated(&theme.id) && !self.config.mini_mode {
            ctx.request_repaint_after(Duration::from_millis(33));
        }

        // Poll System Tray Interactions
        while let Some(action) = self.tray.poll_action() {
            match action {
                TrayAction::ShowWindow => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                TrayAction::ToggleHotspot => {
                    self.hotspot.toggle_hotspot();
                }
                TrayAction::OpenInspector => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                    self.inspector_ui.is_open = true;
                }
                TrayAction::Exit => {
                    self.config.save();
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }

        self.sample_graphs();
        let sys_data = self.stats.get();

        // Dynamically update Taskbar & System Tray icon based on Hotspot status & network activity
        let is_hotspot_on = self.hotspot.get_state() == HotspotState::On;
        let has_traffic = sys_data.net_dl_bps > 1024.0 || sys_data.net_ul_bps > 1024.0;
        if let Some(new_icon) = self.tray.update_status(is_hotspot_on, has_traffic) {
            ctx.send_viewport_cmd(ViewportCommand::Icon(Some(new_icon)));
        }

        // Pass hotspot IP to traffic inspector
        self.traffic.set_hotspot_ip(self.hotspot.get_hotspot_ip());

        // Render Secondary Traffic Inspector Window if requested
        self.inspector_ui.render(&ctx, &self.traffic);

        // Render System & Troubleshooting Logs Window if requested
        self.log_viewer.render(&ctx);

        // Render Settings Window if requested
        if self.show_settings_modal {
            self.render_settings_window(&ctx);
        }

        // Render Bluetooth Device Manager Modal if requested
        if self.show_bluetooth_settings_modal {
            self.render_bluetooth_settings_modal(&ctx, &theme);
        }

        // Render Administrator Privileges Elevation Modal if requested
        if self.show_admin_elevation_modal {
            self.render_admin_elevation_modal(&ctx, &theme);
        }

        // Main Widget Frame
        let frame_resp = egui::Frame::new()
            .fill(theme.bg)
            .inner_margin(4.0)
            .show(ui, |ui| {
                // Animated themed backdrop (only when the active theme has one)
                if !self.config.mini_mode && skins::is_animated(&theme.id) {
                    let t = (ui.input(|i| i.time) % 3600.0) as f32;
                    let bg = ui.max_rect();
                    skins::paint_window_background(&ui.painter_at(bg), bg, &theme, t);
                }

                // Header (Draggable, Double-Click for Mini-Mode, Right-Click for Menu)
                self.render_header(ui, &sys_data, &theme);

                if self.config.mini_mode {
                    return;
                }

                ui.add_space(3.0);

                // If a module is currently unlocked for drag-and-drop, show a subtle banner
                if let Some(ref unlocked) = self.unlocked_module.clone() {
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            theme.warning,
                            RichText::new(format!("🔓 Drag [{}] up/down to move", unlocked.to_uppercase()))
                                .size(9.5)
                                .strong(),
                        );
                        if ui.small_button("Done").clicked() {
                            self.unlocked_module = None;
                        }
                    });
                }

                let prev_rects = self.module_rects.clone();
                self.module_rects.clear();

                // Render Modules in user-defined order
                for module in self.config.module_order.clone() {
                    let is_visible = match module.as_str() {
                        "network" => self.config.show_network,
                        "bluetooth" => self.config.show_bluetooth,
                        "quota" => self.config.show_model_quota,
                        "cpu" => self.config.show_cpu,
                        "ram" => self.config.show_ram,
                        "gpu" => self.config.show_gpu,
                        "vram" => self.config.show_vram,
                        "swap" => self.config.show_swap,
                        "disk" => self.config.show_disk,
                        _ => false,
                    };
                    if !is_visible {
                        continue;
                    }

                    let mod_rect = self.render_movable_module(
                        ui,
                        &module,
                        &sys_data,
                        &prev_rects,
                        &theme,
                    );
                    self.module_rects.push((module, mod_rect));
                }

                // Controls Footer (can be toggled on/off from Visible Modules menu)
                if self.config.show_hotspot_controls {
                    self.render_controls_footer(ui, &theme);
                }
            });

        // Attach right-click context menu to any empty background area of the widget
        frame_resp.response.context_menu(|ui| {
            self.render_global_context_menu(ui, None);
        });

        // Interactive Resize Borders & Bottom-Right Resize Grip Handle
        self.render_resize_handles(ui);

        // Render Taskbar Micro-Gauges Window if enabled
        if self.config.show_taskbar_gauges {
            self.render_taskbar_gauges(&ctx, &sys_data, &theme);
        }
    }
}

impl NetMonitorApp {
    pub fn render_styled_button(
        ui: &mut egui::Ui,
        text: &str,
        text_size: f32,
        accent: Color32,
        base_bg: Color32,
        min_size: Vec2,
        theme: &config::ThemePalette,
    ) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(min_size, egui::Sense::click());
        let is_down = resp.is_pointer_button_down_on();
        let is_hovered = resp.hovered();

        if is_hovered || is_down {
            ui.ctx().request_repaint();
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        let draw_rect = if is_down {
            rect.shrink(1.2)
        } else if is_hovered {
            rect.expand(0.8)
        } else {
            rect
        };

        let (bg, stroke, text_color) = if is_down {
            (
                accent,
                Stroke::new(2.0, Color32::WHITE),
                Color32::BLACK,
            )
        } else if is_hovered {
            // High-visibility illuminated hover: vibrant illuminated background, crisp white outline, razor-sharp black text
            let hover_bg = Color32::from_rgb(
                (accent.r() as u16 * 3 / 4 + 60).min(255) as u8,
                (accent.g() as u16 * 3 / 4 + 60).min(255) as u8,
                (accent.b() as u16 * 3 / 4 + 60).min(255) as u8,
            );
            (
                hover_bg,
                Stroke::new(2.0, Color32::WHITE),
                Color32::BLACK,
            )
        } else {
            (
                base_bg,
                Stroke::new(1.0, theme.border),
                accent,
            )
        };

        let painter = ui.painter();
        painter.rect(draw_rect, egui::CornerRadius::same(3), bg, stroke, egui::StrokeKind::Inside);

        painter.text(
            draw_rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::monospace(text_size),
            text_color,
        );

        resp
    }

    fn render_admin_elevation_modal(&mut self, ctx: &egui::Context, theme: &config::ThemePalette) {
        let mut close_modal = false;
        let mut do_elevate = false;

        egui::Window::new("🛡️ ADMINISTRATOR PRIVILEGES REQUIRED")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(egui::Frame::new().fill(theme.bg).stroke(Stroke::new(2.0, theme.warning)).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.set_width(320.0);
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🛡️ ELEVATION NEEDED").color(theme.warning).strong().size(13.0));
                    });
                    ui.separator();
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Hardware Bluetooth radio control, per-device Connect/Disconnect, and Mobile Hotspot require Windows Administrator privileges.\n\nClick 'Elevate to Admin' to prompt Windows UAC once. NetMonitor will restart in this exact position and retain full administrator permissions for the entire session so you won't be prompted again."
                        )
                        .color(theme.text_bright)
                        .size(11.0),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let elev_btn = Self::render_styled_button(
                            ui,
                            "🛡️ ELEVATE TO ADMIN",
                            10.5,
                            theme.primary,
                            theme.card_bg,
                            Vec2::new(165.0, 26.0),
                            theme,
                        );
                        if elev_btn.on_hover_text("Prompt Windows UAC and restart NetMonitor with full permissions").clicked() {
                            do_elevate = true;
                        }

                        let cancel_btn = Self::render_styled_button(
                            ui,
                            "CANCEL",
                            10.5,
                            theme.text_dim,
                            theme.card_bg,
                            Vec2::new(80.0, 26.0),
                            theme,
                        );
                        if cancel_btn.clicked() {
                            close_modal = true;
                        }
                    });
                    ui.add_space(4.0);
                });
            });

        if do_elevate {
            self.config.save();
            self.show_admin_elevation_modal = false;
            window_fx::elevate_self();
        } else if close_modal {
            self.show_admin_elevation_modal = false;
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui, data: &stats::SystemStatsData, theme: &config::ThemePalette) {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 26.0),
            egui::Sense::click_and_drag(),
        );

        let hover_tip = format!(
            "GPU: {}\n• Drag to move window (Dock to any edge/corner)\n• Double-click for Compact Mini-Mode\n• Right-click for Context Menu & Themes",
            self.gpu_info
        );
        let response = response.on_hover_text(hover_tip);

        // Smooth hardware-accelerated native window dragging
        if response.drag_started() && !self.config.lock_position {
            let in_resize_zone = !self.config.mini_mode
                && ui.ctx().input(|i| {
                    if let Some(pos) = i.pointer.interact_pos() {
                        if let Some(s) = i.raw.screen_rect {
                            pos.y < s.top() + 8.0 || pos.x < s.left() + 18.0 || pos.x > s.right() - 18.0
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                });
            if !in_resize_zone {
                self.is_window_dragging = true;
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }
        }
        if response.drag_stopped() {
            self.is_window_dragging = false;
            self.config.save();
        }

        if response.double_clicked() {
            let ctx = ui.ctx().clone();
            self.toggle_mini_mode(&ctx);
        }

        response.context_menu(|ui| {
            self.render_global_context_menu(ui, None);
        });

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, theme.header_bg);
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, theme.border), egui::StrokeKind::Inside);

        if self.config.mini_mode {
            // Compact Mini-Bar Shade Mode: show live DL/UL, CPU, RAM in the header bar
            let summary = format!(
                "↓{} ↑{} | CPU:{:.0}% | RAM:{:.0}% | GPU:{:.0}%",
                format_bytes(data.net_dl_bps, true),
                format_bytes(data.net_ul_bps, true),
                data.cpu_percent,
                data.ram_percent,
                data.gpu_percent
            );
            painter.text(
                Pos2::new(rect.left() + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                summary,
                egui::FontId::monospace(10.0),
                theme.primary,
            );
        } else {
            // Standard Header Title
            let lock_prefix = if self.config.lock_position { "🔒 " } else { "" };
            painter.text(
                Pos2::new(rect.left() + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{}CORE :: SYSTEM TELEMETRY", lock_prefix),
                egui::FontId::monospace(10.5),
                theme.primary,
            );

            // Admin / Standard Privilege Badge (clickable when non-admin to trigger UAC elevation)
            let (badge_text, badge_color) = if self.is_admin {
                ("🛡️ ADMIN", theme.primary)
            } else {
                ("🛡️ USER ▲", theme.warning)
            };

            let badge_rect = Rect::from_center_size(
                Pos2::new(rect.right() - 60.0, rect.center().y),
                Vec2::new(60.0, 19.0),
            );
            let badge_sub_ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(badge_rect));
            let badge_resp = Self::render_styled_button(
                badge_sub_ui,
                badge_text,
                8.5,
                badge_color,
                theme.header_bg,
                Vec2::new(60.0, 19.0),
                theme,
            );
            if !self.is_admin {
                if badge_resp
                    .on_hover_text("Running without Administrator privileges.\nClick to prompt Windows UAC and grant full admin permissions for Bluetooth & Hotspot.")
                    .clicked()
                {
                    self.show_admin_elevation_modal = true;
                }
            } else {
                badge_resp.on_hover_text("Running with full Administrator privileges.\nBluetooth, Hotspot, and Firewall are unrestricted.");
            }
        }

        // Settings gear button (right side)
        let gear_rect = Rect::from_center_size(
            Pos2::new(rect.right() - 14.0, rect.center().y),
            Vec2::new(18.0, 18.0),
        );
        let gear_sub_ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(gear_rect));
        let gear_resp = Self::render_styled_button(
            gear_sub_ui,
            "⚙",
            10.0,
            theme.text_bright,
            theme.header_bg,
            Vec2::new(18.0, 18.0),
            theme,
        );
        if gear_resp
            .on_hover_text("Open Settings & Themes (or Right-Click anywhere)")
            .clicked()
        {
            self.show_settings_modal = !self.show_settings_modal;
        }
    }

    fn render_movable_module(
        &mut self,
        ui: &mut egui::Ui,
        module_name: &str,
        data: &stats::SystemStatsData,
        prev_rects: &[(String, Rect)],
        theme: &config::ThemePalette,
    ) -> Rect {
        let is_unlocked = self.unlocked_module.as_deref() == Some(module_name);
        let stroke = if is_unlocked {
            Stroke::new(1.5, theme.warning)
        } else {
            Stroke::new(1.0, theme.border)
        };

        let mod_h = self.config.get_module_height(module_name);

        let inner = egui::Frame::group(ui.style())
            .fill(theme.card_bg)
            .stroke(stroke)
            .inner_margin(5.0)
            .show(ui, |ui| {
                // Module Header Row with quick ▲/▼ reorder buttons when unlocked
                if is_unlocked {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("✥ DRAG OR MOVE [{}]", module_name.to_uppercase()))
                                .color(theme.warning)
                                .monospace()
                                .size(9.5)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("🔒").on_hover_text("Lock module").clicked() {
                                self.unlocked_module = None;
                            }
                            if ui.small_button("▼").on_hover_text("Move Down").clicked() {
                                self.move_module(module_name, 1);
                            }
                            if ui.small_button("▲").on_hover_text("Move Up").clicked() {
                                self.move_module(module_name, -1);
                            }
                        });
                    });
                    ui.separator();
                }

                match module_name {
                    "network" => self.render_network_contents(ui, data, mod_h, theme),
                    "bluetooth" => self.render_bluetooth_contents(ui, theme),
                    "quota" => self.render_quota_contents(ui, mod_h, theme),
                    "cpu" => self.render_cpu_contents(ui, data, mod_h, theme),
                    "ram" => self.render_ram_contents(ui, data, mod_h, theme),
                    "gpu" => self.render_gpu_contents(ui, data, mod_h, theme),
                    "vram" => self.render_vram_contents(ui, data, mod_h, theme),
                    "swap" => self.render_swap_contents(ui, data, mod_h, theme),
                    "disk" => self.render_disk_contents(ui, data, mod_h, theme),
                    _ => {}
                }

                // Interactive Individual Module Height Handle (drag vertically to resize this module)
                let (handle_rect, handle_resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 6.0),
                    egui::Sense::click_and_drag(),
                );
                let handle_hovered = handle_resp.hovered() || handle_resp.dragged();
                if handle_hovered {
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                }
                if handle_resp.dragged() {
                    let dy = handle_resp.drag_delta().y;
                    let cur_h = self.config.get_module_height(module_name);
                    let new_h = (cur_h + dy).clamp(32.0, 260.0);
                    self.config.set_module_height(module_name, new_h);
                    self.config.save();
                }
                let handle_painter = ui.painter_at(handle_rect);
                let grip_color = if handle_hovered {
                    theme.primary
                } else {
                    Color32::from_rgba_unmultiplied(theme.text_dim.r(), theme.text_dim.g(), theme.text_dim.b(), 35)
                };
                let cy = handle_rect.center().y;
                let cx = handle_rect.center().x;
                handle_painter.line_segment([Pos2::new(cx - 15.0, cy), Pos2::new(cx + 15.0, cy)], Stroke::new(1.5, grip_color));
            });

        let rect = inner.response.rect;

        // Right-click menu on this specific graph module (on the frame itself, no blocking overlay!)
        inner.response.context_menu(|ui| {
            self.render_global_context_menu(ui, Some(module_name));
        });

        // Drag-to-reorder ONLY when unlocked (or Alt+Drag anytime)
        let alt_held = ui.ctx().input(|i| i.modifiers.alt);
        if is_unlocked || alt_held {
            let sense_id = ui.id().with(("mod_drag", module_name));
            let drag_interact = ui.interact(rect, sense_id, egui::Sense::drag());
            if drag_interact.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                if let Some(pos) = ui.ctx().pointer_interact_pos() {
                    for (other_name, other_rect) in prev_rects {
                        if other_name != module_name && other_rect.contains(pos) {
                            if let (Some(i1), Some(i2)) = (
                                self.config.module_order.iter().position(|m| m == module_name),
                                self.config.module_order.iter().position(|m| m == other_name),
                            ) {
                                self.config.module_order.swap(i1, i2);
                                self.config.save();
                                break;
                            }
                        }
                    }
                }
            }
        }

        ui.add_space(2.0);
        rect
    }

    fn render_network_contents(
        &mut self,
        ui: &mut egui::Ui,
        data: &stats::SystemStatsData,
        graph_h: f32,
        theme: &config::ThemePalette,
    ) {
        ui.style_mut().visuals.override_text_color = Some(theme.text_bright);
        ui.horizontal(|ui| {
            let prefix = if self.config.hotspot_traffic_only {
                "HS "
            } else {
                ""
            };
            ui.label(
                RichText::new(format!("{}DL: {}", prefix, format_bytes(data.net_dl_bps, true)))
                    .color(theme.primary)
                    .strong()
                    .monospace()
                    .size(11.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{}UL: {}", prefix, format_bytes(data.net_ul_bps, true)))
                        .color(theme.secondary)
                        .strong()
                        .monospace()
                        .size(11.0),
                );
            });
        });

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Mx: {}", format_bytes(data.net_max_dl, true)))
                    .color(theme.primary)
                    .monospace()
                    .size(9.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Mx: {}", format_bytes(data.net_max_ul, true)))
                        .color(theme.secondary)
                        .monospace()
                        .size(9.0),
                );
            });
        });

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Tot: {}", format_bytes(data.net_total_dl as f64, false)))
                    .color(theme.text_dim)
                    .monospace()
                    .size(9.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Tot: {}", format_bytes(data.net_total_ul as f64, false)))
                        .color(theme.text_dim)
                        .monospace()
                        .size(9.0),
                );
            });
        });

        // Graph scales with individual module height and theme styling
        self.net_graph.render(ui, graph_h, theme);

        // Connected Clients
        if self.config.show_clients {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("CONNECTED DEVICES:")
                        .color(theme.text_dim)
                        .size(9.0)
                        .strong(),
                );
            });

            let clients = self.hotspot.get_clients();
            let hs_state = self.hotspot.get_state();
            let client_totals = self.traffic.get_client_totals();

            if hs_state != HotspotState::On {
                ui.label(
                    RichText::new("Hotspot Inactive")
                        .italics()
                        .color(theme.text_dim)
                        .size(10.0),
                );
            } else if clients.is_empty() {
                ui.label(
                    RichText::new("Searching for devices...")
                        .italics()
                        .color(theme.text_dim)
                        .size(10.0),
                );
            } else {
                for c in clients {
                    let (bytes_transferred, pkt_count) =
                        client_totals.get(&c.ip).copied().unwrap_or((0, 0));
                    let traffic_badge = if pkt_count > 0 {
                        format!(
                            " | {} ({} pkts)",
                            format_bytes(bytes_transferred as f64, false),
                            pkt_count
                        )
                    } else {
                        String::new()
                    };

                    let host_display = if !c.hostname.is_empty() && c.hostname != "Client Device" {
                        format!(" [{}]", c.hostname)
                    } else {
                        String::new()
                    };

                    let row_resp = ui.horizontal_wrapped(|ui| {
                        let text = format!("{} ({}){}{}", c.ip, c.mac, host_display, traffic_badge);
                        let lbl = ui.add(
                            egui::Label::new(
                                RichText::new(text).color(theme.primary).monospace().size(9.5),
                            )
                            .sense(egui::Sense::click()),
                        );

                        if let Some(ref p) = c.open_ports {
                            let (badge_txt, badge_col) = if p.contains("Scanning") {
                                ("⏳ Scanning...", theme.warning)
                            } else {
                                (p.as_str(), theme.secondary)
                            };
                            ui.label(RichText::new(badge_txt).color(badge_col).monospace().size(8.5));
                        } else {
                            if ui.small_button("Scan").on_hover_text("Fast Port & Service Scan").clicked() {
                                self.hotspot.scan_client_ports(c.ip.clone());
                            }
                        }

                        if ui
                            .small_button("Inspect")
                            .on_hover_text("Inspect live packets for this device")
                            .clicked()
                        {
                            self.inspector_ui.filter_device = c.ip.clone();
                            self.traffic.trigger_probe(&c.ip);
                            self.inspector_ui.is_open = true;
                        }
                        lbl
                    });

                    // Right-click menu on individual connected device
                    row_resp.inner.context_menu(|ui| {
                        ui.label(
                            RichText::new(format!("Device: {}", c.ip))
                                .color(theme.primary)
                                .strong(),
                        );
                        ui.label(RichText::new(format!("MAC: {}", c.mac)).size(10.0).color(theme.text_dim));
                        if let Some(ref p) = c.open_ports {
                            ui.label(RichText::new(p).size(9.5).color(theme.secondary));
                        }
                        ui.separator();
                        if ui.button("🔍 Inspect Device Traffic").clicked() {
                            self.inspector_ui.filter_device = c.ip.clone();
                            self.traffic.trigger_probe(&c.ip);
                            self.inspector_ui.is_open = true;
                            ui.close();
                        }
                        if ui.button("📡 Scan Open Ports").clicked() {
                            self.hotspot.scan_client_ports(c.ip.clone());
                            ui.close();
                        }
                        if ui
                            .button(RichText::new("🚫 Block Device IP in Firewall").color(theme.redline))
                            .clicked()
                        {
                            self.traffic.add_blocked_ip(c.ip.clone());
                            if !self.config.blocked_ips.contains(&c.ip) {
                                self.config.blocked_ips.push(c.ip.clone());
                                self.config.save();
                            }
                            ui.close();
                        }
                        if ui.button("📋 Copy IP & MAC Address").clicked() {
                            ui.ctx().copy_text(format!("{} ({})", c.ip, c.mac));
                            ui.close();
                        }
                    });
                }
            }
        }
    }

    fn render_bluetooth_settings_modal(&mut self, ctx: &egui::Context, theme: &config::ThemePalette) {
        let mut close_modal = false;
        let mut open_win_settings = false;
        let mut unhide_all = false;
        let mut toggled_hide: Option<(String, bool)> = None;
        let mut toggled_fav: Option<String> = None;
        let mut toggled_conn: Option<BluetoothDevice> = None;

        let devices = self.bluetooth.get_devices();
        let hidden_set: std::collections::HashSet<String> = self.config.hidden_bluetooth_devices.iter().cloned().collect();

        egui::Window::new("⚙ BLUETOOTH DEVICE MANAGER")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(380.0, 360.0))
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(theme.text_bright);

                ui.label(RichText::new("Manage paired Bluetooth devices, widget visibility, and favorite shortcuts.").size(10.5).color(theme.text_dim));
                ui.add_space(6.0);

                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    if devices.is_empty() {
                        ui.label(RichText::new("No paired devices found.").italics().color(theme.text_dim));
                    } else {
                        for dev in &devices {
                            let is_hidden = hidden_set.contains(&dev.instance_id) || hidden_set.contains(&dev.name);
                            let is_fav = dev.is_favorite;

                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let icon = dev.dev_type.icon();
                                    let status_str = if dev.connected { "● Connected" } else { "○ Disconnected" };
                                    let status_col = if dev.connected { theme.primary } else { theme.text_dim };

                                    ui.label(RichText::new(format!("{} {}", icon, dev.name)).strong().size(11.0));
                                    ui.label(RichText::new(status_str).color(status_col).size(9.5));
                                });

                                ui.horizontal(|ui| {
                                    let mut visible = !is_hidden;
                                    if ui.checkbox(&mut visible, "Visible in widget").changed() {
                                        toggled_hide = Some((dev.instance_id.clone(), visible));
                                    }

                                    let fav_text = if is_fav { "★ Favorited" } else { "☆ Pin Fav" };
                                    let fav_col = if is_fav { theme.tertiary } else { theme.text_dim };
                                    let fav_btn = Self::render_styled_button(ui, fav_text, 9.0, fav_col, theme.card_bg, Vec2::new(76.0, 18.0), theme);
                                    if fav_btn.clicked() {
                                        toggled_fav = Some(dev.instance_id.clone());
                                    }

                                    let (conn_txt, conn_col) = if dev.connected {
                                        ("Disconnect", theme.redline)
                                    } else {
                                        ("Connect", theme.primary)
                                    };
                                    let conn_btn = Self::render_styled_button(ui, conn_txt, 9.0, conn_col, theme.card_bg, Vec2::new(72.0, 18.0), theme);
                                    if conn_btn.clicked() {
                                        toggled_conn = Some(dev.clone());
                                    }
                                });
                            });
                            ui.add_space(2.0);
                        }
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if !self.config.hidden_bluetooth_devices.is_empty() {
                        let unhide_btn = Self::render_styled_button(
                            ui,
                            "👁 Restore All Devices",
                            9.5,
                            theme.secondary,
                            theme.card_bg,
                            Vec2::new(140.0, 24.0),
                            theme,
                        );
                        if unhide_btn.on_hover_text("Make all previously removed/hidden Bluetooth devices visible again").clicked() {
                            unhide_all = true;
                        }
                    }

                    let win_btn = Self::render_styled_button(
                        ui,
                        "🎛 Windows Settings",
                        9.5,
                        theme.text_dim,
                        theme.card_bg,
                        Vec2::new(120.0, 24.0),
                        theme,
                    );
                    if win_btn.on_hover_text("Open Windows Bluetooth & devices settings panel").clicked() {
                        open_win_settings = true;
                    }

                    let space = ui.available_width() - 60.0;
                    if space > 0.0 {
                        ui.add_space(space);
                    }
                    let close_btn = Self::render_styled_button(
                        ui,
                        "Close",
                        10.0,
                        theme.primary,
                        theme.card_bg,
                        Vec2::new(60.0, 24.0),
                        theme,
                    );
                    if close_btn.clicked() {
                        close_modal = true;
                    }
                });
            });

        if unhide_all {
            self.config.hidden_bluetooth_devices.clear();
            self.config.save();
        }
        if let Some((id, visible)) = toggled_hide {
            if visible {
                self.config.hidden_bluetooth_devices.retain(|x| x != &id);
            } else {
                if !self.config.hidden_bluetooth_devices.contains(&id) {
                    self.config.hidden_bluetooth_devices.push(id);
                }
            }
            self.config.save();
        }
        if let Some(id) = toggled_fav {
            let _ = self.bluetooth.toggle_favorite(&id);
            self.config.bluetooth_favorites = self.bluetooth.get_favorites();
            self.config.save();
        }
        if let Some(dev) = toggled_conn {
            self.bluetooth.toggle_device_connection(dev);
        }
        if open_win_settings {
            bluetooth::BluetoothManager::open_settings();
        }
        if close_modal {
            self.show_bluetooth_settings_modal = false;
        }
    }

    fn render_bluetooth_contents(&mut self, ui: &mut egui::Ui, theme: &config::ThemePalette) {
        ui.style_mut().visuals.override_text_color = Some(theme.text_bright);
        let bt_on = self.bluetooth.is_on();
        let is_trans = self.bluetooth.is_transitioning();
        let devices = self.bluetooth.get_devices();

        // 1. Bluetooth Header & Master Toggle Row
        ui.horizontal(|ui| {
            let (status_text, status_col) = if is_trans {
                ("PROCESSING...", theme.warning)
            } else if bt_on {
                ("BLUETOOTH: ACTIVE", theme.secondary)
            } else {
                ("BLUETOOTH: OFF", theme.text_dim)
            };

            ui.label(RichText::new("⚡ BT").color(theme.secondary).strong().monospace().size(11.0));
            ui.label(RichText::new(status_text).color(status_col).strong().monospace().size(10.0));

            // Clean, non-buggy right alignment
            let gear_w = 22.0;
            let toggle_w = 70.0;
            let needed = gear_w + toggle_w + 8.0;
            let space = ui.available_width() - needed;
            if space > 0.0 {
                ui.add_space(space);
            }

            let set_resp = Self::render_styled_button(
                ui,
                "⚙",
                10.0,
                theme.primary,
                theme.card_bg,
                Vec2::new(gear_w, 20.0),
                theme,
            );
            if set_resp.on_hover_text("Open Bluetooth Device Manager (re-add hidden devices & configure)").clicked() {
                self.show_bluetooth_settings_modal = true;
            }

            ui.add_space(4.0);

            let toggle_txt = if is_trans {
                "..."
            } else if bt_on {
                "TURN OFF"
            } else {
                "TURN ON"
            };
            let toggle_col = if bt_on { theme.redline } else { theme.primary };
            let toggle_resp = Self::render_styled_button(
                ui,
                toggle_txt,
                9.0,
                toggle_col,
                theme.card_bg,
                Vec2::new(toggle_w, 20.0),
                theme,
            );
            if toggle_resp.on_hover_text("Toggle Bluetooth Radio On / Off").clicked() && !is_trans {
                self.bluetooth.toggle_bluetooth();
            }
        });

        // 2. Favorite Quick-Connect Device Action Bar
        let favorites: Vec<BluetoothDevice> = devices.iter().filter(|d| d.is_favorite).cloned().collect();
        if !favorites.is_empty() {
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("FAVORITES:").size(9.0).color(theme.text_dim).strong());
                for fav in favorites {
                    let icon = fav.dev_type.icon();
                    let btn_bg = if fav.connected {
                        theme.header_bg
                    } else {
                        theme.card_bg
                    };

                    let label_str = if fav.is_connecting {
                        format!("{} ⏳", icon)
                    } else if fav.connected {
                        format!("{} ✓", icon)
                    } else {
                        icon.to_string()
                    };

                    let tip = format!(
                        "★ {} ({})\nStatus: {}\nClick to {}",
                        fav.name,
                        fav.instance_id,
                        if fav.connected { "Connected" } else { "Disconnected" },
                        if fav.connected { "Disconnect" } else { "Connect" }
                    );

                    let fav_resp = Self::render_styled_button(
                        ui,
                        &label_str,
                        11.0,
                        if fav.connected { theme.primary } else { theme.text_dim },
                        btn_bg,
                        Vec2::new(32.0, 22.0),
                        theme,
                    );

                    if fav_resp.on_hover_text(tip).clicked() {
                        self.bluetooth.toggle_device_connection(fav);
                    }
                }
            });
        }

        ui.add_space(1.0);

        // 3. Paired Devices List (with hidden devices filtered out)
        if !bt_on {
            ui.label(RichText::new("Bluetooth adapter is turned off").italics().color(theme.text_dim).size(10.0));
        } else if devices.is_empty() {
            ui.label(RichText::new("No paired Bluetooth devices detected").italics().color(theme.text_dim).size(10.0));
        } else {
            let hidden_set: std::collections::HashSet<&str> = self
                .config
                .hidden_bluetooth_devices
                .iter()
                .map(|s| s.as_str())
                .collect();
            let visible_devices: Vec<BluetoothDevice> = devices
                .into_iter()
                .filter(|d| !hidden_set.contains(d.instance_id.as_str()) && !hidden_set.contains(d.name.as_str()))
                .collect();

            if visible_devices.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("All devices hidden.").italics().color(theme.text_dim).size(10.0));
                    if ui.small_button("Restore").clicked() {
                        self.config.hidden_bluetooth_devices.clear();
                        self.config.save();
                    }
                });
            }

            let mut remove_device_id: Option<String> = None;
            let mut toggle_fav_id: Option<String> = None;
            let mut toggle_conn_dev: Option<BluetoothDevice> = None;

            for dev in visible_devices.iter().take(6) {
                let dev_clone = dev.clone();
                let inst_id = dev.instance_id.clone();
                let is_fav = dev.is_favorite;
                let dev_name = dev.name.clone();
                let dev_id_for_menu = inst_id.clone();
                let dev_conn = dev.connected;
                let mut row_clicked_remove = false;
                let mut row_clicked_conn = false;
                let mut row_clicked_fav = false;

                ui.horizontal(|ui| {
                    let star_txt = if is_fav { "★" } else { "☆" };
                    let star_col = if is_fav { theme.tertiary } else { theme.text_dim };
                    let star_resp = Self::render_styled_button(
                        ui,
                        star_txt,
                        10.0,
                        star_col,
                        theme.card_bg,
                        Vec2::new(18.0, 18.0),
                        theme,
                    );
                    if star_resp.on_hover_text(if is_fav { "Unpin from favorites" } else { "Pin as favorite device" }).clicked() {
                        toggle_fav_id = Some(inst_id.clone());
                    }

                    let dev_icon = dev.dev_type.icon();
                    // Adaptive width calculation for device label so it scales cleanly
                    let is_input_dev = matches!(dev.dev_type, bluetooth::BluetoothDeviceType::Keyboard | bluetooth::BluetoothDeviceType::Mouse);
                    let (btn_txt, btn_col) = if dev.is_connecting {
                        ("Connecting...", theme.warning)
                    } else if dev.connected {
                        if is_input_dev {
                            ("Active ✓", theme.primary)
                        } else {
                            ("Disconnect", theme.redline)
                        }
                    } else {
                        ("Connect", theme.primary)
                    };

                    let conn_w = 70.0;
                    let avail = ui.available_width();
                    let max_name_w = (avail - conn_w - 12.0).max(40.0);
                    // Approximate characters that fit into max_name_w with 10.0pt monospace
                    let max_chars = ((max_name_w / 6.2).floor() as usize).max(8);
                    let display_name = if dev.name.len() > max_chars {
                        let cut = max_chars.saturating_sub(2);
                        format!("{}..", &dev.name[..cut])
                    } else {
                        dev.name.clone()
                    };

                    let name_col = if dev.connected { theme.primary } else { theme.text_dim };
                    let label_resp = ui.add(
                        egui::Label::new(RichText::new(format!("{} {}", dev_icon, display_name)).color(name_col).monospace().size(10.0))
                            .sense(egui::Sense::click())
                    ).on_hover_text(format!("Right-click for device actions\nName: {}\nStatus: {}", dev.name, if dev.connected { "Connected" } else { "Disconnected" }));

                    label_resp.context_menu(|ui| {
                        ui.label(RichText::new(format!("📱 {}", dev_name)).strong().color(theme.primary));
                        ui.separator();
                        if ui.button(if dev_conn { "⚡ Disconnect Device" } else { "⚡ Connect Device" }).clicked() {
                            row_clicked_conn = true;
                            ui.close();
                        }
                        if ui.button(if is_fav { "☆ Unpin Favorite" } else { "★ Pin Favorite" }).clicked() {
                            row_clicked_fav = true;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(RichText::new("❌ Remove / Hide from Widget").color(theme.redline)).clicked() {
                            row_clicked_remove = true;
                            ui.close();
                        }
                    });

                    // Precise right-align for the Connect/Disconnect action button
                    let space = ui.available_width() - conn_w;
                    if space > 0.0 {
                        ui.add_space(space);
                    }

                    let conn_resp = Self::render_styled_button(
                        ui,
                        btn_txt,
                        9.0,
                        btn_col,
                        theme.card_bg,
                        Vec2::new(conn_w, 20.0),
                        theme,
                    );

                    let tip = if is_input_dev && dev.connected {
                        "Input peripheral (keyboard/mouse) is active and connected.\nTo unpair or remove, right-click the device."
                    } else if dev.connected {
                        "Click to disconnect this device"
                    } else {
                        "Click to connect to this device"
                    };

                    if conn_resp.on_hover_text(tip).clicked() && !dev.is_connecting {
                        if !(is_input_dev && dev.connected) {
                            toggle_conn_dev = Some(dev_clone.clone());
                        }
                    }
                });

                if row_clicked_remove {
                    remove_device_id = Some(dev_id_for_menu.clone());
                }
                if row_clicked_fav {
                    toggle_fav_id = Some(dev_id_for_menu);
                }
                if row_clicked_conn {
                    toggle_conn_dev = Some(dev_clone);
                }
            }

            if let Some(id) = remove_device_id {
                if !self.config.hidden_bluetooth_devices.contains(&id) {
                    self.config.hidden_bluetooth_devices.push(id);
                    self.config.save();
                }
            }
            if let Some(id) = toggle_fav_id {
                let _ = self.bluetooth.toggle_favorite(&id);
                self.config.bluetooth_favorites = self.bluetooth.get_favorites();
                self.config.save();
            }
            if let Some(d) = toggle_conn_dev {
                self.bluetooth.toggle_device_connection(d);
            }
        }
    }

    fn render_quota_contents(
        &mut self,
        ui: &mut egui::Ui,
        graph_h: f32,
        theme: &config::ThemePalette,
    ) {
        let quota_data = self.quota.get_data();
        let anim_t = ui.input(|i| i.time) as f32;
        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), graph_h),
            egui::Sense::click(),
        );

        let painter = ui.painter_at(rect);
        crate::skins::draw_fuel_gauge(&painter, rect, &quota_data, theme, anim_t, &self.config.quota_graph_style);

        let tip = format!(
            "Antigravity AI Model Quota (Right-click to change style)\nGemini 5-Hour Sprint Limit: {:.1}% (Resets in {})\nGemini Weekly Reserve Limit: {:.1}% (Resets in {})\nClaude & GPT 5-Hour Limit: {:.1}%\nClaude & GPT Weekly Limit: {:.1}%\nLanguage Server: {}",
            quota_data.gemini_5h_pct, quota_data.gemini_5h_reset,
            quota_data.gemini_weekly_pct, quota_data.gemini_weekly_reset,
            quota_data.claude_5h_pct, quota_data.claude_weekly_pct,
            quota_data.status_text
        );
        resp.clone().on_hover_text(tip);
        resp.context_menu(|ui| {
            self.render_global_context_menu(ui, Some("quota"));
        });
    }

    fn render_cpu_contents(
        &mut self,
        ui: &mut egui::Ui,
        data: &stats::SystemStatsData,
        graph_h: f32,
        theme: &config::ThemePalette,
    ) {
        let max_chars = ((ui.available_width() / 7.5) as usize).max(16);
        let model_short = if data.cpu_model.len() > 12 {
            &data.cpu_model[..10]
        } else {
            &data.cpu_model
        };
        let top_proc = if data.top_cpu_process.len() > 10 && max_chars < 38 {
            format!("{}..", &data.top_cpu_process[..8])
        } else {
            data.top_cpu_process.clone()
        };
        let stats_text = format!(
            "CPU: {:.0}% {:.1}GHz ({}) {} {}C",
            data.cpu_percent, data.cpu_freq_ghz, top_proc, model_short, data.cpu_cores
        );

        ui.label(
            RichText::new(stats_text)
                .color(theme.primary)
                .monospace()
                .size(10.0),
        );
        self.cpu_graph.render(ui, graph_h, theme);
    }

    fn render_ram_contents(
        &mut self,
        ui: &mut egui::Ui,
        data: &stats::SystemStatsData,
        graph_h: f32,
        theme: &config::ThemePalette,
    ) {
        let max_chars = ((ui.available_width() / 7.5) as usize).max(16);
        let top_proc = if data.top_ram_process.len() > 12 && max_chars < 38 {
            format!("{}..", &data.top_ram_process[..10])
        } else {
            data.top_ram_process.clone()
        };
        let stats_text = format!(
            "RAM: {:.0}% {:.1}/{:.1}GB ({})",
            data.ram_percent, data.ram_used_gb, data.ram_total_gb, top_proc
        );

        let status = self.mem_cleaner.status();
        let now = ui.input(|i| i.time) as f32;

        let label_resp = ui
            .horizontal(|ui| {
                // Feedback is laid out first from the right so the stats label truncates into the remaining space
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    Self::draw_ram_clean_feedback(ui, &status, theme, now);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(stats_text)
                                    .color(theme.secondary)
                                    .monospace()
                                    .size(10.0),
                            )
                            .truncate()
                            .sense(egui::Sense::click()),
                        )
                    })
                    .inner
                })
                .inner
            })
            .inner;

        let graph_rect = self.ram_graph.render(ui, graph_h, theme);
        let graph_resp = ui.interact(graph_rect, ui.id().with("ram_graph_clean_click"), egui::Sense::click());

        Self::draw_ram_clean_overlay(ui, graph_rect, &status, theme);

        let hover_txt = "Click to free RAM — trims the working set of every accessible process";
        let label_resp = label_resp.on_hover_text(hover_txt);
        let graph_resp = graph_resp.on_hover_text(hover_txt);
        if label_resp.hovered() || graph_resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if label_resp.clicked() || graph_resp.clicked() {
            self.mem_cleaner.start();
        }
        if status.phase == memclean::CleanPhase::Running || Self::ram_feedback_visible(&status) {
            ui.ctx().request_repaint();
        }
    }

    /// Seconds the "done" confirmation remains visible.
    const RAM_FEEDBACK_SECS: f32 = 5.0;

    fn ram_feedback_visible(status: &memclean::CleanStatus) -> bool {
        status.phase == memclean::CleanPhase::Done
            && status
                .finished
                .map_or(false, |f| f.elapsed().as_secs_f32() < Self::RAM_FEEDBACK_SECS)
    }

    /// Theme-specific wording: (running verb, done verb)
    fn ram_clean_words(theme_id: &str) -> (&'static str, &'static str) {
        match theme_id {
            "ferrari_dash" | "corvette_red" | "retro_amber" => ("NITRO PURGE", "PURGED"),
            "matrix" | "hackers" => ("> mem.flush()", "> FLUSHED"),
            "vfd_green" => ("FLUSHING RAM", "RAM CLEAR"),
            "synthwave" => ("RAM WIPE", "WIPED"),
            "stealth_monochrome" => ("OPTIMIZING", "FREED"),
            _ => ("OPTIMIZING RAM", "FREED"),
        }
    }

    fn short_bytes(b: u64) -> String {
        let mb = b as f64 / 1_048_576.0;
        if mb >= 1024.0 { format!("{:.2} GB", mb / 1024.0) } else { format!("{:.0} MB", mb) }
    }

    /// Animated confirmation text drawn next to the RAM gauge (inside a right-to-left layout).
    fn draw_ram_clean_feedback(
        ui: &mut egui::Ui,
        status: &memclean::CleanStatus,
        theme: &config::ThemePalette,
        now: f32,
    ) {
        let (run_word, done_word) = Self::ram_clean_words(&theme.id);
        let fade = |c: Color32, a: f32| {
            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
        };
        match status.phase {
            memclean::CleanPhase::Running => {
                let is_term = matches!(theme.id.as_str(), "matrix" | "hackers");
                let spinner = if is_term {
                    ["|", "/", "-", "\\"][((now * 10.0) as usize) % 4]
                } else {
                    ["◐", "◓", "◑", "◒"][((now * 8.0) as usize) % 4]
                };
                let dots = ".".repeat(((now * 4.0) as usize) % 4);
                let pulse = 0.6 + 0.4 * (now * 7.0).sin().abs();
                ui.label(
                    RichText::new(format!("{} {}{:<3}", spinner, run_word, dots))
                        .color(fade(theme.warning, pulse))
                        .monospace()
                        .size(9.5)
                        .strong(),
                );
            }
            memclean::CleanPhase::Done => {
                let Some(fin) = status.finished else { return };
                let t = fin.elapsed().as_secs_f32();
                if t >= Self::RAM_FEEDBACK_SECS {
                    return;
                }
                // Slide in from the right (first 0.35s), hold, then fade out over the last 1.5s
                let slide = (1.0 - (t / 0.35).min(1.0)).powi(2) * 40.0;
                let alpha = if t > Self::RAM_FEEDBACK_SECS - 1.5 {
                    (Self::RAM_FEEDBACK_SECS - t) / 1.5
                } else {
                    (t / 0.25).min(1.0)
                };
                ui.add_space(slide);
                if !status.message.is_empty() {
                    ui.label(RichText::new(format!("⚠ {}", status.message)).color(fade(theme.redline, alpha)).monospace().size(9.0));
                    return;
                }
                let freed = status.freed_bytes();
                // Count-up effect on the figure during the first 0.8s
                let shown = (freed as f64 * ((t / 0.8).min(1.0) as f64)) as u64;
                let flash = if t < 0.6 && ((t * 12.0) as i32) % 2 == 0 { theme.text_bright } else { theme.primary };
                ui.label(
                    RichText::new(format!("✔ {} {} · {} procs", done_word, Self::short_bytes(shown), status.processes_trimmed))
                        .color(fade(flash, alpha))
                        .monospace()
                        .size(9.5)
                        .strong(),
                );
            }
            memclean::CleanPhase::Idle => {}
        }
    }

    /// Visual sweep over the RAM graph while purging, and a quick flash on completion.
    fn draw_ram_clean_overlay(
        ui: &mut egui::Ui,
        rect: Rect,
        status: &memclean::CleanStatus,
        theme: &config::ThemePalette,
    ) {
        let p = ui.painter_at(rect);
        let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
        match status.phase {
            memclean::CleanPhase::Running => {
                let t = status.started.elapsed().as_secs_f32();
                let w = rect.width();
                let band_w = (w * 0.18).max(14.0);
                let x = rect.left() - band_w + ((t * 1.4) % 1.0) * (w + band_w * 2.0);
                for k in 0..6 {
                    let f = k as f32 / 6.0;
                    let bx = x - f * band_w;
                    let r = Rect::from_min_max(pos2(bx, rect.top()), pos2(bx + band_w / 6.0, rect.bottom()));
                    p.rect_filled(r, 0.0, a(theme.primary, (70.0 * (1.0 - f)) as u8));
                }
                p.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.5, a(theme.text_bright, 200)),
                );
                p.rect_stroke(rect, 2.0, Stroke::new(1.2, a(theme.warning, 160)), egui::StrokeKind::Inside);
            }
            memclean::CleanPhase::Done => {
                if let Some(fin) = status.finished {
                    let t = fin.elapsed().as_secs_f32();
                    if t < 0.7 {
                        let al = ((1.0 - t / 0.7) * 90.0) as u8;
                        p.rect_filled(rect, 2.0, a(theme.primary, al));
                        p.rect_stroke(rect, 2.0, Stroke::new(1.5, a(theme.primary, al.saturating_mul(2))), egui::StrokeKind::Inside);
                    }
                }
            }
            memclean::CleanPhase::Idle => {}
        }
    }

    fn render_gpu_contents(
        &mut self,
        ui: &mut egui::Ui,
        data: &stats::SystemStatsData,
        graph_h: f32,
        theme: &config::ThemePalette,
    ) {
        let name_short = if data.gpu_name.len() > 20 {
            format!("{}..", &data.gpu_name[..18])
        } else if data.gpu_name.is_empty() {
            "GPU".to_string()
        } else {
            data.gpu_name.clone()
        };
        let temp_str = if !data.gpu_temp.is_empty() {
            format!(" {}°C", data.gpu_temp)
        } else {
            String::new()
        };
        let stats_text = format!(
            "GPU: {:.0}%{} ({})",
            data.gpu_percent, temp_str, name_short
        );

        ui.label(
            RichText::new(stats_text)
                .color(theme.secondary)
                .monospace()
                .size(10.0),
        );
        self.gpu_graph.render(ui, graph_h, theme);
    }

    fn render_vram_contents(
        &mut self,
        ui: &mut egui::Ui,
        data: &stats::SystemStatsData,
        graph_h: f32,
        theme: &config::ThemePalette,
    ) {
        let stats_text = format!(
            "VRAM: {:.0}%  {:.1}/{:.1}GB",
            data.vram_percent, data.vram_used_gb, data.vram_total_gb
        );
        ui.label(
            RichText::new(stats_text)
                .color(theme.tertiary)
                .monospace()
                .size(10.0),
        );
        self.vram_graph.render(ui, graph_h, theme);
    }

    fn render_swap_contents(
        &mut self,
        ui: &mut egui::Ui,
        data: &stats::SystemStatsData,
        graph_h: f32,
        theme: &config::ThemePalette,
    ) {
        let stats_text = format!(
            "SWAP: {:.0}%  {:.1}GB / {:.1}GB",
            data.swap_percent, data.swap_used_gb, data.swap_total_gb
        );
        ui.label(
            RichText::new(stats_text)
                .color(theme.warning)
                .monospace()
                .size(10.0),
        );
        self.swap_graph.render(ui, graph_h, theme);
    }

    fn render_disk_contents(
        &mut self,
        ui: &mut egui::Ui,
        data: &stats::SystemStatsData,
        graph_h: f32,
        theme: &config::ThemePalette,
    ) {
        let stats_text = format!(
            "DISK: {:.0}% ({:.0}/{:.0}GB) | UP: {}",
            data.disk_percent,
            data.disk_used_gb,
            data.disk_total_gb,
            format_uptime(data.uptime_secs)
        );
        ui.label(
            RichText::new(stats_text)
                .color(theme.tertiary)
                .monospace()
                .size(10.0),
        );
        self.disk_graph.render(ui, graph_h, theme);
    }

    fn render_controls_footer(&mut self, ui: &mut egui::Ui, theme: &config::ThemePalette) {
        ui.add_space(3.0);

        // 1. Hotspot Toggle Button
        let hs_state = self.hotspot.get_state();
        let (btn_text, btn_bg, btn_fg) = match hs_state {
            HotspotState::On => ("HOTSPOT: ACTIVE", theme.primary, Color32::BLACK),
            HotspotState::Off => (
                "HOTSPOT: INACTIVE",
                theme.card_bg,
                theme.primary,
            ),
            HotspotState::InTransition => (
                "PROCESSING...",
                theme.header_bg,
                theme.warning,
            ),
            HotspotState::Unknown => (
                "HOTSPOT: UNKNOWN",
                theme.header_bg,
                theme.text_dim,
            ),
        };

        let btn_resp = Self::render_styled_button(
            ui,
            btn_text,
            11.0,
            btn_fg,
            btn_bg,
            Vec2::new(ui.available_width(), 26.0),
            theme,
        );

        if btn_resp.on_hover_text("Toggle Windows Mobile Hotspot").clicked() {
            if !self.is_admin {
                self.show_admin_elevation_modal = true;
            } else {
                self.hotspot.toggle_hotspot();
            }
        }

        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.config.autostart_hotspot, RichText::new("⚡ Auto-start").size(9.0).color(theme.text_dim)).changed() {
                self.config.save();
            }

            ui.add_space(4.0);

            // 5 GHz / 2.4 GHz Band Pills
            let cur_band = self.config.hotspot_band.to_lowercase();
            let is_5g = cur_band == "5ghz" || cur_band.is_empty();
            let b5_col = if is_5g { theme.primary } else { theme.text_dim };
            let b5_bg = if is_5g { theme.header_bg } else { Color32::TRANSPARENT };
            if ui.add(egui::Button::new(RichText::new("⚡ 5 GHz").size(9.0).color(b5_col).strong()).fill(b5_bg).small())
                .on_hover_text("5 GHz (High Speed / 802.11ac): Up to 867 Mbps, low interference")
                .clicked()
            {
                self.config.hotspot_band = "5ghz".to_string();
                self.config.save();
                self.hotspot.set_preferred_band("5ghz".to_string());
            }

            let is_24g = cur_band == "2.4ghz";
            let b24_col = if is_24g { theme.secondary } else { theme.text_dim };
            let b24_bg = if is_24g { theme.header_bg } else { Color32::TRANSPARENT };
            if ui.add(egui::Button::new(RichText::new("2.4 GHz").size(9.0).color(b24_col)).fill(b24_bg).small())
                .on_hover_text("2.4 GHz (Long Range): Up to 72-150 Mbps")
                .clicked()
            {
                self.config.hotspot_band = "2.4ghz".to_string();
                self.config.save();
                self.hotspot.set_preferred_band("2.4ghz".to_string());
            }

            let is_auto = cur_band == "auto";
            let ba_col = if is_auto { theme.text_bright } else { theme.text_dim };
            let ba_bg = if is_auto { theme.header_bg } else { Color32::TRANSPARENT };
            if ui.add(egui::Button::new(RichText::new("Auto").size(9.0).color(ba_col)).fill(ba_bg).small())
                .on_hover_text("Auto: Windows selects the best available band")
                .clicked()
            {
                self.config.hotspot_band = "auto".to_string();
                self.config.save();
                self.hotspot.set_preferred_band("auto".to_string());
            }

            // Maximize Throughput button
            if ui.add(egui::Button::new(RichText::new("⚡ Max").size(9.0).color(theme.secondary)).small())
                .on_hover_text("Maximize Throughput: Apply 802.11ac driver settings, disable sleep timeout, optimize TCP buffers")
                .clicked()
            {
                self.hotspot.apply_throughput_optimizations(&self.config.selected_wifi_adapter);
                self.hotspot_opt_status = Some((Instant::now(), "⚡ 5GHz AC Max Throughput Active!".to_string()));
            }
        });

        // Hotspot Network Info (SSID, Password, Live Band)
        let ap_cfg = self.hotspot.get_ap_config();
        if let Some(ref ap) = ap_cfg {
            ui.horizontal(|ui| {
                let pwd_display = if self.show_hotspot_password {
                    &ap.password
                } else {
                    "••••••••"
                };
                ui.label(RichText::new(format!("📶 {}: {}", ap.ssid, pwd_display)).size(8.5).color(theme.text_dim).monospace());
                if ui.small_button(if self.show_hotspot_password { "🔒" } else { "👁" }).on_hover_text("Toggle password visibility").clicked() {
                    self.show_hotspot_password = !self.show_hotspot_password;
                }
                ui.label(RichText::new(format!("[{}]", ap.band)).size(8.5).color(if ap.band.contains("5") { theme.primary } else { theme.text_dim }).strong());
            });
        }

        if let Some((t, ref text)) = self.hotspot_opt_status {
            if t.elapsed().as_secs() < 4 {
                ui.label(RichText::new(text).size(8.5).color(theme.secondary).strong());
            }
        }

        ui.add_space(2.0);

        // 2. Traffic Inspector & Auto-Start Row
        ui.horizontal_wrapped(|ui| {
            let avail_w = ui.available_width();
            let spacing = 6.0;
            let target_w = (avail_w - spacing).max(60.0);
            let insp_w = (target_w * 0.62).clamp(110.0, 240.0);
            let auto_w = (target_w - insp_w).clamp(70.0, 160.0);

            let insp_resp = Self::render_styled_button(
                ui,
                "TRAFFIC INSPECTOR",
                9.5,
                theme.primary,
                theme.card_bg,
                Vec2::new(insp_w, 23.0),
                theme,
            );

            if insp_resp
                .on_hover_text("Open Wireshark-style Hotspot Packet Inspector & Domain/IP Blocker")
                .clicked()
            {
                self.inspector_ui.is_open = true;
            }

            let (auto_txt, auto_col) = if self.autostart_enabled {
                ("AUTO: ON", theme.secondary)
            } else {
                ("AUTO: OFF", theme.text_dim)
            };
            let auto_resp = Self::render_styled_button(
                ui,
                auto_txt,
                9.5,
                auto_col,
                theme.card_bg,
                Vec2::new(auto_w, 23.0),
                theme,
            );

            if auto_resp
                .on_hover_text("Toggle Start on System Startup (Windows Task Scheduler / Registry / Linux Autostart)")
                .clicked()
            {
                let target = !self.autostart_enabled;
                if window_fx::set_autostart_enabled(target) {
                    self.autostart_enabled = target;
                }
            }
        });

        ui.add_space(2.0);

        // 3. Wireless Adapter Selector Indicator (shown if multiple adapters exist)
        let adapters = self.hotspot.get_adapters();
        let mut selected = self.hotspot.get_selected_adapter();
        if adapters.len() > 1 {
            egui::ComboBox::from_label(
                RichText::new("Wi-Fi Adapter")
                    .size(9.5)
                    .color(theme.text_dim),
            )
            .selected_text(RichText::new(&selected).size(9.5).color(theme.primary))
            .show_ui(ui, |ui| {
                for adp in adapters {
                    let is_in_use = !adp.is_virtual && adp.status.eq_ignore_ascii_case("Up");
                    let tag = if is_in_use { " [System Wi-Fi]" } else { " [⚡ Dedicated AP]" };
                    let desc_snippet = if adp.description.len() > 18 {
                        &adp.description[..16]
                    } else {
                        &adp.description
                    };
                    let label = format!("{}{}: {}", adp.name, tag, desc_snippet);
                    if ui
                        .selectable_value(&mut selected, adp.name.clone(), label)
                        .clicked()
                    {
                        self.hotspot.set_selected_adapter(selected.clone());
                        self.config.selected_wifi_adapter = selected.clone();
                        self.config.save();
                    }
                }
            });
            ui.add_space(2.0);
        }

        // 4. Logs, Hide to Tray & Exit Row
        ui.horizontal_wrapped(|ui| {
            let total_w = ui.available_width() - 8.0;
            let log_w = (total_w * 0.28).clamp(50.0, 90.0);
            let hide_w = (total_w * 0.44).clamp(70.0, 140.0);
            let exit_w = (total_w - log_w - hide_w - 8.0).clamp(42.0, 80.0);

            let log_resp = Self::render_styled_button(
                ui,
                "LOGS",
                9.5,
                theme.secondary,
                theme.card_bg,
                Vec2::new(log_w, 22.0),
                theme,
            );
            if log_resp.on_hover_text("Open System & Troubleshooting Logs").clicked() {
                self.log_viewer.is_open = true;
            }

            let hide_resp = Self::render_styled_button(
                ui,
                "HIDE TO TRAY",
                9.5,
                theme.text_dim,
                theme.card_bg,
                Vec2::new(hide_w, 22.0),
                theme,
            );
            if hide_resp.on_hover_text("Minimize NetMonitor to System Tray icon").clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Visible(false));
            }

            let exit_resp = Self::render_styled_button(
                ui,
                "EXIT",
                9.5,
                theme.redline,
                theme.card_bg,
                Vec2::new(exit_w, 22.0),
                theme,
            );
            if exit_resp.on_hover_text("Exit application and save all settings").clicked() {
                self.config.save();
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }
        });
    }

    fn render_resize_handles(&self, ui: &mut egui::Ui) {
        if self.config.lock_position || self.config.mini_mode {
            return;
        }
        let screen = ui.ctx().input(|i| i.raw.screen_rect).unwrap_or_else(|| ui.max_rect());
        let corner = 18.0f32;
        let edge = 8.0f32;

        if screen.width() < corner * 2.5 || screen.height() < corner * 2.5 {
            return;
        }

        let handles: [(&'static str, Rect, ResizeDirection, CursorIcon); 8] = [
            // 4 Corners (generous 18x18px hit zones)
            (
                "resize_nw",
                Rect::from_min_max(Pos2::new(screen.left(), screen.top()), Pos2::new(screen.left() + corner, screen.top() + corner)),
                ResizeDirection::NorthWest,
                CursorIcon::ResizeNwSe,
            ),
            (
                "resize_ne",
                Rect::from_min_max(Pos2::new(screen.right() - corner, screen.top()), Pos2::new(screen.right(), screen.top() + corner)),
                ResizeDirection::NorthEast,
                CursorIcon::ResizeNeSw,
            ),
            (
                "resize_sw",
                Rect::from_min_max(Pos2::new(screen.left(), screen.bottom() - corner), Pos2::new(screen.left() + corner, screen.bottom())),
                ResizeDirection::SouthWest,
                CursorIcon::ResizeNeSw,
            ),
            (
                "resize_se",
                Rect::from_min_max(Pos2::new(screen.right() - corner, screen.bottom() - corner), Pos2::new(screen.right(), screen.bottom())),
                ResizeDirection::SouthEast,
                CursorIcon::ResizeNwSe,
            ),
            // 4 Edges (8px hit strip between corners)
            (
                "resize_n",
                Rect::from_min_max(Pos2::new(screen.left() + corner, screen.top()), Pos2::new(screen.right() - corner, screen.top() + edge)),
                ResizeDirection::North,
                CursorIcon::ResizeVertical,
            ),
            (
                "resize_s",
                Rect::from_min_max(Pos2::new(screen.left() + corner, screen.bottom() - edge), Pos2::new(screen.right() - corner, screen.bottom())),
                ResizeDirection::South,
                CursorIcon::ResizeVertical,
            ),
            (
                "resize_w",
                Rect::from_min_max(Pos2::new(screen.left(), screen.top() + corner), Pos2::new(screen.left() + edge, screen.bottom() - corner)),
                ResizeDirection::West,
                CursorIcon::ResizeHorizontal,
            ),
            (
                "resize_e",
                Rect::from_min_max(Pos2::new(screen.right() - edge, screen.top() + corner), Pos2::new(screen.right(), screen.bottom() - corner)),
                ResizeDirection::East,
                CursorIcon::ResizeHorizontal,
            ),
        ];

        let mut hovered_dir: Option<ResizeDirection> = None;

        for (id_str, rect, dir, cursor) in handles {
            let resp = ui.interact(rect, ui.id().with(id_str), egui::Sense::drag());
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(cursor);
                hovered_dir = Some(dir);
            }
            if resp.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
        }

        // Paint sleek visual corner indicators and edge hover glows
        let painter = ui.painter_at(screen);
        let grip_dim = Color32::from_rgba_unmultiplied(100, 100, 100, 130);
        let grip_lit = ACCENT_CYAN;

        // Top-Left corner grip lines
        let nw_col = if hovered_dir == Some(ResizeDirection::NorthWest) { grip_lit } else { grip_dim };
        for offset in [4.0f32, 8.0, 12.0] {
            painter.line_segment(
                [
                    Pos2::new(screen.left() + offset, screen.top() + 2.0),
                    Pos2::new(screen.left() + 2.0, screen.top() + offset),
                ],
                Stroke::new(1.2, nw_col),
            );
        }

        // Top-Right corner grip lines
        let ne_col = if hovered_dir == Some(ResizeDirection::NorthEast) { grip_lit } else { grip_dim };
        for offset in [4.0f32, 8.0, 12.0] {
            painter.line_segment(
                [
                    Pos2::new(screen.right() - offset, screen.top() + 2.0),
                    Pos2::new(screen.right() - 2.0, screen.top() + offset),
                ],
                Stroke::new(1.2, ne_col),
            );
        }

        // Bottom-Left corner grip lines
        let sw_col = if hovered_dir == Some(ResizeDirection::SouthWest) { grip_lit } else { grip_dim };
        for offset in [4.0f32, 8.0, 12.0] {
            painter.line_segment(
                [
                    Pos2::new(screen.left() + offset, screen.bottom() - 2.0),
                    Pos2::new(screen.left() + 2.0, screen.bottom() - offset),
                ],
                Stroke::new(1.2, sw_col),
            );
        }

        // Bottom-Right corner grip lines
        let se_col = if hovered_dir == Some(ResizeDirection::SouthEast) { grip_lit } else { grip_dim };
        for offset in [4.0f32, 8.0, 12.0] {
            painter.line_segment(
                [
                    Pos2::new(screen.right() - offset, screen.bottom() - 2.0),
                    Pos2::new(screen.right() - 2.0, screen.bottom() - offset),
                ],
                Stroke::new(1.2, se_col),
            );
        }

        // Edge highlight line when hovered
        if let Some(dir) = hovered_dir {
            let edge_stroke = Stroke::new(1.5, ACCENT_CYAN);
            match dir {
                ResizeDirection::North => {
                    painter.line_segment(
                        [Pos2::new(screen.left() + corner, screen.top() + 1.0), Pos2::new(screen.right() - corner, screen.top() + 1.0)],
                        edge_stroke,
                    );
                }
                ResizeDirection::South => {
                    painter.line_segment(
                        [Pos2::new(screen.left() + corner, screen.bottom() - 1.0), Pos2::new(screen.right() - corner, screen.bottom() - 1.0)],
                        edge_stroke,
                    );
                }
                ResizeDirection::West => {
                    painter.line_segment(
                        [Pos2::new(screen.left() + 1.0, screen.top() + corner), Pos2::new(screen.left() + 1.0, screen.bottom() - corner)],
                        edge_stroke,
                    );
                }
                ResizeDirection::East => {
                    painter.line_segment(
                        [Pos2::new(screen.right() - 1.0, screen.top() + corner), Pos2::new(screen.right() - 1.0, screen.bottom() - corner)],
                        edge_stroke,
                    );
                }
                _ => {}
            }
        }
    }

    fn render_settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_settings_modal;
        egui::Window::new("NetMonitor Preferences & Configuration")
            .open(&mut open)
            .default_size([360.0, 480.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading(RichText::new("Widget Preferences").color(ACCENT_CYAN));
                    ui.separator();

                    ui.label(RichText::new("Visible Telemetry Modules:").strong());
                    if ui.checkbox(&mut self.config.show_network, "Network Throughput Graph").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_bluetooth, "Bluetooth Controller & Devices").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_clients, "Connected Hotspot Devices").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_cpu, "CPU Usage & Top Process").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_ram, "RAM Usage & Top Process").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_gpu, "GPU Usage & Temperature").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_vram, "VRAM Video Memory Usage").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_swap, "SWAP Usage").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_disk, "Primary Disk & System Uptime").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_model_quota, "AI Model Quota (Antigravity Fuel)").changed() {
                        self.config.save();
                    }
                    if ui.checkbox(&mut self.config.show_hotspot_controls, "Hotspot Toggle & Controls Footer").changed() {
                        self.config.save();
                    }

                    ui.separator();
                    ui.label(RichText::new("Module Display Order (or Right-Click -> Unlock to Drag):").strong());
                    let order = self.config.module_order.clone();
                    for (idx, m) in order.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{}. {}", idx + 1, m.to_uppercase())).monospace());
                            if ui.small_button("▲ Up").clicked() {
                                self.move_module(m, -1);
                            }
                            if ui.small_button("▼ Down").clicked() {
                                self.move_module(m, 1);
                            }
                        });
                    }

                    ui.separator();
                    ui.label(RichText::new("Whole-Window Transparency:").strong().color(ACCENT_CYAN));
                    if ui
                        .add(egui::Slider::new(&mut self.config.opacity, 0.25..=1.0).text("Opacity"))
                        .changed()
                    {
                        window_fx::apply_window_opacity(self.config.opacity);
                        self.opacity_applied_value = self.config.opacity;
                        self.config.save();
                    }

                    ui.separator();
                    ui.label(RichText::new("Graph Fluidity & Interval:").strong().color(ACCENT_GREEN));
                    if ui
                        .checkbox(&mut self.config.fluid_graphs, "Fluid Continuous Scrolling")
                        .changed()
                    {
                        self.sync_graph_settings();
                        self.config.save();
                    }

                    ui.horizontal(|ui| {
                        ui.label("Sample Interval:");
                        for &ms in &[100u64, 250, 500, 1000] {
                            if ui
                                .selectable_label(self.config.graph_interval_ms == ms, format!("{}ms", ms))
                                .clicked()
                            {
                                self.config.graph_interval_ms = ms;
                                self.sync_graph_settings();
                                self.config.save();
                            }
                        }
                    });

                    ui.separator();
                    let theme = self.config.get_theme_palette();
                    ui.label(RichText::new("🎨 Color & Automotive Themes:").strong().color(theme.primary));
                    let theme_options = [
                        ("ferrari_dash", "🏎 Ferrari Digital Dash (Rosso Corsa & Giallo)"),
                        ("nissan_300zx", "🏎 Nissan 300ZX Digital Dash (Z31 Phosphor)"),
                        ("matrix", "🟢 The Matrix Phosphor (Terminal Code & Obsidian)"),
                        ("hackers", "⚡ Hackers 1995 (Acid Green & Gibson Purple)"),
                        ("retro_amber", "⚡ 80s Retro Digital Dash (Amber & Gold)"),
                        ("vfd_green", "📟 80s VFD Phosphor (Subaru XT / Mint)"),
                        ("cyberpunk", "🌐 Cyberpunk Neon (Cyan & Pink)"),
                        ("synthwave", "🌆 Synthwave Outrun 1984 (Neon Pink & Sky)"),
                        ("corvette_red", "🏎 C4 Corvette Night Cockpit (Redline)"),
                        ("stealth_monochrome", "🕶 Stealth OLED (Titanium Slate & Ice)"),
                    ];
                    for (id, label) in theme_options {
                        if ui.selectable_label(self.config.theme == id, label).clicked() {
                            self.config.apply_theme_preset(id);
                            self.sync_graph_settings();
                            self.config.save();
                        }
                    }
                    if ui.button(RichText::new("🏎 Apply Retro Dash Style to All Graphs").color(theme.secondary).strong()).clicked() {
                        self.config.net_graph_style = "dash".to_string();
                        self.config.cpu_graph_style = "dash".to_string();
                        self.config.ram_graph_style = "dash".to_string();
                        self.config.gpu_graph_style = "dash".to_string();
                        self.config.vram_graph_style = "dash".to_string();
                        self.config.swap_graph_style = "dash".to_string();
                        self.config.disk_graph_style = "dash".to_string();
                        self.sync_graph_settings();
                        self.config.save();
                    }

                    ui.separator();
                    ui.label(RichText::new("📏 Individual Module Heights:").strong().color(theme.primary));
                    for (label, key) in [
                        ("Network Card:", "network"),
                        ("Bluetooth Card:", "bluetooth"),
                        ("CPU Card:", "cpu"),
                        ("RAM Card:", "ram"),
                        ("GPU Card:", "gpu"),
                        ("VRAM Card:", "vram"),
                        ("SWAP Card:", "swap"),
                        ("Disk Card:", "disk"),
                    ] {
                        let mut h = self.config.get_module_height(key);
                        if ui.add(egui::Slider::new(&mut h, 32.0..=260.0).text(label)).changed() {
                            self.config.set_module_height(key, h);
                            self.config.save();
                        }
                    }
                    if ui.small_button("↺ Reset Heights to Default").clicked() {
                        self.config.set_module_height("network", 68.0);
                        self.config.set_module_height("bluetooth", 90.0);
                        self.config.set_module_height("cpu", 56.0);
                        self.config.set_module_height("ram", 56.0);
                        self.config.set_module_height("gpu", 56.0);
                        self.config.set_module_height("vram", 56.0);
                        self.config.set_module_height("swap", 56.0);
                        self.config.set_module_height("disk", 56.0);
                        self.config.save();
                    }

                    ui.separator();
                    ui.label(RichText::new("Graph Visual Styles:").strong());
                    for (label, field_name) in [
                        ("NET:", "network"),
                        ("CPU:", "cpu"),
                        ("RAM:", "ram"),
                        ("GPU:", "gpu"),
                        ("VRAM:", "vram"),
                        ("SWAP:", "swap"),
                        ("DISK:", "disk"),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            let style_ref = match field_name {
                                "network" => &mut self.config.net_graph_style,
                                "cpu" => &mut self.config.cpu_graph_style,
                                "ram" => &mut self.config.ram_graph_style,
                                "gpu" => &mut self.config.gpu_graph_style,
                                "vram" => &mut self.config.vram_graph_style,
                                "swap" => &mut self.config.swap_graph_style,
                                _ => &mut self.config.disk_graph_style,
                            };
                            let c1 = ui.selectable_value(style_ref, "line".to_string(), "Line").clicked();
                            let c2 = ui.selectable_value(style_ref, "block".to_string(), "Block").clicked();
                            let c3 = ui.selectable_value(style_ref, "dash".to_string(), "🏎 Dash").clicked();
                            if c1 || c2 || c3 {
                                self.sync_graph_settings();
                                self.config.save();
                            }
                        });
                    }

                    ui.separator();
                    ui.label(RichText::new("Window & System Startup:").strong());
                    if ui.checkbox(&mut self.config.topmost, "Always on Top").changed() {
                        let level = if self.config.topmost {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        };
                        ui.ctx().send_viewport_cmd(ViewportCommand::WindowLevel(level));
                        self.config.save();
                    }

                    if ui.checkbox(&mut self.config.lock_position, "Lock Window Position & Size").changed() {
                        self.config.save();
                    }

                    let mut auto_start = self.autostart_enabled;
                    if ui
                        .checkbox(&mut auto_start, "Start Automatically on System Boot")
                        .changed()
                    {
                        if window_fx::set_autostart_enabled(auto_start) {
                            self.autostart_enabled = auto_start;
                        }
                    }

                    if ui
                        .checkbox(&mut self.config.show_taskbar_gauges, "📊 Show Taskbar Micro-Gauges (Matches Theme)")
                        .changed()
                    {
                        self.config.save();
                    }

                    if self.config.show_taskbar_gauges {
                        ui.horizontal(|ui| {
                            if ui.button("📍 Snap Lower-Left").clicked() {
                                self.config.taskbar_gauge_x = Some(48.0);
                                self.config.save();
                            }
                            let cur_w = self.config.taskbar_gauge_width.unwrap_or(520.0);
                            let mut w = cur_w;
                            if ui.add(egui::Slider::new(&mut w, 380.0..=750.0).text("Width")).changed() {
                                self.config.taskbar_gauge_width = Some(w);
                                self.config.save();
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⏱ Menu Hover Delay:").color(theme.secondary));
                        if ui.add(egui::Slider::new(&mut self.config.menu_show_delay_ms, 0..=600).text("ms")).changed() {
                            self.config.save();
                        }
                    });

                    ui.separator();
                    ui.label(RichText::new("🌙 Night Screen Dimmer:").strong().color(theme.secondary));
                    ui.horizontal(|ui| {
                        ui.label(format!("Dim Level: {}%", self.config.screen_dim_percent));
                        if ui.add(egui::Slider::new(&mut self.config.screen_dim_percent, 0..=90).text("%")).changed() {
                            window_fx::set_screen_dimmer(self.config.screen_dim_percent);
                            self.config.save();
                        }
                    });

                    if ui
                        .checkbox(&mut self.config.autostart_hotspot, "⚡ Auto-start Mobile Hotspot on App Launch")
                        .changed()
                    {
                        self.config.save();
                    }

                    if ui
                        .checkbox(&mut self.config.hotspot_traffic_only, "Monitor Hotspot Traffic Only")
                        .changed()
                    {
                        self.stats.set_hotspot_only(self.config.hotspot_traffic_only);
                        self.config.save();
                    }

                    ui.separator();
                    ui.label(RichText::new("⚡ Mobile Hotspot & Throughput Optimization:").strong().color(theme.primary));
                    ui.horizontal(|ui| {
                        ui.label("Broadcast Band:");
                        for &(band_val, band_label) in &[("5ghz", "⚡ 5 GHz (867 Mbps)"), ("2.4ghz", "🌐 2.4 GHz"), ("auto", "🔄 Auto")] {
                            if ui.selectable_label(self.config.hotspot_band == band_val, band_label).clicked() {
                                self.config.hotspot_band = band_val.to_string();
                                self.config.save();
                                self.hotspot.set_preferred_band(band_val.to_string());
                            }
                        }
                    });

                    if ui
                        .checkbox(&mut self.config.hotspot_no_timeout, "⚡ Prevent Hotspot Inactivity Timeout (Always-On AP)")
                        .on_hover_text("Prevents Windows from shutting down the hotspot when client devices are inactive")
                        .changed()
                    {
                        self.config.save();
                        if self.config.hotspot_no_timeout {
                            self.hotspot.apply_throughput_optimizations(&self.config.selected_wifi_adapter);
                        }
                    }

                    if ui
                        .checkbox(&mut self.config.hotspot_max_throughput, "⚡ High-Throughput Adapter Driver & TCP Window Auto-Tuning")
                        .on_hover_text("Enables 802.11ac VHT mode, LDPC/STBC diversity, and normal TCP window auto-tuning")
                        .changed()
                    {
                        self.config.save();
                        if self.config.hotspot_max_throughput {
                            self.hotspot.apply_throughput_optimizations(&self.config.selected_wifi_adapter);
                        }
                    }

                    if ui.button(RichText::new("⚡ Apply 802.11ac & TCP Optimizations Now").color(theme.secondary).strong()).clicked() {
                        self.hotspot.apply_throughput_optimizations(&self.config.selected_wifi_adapter);
                        self.hotspot_opt_status = Some((Instant::now(), "⚡ High Throughput Optimizations Applied!".to_string()));
                    }

                    ui.add_space(3.0);
                    ui.label(RichText::new("Hotspot Network Credentials:").strong());
                    if self.hotspot_edit_ssid.is_empty() {
                        if let Some(ap) = self.hotspot.get_ap_config() {
                            self.hotspot_edit_ssid = ap.ssid;
                            self.hotspot_edit_password = ap.password;
                        }
                    }

                    ui.horizontal(|ui| {
                        ui.label("Network SSID:");
                        ui.text_edit_singleline(&mut self.hotspot_edit_ssid);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Password (8+ chars):");
                        ui.text_edit_singleline(&mut self.hotspot_edit_password);
                    });
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("💾 Save & Apply Hotspot Credentials").color(theme.primary).strong()).clicked() {
                            if !self.hotspot_edit_ssid.is_empty() && self.hotspot_edit_password.len() >= 8 {
                                self.hotspot.set_ap_credentials(&self.hotspot_edit_ssid, &self.hotspot_edit_password);
                                self.hotspot_cred_status = Some((Instant::now(), "✔ Hotspot credentials updated!".to_string()));
                            } else {
                                self.hotspot_cred_status = Some((Instant::now(), "⚠ Password must be at least 8 characters".to_string()));
                            }
                        }
                        if let Some((t, ref msg)) = self.hotspot_cred_status {
                            if t.elapsed() < Duration::from_secs(5) {
                                ui.label(RichText::new(msg).color(theme.warning).size(10.0));
                            }
                        }
                    });

                    ui.separator();
                    ui.label(RichText::new("Hardware Acceleration:").strong());
                    ui.label(RichText::new(format!("GPU: {}", self.gpu_info)).color(ACCENT_GREEN));

                    ui.separator();
                    ui.label(RichText::new("🧹 RAM Optimization & Working Set:").strong().color(theme.primary));
                    if ui.button(RichText::new("🧹 Clean Working Set RAM Now").color(theme.secondary).strong()).clicked() {
                        self.mem_cleaner.start();
                    }
                    let int_label = format!("Clean every {} minutes", self.config.auto_clean_ram_interval_mins);
                    if ui.checkbox(&mut self.config.auto_clean_ram_interval_enabled, int_label).changed() {
                        self.last_auto_clean_interval = Instant::now();
                        self.config.save();
                    }
                    if self.config.auto_clean_ram_interval_enabled {
                        ui.horizontal(|ui| {
                            ui.label("Interval:");
                            for &m in &[5u32, 10, 15, 30, 60] {
                                if ui.selectable_label(self.config.auto_clean_ram_interval_mins == m, format!("{}m", m)).clicked() {
                                    self.config.auto_clean_ram_interval_mins = m;
                                    self.last_auto_clean_interval = Instant::now();
                                    self.config.save();
                                }
                            }
                        });
                        if ui.add(egui::Slider::new(&mut self.config.auto_clean_ram_interval_mins, 1..=120).text("mins")).changed() {
                            self.config.save();
                        }
                    }

                    let thresh_label = format!("Clean when RAM reaches {}%", self.config.auto_clean_ram_threshold_pct);
                    if ui.checkbox(&mut self.config.auto_clean_ram_threshold_enabled, thresh_label).changed() {
                        self.config.save();
                    }
                    if self.config.auto_clean_ram_threshold_enabled {
                        ui.horizontal(|ui| {
                            ui.label("Threshold:");
                            for &pct in &[65u8, 75, 80, 85, 90, 95] {
                                if ui.selectable_label(self.config.auto_clean_ram_threshold_pct == pct, format!("{}%", pct)).clicked() {
                                    self.config.auto_clean_ram_threshold_pct = pct;
                                    self.config.save();
                                }
                            }
                        });
                        if ui.add(egui::Slider::new(&mut self.config.auto_clean_ram_threshold_pct, 40..=98).text("%")).changed() {
                            self.config.save();
                        }
                    }

                    ui.separator();
                    if ui.button("📜 View System & Troubleshooting Logs").clicked() {
                        self.log_viewer.is_open = true;
                    }
                    if ui.button("Save & Close").clicked() {
                        self.config.save();
                        self.show_settings_modal = false;
                    }
                });
            });
        self.show_settings_modal = open;
    }

    fn render_taskbar_gauges(&mut self, ctx: &egui::Context, sys_data: &stats::SystemStatsData, theme: &config::ThemePalette) {
        let (scr_w, scr_h) = window_fx::get_screen_resolution();
        let (_wa_l, _wa_t, _wa_r, wa_b) = window_fx::get_work_area();
        let tb_height = (scr_h - wa_b).max(36.0).min(64.0);

        let saved_w = self.config.taskbar_gauge_width.unwrap_or(520.0f32);
        let gauge_w = if saved_w < 500.0 {
            520.0f32
        } else {
            saved_w.clamp(380.0, 750.0)
        };
        let gauge_h = (tb_height - 6.0).max(36.0);
        let default_x = 48.0f32; // Lower-left empty space on Windows 11 taskbar

        let mut current_x = self.config.taskbar_gauge_x.unwrap_or(default_x);
        if current_x > (scr_w * 0.6) {
            current_x = default_x;
            self.config.taskbar_gauge_x = Some(default_x);
            self.config.save();
        }

        let y_pos = wa_b + (tb_height - gauge_h) * 0.5;

        // Ensure HWND remains TOPMOST above Shell_TrayWnd and physically clipped to the exact stadium pill
        window_fx::pin_taskbar_window_topmost_pill("NetMonitor Micro-Gauges");

        let vp_id = egui::ViewportId::from_hash_of("netmonitor_taskbar_micro_gauges");
        let vp_builder = egui::ViewportBuilder::default()
            .with_title("NetMonitor Micro-Gauges")
            .with_position(Pos2::new(current_x, y_pos))
            .with_inner_size(Vec2::new(gauge_w, gauge_h))
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_always_on_top()
            .with_taskbar(false);

        let mut next_x = current_x;
        let mut set_width: Option<f32> = None;
        let mut save_needed = false;
        let mut toggle_menu = false;
        let mut focus_main = false;
        let mut trigger_ram_clean = false;
        let ram_status = self.mem_cleaner.status();

        // Compute targets & smoothly damp all taskbar gauges for buttery 60fps needle motion
        let q_data = self.quota.get_data();
        let target_dl = ((sys_data.net_dl_bps / 5_000_000.0) * 100.0).clamp(0.0, 100.0) as f32;
        let target_ul = ((sys_data.net_ul_bps / 2_000_000.0) * 100.0).clamp(0.0, 100.0) as f32;
        let dt = self.tb_smooth.last_frame.elapsed().as_secs_f32().clamp(0.001, 0.1);
        self.tb_smooth.last_frame = Instant::now();
        let alpha = if self.config.fluid_graphs {
            1.0 - (-8.0 * dt).exp()
        } else {
            1.0
        };
        let smooth_step = |cur: &mut f32, target: f32| {
            *cur += (target - *cur) * alpha;
            if (*cur - target).abs() < 0.05 {
                *cur = target;
            }
        };
        smooth_step(&mut self.tb_smooth.cpu, sys_data.cpu_percent);
        smooth_step(&mut self.tb_smooth.ram, sys_data.ram_percent);
        smooth_step(&mut self.tb_smooth.gpu, sys_data.gpu_percent);
        smooth_step(&mut self.tb_smooth.vram, sys_data.vram_percent);
        smooth_step(&mut self.tb_smooth.dl, target_dl);
        smooth_step(&mut self.tb_smooth.ul, target_ul);
        smooth_step(&mut self.tb_smooth.fuel, q_data.gemini_5h_pct);

        let decay = 8.0 * dt;
        let update_peak = |pk: &mut f32, val: f32| {
            if val >= *pk {
                *pk = val;
            } else {
                *pk = (*pk - decay).max(val);
            }
        };
        update_peak(&mut self.tb_smooth.pk_cpu, self.tb_smooth.cpu);
        update_peak(&mut self.tb_smooth.pk_ram, self.tb_smooth.ram);
        update_peak(&mut self.tb_smooth.pk_gpu, self.tb_smooth.gpu);
        update_peak(&mut self.tb_smooth.pk_vram, self.tb_smooth.vram);
        update_peak(&mut self.tb_smooth.pk_dl, self.tb_smooth.dl);
        update_peak(&mut self.tb_smooth.pk_ul, self.tb_smooth.ul);

        let sm = self.tb_smooth;
        let fluid_on = self.config.fluid_graphs;

        ctx.show_viewport_immediate(vp_id, vp_builder, |vp_ctx, _class| {
            if fluid_on {
                vp_ctx.request_repaint_after(Duration::from_millis(22));
            }
            // Sculpted unified sports-car instrument binnacle (full stadium pill housing with layered glass sheen)
            let dash_fill = Color32::from_rgb(
                (theme.graph_bg.r() as u16 * 3 / 4) as u8,
                (theme.graph_bg.g() as u16 * 3 / 4) as u8,
                (theme.graph_bg.b() as u16 * 3 / 4) as u8,
            );
            let pill_r = ((gauge_h * 0.5).round() as u8).min(32);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(dash_fill)
                        .stroke(Stroke::new(1.4, Color32::from_rgba_unmultiplied(theme.primary.r(), theme.primary.g(), theme.primary.b(), 155)))
                        .corner_radius(egui::CornerRadius::same(pill_r))
                        .inner_margin(egui::Margin::symmetric(12, 2)),
                )
                .show(vp_ctx, |ui| {
                    let total_rect = ui.max_rect();

                    // Layered cockpit glass depth: subtle upper glass sheen band + inner recessed bevel + soft horizon line
                    let bg_p = ui.painter();
                    let upper_glass = Rect::from_min_max(
                        Pos2::new(total_rect.left() + 6.0, total_rect.top()),
                        Pos2::new(total_rect.right() - 6.0, total_rect.top() + total_rect.height() * 0.44),
                    );
                    bg_p.rect_filled(
                        upper_glass,
                        egui::CornerRadius::same(pill_r.saturating_sub(2)),
                        Color32::from_rgba_unmultiplied(theme.text_bright.r(), theme.text_bright.g(), theme.text_bright.b(), 10),
                    );
                    bg_p.line_segment(
                        [Pos2::new(total_rect.left() + 14.0, total_rect.top() + 0.5), Pos2::new(total_rect.right() - 14.0, total_rect.top() + 0.5)],
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(theme.text_bright.r(), theme.text_bright.g(), theme.text_bright.b(), 42)),
                    );
                    bg_p.line_segment(
                        [Pos2::new(total_rect.left() + 18.0, total_rect.center().y), Pos2::new(total_rect.right() - 14.0, total_rect.center().y)],
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(theme.primary.r(), theme.primary.g(), theme.primary.b(), 24)),
                    );
                    bg_p.line_segment(
                        [Pos2::new(total_rect.left() + 14.0, total_rect.bottom() - 0.5), Pos2::new(total_rect.right() - 14.0, total_rect.bottom() - 0.5)],
                        Stroke::new(0.8, Color32::from_rgba_unmultiplied(theme.primary.r(), theme.primary.g(), theme.primary.b(), 55)),
                    );

                    // Dedicated right-edge drag handle for interactive width resizing
                    let resize_strip = Rect::from_min_max(
                        Pos2::new(total_rect.right() - 8.0, total_rect.top()),
                        total_rect.max,
                    );
                    let resize_resp = ui.interact(resize_strip, ui.id().with("tb_resize_r"), egui::Sense::drag());
                    if resize_resp.hovered() || resize_resp.dragged() {
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                    }
                    if resize_resp.dragged() {
                        let delta = resize_resp.drag_delta().x;
                        let new_w = (gauge_w + delta).clamp(380.0, 750.0);
                        set_width = Some(new_w);
                    }

                    // Main bar dragging (excluding right edge resize handle)
                    let move_rect = Rect::from_min_max(
                        total_rect.min,
                        Pos2::new(total_rect.right() - 8.0, total_rect.bottom()),
                    );
                    let drag_id = ui.id().with("tb_drag_area");
                    let resp = ui.interact(move_rect, drag_id, egui::Sense::click_and_drag());

                    if resp.dragged() {
                        let delta_x = resp.drag_delta().x;
                        next_x = (current_x + delta_x).clamp(0.0, scr_w - gauge_w);
                        save_needed = true;
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(next_x, y_pos)));
                    }

                    if resp.secondary_clicked() || resize_resp.secondary_clicked() {
                        toggle_menu = true;
                    }

                    if resp.double_clicked() {
                        focus_main = true;
                    }

                    // Visual glow indicator on right border when hovering/resizing
                    if resize_resp.hovered() || resize_resp.dragged() {
                        ui.painter().line_segment(
                            [
                                Pos2::new(total_rect.right() - 2.0, total_rect.top() + 5.0),
                                Pos2::new(total_rect.right() - 2.0, total_rect.bottom() - 5.0),
                            ],
                            Stroke::new(2.0, theme.primary),
                        );
                    }

                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;

                        // Subtle vector 2x3 tactile drag grip on far left
                        let (grip_rect, _) = ui.allocate_exact_size(Vec2::new(8.0, gauge_h - 6.0), egui::Sense::hover());
                        let gp = ui.painter_at(grip_rect);
                        let dot_col = Color32::from_rgba_unmultiplied(theme.text_dim.r(), theme.text_dim.g(), theme.text_dim.b(), 130);
                        for row in 0..3 {
                            for col in 0..2 {
                                let gx = grip_rect.center().x + if col == 0 { -1.8 } else { 1.8 };
                                let gy = grip_rect.center().y + (row as f32 - 1.0) * 4.2;
                                gp.circle_filled(pos2(gx, gy), 1.0, dot_col);
                            }
                        }

                        let show_quota = self.config.show_model_quota;
                        let count = if show_quota { 7.0 } else { 6.0 };
                        let fixed_overhead = 10.0 + (count + 1.0) * 2.0 + 4.0;
                        let sub_w = ((ui.available_width() - fixed_overhead) / count).floor().max(46.0);

                        // 1. CPU Dial
                        let cpu_val = format!("{:.0}", sm.cpu);
                        Self::draw_micro_gauge(
                            ui,
                            "CPU",
                            sm.cpu,
                            &cpu_val,
                            sub_w,
                            gauge_h - 4.0,
                            sm.pk_cpu,
                            theme,
                            false,
                        );

                        // 2. RAM Dial (click to free RAM)
                        let ram_val = format!("{:.0}", sm.ram);
                        let ram_rect = Self::draw_micro_gauge(
                            ui,
                            "RAM",
                            sm.ram,
                            &ram_val,
                            sub_w,
                            gauge_h - 4.0,
                            sm.pk_ram,
                            theme,
                            false,
                        );
                        let ram_resp = ui
                            .interact(ram_rect, ui.id().with("tb_ram_clean"), egui::Sense::click())
                            .on_hover_text("Click to free RAM");
                        if ram_resp.clicked() {
                            trigger_ram_clean = true;
                        }
                        Self::draw_micro_ram_clean_badge(ui, ram_rect, &ram_status, theme);

                        // 3. GPU Dial
                        let gpu_val = format!("{:.0}", sm.gpu);
                        Self::draw_micro_gauge(
                            ui,
                            "GPU",
                            sm.gpu,
                            &gpu_val,
                            sub_w,
                            gauge_h - 4.0,
                            sm.pk_gpu,
                            theme,
                            false,
                        );

                        // 4. VRAM Dial
                        let vram_val = format!("{:.0}", sm.vram);
                        Self::draw_micro_gauge(
                            ui,
                            "VRAM",
                            sm.vram,
                            &vram_val,
                            sub_w,
                            gauge_h - 4.0,
                            sm.pk_vram,
                            theme,
                            false,
                        );

                        // 5. DL Dial
                        let dl_val = if sys_data.net_dl_bps >= 1_000_000.0 {
                            format!("{:.1}M", sys_data.net_dl_bps / 1_000_000.0)
                        } else if sys_data.net_dl_bps >= 1_000.0 {
                            format!("{:.0}K", sys_data.net_dl_bps / 1_000.0)
                        } else {
                            format!("{:.0}B", sys_data.net_dl_bps)
                        };
                        Self::draw_micro_gauge(
                            ui,
                            "DL",
                            sm.dl,
                            &dl_val,
                            sub_w,
                            gauge_h - 4.0,
                            sm.pk_dl,
                            theme,
                            false,
                        );

                        // 6. UL Dial
                        let ul_val = if sys_data.net_ul_bps >= 1_000_000.0 {
                            format!("{:.1}M", sys_data.net_ul_bps / 1_000_000.0)
                        } else if sys_data.net_ul_bps >= 1_000.0 {
                            format!("{:.0}K", sys_data.net_ul_bps / 1_000.0)
                        } else {
                            format!("{:.0}B", sys_data.net_ul_bps)
                        };
                        Self::draw_micro_gauge(
                            ui,
                            "UL",
                            sm.ul,
                            &ul_val,
                            sub_w,
                            gauge_h - 4.0,
                            sm.pk_ul,
                            theme,
                            false,
                        );

                        if show_quota {
                            // 7. AI Model Quota / Fuel Dial (inverted: full = healthy, low = red reserve alert!)
                            let fuel_val = format!("{:.0}", sm.fuel);
                            let fuel_rect = Self::draw_micro_gauge(
                                ui,
                                "FUEL",
                                sm.fuel,
                                &fuel_val,
                                sub_w,
                                gauge_h - 4.0,
                                0.0,
                                theme,
                                true,
                            );
                            let tip = format!(
                                "AI Model Fuel (Antigravity)\nGemini 5h Limit: {:.0}% ({})\nGemini Wk Limit: {:.0}% ({})\nClaude/GPT 5h: {:.0}% · Wk: {:.0}%\nLanguage Server: {}",
                                q_data.gemini_5h_pct, q_data.gemini_5h_reset,
                                q_data.gemini_weekly_pct, q_data.gemini_weekly_reset,
                                q_data.claude_5h_pct, q_data.claude_weekly_pct,
                                q_data.status_text
                            );
                            ui.interact(fuel_rect, ui.id().with("tb_fuel_hover"), egui::Sense::hover())
                                .on_hover_text(tip);
                        }
                    });
                });
        });

        if toggle_menu {
            self.show_taskbar_context_menu = !self.show_taskbar_context_menu;
        }

        if trigger_ram_clean {
            self.mem_cleaner.start();
        }

        if focus_main {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        // Dedicated context menu that floats cleanly ABOVE the taskbar into full view
        if self.show_taskbar_context_menu {
            let menu_vp_id = egui::ViewportId::from_hash_of("netmonitor_taskbar_menu_vp");
            let menu_w = 296.0f32;
            let menu_h = 250.0f32;
            let menu_x = current_x.clamp(8.0, (scr_w - menu_w - 8.0).max(8.0));
            let menu_y = (y_pos - menu_h - 6.0).max(10.0);

            window_fx::pin_taskbar_window_topmost("NetMonitor Micro-Gauges Menu");

            let menu_builder = egui::ViewportBuilder::default()
                .with_title("NetMonitor Micro-Gauges Menu")
                .with_position(Pos2::new(menu_x, menu_y))
                .with_inner_size(Vec2::new(menu_w, menu_h))
                .with_decorations(false)
                .with_transparent(true)
                .with_resizable(false)
                .with_always_on_top()
                .with_taskbar(false);

            let mut close_menu = false;
            let mut reset_to_x: Option<f32> = None;
            let mut set_width: Option<f32> = None;
            let mut toggle_hide = false;
            let mut request_focus_main = false;
            let mut trigger_clean_now = false;

            ctx.show_viewport_immediate(menu_vp_id, menu_builder, |m_ctx, _class| {
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(theme.card_bg)
                            .stroke(Stroke::new(1.0, theme.border))
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(egui::Margin::symmetric(10, 8)),
                    )
                    .show(m_ctx, |ui| {
                        ui.label(RichText::new("⚙ TASKBAR MICRO-GAUGES").strong().color(theme.primary));
                        ui.separator();

                        if ui.button(RichText::new("✦ Clean Working Set RAM Now").color(theme.primary).strong()).clicked() {
                            trigger_clean_now = true;
                            close_menu = true;
                        }

                        // Automated Memory Optimization Sub-section
                        ui.collapsing(RichText::new("⚙ Auto-Clean RAM Options").color(theme.secondary).size(10.5), |ui| {
                            let int_label = format!("Every {} mins", self.config.auto_clean_ram_interval_mins);
                            if ui.checkbox(&mut self.config.auto_clean_ram_interval_enabled, int_label).changed() {
                                self.last_auto_clean_interval = Instant::now();
                                self.config.save();
                            }
                            if self.config.auto_clean_ram_interval_enabled {
                                ui.horizontal(|ui| {
                                    for &m in &[5u32, 10, 15, 30] {
                                        if ui.selectable_label(self.config.auto_clean_ram_interval_mins == m, format!("{}m", m)).clicked() {
                                            self.config.auto_clean_ram_interval_mins = m;
                                            self.last_auto_clean_interval = Instant::now();
                                            self.config.save();
                                        }
                                    }
                                });
                            }

                            let thresh_label = format!("If RAM ≥ {}%", self.config.auto_clean_ram_threshold_pct);
                            if ui.checkbox(&mut self.config.auto_clean_ram_threshold_enabled, thresh_label).changed() {
                                self.config.save();
                            }
                            if self.config.auto_clean_ram_threshold_enabled {
                                ui.horizontal(|ui| {
                                    for &pct in &[70u8, 80, 85, 90] {
                                        if ui.selectable_label(self.config.auto_clean_ram_threshold_pct == pct, format!("{}%", pct)).clicked() {
                                            self.config.auto_clean_ram_threshold_pct = pct;
                                            self.config.save();
                                        }
                                    }
                                });
                            }
                        });

                        ui.separator();
                        if ui.button("⌖ Snap Lower-Left Empty Space").clicked() {
                            reset_to_x = Some(48.0);
                            close_menu = true;
                        }
                        if ui.button("⌖ Snap Far-Left (x=16)").clicked() {
                            reset_to_x = Some(16.0);
                            close_menu = true;
                        }
                        if ui.button("⌖ Snap Center-Left (x=180)").clicked() {
                            reset_to_x = Some(180.0);
                            close_menu = true;
                        }
                        ui.separator();
                        ui.label(RichText::new("Width Preset:").color(theme.secondary).size(10.0));
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            for &(lbl, w_val) in &[
                                ("460px", 460.0f32),
                                ("520px", 520.0),
                                ("580px", 580.0),
                                ("640px", 640.0),
                            ] {
                                let active = (gauge_w - w_val).abs() < 15.0;
                                if ui.selectable_label(active, lbl).clicked() {
                                    set_width = Some(w_val);
                                    close_menu = true;
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("◈ Bring Main Widget to Front").clicked() {
                            request_focus_main = true;
                            close_menu = true;
                        }
                        ui.horizontal(|ui| {
                            if ui.button("× Hide Taskbar Gauges").clicked() {
                                toggle_hide = true;
                                close_menu = true;
                            }
                            if ui.button("Close").clicked() {
                                close_menu = true;
                            }
                        });
                    });
            });

            if request_focus_main {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }

            if trigger_clean_now {
                self.mem_cleaner.start();
            }

            if let Some(target_x) = reset_to_x {
                next_x = target_x;
                save_needed = true;
                ctx.send_viewport_cmd_to(vp_id, egui::ViewportCommand::OuterPosition(Pos2::new(next_x, y_pos)));
            }

            if let Some(w) = set_width {
                self.config.taskbar_gauge_width = Some(w);
                ctx.send_viewport_cmd_to(vp_id, egui::ViewportCommand::InnerSize(Vec2::new(w, gauge_h)));
                self.config.save();
            }

            if toggle_hide {
                self.config.show_taskbar_gauges = false;
                self.config.save();
            }

            if close_menu {
                self.show_taskbar_context_menu = false;
            }
        }

        if save_needed {
            self.config.taskbar_gauge_x = Some(next_x);
            if self.last_position_save.elapsed() > Duration::from_millis(250) {
                self.config.save();
                self.last_position_save = Instant::now();
            }
        }
    }

    /// Compact animated purge badge overlaid on the taskbar RAM micro-gauge.
    fn draw_micro_ram_clean_badge(
        ui: &mut egui::Ui,
        rect: Rect,
        status: &memclean::CleanStatus,
        theme: &config::ThemePalette,
    ) {
        let a = |c: Color32, al: f32| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (al.clamp(0.0, 1.0) * 255.0) as u8);
        let p = ui.painter_at(rect);
        let (text, col, alpha) = match status.phase {
            memclean::CleanPhase::Running => {
                let t = status.started.elapsed().as_secs_f32();
                let dots = ".".repeat(((t * 4.0) as usize) % 4);
                (format!("PURGE{:<3}", dots), theme.warning, 0.6 + 0.4 * (t * 7.0).sin().abs())
            }
            memclean::CleanPhase::Done if Self::ram_feedback_visible(status) => {
                let t = status.finished.map(|f| f.elapsed().as_secs_f32()).unwrap_or(0.0);
                let alpha = if t > Self::RAM_FEEDBACK_SECS - 1.5 { (Self::RAM_FEEDBACK_SECS - t) / 1.5 } else { 1.0 };
                let mb = status.freed_bytes() as f64 / 1_048_576.0;
                let txt = if mb >= 1024.0 { format!("-{:.1}G", mb / 1024.0) } else { format!("-{:.0}M", mb) };
                (txt, theme.primary, alpha)
            }
            _ => return,
        };
        let bg = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 2.0, 13.0));
        p.rect_filled(bg, 3.0, a(theme.card_bg, 0.92 * alpha));
        p.rect_stroke(bg, 3.0, Stroke::new(1.0, a(col, alpha)), egui::StrokeKind::Inside);
        p.text(bg.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::monospace(9.0), a(col, alpha));
        ui.ctx().request_repaint();
    }

    fn draw_micro_gauge(
        ui: &mut egui::Ui,
        title: &str,
        percent: f32,
        value_display: &str,
        w: f32,
        h: f32,
        peak_percent: f32,
        theme: &config::ThemePalette,
        is_fuel: bool,
    ) -> Rect {
        let (rect, _resp) = ui.allocate_exact_size(Vec2::new(w, h), egui::Sense::hover());
        let p = ui.painter_at(rect);
        let pct = percent.clamp(0.0, 100.0);
        let now_t = ui.input(|i| i.time);
        let low_fuel_alarm = is_fuel && pct < 15.0;
        let is_flash = if is_fuel {
            low_fuel_alarm && ((now_t * 6.0) as i32 % 2 == 0)
        } else {
            pct > 90.0 && ((now_t * 10.0) as i32 % 2 == 0)
        };

        // Sculpted 240-degree circular sports-car instrument dial on the left, integrated digital readout on the right
        let rad = ((h * 0.45).min(w * 0.28)).clamp(11.0, 16.5);
        let dial_cx = rect.left() + rad + 3.0;
        let dial_cy = rect.center().y + 0.5;
        let center = pos2(dial_cx, dial_cy);

        // 240-degree radial sweep: 7:30 o'clock (210 deg) clockwise over 12:00 (90 deg) to 4:30 o'clock (-30 deg)
        let start_deg = 210.0f32.to_radians();
        let sweep_rad = 240.0f32.to_radians();
        let ang = |f: f32| start_deg - f.clamp(0.0, 1.0) * sweep_rad;
        let pt = |f: f32, rr: f32| {
            let a = ang(f);
            pos2(dial_cx + rr * a.cos(), dial_cy - rr * a.sin())
        };
        let arc_pts = |from: f32, to: f32, rr: f32| -> Vec<Pos2> {
            let steps = 22;
            (0..=steps).map(|k| pt(from + (to - from) * (k as f32 / steps as f32), rr)).collect()
        };

        // 1. Recessed circular instrument pod bezel (machined outer chrome + deep dial face + subtle inner rim)
        p.circle_filled(center, rad + 1.8, Color32::from_rgba_unmultiplied(0, 0, 0, 195));
        p.circle_filled(center, rad - 0.8, Color32::from_rgba_unmultiplied(theme.card_bg.r() / 2, theme.card_bg.g() / 2, theme.card_bg.b() / 2, 160));
        let bezel_col = if low_fuel_alarm && is_flash {
            theme.redline
        } else {
            Color32::from_rgba_unmultiplied(theme.primary.r(), theme.primary.g(), theme.primary.b(), 85)
        };
        p.circle_stroke(center, rad + 1.6, Stroke::new(1.1, bezel_col));
        p.circle_stroke(center, rad + 0.5, Stroke::new(0.6, Color32::from_rgba_unmultiplied(theme.text_bright.r(), theme.text_bright.g(), theme.text_bright.b(), 38)));

        // 2. Dial arc tracks (radius inset slightly inside bezel)
        let r_arc = rad - 1.5;
        if is_fuel {
            // Reserve zone on bottom-left (0..20% red), healthy scale 20..100%
            p.add(PathShape::line(arc_pts(0.0, 0.20, r_arc), Stroke::new(2.4, Color32::from_rgba_unmultiplied(theme.redline.r(), theme.redline.g(), theme.redline.b(), 140))));
            p.add(PathShape::line(arc_pts(0.20, 1.0, r_arc), Stroke::new(2.0, Color32::from_rgba_unmultiplied(theme.secondary.r(), theme.secondary.g(), theme.secondary.b(), 52))));
        } else {
            p.add(PathShape::line(arc_pts(0.0, 0.80, r_arc), Stroke::new(2.0, Color32::from_rgba_unmultiplied(theme.secondary.r(), theme.secondary.g(), theme.secondary.b(), 52))));
            p.add(PathShape::line(arc_pts(0.80, 1.0, r_arc), Stroke::new(2.4, Color32::from_rgba_unmultiplied(theme.redline.r(), theme.redline.g(), theme.redline.b(), 120))));
        }

        // 3. Illuminated active sweep arc with backlit phosphor bloom halo
        let active_col = if is_fuel {
            if pct < 15.0 {
                if is_flash { theme.warning } else { theme.redline }
            } else if pct < 35.0 {
                theme.warning
            } else {
                theme.primary
            }
        } else {
            if pct > 80.0 {
                if is_flash { Color32::WHITE } else { theme.redline }
            } else if pct > 55.0 {
                theme.warning
            } else {
                theme.primary
            }
        };
        if pct > 1.0 {
            let sweep_f = (pct / 100.0).clamp(0.0, 1.0);
            let pts = arc_pts(0.0, sweep_f, r_arc);
            // Soft outer phosphor glow pass
            p.add(PathShape::line(
                pts.clone(),
                Stroke::new(4.8, Color32::from_rgba_unmultiplied(active_col.r(), active_col.g(), active_col.b(), 42)),
            ));
            // Crisp bright core sweep pass
            p.add(PathShape::line(pts, Stroke::new(2.3, active_col)));
        }

        // 4. Fine radial tick marks around the 240-degree face
        for k in 0..=8 {
            let f = k as f32 / 8.0;
            let is_major = k % 4 == 0;
            let inner = if is_major { r_arc - 3.8 } else { r_arc - 2.1 };
            let tick_col = if is_fuel {
                if f <= 0.2 { theme.redline } else if is_major { theme.text_bright } else { theme.text_dim }
            } else {
                if f >= 0.8 { theme.redline } else if is_major { theme.text_bright } else { theme.text_dim }
            };
            p.line_segment(
                [pt(f, inner), pt(f, r_arc - 0.3)],
                Stroke::new(if is_major { 1.0 } else { 0.75 }, Color32::from_rgba_unmultiplied(tick_col.r(), tick_col.g(), tick_col.b(), if is_major { 215 } else { 155 })),
            );
        }

        // Tiny E / F indicators at bottom notch of the FUEL dial
        if is_fuel {
            p.text(
                pos2(dial_cx - rad * 0.42, dial_cy + rad * 0.62),
                egui::Align2::CENTER_CENTER,
                "E",
                egui::FontId::monospace(6.0),
                if low_fuel_alarm { theme.redline } else { theme.text_dim },
            );
            p.text(
                pos2(dial_cx + rad * 0.42, dial_cy + rad * 0.62),
                egui::Align2::CENTER_CENTER,
                "F",
                egui::FontId::monospace(6.0),
                theme.text_dim,
            );
        }

        // 5. Sweeping damped needle with glow halo + machined metallic center hub
        let needle_val = pct / 100.0;
        let needle_col = theme.tertiary;
        p.line_segment(
            [pt(needle_val, 1.5), pt(needle_val, r_arc - 1.2)],
            Stroke::new(3.4, Color32::from_rgba_unmultiplied(needle_col.r(), needle_col.g(), needle_col.b(), 50)),
        );
        p.line_segment([pt(needle_val, 1.5), pt(needle_val, r_arc - 1.5)], Stroke::new(1.6, needle_col));
        p.line_segment([pt(needle_val, r_arc - 4.0), pt(needle_val, r_arc - 1.0)], Stroke::new(1.6, Color32::WHITE));
        p.circle_filled(center, 2.7, needle_col);
        p.circle_filled(center, 1.3, Color32::from_rgb(18, 22, 20));
        p.circle_filled(pos2(dial_cx - 0.4, dial_cy - 0.4), 0.6, Color32::from_rgba_unmultiplied(255, 255, 255, 160));

        // Peak hold pip (load gauges only)
        if !is_fuel && peak_percent > 3.0 {
            let pk_f = (peak_percent / 100.0).clamp(0.0, 1.0);
            p.line_segment([pt(pk_f, r_arc - 3.5), pt(pk_f, r_arc + 1.0)], Stroke::new(1.2, Color32::WHITE));
        }

        // Low-fuel warning lamp inside dial ONLY when fuel < 15%
        if low_fuel_alarm {
            let lamp_pos = pos2(dial_cx, dial_cy + rad * 0.42);
            let lcol = if is_flash { theme.redline } else { theme.warning };
            p.circle_filled(lamp_pos, 3.4, Color32::from_rgba_unmultiplied(lcol.r(), lcol.g(), lcol.b(), 80));
            p.circle_filled(lamp_pos, 1.7, lcol);
        }

        // 6. Integrated floating digital readout & micro-status dot seamlessly beside the circular dial
        let readout_x = (dial_cx + rad + 5.5).min(rect.right() - 22.0);
        let status_dot_pos = pos2(readout_x + 2.0, dial_cy - 5.5);
        p.circle_filled(status_dot_pos, 1.5, Color32::from_rgba_unmultiplied(active_col.r(), active_col.g(), active_col.b(), 210));
        let title_col = if low_fuel_alarm { theme.redline } else { theme.text_dim };
        p.text(
            pos2(readout_x + 5.5, dial_cy - 1.5),
            egui::Align2::LEFT_BOTTOM,
            title,
            egui::FontId::monospace(7.5),
            title_col,
        );

        let num_font_size = if value_display.len() >= 4 {
            11.0
        } else if value_display.len() == 3 {
            12.0
        } else {
            13.5
        };
        let num_col = if low_fuel_alarm { theme.redline } else { theme.text_bright };
        // Subtle drop-shadow for crisp contrast
        p.text(
            pos2(readout_x + 0.8, dial_cy + 0.3),
            egui::Align2::LEFT_TOP,
            value_display,
            egui::FontId::monospace(num_font_size),
            Color32::from_rgba_unmultiplied(0, 0, 0, 170),
        );
        p.text(
            pos2(readout_x, dial_cy - 0.5),
            egui::Align2::LEFT_TOP,
            value_display,
            egui::FontId::monospace(num_font_size),
            num_col,
        );
        rect
    }
}

fn main() -> Result<(), eframe::Error> {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {}", info);
        logger::log_error("PANIC", &msg);
        let _ = std::fs::write("panic.txt", &msg);
    }));

    let mut cfg = WidgetConfig::load();
    let width = cfg.width.clamp(260.0, 1200.0);
    let height = if cfg.mini_mode {
        34.0
    } else {
        cfg.height.clamp(280.0, 1200.0)
    };

    // Determine initial position: saved coordinates or bottom-right of primary work area
    let (wa_l, wa_t, wa_r, wa_b) = window_fx::get_work_area();
    if cfg.x < 0.0 || cfg.y < 0.0 {
        cfg.x = (wa_r - width - 20.0).max(wa_l + 10.0);
        cfg.y = (wa_b - height - 20.0).max(wa_t + 10.0);
        cfg.save();
    } else {
        // Anti-clipping safety on launch: ensure saved position spawns completely inside current monitor work area
        let clamped_x = cfg.x.clamp(wa_l, (wa_r - width).max(wa_l));
        let clamped_y = cfg.y.clamp(wa_t, (wa_b - height).max(wa_t));
        if (clamped_x - cfg.x).abs() > 0.5 || (clamped_y - cfg.y).abs() > 0.5 {
            cfg.x = clamped_x;
            cfg.y = clamped_y;
            cfg.save();
        }
    }

    let initial_icon = std::sync::Arc::new(create_cyberpunk_icon(false, false));

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("NetMonitor")
        .with_position(Pos2::new(cfg.x, cfg.y))
        .with_inner_size([width, height])
        .with_min_inner_size([260.0, 34.0])
        .with_decorations(false)
        .with_resizable(true)
        .with_transparent(false)
        .with_icon(initial_icon);

    if cfg.topmost {
        viewport = viewport.with_always_on_top();
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    let res = eframe::run_native(
        "NetMonitor",
        options,
        Box::new(|cc| Ok(Box::new(NetMonitorApp::new(cc)))),
    );
    if let Err(ref e) = res {
        logger::log_error("APP", &format!("eframe::run_native exited with error: {:?}", e));
    }
    res
}
