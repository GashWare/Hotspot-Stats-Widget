use egui::{epaint::PathShape, pos2, Color32, Pos2, Rect, Stroke, Ui, Vec2};
use std::f32::consts::{FRAC_PI_2, PI};
use crate::globe_data::{hd_landmass_matrix_dots, hd_world_borders, hd_world_coastlines};

#[derive(Clone, Debug)]
pub struct GlobeConnection {
    pub client_ip: String,
    pub remote_ip: String,
    pub domain: String,
    pub company: String,
    pub location: String,
    pub proto: String,
    pub remote_lat: f32,
    pub remote_lon: f32,
    pub packet_count: usize,
    pub bytes: usize,
    pub is_blocked: bool,
    pub last_seen_secs_ago: f32,
}

pub struct ThreatGlobe {
    pub yaw: f32,
    pub pitch: f32,
    pub target_yaw: Option<f32>,
    pub target_pitch: Option<f32>,
    pub auto_rotate: bool,
    pub rotate_speed: f32,
    pub zoom: f32,
    pub show_matrix_dots: bool,
    pub show_labels: bool,
    pub show_grid: bool,
    pub show_borders: bool,
    pub high_detail_mode: bool,
    pub endpoint_filter: String,
    pub hovered_ip: Option<String>,
}

impl ThreatGlobe {
    pub fn new() -> Self {
        Self {
            yaw: 0.8,
            pitch: 0.25,
            target_yaw: None,
            target_pitch: None,
            auto_rotate: true,
            rotate_speed: 0.35,
            zoom: 1.0,
            show_matrix_dots: true,
            show_labels: true,
            show_grid: true,
            show_borders: true,
            high_detail_mode: true,
            endpoint_filter: String::new(),
            hovered_ip: None,
        }
    }

    /// Focus camera smoothly on a specific coordinate
    pub fn focus_on(&mut self, lat: f32, lon: f32) {
        let ty = -lon.to_radians() - FRAC_PI_2;
        let tp = (lat.to_radians() * 0.4).clamp(-0.8, 0.8);
        self.target_yaw = Some(ty);
        self.target_pitch = Some(tp);
        self.auto_rotate = false;
    }

    pub fn render(
        &mut self,
        ui: &mut Ui,
        origin_lat: f32,
        origin_lon: f32,
        origin_label: &str,
        connections: &[GlobeConnection],
        time_sec: f32,
    ) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::new(320.0, 320.0)),
            egui::Sense::click_and_drag(),
        );

        // Smooth camera glide towards target
        if let (Some(ty), Some(tp)) = (self.target_yaw, self.target_pitch) {
            let dt = ui.input(|i| i.stable_dt).min(0.1);
            let factor = (dt * 6.0).min(1.0);
            self.yaw += (ty - self.yaw) * factor;
            self.pitch += (tp - self.pitch) * factor;
            if (ty - self.yaw).abs() < 0.005 && (tp - self.pitch).abs() < 0.005 {
                self.yaw = ty;
                self.pitch = tp;
                self.target_yaw = None;
                self.target_pitch = None;
            }
        }

        // Handle user interaction: Drag to rotate
        if response.dragged() {
            self.target_yaw = None;
            self.target_pitch = None;
            let delta = response.drag_delta();
            self.yaw += delta.x * 0.008;
            self.pitch = (self.pitch - delta.y * 0.008).clamp(-PI * 0.42, PI * 0.42);
        }

        // Scroll to zoom
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                self.zoom = (self.zoom + scroll * 0.0015).clamp(0.65, 2.5);
            }
        }

        // Auto spin when enabled
        if self.auto_rotate && !response.dragged() && self.target_yaw.is_none() {
            let dt = ui.input(|i| i.stable_dt).min(0.1);
            self.yaw += dt * self.rotate_speed;
        }

        // Request continuous repaint for smooth 60fps threat animation
        ui.ctx().request_repaint();

        let painter = ui.painter_at(rect);
        let center = rect.center();
        let base_radius = (rect.width().min(rect.height()) * 0.42 * self.zoom).clamp(70.0, 420.0);

        // 1. Cyberpunk Dark Space Background with subtle starfield
        painter.rect_filled(rect, 4.0, Color32::from_rgb(6, 10, 16));
        Self::draw_starfield(&painter, rect);

        // 2. Outer Atmospheric Glow Halo (Layered Concentric Translucent Spheres)
        let glow_layers = [
            (base_radius * 1.22, Color32::from_rgba_unmultiplied(0, 180, 255, 5)),
            (base_radius * 1.14, Color32::from_rgba_unmultiplied(0, 210, 255, 12)),
            (base_radius * 1.07, Color32::from_rgba_unmultiplied(0, 230, 255, 24)),
            (base_radius * 1.02, Color32::from_rgba_unmultiplied(0, 240, 255, 45)),
            (base_radius * 1.00, Color32::from_rgba_unmultiplied(8, 26, 44, 220)),
        ];
        for (rad, col) in glow_layers {
            painter.circle_filled(center, rad, col);
        }
        painter.circle_stroke(
            center,
            base_radius,
            Stroke::new(1.4, Color32::from_rgba_unmultiplied(0, 220, 255, 170)),
        );

        // 3D Projection Engine
        let yaw = self.yaw;
        let pitch = self.pitch;

        let project_vector = |v: [f32; 3], alt_factor: f32| -> (Pos2, f32) {
            let r = base_radius * alt_factor;
            let x0 = v[0] * r;
            let y0 = v[1] * r;
            let z0 = v[2] * r;

            // Apply yaw around Y axis
            let x1 = x0 * yaw.cos() + z0 * yaw.sin();
            let y1 = y0;
            let z1 = -x0 * yaw.sin() + z0 * yaw.cos();

            // Apply pitch around X axis
            let x = x1;
            let y = y1 * pitch.cos() - z1 * pitch.sin();
            let z = y1 * pitch.sin() + z1 * pitch.cos();

            // Perspective projection
            let dist = base_radius * 3.8;
            let p_factor = dist / (dist - z);

            let screen_pos = pos2(center.x + x * p_factor, center.y + y * p_factor);
            (screen_pos, z / base_radius)
        };

        let project_lat_lon = |lat_deg: f32, lon_deg: f32, alt_factor: f32| -> (Pos2, f32) {
            let phi = lat_deg.to_radians();
            let lambda = lon_deg.to_radians();
            let v = [
                phi.cos() * lambda.sin(),
                -phi.sin(),
                phi.cos() * lambda.cos(),
            ];
            project_vector(v, alt_factor)
        };

        // 3. Latitude & Longitude Grid Rings
        if self.show_grid {
            // Parallels (Latitude)
            for lat in &[-60.0f32, -30.0f32, 0.0f32, 30.0f32, 60.0f32] {
                let is_equator = lat.abs() < 1.0;
                let stroke_alpha = if is_equator { 75 } else { 32 };
                let stroke_col = Color32::from_rgba_unmultiplied(0, 200, 255, stroke_alpha);
                let stroke_w = if is_equator { 1.3 } else { 0.7 };

                let steps = 48;
                let mut pts_front = Vec::new();
                for i in 0..=steps {
                    let lon = -180.0 + (360.0 * i as f32 / steps as f32);
                    let (pt, depth) = project_lat_lon(*lat, lon, 1.0);
                    if depth > -0.05 {
                        pts_front.push(pt);
                    } else if pts_front.len() > 1 {
                        painter.add(PathShape::line(pts_front.clone(), Stroke::new(stroke_w, stroke_col)));
                        pts_front.clear();
                    } else {
                        pts_front.clear();
                    }
                }
                if pts_front.len() > 1 {
                    painter.add(PathShape::line(pts_front, Stroke::new(stroke_w, stroke_col)));
                }
            }

            // Meridians (Longitude)
            for lon_idx in 0..12 {
                let lon = -180.0 + lon_idx as f32 * 30.0;
                let stroke_col = Color32::from_rgba_unmultiplied(0, 180, 255, 28);
                let steps = 36;
                let mut pts_front = Vec::new();
                for i in 0..=steps {
                    let lat = -90.0 + (180.0 * i as f32 / steps as f32);
                    let (pt, depth) = project_lat_lon(lat, lon, 1.0);
                    if depth > -0.05 {
                        pts_front.push(pt);
                    } else if pts_front.len() > 1 {
                        painter.add(PathShape::line(pts_front.clone(), Stroke::new(0.6, stroke_col)));
                        pts_front.clear();
                    } else {
                        pts_front.clear();
                    }
                }
                if pts_front.len() > 1 {
                    painter.add(PathShape::line(pts_front, Stroke::new(0.6, stroke_col)));
                }
            }
        }

        // 4. Kaspersky-Style Cyber Matrix Landmass Dots (High Definition)
        if self.show_matrix_dots {
            let dots = if self.high_detail_mode {
                hd_landmass_matrix_dots()
            } else {
                Self::landmass_matrix_dots()
            };
            let dot_radius = if self.high_detail_mode { 1.05 } else { 1.25 };

            for &(lat, lon) in dots {
                let (pt, depth) = project_lat_lon(lat, lon, 1.0);
                if depth > -0.02 {
                    let alpha = ((depth + 0.05) * 230.0).clamp(25.0, 205.0) as u8;
                    let dot_col = Color32::from_rgba_unmultiplied(0, 240, 200, alpha);
                    painter.circle_filled(pt, dot_radius, dot_col);
                }
            }
        }

        // 5. Detailed Continents Vector Coastline Polylines
        let continent_color = Color32::from_rgba_unmultiplied(0, 255, 210, 175);
        let continent_stroke = Stroke::new(1.2, continent_color);

        let coastlines = if self.high_detail_mode {
            hd_world_coastlines()
        } else {
            Self::world_coastlines()
        };

        for line in coastlines {
            let mut current_segment = Vec::new();
            for &(lat, lon) in *line {
                let (pt, depth) = project_lat_lon(lat, lon, 1.0);
                if depth > -0.08 {
                    current_segment.push(pt);
                } else {
                    if current_segment.len() > 1 {
                        painter.add(PathShape::line(current_segment.clone(), continent_stroke));
                    }
                    current_segment.clear();
                }
            }
            if current_segment.len() > 1 {
                painter.add(PathShape::line(current_segment, continent_stroke));
            }
        }

        // 5b. Political Borders & Division Lines (Optional overlay)
        if self.show_borders {
            let border_color = Color32::from_rgba_unmultiplied(0, 180, 255, 95);
            let border_stroke = Stroke::new(0.85, border_color);

            for line in hd_world_borders() {
                let mut current_segment = Vec::new();
                for &(lat, lon) in *line {
                    let (pt, depth) = project_lat_lon(lat, lon, 1.0);
                    if depth > -0.06 {
                        current_segment.push(pt);
                    } else {
                        if current_segment.len() > 1 {
                            painter.add(PathShape::line(current_segment.clone(), border_stroke));
                        }
                        current_segment.clear();
                    }
                }
                if current_segment.len() > 1 {
                    painter.add(PathShape::line(current_segment, border_stroke));
                }
            }
        }

        // 6. Local Origin Node (Host Hotspot Gateway)
        let (origin_pt, origin_depth) = project_lat_lon(origin_lat, origin_lon, 1.0);
        let origin_visible = origin_depth > -0.15;
        if origin_visible {
            let pulse_r = 5.0 + ((time_sec * 2.8).sin().abs() * 7.0);
            painter.circle_stroke(
                origin_pt,
                pulse_r,
                Stroke::new(1.2, Color32::from_rgba_unmultiplied(0, 255, 120, 170)),
            );
            painter.circle_filled(origin_pt, 4.0, Color32::from_rgb(0, 255, 120));
            painter.circle_filled(origin_pt, 2.0, Color32::WHITE);

            if self.show_labels {
                let label_pos = origin_pt + Vec2::new(8.0, -12.0);
                painter.line_segment([origin_pt, label_pos], Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 255, 120, 140)));
                painter.text(
                    label_pos + Vec2::new(4.0, -4.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("HOST: {}", origin_label),
                    egui::FontId::monospace(9.5),
                    Color32::from_rgb(0, 255, 160),
                );
            }
        }

        // 7. Active Ballistic 3D Arcs (SLERP) & Multi-Bead Packet Animation
        let mouse_pos = ui.input(|i| i.pointer.hover_pos());
        let mut new_hovered = None;

        let v_origin = Self::to_unit_vector(origin_lat, origin_lon);

        for (conn_idx, conn) in connections.iter().enumerate() {
            if conn.remote_lat.abs() < 0.1 && conn.remote_lon.abs() < 0.1 {
                continue;
            }

            let (target_pt, target_depth) = project_lat_lon(conn.remote_lat, conn.remote_lon, 1.0);
            let target_visible = target_depth > -0.18;

            let arc_color = if conn.is_blocked {
                Color32::from_rgba_unmultiplied(255, 45, 45, 220)
            } else if conn.proto == "DNS" {
                Color32::from_rgba_unmultiplied(50, 255, 130, 200)
            } else if conn.proto == "HTTPS" {
                Color32::from_rgba_unmultiplied(0, 235, 255, 215)
            } else if conn.proto.starts_with("UDP") {
                Color32::from_rgba_unmultiplied(255, 165, 45, 205)
            } else {
                Color32::from_rgba_unmultiplied(190, 145, 255, 195)
            };

            let v_target = Self::to_unit_vector(conn.remote_lat, conn.remote_lon);
            let dot = (v_origin[0]*v_target[0] + v_origin[1]*v_target[1] + v_origin[2]*v_target[2]).clamp(-1.0, 1.0);
            let omega = dot.acos();

            // Dynamic peak altitude based on spherical angular distance (smooth ballistic trajectory)
            let peak_alt = ((omega / PI) * 0.22 + 0.04).clamp(0.04, 0.24);

            let steps = 28;
            let mut arc_pts = Vec::new();
            let mut avg_depth = 0.0;

            for step in 0..=steps {
                let s = step as f32 / steps as f32;
                let v_interp = Self::slerp(v_origin, v_target, s, omega);
                let alt = 1.0 + (s * PI).sin() * peak_alt;
                let (pt, depth) = project_vector(v_interp, alt);
                avg_depth += depth;
                arc_pts.push((pt, depth));
            }
            avg_depth /= (steps + 1) as f32;

            if avg_depth > -0.16 {
                // Draw 3D curved trajectory line
                let poly: Vec<Pos2> = arc_pts.iter().map(|(p, _)| *p).collect();
                painter.add(PathShape::line(poly.clone(), Stroke::new(1.35, arc_color)));

                // 8. Multi-Bead Packet Pulse Stream
                let pulse_count = 2;
                for p_i in 0..pulse_count {
                    let speed = 0.85 + ((conn_idx % 3) as f32 * 0.15);
                    let phase = (conn_idx as f32 * 0.31 + p_i as f32 * 0.5) % 1.0;
                    let bead_t = (time_sec * speed + phase) % 1.0;

                    let bead_idx = (bead_t * steps as f32) as usize;
                    let next_idx = (bead_idx + 1).min(steps);
                    let frac = (bead_t * steps as f32) - bead_idx as f32;

                    let (p0, d0) = arc_pts[bead_idx];
                    let (p1, _) = arc_pts[next_idx];
                    let bead_pos = pos2(p0.x + (p1.x - p0.x) * frac, p0.y + (p1.y - p0.y) * frac);

                    if d0 > -0.12 {
                        let bead_core = if conn.is_blocked { Color32::RED } else { Color32::WHITE };
                        painter.circle_filled(bead_pos, 3.2, arc_color);
                        painter.circle_filled(bead_pos, 1.8, bead_core);

                        // Trailing laser tail
                        if bead_idx > 1 {
                            let (p_tail, _) = arc_pts[bead_idx - 1];
                            painter.line_segment(
                                [bead_pos, p_tail],
                                Stroke::new(2.4, Color32::from_rgba_unmultiplied(arc_color.r(), arc_color.g(), arc_color.b(), 130)),
                            );
                        }
                    }

                    // Packet arrival impact shockwave ring at destination
                    if bead_t > 0.94 && target_visible {
                        let ripple_frac = (bead_t - 0.94) / 0.06;
                        let rip_r = 4.0 + (ripple_frac * 14.0);
                        let rip_alpha = ((1.0 - ripple_frac) * 180.0) as u8;
                        painter.circle_stroke(
                            target_pt,
                            rip_r,
                            Stroke::new(1.2, Color32::from_rgba_unmultiplied(arc_color.r(), arc_color.g(), arc_color.b(), rip_alpha)),
                        );
                    }
                }
            }

            // Target Node Marker
            if target_visible {
                let node_pulse = ((time_sec * 3.2 + conn_idx as f32).sin().abs() * 4.0) + 3.2;
                painter.circle_stroke(target_pt, node_pulse, Stroke::new(1.1, arc_color));
                painter.circle_filled(target_pt, 3.2, arc_color);
                painter.circle_filled(target_pt, 1.6, Color32::WHITE);

                // Floating 3D Node Label
                if self.show_labels && avg_depth > -0.10 {
                    let tag_pos = target_pt + Vec2::new(10.0, -14.0);
                    painter.line_segment([target_pt, tag_pos], Stroke::new(0.8, Color32::from_rgba_unmultiplied(arc_color.r(), arc_color.g(), arc_color.b(), 100)));
                    let tag_text = if !conn.location.is_empty() && conn.location != "-" {
                        format!("{} [{}]", conn.location, if conn.company != "-" { &conn.company } else { &conn.remote_ip })
                    } else {
                        conn.remote_ip.clone()
                    };
                    let short_tag = if tag_text.len() > 24 { format!("{}..", &tag_text[..22]) } else { tag_text };
                    painter.text(
                        tag_pos + Vec2::new(2.0, -3.0),
                        egui::Align2::LEFT_BOTTOM,
                        short_tag,
                        egui::FontId::monospace(8.5),
                        Color32::from_rgba_unmultiplied(220, 240, 255, 210),
                    );
                }

                // Mouse hover detection on remote node
                if let Some(mpos) = mouse_pos {
                    if mpos.distance(target_pt) < 12.0 {
                        new_hovered = Some(conn.remote_ip.clone());
                    }
                }
            }
        }

        self.hovered_ip = new_hovered;

        // 9. Cyberpunk HUD Overlay (Top-Left Threat Stats)
        let hud_rect = Rect::from_min_size(rect.min + Vec2::new(12.0, 12.0), Vec2::new(250.0, 84.0));
        painter.rect_filled(hud_rect, 4.0, Color32::from_rgba_unmultiplied(10, 16, 24, 225));
        painter.rect_stroke(hud_rect, 4.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 220, 255, 130)), egui::StrokeKind::Outside);

        painter.text(
            hud_rect.min + Vec2::new(8.0, 6.0),
            egui::Align2::LEFT_TOP,
            "🌐 KASPERSKY CYBER THREAT RADAR",
            egui::FontId::monospace(10.0),
            Color32::from_rgb(0, 255, 210),
        );

        let active_count = connections.len();
        let blocked_count = connections.iter().filter(|c| c.is_blocked).count();
        let total_pkts: usize = connections.iter().map(|c| c.packet_count).sum();
        let total_bytes: usize = connections.iter().map(|c| c.bytes).sum();

        painter.text(
            hud_rect.min + Vec2::new(8.0, 23.0),
            egui::Align2::LEFT_TOP,
            format!("ACTIVE STREAMS:   {} clusters", active_count),
            egui::FontId::monospace(9.5),
            Color32::WHITE,
        );
        painter.text(
            hud_rect.min + Vec2::new(8.0, 39.0),
            egui::Align2::LEFT_TOP,
            format!("TRAFFIC VOLUME:   {} pkts ({} KB)", total_pkts, total_bytes / 1024),
            egui::FontId::monospace(9.5),
            Color32::from_rgb(0, 235, 255),
        );
        painter.text(
            hud_rect.min + Vec2::new(8.0, 54.0),
            egui::Align2::LEFT_TOP,
            format!("BLOCKED THREATS:  {}", blocked_count),
            egui::FontId::monospace(9.5),
            if blocked_count > 0 { Color32::from_rgb(255, 70, 70) } else { Color32::GRAY },
        );
        painter.text(
            hud_rect.min + Vec2::new(8.0, 68.0),
            egui::Align2::LEFT_TOP,
            format!("HOST GATEWAY:     {}", origin_label),
            egui::FontId::monospace(9.0),
            Color32::from_rgb(0, 255, 140),
        );

        // 10. Hover Info Card for hovered destination node
        if let Some(ref h_ip) = self.hovered_ip {
            if let Some(conn) = connections.iter().find(|c| &c.remote_ip == h_ip) {
                if let Some(mpos) = mouse_pos {
                    let card_pos = mpos + Vec2::new(14.0, -22.0);
                    let card_rect = Rect::from_min_size(card_pos, Vec2::new(230.0, 92.0));

                    painter.rect_filled(card_rect, 4.0, Color32::from_rgba_unmultiplied(8, 14, 22, 240));
                    painter.rect_stroke(card_rect, 4.0, Stroke::new(1.2, Color32::from_rgb(0, 255, 220)), egui::StrokeKind::Outside);

                    painter.text(
                        card_rect.min + Vec2::new(8.0, 6.0),
                        egui::Align2::LEFT_TOP,
                        format!("NODE: {}", conn.remote_ip),
                        egui::FontId::monospace(10.5),
                        Color32::from_rgb(0, 255, 220),
                    );
                    painter.text(
                        card_rect.min + Vec2::new(8.0, 23.0),
                        egui::Align2::LEFT_TOP,
                        format!("ORG:  {}", if conn.company.len() > 24 { format!("{}..", &conn.company[..22]) } else { conn.company.clone() }),
                        egui::FontId::monospace(9.5),
                        Color32::WHITE,
                    );
                    painter.text(
                        card_rect.min + Vec2::new(8.0, 39.0),
                        egui::Align2::LEFT_TOP,
                        format!("LOC:  {}", conn.location),
                        egui::FontId::monospace(9.5),
                        Color32::from_rgb(255, 205, 90),
                    );
                    painter.text(
                        card_rect.min + Vec2::new(8.0, 55.0),
                        egui::Align2::LEFT_TOP,
                        format!("HOST: {}", if conn.domain.len() > 24 { format!("{}..", &conn.domain[..22]) } else { conn.domain.clone() }),
                        egui::FontId::monospace(9.5),
                        Color32::from_rgb(180, 180, 180),
                    );
                    painter.text(
                        card_rect.min + Vec2::new(8.0, 72.0),
                        egui::Align2::LEFT_TOP,
                        format!("FLOW: {} | {} pkts ({}B)", conn.proto, conn.packet_count, conn.bytes),
                        egui::FontId::monospace(8.5),
                        if conn.is_blocked { Color32::RED } else { Color32::GREEN },
                    );
                }
            }
        }
    }

    /// Background subtle starry particle field
    fn draw_starfield(painter: &egui::Painter, rect: Rect) {
        let stars = [
            (0.08, 0.12), (0.15, 0.28), (0.24, 0.09), (0.35, 0.22),
            (0.78, 0.14), (0.88, 0.26), (0.92, 0.08), (0.84, 0.45),
            (0.06, 0.72), (0.18, 0.85), (0.12, 0.62), (0.25, 0.91),
            (0.76, 0.78), (0.89, 0.88), (0.94, 0.65), (0.82, 0.95),
            (0.48, 0.05), (0.55, 0.96), (0.04, 0.44), (0.96, 0.42),
        ];
        for &(sx, sy) in &stars {
            let p = pos2(rect.left() + rect.width() * sx, rect.top() + rect.height() * sy);
            painter.circle_filled(p, 1.0, Color32::from_rgba_unmultiplied(180, 220, 255, 80));
        }
    }

    fn to_unit_vector(lat_deg: f32, lon_deg: f32) -> [f32; 3] {
        let phi = lat_deg.to_radians();
        let lambda = lon_deg.to_radians();
        [
            phi.cos() * lambda.sin(),
            -phi.sin(),
            phi.cos() * lambda.cos(),
        ]
    }

    fn slerp(v0: [f32; 3], v1: [f32; 3], t: f32, omega: f32) -> [f32; 3] {
        if omega < 0.001 {
            return v0;
        }
        let sin_omega = omega.sin();
        let c0 = ((1.0 - t) * omega).sin() / sin_omega;
        let c1 = (t * omega).sin() / sin_omega;
        [
            c0 * v0[0] + c1 * v1[0],
            c0 * v0[1] + c1 * v1[1],
            c0 * v0[2] + c1 * v1[2],
        ]
    }

    /// Kaspersky Cyber Matrix Dot Grid Coordinates (sample points on landmasses)
    fn landmass_matrix_dots() -> &'static [(f32, f32)] {
        static DOTS: std::sync::OnceLock<Vec<(f32, f32)>> = std::sync::OnceLock::new();
        DOTS.get_or_init(|| {
            let regions = [
                // North America (US, Canada, Mexico)
                (25.0, 49.0, -125.0, -70.0, 4.0),
                (50.0, 68.0, -135.0, -65.0, 5.0),
                (16.0, 25.0, -105.0, -86.0, 4.0),
                (58.0, 68.0, -165.0, -140.0, 4.5), // Alaska
                // South America
                (-52.0, 10.0, -78.0, -36.0, 4.5),
                // Europe
                (36.0, 68.0, -10.0, 38.0, 3.5),
                (50.0, 58.0, -8.0, 1.0, 2.5), // UK
                // Africa
                (-34.0, 36.0, -17.0, 48.0, 4.5),
                // Middle East
                (14.0, 35.0, 36.0, 60.0, 4.0),
                // Asia
                (8.0, 68.0, 62.0, 138.0, 4.5),
                (31.0, 44.0, 130.0, 144.0, 3.0), // Japan
                // Australia & New Zealand
                (-38.0, -12.0, 114.0, 152.0, 4.5),
                (-46.0, -35.0, 166.0, 178.0, 3.0),
            ];

            let mut out = Vec::with_capacity(1250);
            for &(min_lat, max_lat, min_lon, max_lon, step) in &regions {
                let mut lat = min_lat;
                while lat <= max_lat {
                    let mut lon = min_lon;
                    while lon <= max_lon {
                        out.push((lat, lon));
                        lon += step;
                    }
                    lat += step;
                }
            }
            out
        })
    }

    /// World coastlines dataset represented as polylines of (lat, lon) degrees
    fn world_coastlines() -> &'static [&'static [(f32, f32)]] {
        &[
            // North America
            &[
                (71.0, -156.0), (65.0, -168.0), (58.0, -158.0), (54.0, -165.0), (59.0, -140.0),
                (54.0, -130.0), (48.0, -124.0), (38.0, -123.0), (33.0, -118.0), (23.0, -110.0),
                (16.0, -97.0), (14.0, -92.0), (9.0, -83.0), (8.0, -77.0),
                (10.0, -75.0), (16.0, -88.0), (21.0, -97.0), (26.0, -97.0), (30.0, -88.0),
                (25.0, -81.0), (31.0, -81.0), (35.0, -75.0), (40.0, -74.0), (44.0, -66.0),
                (47.0, -65.0), (52.0, -56.0), (58.0, -62.0), (62.0, -75.0), (55.0, -82.0),
                (52.0, -80.0), (56.0, -88.0), (63.0, -92.0), (70.0, -85.0), (72.0, -120.0),
                (71.0, -156.0),
            ],
            // Baja California
            &[
                (32.0, -115.0), (24.0, -110.0), (23.0, -109.0), (28.0, -112.0), (32.0, -115.0),
            ],
            // Great Lakes
            &[
                (45.0, -87.0), (47.0, -88.0), (48.0, -84.0), (45.0, -83.0), (43.0, -82.0), (42.0, -83.0), (45.0, -87.0),
            ],
            // South America
            &[
                (11.0, -74.0), (10.0, -62.0), (6.0, -55.0), (4.0, -51.0),
                (0.0, -48.0), (-5.0, -35.0), (-13.0, -39.0), (-23.0, -43.0), (-30.0, -50.0), (-35.0, -54.0),
                (-40.0, -62.0), (-48.0, -66.0), (-55.0, -66.0), (-54.0, -71.0),
                (-50.0, -74.0), (-40.0, -73.0), (-30.0, -71.0), (-20.0, -70.0), (-12.0, -77.0),
                (-4.0, -81.0), (1.0, -79.0), (8.0, -77.0), (11.0, -74.0),
            ],
            // Europe & Mediterranean
            &[
                (36.0, -6.0), (37.0, -9.0), (43.0, -9.0), (43.0, -2.0), (44.0, 0.0),
                (43.0, 3.0), (46.0, -1.0), (48.0, -4.0), (50.0, 1.0), (51.0, 2.0),
                (51.0, 3.0), (53.0, 7.0), (54.0, 9.0),
                (58.0, 6.0), (62.0, 5.0), (70.0, 24.0), (71.0, 28.0), (66.0, 25.0), (60.0, 19.0), (56.0, 13.0),
                (54.0, 14.0), (55.0, 19.0), (59.0, 25.0), (60.0, 29.0), (60.0, 25.0),
            ],
            // Italy
            &[
                (44.0, 8.0), (41.0, 13.0), (38.0, 16.0), (40.0, 18.0), (42.0, 14.0), (45.0, 12.0),
            ],
            // Greece & Balkans
            &[
                (41.0, 19.0), (38.0, 23.0), (36.0, 23.0), (38.0, 24.0), (41.0, 26.0), (44.0, 29.0), (46.0, 31.0),
            ],
            // Black Sea
            &[
                (42.0, 28.0), (41.0, 30.0), (42.0, 41.0), (45.0, 37.0), (46.0, 31.0),
            ],
            // British Isles
            &[
                (50.0, -5.0), (51.0, 1.0), (53.0, 0.0), (57.0, -2.0), (58.0, -4.0), (56.0, -6.0), (52.0, -5.0), (50.0, -5.0),
            ],
            // Ireland
            &[
                (52.0, -10.0), (55.0, -8.0), (55.0, -6.0), (52.0, -6.0), (52.0, -10.0),
            ],
            // Africa
            &[
                (36.0, -5.0), (37.0, 10.0), (32.0, 20.0), (31.0, 32.0),
                (28.0, 34.0), (22.0, 38.0), (13.0, 43.0), (12.0, 51.0), (2.0, 45.0),
                (-4.0, 39.0), (-15.0, 40.0), (-26.0, 33.0), (-34.0, 18.0), (-34.0, 25.0),
                (-28.0, 16.0), (-15.0, 12.0), (-5.0, 12.0), (4.0, 9.0), (5.0, 0.0), (6.0, -2.0),
                (4.0, -7.0), (8.0, -13.0), (15.0, -17.0), (21.0, -17.0), (28.0, -13.0), (36.0, -5.0),
            ],
            // Madagascar
            &[
                (-12.0, 49.0), (-16.0, 50.0), (-25.0, 47.0), (-25.0, 44.0), (-17.0, 44.0), (-12.0, 49.0),
            ],
            // Asia & Middle East
            &[
                (30.0, 35.0), (28.0, 34.0), (13.0, 43.0), (14.0, 53.0), (22.0, 60.0), (26.0, 56.0), (30.0, 48.0),
                (24.0, 69.0), (19.0, 73.0), (10.0, 76.0), (8.0, 77.0), (10.0, 80.0), (16.0, 82.0), (22.0, 89.0),
                (21.0, 92.0), (16.0, 96.0), (10.0, 99.0), (1.0, 104.0), (7.0, 103.0), (13.0, 101.0),
                (10.0, 107.0), (16.0, 108.0), (21.0, 108.0), (22.0, 114.0), (25.0, 119.0), (31.0, 122.0),
                (35.0, 120.0), (39.0, 118.0), (40.0, 124.0),
            ],
            // Korea
            &[
                (40.0, 125.0), (38.0, 128.0), (35.0, 129.0), (35.0, 127.0), (38.0, 125.0),
            ],
            // Russia / Siberia
            &[
                (43.0, 132.0), (48.0, 140.0), (59.0, 150.0), (60.0, 163.0), (66.0, 170.0),
                (72.0, 130.0), (76.0, 100.0), (72.0, 60.0),
            ],
            // Japan
            &[
                (31.0, 130.0), (35.0, 136.0), (41.0, 141.0), (45.0, 142.0), (42.0, 144.0), (34.0, 131.0), (31.0, 130.0),
            ],
            // Australia
            &[
                (-12.0, 131.0), (-12.0, 142.0), (-25.0, 153.0), (-38.0, 145.0),
                (-35.0, 115.0), (-22.0, 114.0), (-12.0, 131.0),
            ],
            // Tasmania
            &[
                (-41.0, 145.0), (-43.0, 147.0), (-43.0, 145.0), (-41.0, 145.0),
            ],
            // New Zealand North & South
            &[
                (-35.0, 174.0), (-37.0, 178.0), (-41.0, 175.0), (-38.0, 175.0), (-35.0, 174.0),
            ],
            &[
                (-41.0, 174.0), (-44.0, 171.0), (-46.0, 167.0), (-43.0, 170.0), (-41.0, 174.0),
            ],
        ]
    }
}
