use egui::{Color32, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::config::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_ORANGE, ACCENT_PURPLE, ACCENT_RED, ACCENT_YELLOW};
use crate::globe::ThreatGlobe;
use crate::traffic::{TrafficInspector, TrafficRecord};
use std::collections::BTreeSet;

pub struct InspectorUI {
    pub is_open: bool,
    pub filter_text: String,
    pub filter_device: String, // "All" or a specific client IP
    pub filter_proto: String,  // "All", "DNS", "HTTPS", "HTTP", "TCP", "UDP"
    pub filter_blocked_only: bool,
    pub paused: bool,
    pub paused_snapshot: Option<Vec<TrafficRecord>>,
    pub new_domain_input: String,
    pub selected_tab: usize,   // 0 = Live Traffic, 1 = 3D Threat Globe, 2 = Blocklist
    pub selected_packet: Option<TrafficRecord>,
    pub payload_view_mode: usize, // 0 = Hex Dump (Wireshark), 1 = Decoded Text, 2 = Protocol Layers
    pub last_export_msg: String,
    pub col_widths: [f32; 10], // Resizable column widths for packet log
    pub globe: ThreatGlobe,
    pub start_time: std::time::Instant,
}

impl InspectorUI {
    pub fn new() -> Self {
        Self {
            is_open: false,
            filter_text: String::new(),
            filter_device: "All".to_string(),
            filter_proto: "All".to_string(),
            filter_blocked_only: false,
            paused: false,
            paused_snapshot: None,
            new_domain_input: String::new(),
            selected_tab: 0,
            selected_packet: None,
            payload_view_mode: 0,
            last_export_msg: String::new(),
            col_widths: [
                62.0,  // 0: Time
                95.0,  // 1: Client IP
                145.0, // 2: Domain / SNI
                135.0, // 3: Remote Destination
                125.0, // 4: Location
                140.0, // 5: Company / Org
                55.0,  // 6: Proto
                48.0,  // 7: Length
                72.0,  // 8: Status
                150.0, // 9: Quick Actions
            ],
            globe: ThreatGlobe::new(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, inspector: &TrafficInspector) {
        if !self.is_open {
            return;
        }

        let viewport_id = egui::ViewportId::from_hash_of("hotspot_traffic_inspector");
        let builder = egui::ViewportBuilder::default()
            .with_title("Hotspot Traffic Inspector & Packet Analyzer")
            .with_inner_size([1060.0, 640.0])
            .with_min_inner_size([780.0, 460.0])
            .with_decorations(true)
            .with_resizable(true);

        let mut still_open = true;
        ctx.show_viewport_immediate(viewport_id, builder, |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                still_open = false;
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_content(ui, inspector);
            });
        });
        if !still_open {
            self.is_open = false;
        }
    }

    fn render_content(&mut self, ui: &mut Ui, inspector: &TrafficInspector) {
        // Title Bar & Global Blocker Toggle
        ui.horizontal(|ui| {
            ui.heading(RichText::new("TRAFFIC INSPECTOR & PACKET ANALYZER").color(ACCENT_CYAN).strong());

            let blocker_on = *inspector.blocker_enabled.lock().unwrap();
            let status_color = if blocker_on { ACCENT_GREEN } else { Color32::GRAY };
            let status_text = if blocker_on { "BLOCKER: ACTIVE" } else { "BLOCKER: OFF" };
            ui.colored_label(status_color, RichText::new(status_text).size(11.0).strong());

            if !self.last_export_msg.is_empty() {
                ui.colored_label(ACCENT_YELLOW, RichText::new(&self.last_export_msg).size(10.0));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut blocker_flag = blocker_on;
                if ui.checkbox(&mut blocker_flag, RichText::new("Blocker Enabled").color(ACCENT_RED)).changed() {
                    inspector.set_blocker_enabled(blocker_flag);
                }
            });
        });

        ui.add_space(4.0);

        // Tab selection & Action Controls
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.selected_tab, 0, "Live Packet Feed & Analyzer");
            ui.selectable_value(&mut self.selected_tab, 1, "🌐 3D Cyber Threat Globe (Kaspersky View)");
            ui.selectable_value(&mut self.selected_tab, 2, "Firewall Blocklist Rules");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.selected_tab == 0 {
                    if ui.button("Export CSV").clicked() {
                        if let Some(p) = inspector.export_csv() {
                            self.last_export_msg = format!("Saved: {}", p);
                        }
                    }
                    if ui.button("Clear Log").clicked() {
                        inspector.clear_records();
                        self.selected_packet = None;
                        self.last_export_msg.clear();
                        if self.paused {
                            self.paused_snapshot = Some(Vec::new());
                        }
                    }
                    let pause_btn_text = if self.paused { "▶ Resume Stream" } else { "⏸ Pause Stream" };
                    let pause_btn = if self.paused {
                        ui.button(RichText::new(pause_btn_text).color(ACCENT_YELLOW).strong())
                    } else {
                        ui.button(pause_btn_text)
                    };
                    if pause_btn.clicked() {
                        self.paused = !self.paused;
                        if self.paused {
                            self.paused_snapshot = Some(inspector.get_records());
                        } else {
                            self.paused_snapshot = None;
                        }
                    }
                }
            });
        });

        ui.separator();

        match self.selected_tab {
            0 => self.render_traffic_feed(ui, inspector),
            1 => self.render_globe_view(ui, inspector),
            _ => self.render_blocklist_manager(ui, inspector),
        }
    }

    fn render_traffic_feed(&mut self, ui: &mut Ui, inspector: &TrafficInspector) {
        let records = if self.paused {
            if self.paused_snapshot.is_none() {
                self.paused_snapshot = Some(inspector.get_records());
            }
            self.paused_snapshot.as_ref().unwrap().clone()
        } else {
            inspector.get_records()
        };

        // 1. Gather all unique client devices for Device Selector
        let mut devices = BTreeSet::new();
        devices.insert("All".to_string());
        for r in &records {
            if !r.client_ip.is_empty() && r.client_ip != "-" {
                devices.insert(r.client_ip.clone());
            }
        }

        // 2. Filter Bar
        ui.horizontal(|ui| {
            if self.paused {
                ui.colored_label(
                    ACCENT_YELLOW,
                    RichText::new("⏸ STREAM PAUSED (Table Frozen)").strong(),
                );
                ui.separator();
            }
            // Device Filter
            ui.label(RichText::new("Device:").strong().color(ACCENT_ORANGE));
            let device_label = if self.filter_device == "All" {
                "All Hotspot Devices".to_string()
            } else {
                format!("Device: {}", self.filter_device)
            };
            egui::ComboBox::from_id_salt("device_filter_combo")
                .selected_text(RichText::new(&device_label).color(ACCENT_CYAN))
                .show_ui(ui, |ui| {
                    for dev in &devices {
                        let text = if dev == "All" {
                            "All Devices (Entire Hotspot Network)".to_string()
                        } else {
                            format!("Single Device: {}", dev)
                        };
                        ui.selectable_value(&mut self.filter_device, dev.clone(), text);
                    }
                });

            ui.separator();

            // Protocol Filter
            ui.label(RichText::new("Proto:").strong());
            egui::ComboBox::from_id_salt("proto_filter_combo")
                .selected_text(&self.filter_proto)
                .width(75.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filter_proto, "All".to_string(), "All");
                    ui.selectable_value(&mut self.filter_proto, "DNS".to_string(), "DNS");
                    ui.selectable_value(&mut self.filter_proto, "HTTPS".to_string(), "HTTPS");
                    ui.selectable_value(&mut self.filter_proto, "HTTP".to_string(), "HTTP");
                    ui.selectable_value(&mut self.filter_proto, "TCP".to_string(), "TCP");
                    ui.selectable_value(&mut self.filter_proto, "UDP".to_string(), "UDP");
                });

            ui.separator();

            // Search Filter
            ui.label("Search:");
            ui.add(egui::TextEdit::singleline(&mut self.filter_text).hint_text("Domain, IP, Location, Org...").desired_width(170.0));

            ui.checkbox(&mut self.filter_blocked_only, "Blocked Only");
        });

        ui.separator();

        let filter = self.filter_text.to_lowercase();
        let blocked_only = self.filter_blocked_only;
        let selected_dev = self.filter_device.clone();
        let selected_proto = self.filter_proto.clone();

        // 3. Apply Filters
        let filtered: Vec<TrafficRecord> = records
            .into_iter()
            .rev()
            .filter(|r| {
                if blocked_only && !r.blocked {
                    return false;
                }
                if selected_dev != "All" && r.client_ip != selected_dev {
                    return false;
                }
                if selected_proto != "All" && !r.proto.starts_with(&selected_proto) {
                    return false;
                }
                if !filter.is_empty() {
                    if !r.domain.to_lowercase().contains(&filter)
                        && !r.client_ip.contains(&filter)
                        && !r.remote_ip.contains(&filter)
                        && !r.location.to_lowercase().contains(&filter)
                        && !r.company.to_lowercase().contains(&filter)
                        && !r.info.to_lowercase().contains(&filter)
                    {
                        return false;
                    }
                }
                true
            })
            .take(500)
            .collect();

        // Calculate heights: table area vs payload inspection drawer
        let has_selection = self.selected_packet.is_some();
        let table_height = if has_selection { 220.0 } else { ui.available_height() - 10.0 };

        // 4. Packet Stream Table Header (with Resizable Draggable Columns!)
        ui.horizontal(|ui| {
            let col_labels = [
                "Time",
                "Client",
                "Domain / SNI",
                "Remote Destination",
                "Location",
                "Company / Org",
                "Proto",
                "Length",
                "Status",
            ];

            for (idx, &label) in col_labels.iter().enumerate() {
                let w = self.col_widths[idx];
                ui.add_sized([w, 20.0], egui::Label::new(RichText::new(label).strong().color(ACCENT_CYAN)).truncate());

                // Interactive resize grip divider
                let (grip_rect, grip_resp) = ui.allocate_exact_size(Vec2::new(5.0, 18.0), egui::Sense::click_and_drag());
                let is_hovered = grip_resp.hovered() || grip_resp.dragged();
                let grip_col = if is_hovered {
                    Color32::from_rgb(0, 240, 255)
                } else {
                    Color32::from_rgba_unmultiplied(80, 160, 200, 60)
                };
                ui.painter().vline(grip_rect.center().x, grip_rect.y_range(), Stroke::new(if is_hovered { 1.5 } else { 1.0 }, grip_col));

                if grip_resp.dragged() {
                    let delta = grip_resp.drag_delta().x;
                    self.col_widths[idx] = (self.col_widths[idx] + delta).clamp(30.0, 400.0);
                }
                if grip_resp.double_clicked() {
                    // Double click resets to standard default width
                    let defaults = [62.0, 95.0, 145.0, 135.0, 125.0, 140.0, 55.0, 48.0, 72.0, 150.0];
                    self.col_widths[idx] = defaults[idx];
                }
            }

            ui.add_sized([self.col_widths[9], 20.0], egui::Label::new(RichText::new("Quick Actions").strong().color(ACCENT_CYAN)).truncate());
        });

        ui.separator();

        // 5. Scrollable Packet Table
        ScrollArea::vertical()
            .max_height(table_height)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No packet traffic matching current filters.").color(Color32::GRAY));
                    });
                    return;
                }

                let cw = self.col_widths;

                for r in filtered {
                    let is_selected = self.selected_packet.as_ref().map_or(false, |sp| {
                        sp.timestamp == r.timestamp && sp.client_ip == r.client_ip && sp.remote_port == r.remote_port && sp.size == r.size
                    });

                    let row_bg = if is_selected {
                        Color32::from_rgb(20, 48, 62)
                    } else if r.blocked {
                        Color32::from_rgb(45, 15, 15)
                    } else {
                        Color32::TRANSPARENT
                    };

                    egui::Frame::new().fill(row_bg).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Column 0: Time
                            let time_btn = ui.add_sized(
                                [cw[0], 18.0],
                                egui::Button::new(RichText::new(&r.timestamp).monospace().size(11.0))
                                    .selected(is_selected),
                            );
                            if time_btn.clicked() {
                                self.selected_packet = Some(r.clone());
                            }
                            ui.add_space(5.0);

                            // Column 1: Client IP
                            ui.add_sized([cw[1], 18.0], egui::Label::new(RichText::new(&r.client_ip).monospace().size(11.0)).truncate());
                            ui.add_space(5.0);

                            // Column 2: Domain / SNI
                            let dom_color = if r.domain != "-" { Color32::WHITE } else { Color32::GRAY };
                            ui.add_sized([cw[2], 18.0], egui::Label::new(RichText::new(&r.domain).monospace().size(11.0).color(dom_color)).truncate());
                            ui.add_space(5.0);

                            // Column 3: Remote Destination
                            let target = format!("{}:{}", r.remote_ip, r.remote_port);
                            ui.add_sized([cw[3], 18.0], egui::Label::new(RichText::new(target).monospace().size(11.0)).truncate());
                            ui.add_space(5.0);

                            // Column 4: Location
                            let loc_color = if r.location != "-" { Color32::from_rgb(255, 205, 100) } else { Color32::GRAY };
                            ui.add_sized([cw[4], 18.0], egui::Label::new(RichText::new(&r.location).monospace().size(11.0).color(loc_color)).truncate());
                            ui.add_space(5.0);

                            // Column 5: Company / Org
                            let comp_color = if r.company != "-" { Color32::from_rgb(180, 220, 255) } else { Color32::GRAY };
                            ui.add_sized([cw[5], 18.0], egui::Label::new(RichText::new(&r.company).monospace().size(11.0).color(comp_color)).truncate());
                            ui.add_space(5.0);

                            // Column 6: Proto
                            let proto_color = if r.proto == "DNS" {
                                ACCENT_GREEN
                            } else if r.proto == "HTTPS" {
                                ACCENT_CYAN
                            } else if r.proto == "HTTP" {
                                ACCENT_YELLOW
                            } else {
                                Color32::LIGHT_GRAY
                            };
                            ui.add_sized([cw[6], 18.0], egui::Label::new(RichText::new(&r.proto).monospace().size(11.0).color(proto_color)).truncate());
                            ui.add_space(5.0);

                            // Column 7: Length
                            ui.add_sized([cw[7], 18.0], egui::Label::new(RichText::new(format!("{}B", r.size)).monospace().size(11.0).color(Color32::GRAY)).truncate());
                            ui.add_space(5.0);

                            // Column 8: Status
                            let status_text = if r.blocked { "[BLOCKED]" } else { "ALLOWED" };
                            let status_col = if r.blocked { ACCENT_RED } else { Color32::from_rgb(160, 160, 160) };
                            ui.add_sized([cw[8], 18.0], egui::Label::new(RichText::new(status_text).monospace().size(11.0).color(status_col)).truncate());
                            ui.add_space(5.0);

                            // Column 9: Quick Actions
                            ui.allocate_ui_with_layout(Vec2::new(cw[9], 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                if ui.small_button(RichText::new("Inspect").color(ACCENT_CYAN)).clicked() {
                                    self.selected_packet = Some(r.clone());
                                }
                                if r.domain != "-" {
                                    if ui.small_button("Block").clicked() {
                                        inspector.add_blocked_domain(r.domain.clone());
                                    }
                                }
                                if ui.small_button("IP").clicked() {
                                    inspector.add_blocked_ip(r.remote_ip.clone());
                                }
                            });
                        });
                    });
                }
            });

        // 6. Wireshark-Style Packet Inspection & Payload Viewer Pane
        if let Some(ref pkt) = self.selected_packet.clone() {
            ui.add_space(4.0);
            ui.separator();

            egui::Frame::new()
                .fill(Color32::from_rgb(12, 18, 24))
                .stroke(Stroke::new(1.0, ACCENT_CYAN))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    // Drawer Header
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("PACKET DETAILS & PAYLOAD (Wireshark View)").color(ACCENT_CYAN).strong());

                        ui.separator();

                        ui.selectable_value(&mut self.payload_view_mode, 0, "Hex Dump + ASCII");
                        ui.selectable_value(&mut self.payload_view_mode, 1, "Decoded Text");
                        ui.selectable_value(&mut self.payload_view_mode, 2, "Protocol Layers");

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("Close [X]").color(Color32::GRAY)).clicked() {
                                self.selected_packet = None;
                            }
                            if ui.button("Copy Payload").clicked() {
                                ui.ctx().copy_text(pkt.format_hex_dump());
                            }
                        });
                    });

                    // Summary Banner with Location and Company
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Time: {}", pkt.timestamp)).monospace().size(11.0).color(Color32::GRAY));
                        ui.label(RichText::new(format!("Length: {}B (cap: {}B)", pkt.size, pkt.payload.len())).monospace().size(11.0).color(ACCENT_YELLOW));
                        ui.label(RichText::new(format!("Client: {} -> Target: {}:{}", pkt.client_ip, pkt.remote_ip, pkt.remote_port)).monospace().size(11.0).color(Color32::WHITE));
                        ui.label(RichText::new(format!("Location: {} | Org: {}", pkt.location, pkt.company)).monospace().size(11.0).color(ACCENT_ORANGE));
                        ui.label(RichText::new(&pkt.info).monospace().size(11.0).color(ACCENT_GREEN));
                    });

                    ui.separator();

                    // Payload Content View
                    ScrollArea::vertical()
                        .max_height(160.0)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            match self.payload_view_mode {
                                0 => {
                                    // Classic Wireshark Hex Dump: Offset | Hex Bytes | ASCII
                                    let hex = pkt.format_hex_dump();
                                    ui.add(
                                        egui::TextEdit::multiline(&mut hex.as_str())
                                            .font(egui::TextStyle::Monospace)
                                            .text_color(Color32::from_rgb(0, 255, 200))
                                            .desired_width(f32::INFINITY)
                                    );
                                }
                                1 => {
                                    // Printable text / ASCII representation
                                    let ascii = pkt.format_ascii_text();
                                    ui.add(
                                        egui::TextEdit::multiline(&mut ascii.as_str())
                                            .font(egui::TextStyle::Monospace)
                                            .text_color(Color32::LIGHT_GREEN)
                                            .desired_width(f32::INFINITY)
                                    );
                                }
                                _ => {
                                    // Protocol Layers Breakdown
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new("Layer 2: Ethernet Frame").color(ACCENT_PURPLE).strong());
                                        ui.label(format!("  Length on wire: {} bytes | Capture buffer: {} bytes", pkt.size, pkt.payload.len()));

                                        ui.label(RichText::new("Layer 3: Internet Protocol Version 4 (IPv4)").color(ACCENT_CYAN).strong());
                                        ui.label(format!("  Hotspot Client: {} | Remote IP: {}", pkt.client_ip, pkt.remote_ip));
                                        ui.label(format!("  Location: {} (Coordinates: {:.2}, {:.2})", pkt.location, pkt.lat, pkt.lon));
                                        ui.label(format!("  Autonomous System / Organization: {}", pkt.company));

                                        ui.label(RichText::new(format!("Layer 4: Transport Layer ({})", pkt.proto)).color(ACCENT_ORANGE).strong());
                                        ui.label(format!("  Destination Port: {} | Protocol: {}", pkt.remote_port, pkt.proto));

                                        if pkt.domain != "-" {
                                            ui.label(RichText::new("Layer 7: Application Layer").color(ACCENT_GREEN).strong());
                                            ui.label(format!("  Extracted Hostname / Domain / SNI: {}", pkt.domain));
                                        }
                                        ui.label(format!("  Frame Summary: {}", pkt.info));
                                    });
                                }
                            }
                        });
                });
        }
    }

    fn render_globe_view(&mut self, ui: &mut Ui, inspector: &TrafficInspector) {
        let (origin_lat, origin_lon, origin_label) = {
            let lock = inspector.geo_resolver.local_origin.lock().unwrap();
            lock.clone()
        };
        let connections = inspector.get_globe_connections();
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Top Control Bar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.globe.auto_rotate, RichText::new("Auto-Spin").color(ACCENT_CYAN));
            ui.add_space(5.0);
            ui.checkbox(&mut self.globe.high_detail_mode, RichText::new("HD Vector").color(ACCENT_GREEN));
            ui.add_space(5.0);
            ui.checkbox(&mut self.globe.show_matrix_dots, "Matrix Dots");
            ui.add_space(5.0);
            ui.checkbox(&mut self.globe.show_borders, "Borders");
            ui.add_space(5.0);
            ui.checkbox(&mut self.globe.show_labels, "3D Labels");
            ui.add_space(5.0);
            ui.checkbox(&mut self.globe.show_grid, "Grid");
            ui.add_space(6.0);
            ui.label("Speed:");
            ui.add(egui::Slider::new(&mut self.globe.rotate_speed, 0.05..=1.5).text("rad/s"));
            ui.add_space(6.0);
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.globe.zoom, 0.6..=2.5).text("x"));
            ui.add_space(6.0);
            if ui.button("Reset Camera").clicked() {
                self.globe.yaw = 0.8;
                self.globe.pitch = 0.25;
                self.globe.zoom = 1.0;
                self.globe.target_yaw = None;
                self.globe.target_pitch = None;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("Drag left-click to rotate • Mouse wheel to zoom").size(10.5).color(Color32::GRAY));
            });
        });

        ui.separator();

        // Split view: 3D Threat Globe on Left/Center, Active Threat Streams on Right
        let avail_w = ui.available_width();
        let list_w = 340.0_f32.min(avail_w * 0.36);
        let globe_w = (avail_w - list_w - 14.0).max(300.0);
        let content_h = (ui.available_height() - 6.0).max(360.0);

        ui.horizontal(|ui| {
            // Left: 3D Globe Canvas Area
            ui.allocate_ui_with_layout(
                Vec2::new(globe_w, content_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    self.globe.render(
                        ui,
                        origin_lat,
                        origin_lon,
                        &origin_label,
                        &connections,
                        elapsed,
                    );
                },
            );

            ui.separator();

            // Right: Active Global Streams side drawer
            ui.vertical(|ui| {
                ui.heading(RichText::new("ACTIVE GLOBAL ENDPOINTS").size(12.0).color(ACCENT_CYAN).strong());

                // Endpoint search/filter box
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Search:").size(11.0));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.globe.endpoint_filter)
                            .hint_text("Filter IP, Domain, City...")
                            .desired_width(170.0),
                    );
                    if !self.globe.endpoint_filter.is_empty() && ui.small_button("✕").clicked() {
                        self.globe.endpoint_filter.clear();
                    }
                });

                let filter_query = self.globe.endpoint_filter.trim().to_lowercase();
                let filtered_conns: Vec<&crate::globe::GlobeConnection> = connections
                    .iter()
                    .filter(|c| {
                        if filter_query.is_empty() {
                            true
                        } else {
                            c.remote_ip.to_lowercase().contains(&filter_query)
                                || c.domain.to_lowercase().contains(&filter_query)
                                || c.location.to_lowercase().contains(&filter_query)
                                || c.company.to_lowercase().contains(&filter_query)
                                || c.proto.to_lowercase().contains(&filter_query)
                        }
                    })
                    .collect();

                ui.label(
                    RichText::new(format!(
                        "Showing {} of {} destination clusters",
                        filtered_conns.len(),
                        connections.len()
                    ))
                    .size(10.0)
                    .color(Color32::GRAY),
                );
                ui.separator();

                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if filtered_conns.is_empty() {
                            ui.add_space(20.0);
                            let empty_msg = if connections.is_empty() {
                                "No external endpoints plotted yet.\nOutbound & inbound hotspot flows will plot here in real-time."
                            } else {
                                "No endpoints matching filter criteria."
                            };
                            ui.label(RichText::new(empty_msg).color(Color32::GRAY));
                            return;
                        }

                        for conn in filtered_conns {
                            let is_hovered = self
                                .globe
                                .hovered_ip
                                .as_deref()
                                .map_or(false, |ip| ip == conn.remote_ip);

                            let border_color = if is_hovered {
                                Color32::from_rgb(0, 240, 255)
                            } else {
                                Color32::from_rgba_unmultiplied(0, 180, 220, 80)
                            };

                            let bg_color = if is_hovered {
                                Color32::from_rgb(18, 30, 42)
                            } else {
                                Color32::from_rgb(12, 18, 24)
                            };

                            egui::Frame::new()
                                .fill(bg_color)
                                .stroke(Stroke::new(if is_hovered { 1.5 } else { 1.0 }, border_color))
                                .inner_margin(6.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let proto_col = if conn.is_blocked {
                                            ACCENT_RED
                                        } else if conn.proto == "HTTPS" {
                                            ACCENT_CYAN
                                        } else if conn.proto == "DNS" {
                                            ACCENT_GREEN
                                        } else {
                                            ACCENT_ORANGE
                                        };
                                        ui.colored_label(proto_col, RichText::new(&conn.proto).monospace().size(10.0).strong());
                                        ui.label(RichText::new(&conn.remote_ip).monospace().size(11.0).strong());
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("📍").size(10.0));
                                        ui.label(RichText::new(&conn.location).size(10.5).color(ACCENT_YELLOW));
                                    });

                                    if conn.company != "-" && !conn.company.is_empty() {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("🏢").size(10.0));
                                            ui.label(RichText::new(&conn.company).size(10.5).color(Color32::WHITE));
                                        });
                                    }

                                    if conn.domain != "-" && !conn.domain.is_empty() {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("🌐").size(10.0));
                                            ui.label(RichText::new(&conn.domain).size(10.5).color(Color32::LIGHT_GRAY));
                                        });
                                    }

                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("{} pkts ({}B)", conn.packet_count, conn.bytes)).size(10.0).color(Color32::GRAY));

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button(RichText::new("Focus 🎯").color(ACCENT_CYAN)).clicked() {
                                                self.globe.focus_on(conn.remote_lat, conn.remote_lon);
                                            }
                                        });
                                    });
                                });
                            ui.add_space(3.0);
                        }
                    });
            });
        });
    }

    fn render_blocklist_manager(&mut self, ui: &mut Ui, inspector: &TrafficInspector) {
        ui.horizontal(|ui| {
            ui.label("Add Domain/IP to Blocklist:");
            ui.add(egui::TextEdit::singleline(&mut self.new_domain_input).hint_text("e.g. ads.google.com or 1.2.3.4").desired_width(220.0));
            if ui.button(RichText::new("+ Add Rule").color(ACCENT_RED).strong()).clicked() {
                let input = self.new_domain_input.trim().to_string();
                if !input.is_empty() {
                    if input.contains('.') && input.chars().all(|c| c.is_ascii_digit() || c == '.') && input.split('.').count() == 4 {
                        inspector.add_blocked_ip(input);
                    } else {
                        inspector.add_blocked_domain(input);
                    }
                    self.new_domain_input.clear();
                }
            }
        });

        ui.add_space(8.0);
        ui.separator();

        ui.columns(2, |columns| {
            // Blocked Domains Column
            columns[0].vertical(|ui| {
                ui.heading(RichText::new("Blocked Domains").color(ACCENT_YELLOW).size(13.0));
                let domains: Vec<String> = inspector.blocked_domains.lock().unwrap().iter().cloned().collect();
                if domains.is_empty() {
                    ui.label(RichText::new("No blocked domains configured.").color(Color32::GRAY));
                }
                ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                    for dom in domains {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&dom).monospace());
                            if ui.small_button(RichText::new("Remove").color(ACCENT_RED)).clicked() {
                                inspector.remove_blocked_domain(&dom);
                            }
                        });
                    }
                });
            });

            // Blocked IPs Column
            columns[1].vertical(|ui| {
                ui.heading(RichText::new("Blocked IP Addresses").color(ACCENT_YELLOW).size(13.0));
                let ips: Vec<String> = inspector.blocked_ips.lock().unwrap().iter().cloned().collect();
                if ips.is_empty() {
                    ui.label(RichText::new("No blocked IPs configured.").color(Color32::GRAY));
                }
                ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                    for ip in ips {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&ip).monospace());
                            if ui.small_button(RichText::new("Remove").color(ACCENT_RED)).clicked() {
                                inspector.remove_blocked_ip(&ip);
                            }
                        });
                    }
                });
            });
        });
    }
}
