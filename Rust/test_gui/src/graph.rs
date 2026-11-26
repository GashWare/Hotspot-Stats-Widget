use egui::{epaint::PathShape, pos2, Color32, Painter, Pos2, Rect, Stroke, Vec2};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const MAX_POINTS: usize = 35;

fn format_rate_compact(bps: f32) -> String {
    if bps >= 1_048_576.0 {
        format!("{:.1}M", bps / 1_048_576.0)
    } else if bps >= 1024.0 {
        format!("{:.0}K", bps / 1024.0)
    } else {
        format!("{:.0}", bps)
    }
}

#[derive(Clone, Debug)]
pub struct FluidGraph {
    pub history: Vec<VecDeque<f32>>,
    pub colors: Vec<Color32>,
    pub max_val: Option<f32>,
    pub style: String, // "line", "block", "dash"
    pub fluid: bool,
    pub interval_ms: u64,
    pub last_push: Instant,
    pub current_percent: f32,
    pub target_percent: f32,
    pub peak_percent: f32,
    pub peak_time: Instant,
    pub peak_val_dl: f32,
    pub peak_val_ul: f32,
    pub peak_dl_time: Instant,
    pub peak_ul_time: Instant,
    pub label: String,
    pub anim_t: f32,
}

impl FluidGraph {
    pub fn new(colors: Vec<Color32>, max_val: Option<f32>, style: &str, fluid: bool, interval_ms: u64) -> Self {
        let count = colors.len();
        let mut history = Vec::with_capacity(count);
        for _ in 0..count {
            let mut q = VecDeque::with_capacity(MAX_POINTS);
            for _ in 0..MAX_POINTS {
                q.push_back(0.0);
            }
            history.push(q);
        }

        Self {
            history,
            colors,
            max_val,
            style: style.to_string(),
            fluid,
            interval_ms: interval_ms.max(50),
            last_push: Instant::now(),
            current_percent: 0.0,
            target_percent: 0.0,
            peak_percent: 0.0,
            peak_time: Instant::now(),
            peak_val_dl: 1024.0 * 1024.0,
            peak_val_ul: 512.0 * 1024.0,
            peak_dl_time: Instant::now(),
            peak_ul_time: Instant::now(),
            label: String::new(),
            anim_t: 0.0,
        }
    }

    pub fn push_values(&mut self, values: &[f32]) {
        for (idx, &v) in values.iter().enumerate() {
            if idx < self.history.len() {
                if self.history[idx].len() >= MAX_POINTS {
                    self.history[idx].pop_front();
                }
                self.history[idx].push_back(v);
            }
        }
        self.last_push = Instant::now();

        if !values.is_empty() {
            let last_v = values[0];
            let max_v = self.max_val.unwrap_or(100.0);
            self.target_percent = (last_v / max_v * 100.0).clamp(0.0, 100.0);
            if !self.fluid {
                self.current_percent = self.target_percent;
            }

            if self.target_percent >= self.peak_percent {
                self.peak_percent = self.target_percent;
                self.peak_time = Instant::now();
            } else if self.peak_time.elapsed() > Duration::from_millis(1500) {
                self.peak_percent = (self.peak_percent - 1.2).max(self.target_percent);
            }

            if values.len() >= 2 {
                let dl = values[0];
                let ul = values[1];
                if dl >= self.peak_val_dl {
                    self.peak_val_dl = dl;
                    self.peak_dl_time = Instant::now();
                } else if self.peak_dl_time.elapsed() > Duration::from_millis(2000) {
                    self.peak_val_dl = (self.peak_val_dl * 0.98).max(dl).max(1024.0 * 1024.0);
                }

                if ul >= self.peak_val_ul {
                    self.peak_val_ul = ul;
                    self.peak_ul_time = Instant::now();
                } else if self.peak_ul_time.elapsed() > Duration::from_millis(2000) {
                    self.peak_val_ul = (self.peak_val_ul * 0.98).max(ul).max(512.0 * 1024.0);
                }
            }
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, height: f32, theme: &crate::config::ThemePalette) -> Rect {
        let (rect, _response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height),
            egui::Sense::hover(),
        );

        let painter = ui.painter_at(rect);
        // Background & Bezel
        painter.rect_filled(rect, 2.0, theme.graph_bg);
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, theme.border), egui::StrokeKind::Inside);

        self.anim_t = (ui.input(|i| i.time) % 3600.0) as f32;

        if self.style == "dash" {
            if !(crate::skins::has_skin(&theme.id) && self.render_skin(&painter, rect, theme)) {
                self.render_digital_dash(&painter, rect, theme);
            }
        } else if self.style == "block" {
            self.render_blocks(&painter, rect, theme);
        } else {
            self.render_lines(&painter, rect, theme);
        }
        rect
    }

    /// Theme-specific layout (see skins.rs). Returns false if the theme has no skin.
    fn render_skin(&mut self, painter: &Painter, rect: Rect, theme: &crate::config::ThemePalette) -> bool {
        if rect.width() < 40.0 || rect.height() < 16.0 {
            return true;
        }
        let label = if self.label.is_empty() { "SYS".to_string() } else { self.label.clone() };
        let seed: u32 = label.bytes().fold(17u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32));

        // Dual metric (network DL / UL): split into two bands
        if self.history.len() >= 2 && self.max_val.is_none() {
            let dl_val = self.history[0].back().copied().unwrap_or(0.0);
            let ul_val = self.history[1].back().copied().unwrap_or(0.0);
            let pk_dl = self.peak_val_dl.max(1024.0 * 1024.0);
            let pk_ul = self.peak_val_ul.max(512.0 * 1024.0);
            let dl_pct = (dl_val / pk_dl * 100.0).clamp(0.0, 100.0);
            let ul_pct = (ul_val / pk_ul * 100.0).clamp(0.0, 100.0);
            let half = (rect.height() - 2.0) / 2.0;
            let top = Rect::from_min_size(rect.min, Vec2::new(rect.width(), half));
            let bot = Rect::from_min_size(pos2(rect.left(), rect.top() + half + 2.0), Vec2::new(rect.width(), half));
            let mk = |r: Rect, pct: f32, hist: &VecDeque<f32>, pk: f32, lbl: &str, val: f32, sd: u32| crate::skins::Band {
                rect: r,
                pct,
                peak: pct,
                label: lbl.to_string(),
                sub: format!("{}/s", format_rate_compact(val)),
                history: hist.iter().map(|v| (v / pk * 100.0).clamp(0.0, 100.0)).collect(),
                t: self.anim_t,
                seed: seed ^ sd,
            };
            let b1 = mk(top, dl_pct, &self.history[0], pk_dl, "DL", dl_val, 1);
            let b2 = mk(bot, ul_pct, &self.history[1], pk_ul, "UL", ul_val, 2);
            let p1 = painter.with_clip_rect(top);
            let p2 = painter.with_clip_rect(bot);
            crate::skins::draw(&p1, &b1, theme);
            return crate::skins::draw(&p2, &b2, theme);
        }

        if self.fluid {
            let diff = self.target_percent - self.current_percent;
            if diff.abs() > 0.15 {
                self.current_percent += diff * 0.22;
            }
        } else {
            self.current_percent = self.target_percent;
        }
        let max_v = self.max_val.unwrap_or(100.0).max(1.0);
        let band = crate::skins::Band {
            rect,
            pct: self.current_percent,
            peak: self.peak_percent,
            label,
            sub: format!("PK:{:.0}", self.peak_percent),
            history: self
                .history
                .first()
                .map(|h| h.iter().map(|v| (v / max_v * 100.0).clamp(0.0, 100.0)).collect())
                .unwrap_or_default(),
            t: self.anim_t,
            seed,
        };
        crate::skins::draw(painter, &band, theme)
    }

    fn render_digital_dash(&mut self, painter: &Painter, rect: Rect, theme: &crate::config::ThemePalette) {
        let w = rect.width();
        let h = rect.height();
        if w < 20.0 || h < 16.0 {
            return;
        }

        // Check if multi-metric (e.g. Network DL & UL)
        if self.history.len() >= 2 && self.max_val.is_none() {
            self.render_dual_digital_dash(painter, rect, theme);
            return;
        }

        // Single metric: CPU / RAM / Swap / Disk
        if self.fluid {
            let diff = self.target_percent - self.current_percent;
            if diff.abs() > 0.15 {
                self.current_percent += diff * 0.22;
            }
        } else {
            self.current_percent = self.target_percent;
        }

        let is_compact = h < 46.0;

        // Top Status & Digital Readout Row
        let readout_str = format!("{:>5.1}%", self.current_percent);
        let peak_str = format!("PK:{:.0}%", self.peak_percent);

        if !is_compact {
            painter.text(
                pos2(rect.left() + 6.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "TACH :: BOOST",
                egui::FontId::monospace(8.5),
                theme.text_dim,
            );
            painter.text(
                pos2(rect.right() - 6.0, rect.top() + 7.0),
                egui::Align2::RIGHT_TOP,
                readout_str,
                egui::FontId::monospace(12.0),
                theme.primary,
            );
            painter.text(
                pos2(rect.right() - 65.0, rect.top() + 8.5),
                egui::Align2::RIGHT_TOP,
                peak_str,
                egui::FontId::monospace(8.5),
                theme.secondary,
            );
        } else {
            painter.text(
                pos2(rect.right() - 4.0, rect.top() + 3.0),
                egui::Align2::RIGHT_TOP,
                readout_str,
                egui::FontId::monospace(10.0),
                theme.primary,
            );
        }

        // Segmented Slanted Tachometer Arc Ladder
        let seg_count = if w > 360.0 { 34 } else if w > 280.0 { 28 } else { 22 };
        let spacing = 2.0;
        let slant_dx = if is_compact { 3.5 } else { 5.0 };

        let total_avail_w = w - 16.0 - slant_dx;
        let seg_w = ((total_avail_w - (seg_count as f32 * spacing)) / seg_count as f32).max(3.0);

        let lit_count = ((self.current_percent / 100.0) * seg_count as f32).round() as usize;
        let peak_idx = ((self.peak_percent / 100.0) * seg_count as f32).round() as usize;

        let y_bottom = if is_compact { rect.bottom() - 4.0 } else { rect.bottom() - 11.0 };
        let base_top = if is_compact { rect.top() + 14.0 } else { rect.top() + 20.0 };

        for i in 0..seg_count {
            let t = i as f32 / (seg_count - 1).max(1) as f32;
            let x1 = rect.left() + 8.0 + i as f32 * (seg_w + spacing);
            let x2 = x1 + seg_w;

            // Arched tachometer curve: segments rise progressively toward the right
            let arc_rise = (1.0 - (t * std::f32::consts::PI * 0.5).sin()) * ((y_bottom - base_top) * 0.28);
            let y_top = base_top + arc_rise;

            // Segment color by load zone
            let seg_color = if t < 0.65 {
                theme.primary
            } else if t < 0.82 {
                theme.warning
            } else {
                theme.redline
            };

            let p1 = pos2(x1 + slant_dx, y_top);
            let p2 = pos2(x2 + slant_dx, y_top);
            let p3 = pos2(x2, y_bottom);
            let p4 = pos2(x1, y_bottom);

            if i < lit_count {
                // Lit active glowing segment
                painter.add(PathShape::convex_polygon(vec![p1, p2, p3, p4], seg_color, Stroke::NONE));
            } else {
                // Phantom unlit glass segment (authentic dark LCD/VFD instrument bezel)
                let dim_fill = Color32::from_rgba_unmultiplied(seg_color.r(), seg_color.g(), seg_color.b(), 24);
                let dim_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(seg_color.r(), seg_color.g(), seg_color.b(), 42));
                painter.add(PathShape::convex_polygon(vec![p1, p2, p3, p4], dim_fill, dim_stroke));
            }

            // Peak Hold Marker Pip
            if i == peak_idx.min(seg_count - 1) && peak_idx > 0 {
                let peak_fill = Color32::from_rgb(255, 255, 255);
                painter.add(PathShape::convex_polygon(vec![p1, p2, p3, p4], peak_fill, Stroke::new(1.5, theme.redline)));
            }
        }

        // Bottom Scale Ticks & Redline Warning
        if !is_compact {
            let tick_y = rect.bottom() - 3.0;
            painter.text(
                pos2(rect.left() + 8.0, tick_y),
                egui::Align2::LEFT_BOTTOM,
                "0",
                egui::FontId::monospace(7.5),
                theme.text_dim,
            );
            painter.text(
                pos2(rect.left() + (w * 0.5), tick_y),
                egui::Align2::CENTER_BOTTOM,
                "50%",
                egui::FontId::monospace(7.5),
                theme.text_dim,
            );
            painter.text(
                pos2(rect.right() - 8.0, tick_y),
                egui::Align2::RIGHT_BOTTOM,
                "▲ REDLINE",
                egui::FontId::monospace(7.5),
                theme.redline,
            );
        }
    }

    fn render_dual_digital_dash(&mut self, painter: &Painter, rect: Rect, theme: &crate::config::ThemePalette) {
        let w = rect.width();
        let h = rect.height();

        let dl_val = self.history[0].back().copied().unwrap_or(0.0);
        let ul_val = self.history[1].back().copied().unwrap_or(0.0);

        let dl_pct = (dl_val / self.peak_val_dl.max(1024.0 * 1024.0) * 100.0).clamp(0.0, 100.0);
        let ul_pct = (ul_val / self.peak_val_ul.max(512.0 * 1024.0) * 100.0).clamp(0.0, 100.0);

        let half_h = (h - 6.0) / 2.0;
        let seg_count = if w > 360.0 { 28 } else { 22 };
        let spacing = 1.8;
        let slant_dx = 4.0;
        let total_avail_w = w - 16.0 - slant_dx;
        let seg_w = ((total_avail_w - (seg_count as f32 * spacing)) / seg_count as f32).max(3.0);

        // Top Row: Download (DL)
        let dl_rect = Rect::from_min_size(rect.min, Vec2::new(w, half_h));
        let dl_lit = ((dl_pct / 100.0) * seg_count as f32).round() as usize;
        let dl_str = format!("↓ DL [{:>5}/s]", format_rate_compact(dl_val));
        painter.text(
            pos2(dl_rect.left() + 6.0, dl_rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            dl_str,
            egui::FontId::monospace(9.0),
            theme.primary,
        );

        let dl_y_bot = dl_rect.bottom() - 2.0;
        let dl_y_top = dl_rect.top() + 15.0;
        for i in 0..seg_count {
            let t = i as f32 / (seg_count - 1).max(1) as f32;
            let x1 = dl_rect.left() + 8.0 + i as f32 * (seg_w + spacing);
            let x2 = x1 + seg_w;
            let col = if t < 0.75 { theme.primary } else { theme.warning };

            let p1 = pos2(x1 + slant_dx, dl_y_top);
            let p2 = pos2(x2 + slant_dx, dl_y_top);
            let p3 = pos2(x2, dl_y_bot);
            let p4 = pos2(x1, dl_y_bot);

            if i < dl_lit {
                painter.add(PathShape::convex_polygon(vec![p1, p2, p3, p4], col, Stroke::NONE));
            } else {
                let dim_fill = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 22);
                painter.add(PathShape::convex_polygon(vec![p1, p2, p3, p4], dim_fill, Stroke::NONE));
            }
        }

        // Bottom Row: Upload (UL)
        let ul_rect = Rect::from_min_size(pos2(rect.left(), rect.top() + half_h + 4.0), Vec2::new(w, half_h));
        let ul_lit = ((ul_pct / 100.0) * seg_count as f32).round() as usize;
        let ul_str = format!("↑ UL [{:>5}/s]", format_rate_compact(ul_val));
        painter.text(
            pos2(ul_rect.left() + 6.0, ul_rect.top() + 2.0),
            egui::Align2::LEFT_TOP,
            ul_str,
            egui::FontId::monospace(9.0),
            theme.secondary,
        );

        let ul_y_bot = ul_rect.bottom() - 2.0;
        let ul_y_top = ul_rect.top() + 13.0;
        for i in 0..seg_count {
            let t = i as f32 / (seg_count - 1).max(1) as f32;
            let x1 = ul_rect.left() + 8.0 + i as f32 * (seg_w + spacing);
            let x2 = x1 + seg_w;
            let col = if t < 0.75 { theme.secondary } else { theme.redline };

            let p1 = pos2(x1 + slant_dx, ul_y_top);
            let p2 = pos2(x2 + slant_dx, ul_y_top);
            let p3 = pos2(x2, ul_y_bot);
            let p4 = pos2(x1, ul_y_bot);

            if i < ul_lit {
                painter.add(PathShape::convex_polygon(vec![p1, p2, p3, p4], col, Stroke::NONE));
            } else {
                let dim_fill = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 22);
                painter.add(PathShape::convex_polygon(vec![p1, p2, p3, p4], dim_fill, Stroke::NONE));
            }
        }
    }

    fn render_lines(&self, painter: &Painter, rect: Rect, theme: &crate::config::ThemePalette) {
        let w = rect.width();
        let h = rect.height();
        if w < 10.0 || h < 10.0 {
            return;
        }

        // Draw subtle grid lines
        let grid_stroke = Stroke::new(1.0, theme.grid);
        for i in 1..4 {
            let y = rect.top() + (h / 4.0) * i as f32;
            painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], grid_stroke);
        }

        // Determine current max
        let current_max = if let Some(m) = self.max_val {
            m.max(1.0)
        } else {
            let mut peak = 1024.0f32;
            for line in &self.history {
                for &v in line {
                    if v > peak {
                        peak = v;
                    }
                }
            }
            peak
        };

        let step_x = w / ((MAX_POINTS - 1) as f32);

        // Fluid sub-tick shift
        let shift_x = if self.fluid && self.interval_ms > 0 {
            let elapsed = self.last_push.elapsed().as_millis() as f32;
            let frac = (elapsed / self.interval_ms as f32).clamp(0.0, 1.0);
            frac * step_x
        } else {
            0.0
        };

        for (line_idx, data) in self.history.iter().enumerate() {
            let color = self.colors.get(line_idx).copied().unwrap_or(theme.primary);
            let mut points = Vec::with_capacity(data.len() + 1);

            for (i, &v) in data.iter().enumerate() {
                let x = rect.left() + (i as f32 * step_x) - shift_x;
                let norm = (v / current_max).clamp(0.0, 1.0);
                let y = rect.bottom() - (norm * (h - 8.0)) - 4.0;
                points.push(pos2(x, y));
            }

            // Extend to the right edge if fluid
            if self.fluid && !points.is_empty() {
                let last_y = points.last().unwrap().y;
                points.push(pos2(rect.right(), last_y));
            }

            if points.len() >= 2 {
                painter.add(PathShape::line(points, Stroke::new(2.0, color)));
            }
        }
    }

    fn render_blocks(&mut self, painter: &Painter, rect: Rect, theme: &crate::config::ThemePalette) {
        let cols = 45;
        let rows = 4;
        let spacing = 1.5;

        let total_w = rect.width();
        let total_h = rect.height();

        let block_w = (total_w - ((cols + 1) as f32 * spacing)) / cols as f32;
        let block_h = (total_h - ((rows + 1) as f32 * spacing)) / rows as f32;

        if self.fluid {
            let diff = self.target_percent - self.current_percent;
            if diff.abs() > 0.3 {
                self.current_percent += diff * 0.22;
            }
        } else {
            self.current_percent = self.target_percent;
        }

        let total_blocks = cols * rows;
        let lit_count = ((self.current_percent / 100.0) * total_blocks as f32).round() as usize;
        let active_c = self.colors.first().copied().unwrap_or(theme.primary);
        let dim_c = theme.grid;

        let mut block_idx = 0;
        for r in (0..rows).rev() {
            for c in 0..cols {
                let x1 = rect.left() + spacing + c as f32 * (block_w + spacing);
                let y1 = rect.top() + spacing + r as f32 * (block_h + spacing);
                let x2 = x1 + block_w;
                let y2 = y1 + block_h;

                let col = if block_idx < lit_count { active_c } else { dim_c };
                painter.rect_filled(Rect::from_min_max(Pos2::new(x1, y1), Pos2::new(x2, y2)), 1.0, col);
                block_idx += 1;
            }
        }
    }
}

