//! Theme "skins": each theme gets its own graph *layout* (not just a recolor).
//!
//! * `ferrari_dash`  - shift-light LED strip + sweeping needle gauge with big readout
//! * `corvette_red`  - C4 style 7-segment LCD digits + ascending wedge bar-graph
//! * `vfd_green`     - dot-matrix VFD history columns with phosphor glow
//! * `matrix`        - animated falling code rain, density/speed driven by load
//! * `hackers`       - Gibson wireframe skyline over a perspective grid + terminal prompt
//! * `synthwave`     - rising striped sun over a scrolling perspective grid
//!
//! `retro_amber` (and every other theme) keeps the classic slanted tach ladder in graph.rs.

use egui::{epaint::PathShape, pos2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::config::ThemePalette;

/// One metric drawn into one rectangle.
pub struct Band {
    pub rect: Rect,
    /// Current value 0..100
    pub pct: f32,
    /// Peak-hold 0..100
    pub peak: f32,
    pub label: String,
    /// Secondary readout (peak, or a rate like "1.2M/s")
    pub sub: String,
    /// Oldest -> newest, each 0..100
    pub history: Vec<f32>,
    /// Animation clock in seconds
    pub t: f32,
    pub seed: u32,
}

/// Themes that have a dedicated skin renderer.
pub fn has_skin(theme_id: &str) -> bool {
    matches!(
        theme_id,
        "ferrari_dash" | "corvette_red" | "nissan_300zx" | "vfd_green" | "matrix" | "hackers" | "synthwave"
    )
}

/// Themes whose skin animates continuously (needs faster repaint).
pub fn is_animated(theme_id: &str) -> bool {
    matches!(theme_id, "matrix" | "synthwave" | "hackers" | "ferrari_dash")
}

pub fn draw(p: &Painter, b: &Band, th: &ThemePalette) -> bool {
    match th.id.as_str() {
        "ferrari_dash" => ferrari(p, b, th),
        "corvette_red" => corvette(p, b, th),
        "nissan_300zx" => nissan_300zx(p, b, th),
        "vfd_green" => vfd(p, b, th),
        "matrix" => matrix(p, b, th),
        "hackers" => hackers(p, b, th),
        "synthwave" => synthwave(p, b, th),
        _ => return false,
    }
    true
}

/// Full-window animated backdrop (drawn under the cards).
pub fn paint_window_background(p: &Painter, rect: Rect, th: &ThemePalette, t: f32) {
    match th.id.as_str() {
        "matrix" => rain(p, rect, th, t, 0.30, 0.0, 17, 12.0, 9, 0.16, th.primary, false),
        "hackers" => {
            // faint CRT scanlines with one slow rolling bright band
            let band_y = rect.top() + ((t * 40.0) % (rect.height() + 40.0)) - 20.0;
            let mut y = rect.top();
            while y < rect.bottom() {
                let near = (1.0 - ((y - band_y).abs() / 22.0)).clamp(0.0, 1.0);
                let alpha = (10.0 + near * 34.0) as u8;
                p.line_segment(
                    [pos2(rect.left(), y), pos2(rect.right(), y)],
                    Stroke::new(1.0, a(th.primary, alpha)),
                );
                y += 3.0;
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------- helpers

fn a(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

fn lerp_c(x: Color32, y: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * t) as u8;
    Color32::from_rgb(l(x.r(), y.r()), l(x.g(), y.g()), l(x.b(), y.b()))
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

fn hashf(x: u32) -> f32 {
    (hash(x) & 0xffff) as f32 / 65535.0
}

fn resample(hist: &[f32], n: usize) -> Vec<f32> {
    if hist.is_empty() || n == 0 {
        return vec![0.0; n];
    }
    (0..n)
        .map(|i| {
            let idx = (i * hist.len()) / n;
            hist[idx.min(hist.len() - 1)]
        })
        .collect()
}

fn glow_text(p: &Painter, pos: Pos2, al: Align2, s: &str, size: f32, col: Color32) {
    let f = FontId::monospace(size);
    p.text(pos + Vec2::new(0.0, 0.0), al, s, f.clone(), col);
}

// ---------------------------------------------------------------- Ferrari

fn ferrari(p: &Painter, b: &Band, th: &ThemePalette) {
    let r = b.rect;
    let (w, h) = (r.width(), r.height());
    let pct = b.pct.clamp(0.0, 100.0);
    let compact = h < 44.0;

    // Shift-light LED strip across the top
    let led_r = if compact { 2.6 } else { 3.4 };
    let n = (((w - 16.0) / 11.0) as usize).clamp(8, 30);
    let pitch = (w - 16.0) / n as f32;
    let lit = ((pct / 100.0) * n as f32).round() as usize;
    let flash = pct > 92.0 && ((b.t * 9.0) as i32 % 2 == 0);
    let y = r.top() + 4.0 + led_r;
    for i in 0..n {
        let t = i as f32 / (n - 1).max(1) as f32;
        let base = if t < 0.5 {
            th.secondary
        } else if t < 0.8 {
            th.warning
        } else {
            th.redline
        };
        let c = pos2(r.left() + 8.0 + pitch * (i as f32 + 0.5), y);
        if i < lit {
            let col = if flash { Color32::WHITE } else { base };
            p.circle_filled(c, led_r * 2.0, a(col, 38));
            p.circle_filled(c, led_r, col);
        } else {
            p.circle_filled(c, led_r, a(base, 26));
            p.circle_stroke(c, led_r, Stroke::new(0.8, a(base, 60)));
        }
    }

    let body_top = y + led_r + 3.0;
    let body = Rect::from_min_max(pos2(r.left() + 6.0, body_top), pos2(r.right() - 6.0, r.bottom() - 2.0));
    if body.height() < 8.0 {
        return;
    }

    if compact {
        p.text(
            pos2(body.left(), body.center().y),
            Align2::LEFT_CENTER,
            &b.label,
            FontId::monospace(9.0),
            th.text_dim,
        );
        p.text(
            pos2(body.right(), body.center().y),
            Align2::RIGHT_CENTER,
            format!("{:.0}  {}", pct, b.sub),
            FontId::monospace(11.0),
            th.text_bright,
        );
        return;
    }

    // Semicircular needle gauge (left)
    let rad = (body.height() - 3.0).min(w * 0.30).max(10.0);
    let cx = body.left() + rad + 2.0;
    let cy = body.bottom() - 1.0;
    let ang = |f: f32| std::f32::consts::PI * (1.0 - f.clamp(0.0, 1.0));
    let pt = |f: f32, rr: f32| {
        let an = ang(f);
        pos2(cx + rr * an.cos(), cy - rr * an.sin())
    };
    let arc = |from: f32, to: f32, rr: f32| -> Vec<Pos2> {
        let steps = 28;
        (0..=steps).map(|k| pt(from + (to - from) * k as f32 / steps as f32, rr)).collect()
    };
    p.add(PathShape::line(arc(0.0, 0.8, rad), Stroke::new(3.0, a(th.secondary, 40))));
    p.add(PathShape::line(arc(0.8, 1.0, rad), Stroke::new(3.0, a(th.redline, 90))));
    if pct > 0.5 {
        let col = if pct > 80.0 { th.redline } else if pct > 55.0 { th.warning } else { th.primary };
        p.add(PathShape::line(arc(0.0, pct / 100.0, rad), Stroke::new(3.0, col)));
    }
    for k in 0..=10 {
        let f = k as f32 / 10.0;
        let inner = if k % 5 == 0 { rad - 7.0 } else { rad - 4.0 };
        p.line_segment([pt(f, inner), pt(f, rad - 1.5)], Stroke::new(1.0, a(th.text_dim, 200)));
    }
    // needle + hub
    p.line_segment([pt(pct / 100.0, 3.0), pt(pct / 100.0, rad - 5.0)], Stroke::new(2.0, th.tertiary));
    p.circle_filled(pos2(cx, cy), 3.2, th.tertiary);
    // peak tick
    p.line_segment(
        [pt(b.peak / 100.0, rad - 9.0), pt(b.peak / 100.0, rad + 1.0)],
        Stroke::new(1.6, Color32::WHITE),
    );

    // Big numeric readout
    let tx = cx + rad + 12.0;
    let big = (body.height() * 0.78).clamp(14.0, 30.0);
    p.text(
        pos2(tx, body.center().y + 1.0),
        Align2::LEFT_CENTER,
        format!("{:.0}", pct),
        FontId::monospace(big),
        th.text_bright,
    );
    let rx = body.right();
    p.text(pos2(rx, body.top() + 1.0), Align2::RIGHT_TOP, &b.label, FontId::monospace(10.0), th.primary);
    p.text(pos2(rx, body.top() + 14.0), Align2::RIGHT_TOP, &b.sub, FontId::monospace(8.5), th.text_dim);
}

// ---------------------------------------------------------------- Corvette (7-seg LCD + wedge)

fn seven_seg(p: &Painter, x: f32, y: f32, w: f32, h: f32, digit: Option<u32>, col: Color32) {
    // a b c d e f g
    const MAP: [[bool; 7]; 10] = [
        [true, true, true, true, true, true, false],
        [false, true, true, false, false, false, false],
        [true, true, false, true, true, false, true],
        [true, true, true, true, false, false, true],
        [false, true, true, false, false, true, true],
        [true, false, true, true, false, true, true],
        [true, false, true, true, true, true, true],
        [true, true, true, false, false, false, false],
        [true, true, true, true, true, true, true],
        [true, true, true, true, false, true, true],
    ];
    let t = (w * 0.22).max(2.0);
    let g = 0.8;
    let mid = y + h / 2.0;
    let bottom = y + h;
    let skew = |q: Pos2| pos2(q.x + (bottom - q.y) * 0.14, q.y);
    let segs: [(f32, f32, f32, f32); 7] = [
        (x + t + g, y, x + w - t - g, y + t),                       // a
        (x + w - t, y + t + g, x + w, mid - g),                     // b
        (x + w - t, mid + g, x + w, bottom - t - g),                // c
        (x + t + g, bottom - t, x + w - t - g, bottom),             // d
        (x, mid + g, x + t, bottom - t - g),                        // e
        (x, y + t + g, x + t, mid - g),                             // f
        (x + t + g, mid - t / 2.0, x + w - t - g, mid + t / 2.0),   // g
    ];
    for (i, (x1, y1, x2, y2)) in segs.iter().enumerate() {
        let on = digit.map(|d| MAP[d as usize % 10][i]).unwrap_or(false);
        let pts = vec![skew(pos2(*x1, *y1)), skew(pos2(*x2, *y1)), skew(pos2(*x2, *y2)), skew(pos2(*x1, *y2))];
        let fill = if on { col } else { a(col, 20) };
        p.add(PathShape::convex_polygon(pts, fill, Stroke::NONE));
    }
}

fn corvette(p: &Painter, b: &Band, th: &ThemePalette) {
    let r = b.rect;
    let h = r.height();
    let pct = b.pct.clamp(0.0, 100.0);

    // Recessed LCD window
    let lcd = Rect::from_min_max(pos2(r.left() + 3.0, r.top() + 3.0), pos2(r.right() - 3.0, r.bottom() - 3.0));
    p.rect_filled(lcd, 2.0, a(th.card_bg, 255));
    p.rect_stroke(lcd, 2.0, Stroke::new(1.0, a(th.primary, 50)), egui::StrokeKind::Inside);

    let dh = (h - 14.0).clamp(12.0, 38.0);
    let dw = dh * 0.52;
    let v = pct.round() as u32;
    let digs = [
        if v >= 100 { Some(v / 100 % 10) } else { None },
        if v >= 10 { Some(v / 10 % 10) } else { None },
        Some(v % 10),
    ];
    let x0 = lcd.left() + 8.0;
    let y0 = lcd.center().y - dh / 2.0;
    for (i, d) in digs.iter().enumerate() {
        seven_seg(p, x0 + i as f32 * (dw + 5.0), y0, dw, dh, *d, th.primary);
    }
    let after = x0 + 3.0 * (dw + 5.0) + 2.0;
    p.text(pos2(after, y0 + dh), Align2::LEFT_BOTTOM, "%", FontId::monospace(9.0), th.text_dim);

    // Ascending wedge bar-graph
    let bx0 = after + 22.0;
    let avail = lcd.right() - 8.0 - bx0;
    if avail < 30.0 {
        return;
    }
    let n = ((avail / 6.0) as usize).clamp(8, 44);
    let step = avail / n as f32;
    let max_h = lcd.height() - 6.0;
    let peak_i = ((b.peak / 100.0) * n as f32).round() as usize;
    for i in 0..n {
        let t = i as f32 / (n - 1).max(1) as f32;
        let bh = max_h * (0.22 + 0.78 * t);
        let col = if t < 0.6 { th.tertiary } else if t < 0.85 { th.warning } else { th.redline };
        let x = bx0 + i as f32 * step;
        let bar = Rect::from_min_max(pos2(x, lcd.bottom() - 3.0 - bh), pos2(x + step - 1.8, lcd.bottom() - 3.0));
        let on = t <= pct / 100.0 + 0.0001 && pct > 0.5;
        p.rect_filled(bar, 0.5, if on { col } else { a(col, 24) });
        if i == peak_i.min(n - 1) && peak_i > 0 {
            p.rect_stroke(bar, 0.5, Stroke::new(1.2, Color32::WHITE), egui::StrokeKind::Inside);
        }
    }
    p.text(
        pos2(bx0, lcd.top() + 3.0),
        Align2::LEFT_TOP,
        format!("{}  {}", b.label, b.sub),
        FontId::monospace(8.5),
        th.text_dim,
    );
}

// ---------------------------------------------------------------- Nissan 300ZX Z31 Digital Dash

fn nissan_300zx(p: &Painter, b: &Band, th: &ThemePalette) {
    let r = b.rect;
    let (w, _h) = (r.width(), r.height());
    let pct = b.pct.clamp(0.0, 100.0);

    // Deep cockpit VFD bezel with dual bezel borders
    let bezel = Rect::from_min_max(pos2(r.left() + 2.0, r.top() + 1.0), pos2(r.right() - 2.0, r.bottom() - 1.0));
    p.rect_filled(bezel, 2.0, th.graph_bg);
    p.rect_stroke(bezel, 2.0, Stroke::new(1.0, a(th.border, 180)), egui::StrokeKind::Inside);
    p.rect_stroke(bezel.shrink(1.5), 1.0, Stroke::new(0.6, a(th.grid, 220)), egui::StrokeKind::Inside);

    // Top status strip: HUD digital readout & boxed speedometer style
    let top_h = 13.0;
    let label_rect = Rect::from_min_max(pos2(bezel.left() + 4.0, bezel.top() + 2.0), pos2(bezel.right() - 4.0, bezel.top() + top_h));

    // Top left: Metric Label (e.g. "CPU 3.8 GHz" or "RX/TX")
    p.text(
        pos2(label_rect.left() + 2.0, label_rect.center().y),
        Align2::LEFT_CENTER,
        format!("• {}", b.label.to_uppercase()),
        FontId::monospace(8.5),
        th.secondary,
    );

    // Top center/right: Digital speedo readout box "[ 88 % ]"
    let val_str = format!("{:>3.0}%", pct);
    let box_w = 44.0;
    let box_x = if w > 190.0 {
        label_rect.right() - box_w - 75.0
    } else {
        label_rect.right() - box_w
    };
    let val_box = Rect::from_min_size(pos2(box_x, label_rect.top() - 0.5), Vec2::new(box_w, 12.0));
    p.rect_filled(val_box, 1.0, a(th.card_bg, 230));
    p.rect_stroke(val_box, 1.0, Stroke::new(0.8, a(th.primary, 130)), egui::StrokeKind::Inside);
    p.text(
        val_box.center(),
        Align2::CENTER_CENTER,
        val_str,
        FontId::monospace(9.5),
        th.text_bright,
    );

    // If space allows, top right subtext (like "PEAK 94%" or rate)
    if w > 190.0 {
        p.text(
            pos2(label_rect.right() - 2.0, label_rect.center().y),
            Align2::RIGHT_CENTER,
            &b.sub,
            FontId::monospace(8.0),
            th.text_dim,
        );
    }

    // Main display area below top bar
    let main_area = Rect::from_min_max(
        pos2(bezel.left() + 4.0, bezel.top() + top_h + 1.0),
        pos2(bezel.right() - 4.0, bezel.bottom() - 3.0),
    );
    if main_area.height() < 12.0 {
        return;
    }

    // Right-hand twin vertical segmented ladders (Z31 Oil / Temp / Fuel / Battery clusters)
    let right_gauge_w = if w > 150.0 { 18.0 } else { 0.0 };
    let tach_w = main_area.width() - right_gauge_w - (if right_gauge_w > 0.0 { 6.0 } else { 0.0 });

    if right_gauge_w > 0.0 {
        let gx0 = main_area.right() - right_gauge_w;
        let ladder_h = main_area.height();
        let num_ladder_segs = ((ladder_h / 4.0) as usize).clamp(4, 12);
        let seg_step = ladder_h / num_ladder_segs as f32;
        let seg_w = 6.0;

        // Left ladder (b.pct), right ladder (b.peak)
        let lit_l = ((pct / 100.0) * num_ladder_segs as f32).round() as usize;
        let lit_r = ((b.peak.clamp(0.0, 100.0) / 100.0) * num_ladder_segs as f32).round() as usize;

        for s in 0..num_ladder_segs {
            // Drawn from bottom up
            let sy = main_area.bottom() - (s as f32 + 1.0) * seg_step;
            let on_l = s < lit_l;
            let on_r = s < lit_r;

            let col_seg = if s >= (num_ladder_segs * 8 / 10) {
                th.redline
            } else if s >= (num_ladder_segs * 6 / 10) {
                th.warning
            } else {
                th.primary
            };

            // Ladder 1
            let seg_rect1 = Rect::from_min_size(pos2(gx0, sy + 0.8), Vec2::new(seg_w, seg_step - 1.6));
            p.rect_filled(seg_rect1, 0.5, if on_l { col_seg } else { a(col_seg, 24) });

            // Ladder 2
            let seg_rect2 = Rect::from_min_size(pos2(gx0 + seg_w + 3.0, sy + 0.8), Vec2::new(seg_w, seg_step - 1.6));
            p.rect_filled(seg_rect2, 0.5, if on_r { col_seg } else { a(col_seg, 24) });
        }
    }

    // Left area: Iconic Nissan 300ZX Swept Curved Tachometer
    // Power curve profile: y(x) = base_y - max_h * (x / tach_w)^1.75
    let tach_rect = Rect::from_min_size(main_area.min, Vec2::new(tach_w, main_area.height()));
    let base_y = tach_rect.bottom();
    let max_tach_h = tach_rect.height() - 4.0;
    let n_bars = ((tach_w / 4.8) as usize).clamp(14, 60);
    let bar_step = tach_w / n_bars as f32;
    let bar_w = (bar_step - 1.5).max(1.8);

    // Draw background grid ticks along the baseline
    p.line_segment(
        [pos2(tach_rect.left(), base_y), pos2(tach_rect.right(), base_y)],
        Stroke::new(1.0, a(th.secondary, 70)),
    );

    // Compute lit bars & peak bar
    let lit_count = ((pct / 100.0) * n_bars as f32).round() as usize;
    let peak_idx = ((b.peak.clamp(0.0, 100.0) / 100.0) * n_bars as f32).round() as usize;

    // Power curve outline points for top boundary
    let mut curve_pts = Vec::with_capacity(n_bars + 1);
    for i in 0..=n_bars {
        let frac = i as f32 / n_bars as f32;
        let curve_h = max_tach_h * (0.16 + 0.84 * frac.powf(1.75));
        let cx = tach_rect.left() + frac * tach_w;
        let cy = base_y - curve_h;
        curve_pts.push(pos2(cx, cy));
    }
    // Subtle phosphor envelope guide line
    p.add(PathShape::line(curve_pts, Stroke::new(1.0, a(th.secondary, 50))));

    // Draw vertical segmented phosphor bars under the curve
    for i in 0..n_bars {
        let frac = (i as f32 + 0.5) / n_bars as f32;
        let curve_h = max_tach_h * (0.16 + 0.84 * frac.powf(1.75));
        let bx = tach_rect.left() + i as f32 * bar_step;

        let col = if frac > 0.84 {
            th.redline
        } else if frac > 0.65 {
            th.warning
        } else {
            th.primary
        };

        let on = i < lit_count && pct > 0.5;

        // Vertical tick segment
        let bar = Rect::from_min_max(pos2(bx, base_y - curve_h), pos2(bx + bar_w, base_y - 1.0));
        if on {
            // Phosphor glow + solid bar
            p.rect_filled(bar.expand(0.5), 0.5, a(col, 40));
            p.rect_filled(bar, 0.5, col);
            // Highlight topmost pixel of active bar
            let cap = Rect::from_min_max(pos2(bx, base_y - curve_h), pos2(bx + bar_w, base_y - curve_h + 1.8));
            p.rect_filled(cap, 0.5, th.tertiary);
        } else {
            // Unlit phosphor matrix ghost
            p.rect_filled(bar, 0.5, a(col, 20));
        }

        // Peak hold marker
        if i == peak_idx.min(n_bars - 1) && peak_idx > 0 {
            let peak_cap = Rect::from_min_max(
                pos2(bx - 0.4, base_y - curve_h - 1.5),
                pos2(bx + bar_w + 0.4, base_y - curve_h + 2.5),
            );
            p.rect_filled(peak_cap, 0.5, Color32::WHITE);
        }
    }

    // Baseline RPM / scale ticks (0 10 20 40 60 80 100)
    if tach_rect.height() > 32.0 {
        let tick_indices = [0.0, 0.25, 0.5, 0.75, 1.0];
        for tf in tick_indices {
            let tx = tach_rect.left() + tf * (tach_w - 2.0);
            p.line_segment(
                [pos2(tx, base_y), pos2(tx, base_y - 2.5)],
                Stroke::new(1.0, a(th.secondary, 140)),
            );
        }
        p.text(
            pos2(tach_rect.left() + 2.0, base_y - 1.5),
            Align2::LEFT_BOTTOM,
            "TACH x1000r/min",
            FontId::monospace(6.5),
            a(th.text_dim, 180),
        );
    }
}

// ---------------------------------------------------------------- VFD dot matrix history

fn vfd(p: &Painter, b: &Band, th: &ThemePalette) {
    let r = b.rect;
    let h = r.height();
    let compact = h < 44.0;
    let top_pad = if compact { 2.0 } else { 13.0 };

    if !compact {
        p.text(pos2(r.left() + 6.0, r.top() + 3.0), Align2::LEFT_TOP, &b.label, FontId::monospace(8.5), th.text_dim);
        p.text(
            pos2(r.right() - 6.0, r.top() + 2.0),
            Align2::RIGHT_TOP,
            format!("[{:>3.0}%] {}", b.pct, b.sub),
            FontId::monospace(10.0),
            th.primary,
        );
    } else {
        p.text(
            pos2(r.left() + 6.0, r.top() + 2.0),
            Align2::LEFT_TOP,
            format!("{} {:.0}%", b.label, b.pct),
            FontId::monospace(8.0),
            a(th.primary, 230),
        );
    }

    let area = Rect::from_min_max(
        pos2(r.left() + 6.0, r.top() + top_pad + if compact { 8.0 } else { 0.0 }),
        pos2(r.right() - 6.0, r.bottom() - 3.0),
    );
    let pitch = 4.5;
    let rows = (area.height() / pitch).floor().max(2.0) as usize;
    let cols = (area.width() / pitch).floor().max(4.0) as usize;
    let samples = resample(&b.history, cols);
    for (c, v) in samples.iter().enumerate() {
        let lit_rows = ((v / 100.0) * rows as f32).round() as usize;
        let x = area.left() + (c as f32 + 0.5) * pitch;
        for rr in 0..rows {
            let y = area.bottom() - (rr as f32 + 0.5) * pitch;
            let pos = pos2(x, y);
            if rr < lit_rows {
                let col = lerp_c(th.primary, th.tertiary, rr as f32 / rows as f32);
                p.circle_filled(pos, 2.1, a(col, 40));
                p.circle_filled(pos, 1.15, col);
            } else {
                p.circle_filled(pos, 0.6, a(th.primary, 38));
            }
        }
    }
}

// ---------------------------------------------------------------- Matrix rain

const GLYPHS: &[u8] = b"0123456789ZXMKRW:=*+<>|/\\#@$%&";

#[allow(clippy::too_many_arguments)]
fn rain(
    p: &Painter,
    rect: Rect,
    th: &ThemePalette,
    t: f32,
    load: f32,
    min_active: f32,
    seed: u32,
    cell_h: f32,
    trail: usize,
    max_alpha: f32,
    col: Color32,
    head_bright: bool,
) {
    let cell_w = cell_h * 0.8;
    let cols = (rect.width() / cell_w).floor().max(1.0) as usize;
    let rows = (rect.height() / cell_h).ceil().max(1.0) as f32;
    let font = FontId::monospace(cell_h * 0.88);
    for c in 0..cols {
        let cs = seed.wrapping_mul(7919).wrapping_add(c as u32 * 104_729);
        let rank = hashf(cs);
        let active = rank < (min_active + (1.0 - min_active) * load.clamp(0.0, 1.0));
        let cps = 2.5 + hashf(cs ^ 0x55) * 4.5 + load * 7.0;
        let period = rows + trail as f32 + hashf(cs ^ 0xaa) * 8.0;
        let head = (t * cps + hashf(cs ^ 0x33) * period) % period;
        let x = rect.left() + c as f32 * cell_w + cell_w * 0.5;
        for k in 0..trail {
            let row = head.floor() as i32 - k as i32;
            if row < 0 || row as f32 >= rows {
                continue;
            }
            let fade = 1.0 - k as f32 / trail as f32;
            let mut alpha = max_alpha * fade * fade;
            if !active {
                alpha *= 0.22;
            }
            if alpha < 0.02 {
                continue;
            }
            let churn = if k == 0 { (t * 14.0) as u32 } else { (t * 2.5) as u32 };
            let g = GLYPHS[(hash(cs ^ (row as u32 * 31) ^ churn.wrapping_mul(2_654_435_761)) as usize) % GLYPHS.len()] as char;
            let c_use = if k == 0 && head_bright { lerp_c(col, Color32::WHITE, 0.65) } else { col };
            let aa = if k == 0 && head_bright { (alpha.min(1.0) * 255.0) as u8 } else { (alpha.min(1.0) * 255.0) as u8 };
            p.text(
                pos2(x, rect.top() + row as f32 * cell_h),
                Align2::CENTER_TOP,
                g,
                font.clone(),
                a(c_use, aa),
            );
        }
    }
    let _ = th;
}

fn matrix(p: &Painter, b: &Band, th: &ThemePalette) {
    let r = b.rect;
    let (w, h) = (r.width(), r.height());
    let pct = b.pct.clamp(0.0, 100.0);
    let load = pct / 100.0;
    let compact = h < 42.0;

    // Outer terminal matrix frame
    let frame = Rect::from_min_max(pos2(r.left() + 2.0, r.top() + 1.0), pos2(r.right() - 2.0, r.bottom() - 1.0));
    p.rect_filled(frame, 2.0, th.graph_bg);
    p.rect_stroke(frame, 2.0, Stroke::new(1.0, a(th.border, 160)), egui::StrokeKind::Inside);

    // Left Operator HUD: Metric designation & live data percentage
    let hud_w = if compact {
        (w * 0.32).clamp(70.0, 100.0)
    } else {
        (w * 0.34).clamp(80.0, 125.0)
    };
    let hud_rect = Rect::from_min_size(pos2(frame.left() + 3.0, frame.top() + 2.0), Vec2::new(hud_w, frame.height() - 4.0));
    
    // Subtle cyber terminal divider line
    p.line_segment(
        [pos2(hud_rect.right() + 2.0, frame.top() + 3.0), pos2(hud_rect.right() + 2.0, frame.bottom() - 3.0)],
        Stroke::new(1.0, a(th.primary, 75)),
    );

    // Operator Prompt: "> LABEL"
    let pfx = format!("> {}", b.label.to_uppercase());
    p.text(
        pos2(hud_rect.left() + 2.0, hud_rect.top() + 2.0),
        Align2::LEFT_TOP,
        pfx,
        FontId::monospace(8.5),
        th.text_dim,
    );

    // Large phosphor data value e.g. "88%" or "[88%]"
    let val_str = format!("{:>3.0}%", pct);
    let big_sz = if compact { 13.0 } else { 16.0 };
    glow_text(
        p,
        pos2(hud_rect.left() + 2.0, hud_rect.top() + 13.0),
        Align2::LEFT_TOP,
        &val_str,
        big_sz,
        th.text_bright,
    );

    // Live Hex address / rate display under the big percent
    if !compact && hud_rect.height() > 34.0 {
        let hex_val = ((load * 65535.0) as u16) ^ (b.seed as u16);
        let sub_display = if !b.sub.is_empty() {
            b.sub.clone()
        } else {
            format!("0x{:04X}", hex_val)
        };
        p.text(
            pos2(hud_rect.left() + 2.0, hud_rect.bottom() - 2.0),
            Align2::LEFT_BOTTOM,
            sub_display,
            FontId::monospace(7.5),
            a(th.primary, 190),
        );
    }

    // Right Matrix Data Grid: Each column represents a sampled point in time!
    let data_rect = Rect::from_min_max(
        pos2(hud_rect.right() + 5.0, frame.top() + 2.0),
        pos2(frame.right() - 4.0, frame.bottom() - 2.0),
    );

    if data_rect.width() < 24.0 {
        return;
    }

    let cell_h = if compact { 8.5 } else { 9.5 };
    let cell_w = cell_h * 0.78;
    let n_cols = ((data_rect.width() / cell_w).floor() as usize).max(4);
    let actual_w = n_cols as f32 * cell_w;
    let x_offset = data_rect.left() + (data_rect.width() - actual_w) * 0.5;
    let n_rows = ((data_rect.height() / cell_h).floor() as usize).max(3);

    // Resample history so newest metrics are on the rightmost columns
    let history_samples = resample(&b.history, n_cols);

    // Draw baseline threshold line
    p.line_segment(
        [pos2(x_offset, data_rect.bottom()), pos2(x_offset + actual_w, data_rect.bottom())],
        Stroke::new(1.0, a(th.grid, 180)),
    );

    for col in 0..n_cols {
        // Col metric load (0.0 .. 1.0)
        let col_pct = if col == n_cols - 1 {
            pct
        } else {
            history_samples[col].clamp(0.0, 100.0)
        };
        let col_load = col_pct / 100.0;

        // How many rows of code rain represent this metric height (bottom to top)
        let lit_rows = ((col_load * n_rows as f32).ceil() as usize).min(n_rows);

        let cx = x_offset + col as f32 * cell_w + cell_w * 0.5;
        let col_seed = b.seed.wrapping_add((col as u32).wrapping_mul(1337));

        // Speed of matrix glyph cycling is proportional to that column's data load!
        let speed = 2.0 + col_load * 12.0;

        for r_idx in 0..n_rows {
            // r_idx 0 is top, n_rows - 1 is bottom
            let dist_from_bottom = n_rows - 1 - r_idx;
            let cy = data_rect.top() + r_idx as f32 * cell_h;

            let is_data_active = dist_from_bottom < lit_rows;
            let is_lead_glyph = dist_from_bottom == lit_rows.saturating_sub(1) && lit_rows > 0;

            // Churn calculation
            let churn = ((b.t * speed) as u32).wrapping_add(r_idx as u32 * 17);
            let glyph_idx = (hash(col_seed ^ churn ^ (r_idx as u32 * 31)) as usize) % GLYPHS.len();
            let ch = GLYPHS[glyph_idx] as char;

            let font = FontId::monospace(cell_h * 0.92);

            if is_lead_glyph {
                // Bright white-green falling head of the data stream
                p.text(
                    pos2(cx, cy),
                    Align2::CENTER_TOP,
                    ch,
                    font,
                    Color32::from_rgb(220, 255, 230),
                );
            } else if is_data_active {
                // Solid green data stream body with gradient fade towards baseline
                let fade = 0.55 + 0.45 * (dist_from_bottom as f32 / lit_rows.max(1) as f32);
                let col_use = if col_pct > 88.0 {
                    th.redline
                } else if col_pct > 70.0 {
                    th.warning
                } else {
                    th.primary
                };
                let alpha = (fade * 240.0) as u8;
                p.text(
                    pos2(cx, cy),
                    Align2::CENTER_TOP,
                    ch,
                    font,
                    a(col_use, alpha),
                );
            } else {
                // Ghost matrix code: faint inactive background glyph
                p.text(
                    pos2(cx, cy),
                    Align2::CENTER_TOP,
                    ch,
                    font,
                    a(th.primary, 22),
                );
            }
        }

        // Peak marker on top of column if matching peak
        let peak_row = ((b.peak.clamp(0.0, 100.0) / 100.0 * n_rows as f32).round() as usize).min(n_rows);
        if col == n_cols - 1 && peak_row > 0 {
            let py = data_rect.top() + (n_rows - peak_row) as f32 * cell_h;
            p.line_segment(
                [pos2(cx - cell_w * 0.45, py), pos2(cx + cell_w * 0.45, py)],
                Stroke::new(1.2, Color32::WHITE),
            );
        }
    }
}

// ---------------------------------------------------------------- Hackers (Gibson skyline)

fn hackers(p: &Painter, b: &Band, th: &ThemePalette) {
    let r = b.rect;
    let (w, h) = (r.width(), r.height());
    let horizon = r.bottom() - h * 0.26;
    let vp = pos2(r.center().x, horizon - 1.0);

    // perspective floor
    for i in -7..=7 {
        let bx = r.center().x + i as f32 * (w / 11.0);
        p.line_segment([pos2(bx, r.bottom()), vp], Stroke::new(1.0, a(th.tertiary, 55)));
    }
    let scroll = (b.t * 0.5) % 1.0;
    for k in 0..5 {
        let f = ((k as f32 + scroll) / 5.0).powi(2);
        let y = horizon + (r.bottom() - horizon) * f;
        p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], Stroke::new(1.0, a(th.tertiary, 70)));
    }
    p.line_segment([pos2(r.left(), horizon), pos2(r.right(), horizon)], Stroke::new(1.0, a(th.secondary, 160)));

    // wireframe towers = history
    let n = (((w - 16.0) / 13.0) as usize).clamp(8, 36);
    let s = resample(&b.history, n);
    let slot = (w - 16.0) / n as f32;
    let tw = slot * 0.62;
    let max_h = (horizon - r.top() - 6.0).max(4.0);
    let cols = [th.primary, th.secondary, th.tertiary];
    for (i, v) in s.iter().enumerate() {
        let col = cols[i % 3];
        let hh = (v / 100.0 * max_h).max(2.0);
        let x = r.left() + 8.0 + i as f32 * slot;
        let (dx, dy) = (3.0, -3.0);
        let base = horizon;
        let top = base - hh;
        let front = Rect::from_min_max(pos2(x, top), pos2(x + tw, base));
        p.rect_filled(front, 0.0, a(col, 22));
        p.rect_stroke(front, 0.0, Stroke::new(1.0, col), egui::StrokeKind::Inside);
        // roof + side
        p.add(Shape::line(
            vec![pos2(x, top), pos2(x + dx, top + dy), pos2(x + tw + dx, top + dy), pos2(x + tw, top)],
            Stroke::new(1.0, col),
        ));
        p.line_segment([pos2(x + tw + dx, top + dy), pos2(x + tw + dx, base + dy)], Stroke::new(1.0, a(col, 150)));
        p.line_segment([pos2(x + tw, base), pos2(x + tw + dx, base + dy)], Stroke::new(1.0, a(col, 150)));
    }

    // terminal prompt
    let cursor = if (b.t * 2.0) as i32 % 2 == 0 { "_" } else { " " };
    let txt = format!("> {} {:.0}% {}{}", b.label, b.pct, b.sub, cursor);
    let font = FontId::monospace(9.0);
    let galley = p.layout_no_wrap(txt.clone(), font.clone(), th.primary);
    let plate = Rect::from_min_size(pos2(r.left() + 4.0, r.top() + 3.0), galley.size() + Vec2::new(8.0, 2.0));
    p.rect_filled(plate, 1.0, a(th.graph_bg, 210));
    p.text(pos2(plate.left() + 4.0, plate.top() + 1.0), Align2::LEFT_TOP, txt, font, th.primary);
}

// ---------------------------------------------------------------- Synthwave sun

fn synthwave(p: &Painter, b: &Band, th: &ThemePalette) {
    let r = b.rect;
    let (w, h) = (r.width(), r.height());
    let f = (b.pct / 100.0).clamp(0.0, 1.0);
    let horizon = r.top() + h * 0.64;

    // sun rises with load
    let sun_r = (h * 0.52).max(10.0);
    let cx = r.left() + w * 0.28;
    let cy = horizon + sun_r * 0.55 - f * sun_r * 1.25;
    let step = 2.0;
    let mut y = (cy - sun_r).max(r.top());
    while y < (cy + sun_r).min(horizon) {
        let dy = y - cy;
        let hw = (sun_r * sun_r - dy * dy).max(0.0).sqrt();
        let rel = ((y - (cy - sun_r)) / (2.0 * sun_r)).clamp(0.0, 1.0);
        let period = 7.0;
        let phase = ((y - r.top()) % period) / period;
        let gap = if rel > 0.42 { (rel - 0.42) * 1.5 } else { 0.0 };
        if phase >= gap {
            p.rect_filled(
                Rect::from_min_max(pos2(cx - hw, y), pos2(cx + hw, y + step)),
                0.0,
                lerp_c(th.warning, th.primary, rel),
            );
        }
        y += step;
    }

    // floor
    let floor = Rect::from_min_max(pos2(r.left(), horizon), r.right_bottom());
    p.rect_filled(floor, 0.0, a(th.tertiary, 28));
    p.line_segment([pos2(r.left(), horizon), pos2(r.right(), horizon)], Stroke::new(1.5, th.secondary));
    let vx = r.center().x;
    for i in -8..=8 {
        let bx = vx + i as f32 * (w / 7.0);
        p.line_segment([pos2(bx, r.bottom()), pos2(vx + i as f32 * 3.0, horizon)], Stroke::new(1.0, a(th.secondary, 110)));
    }
    let scroll = (b.t * 0.8) % 1.0;
    for k in 0..5 {
        let q = ((k as f32 + scroll) / 5.0).powi(2);
        let yy = horizon + (r.bottom() - horizon) * q;
        p.line_segment([pos2(r.left(), yy), pos2(r.right(), yy)], Stroke::new(1.0, a(th.secondary, 120)));
    }

    // readout
    p.text(pos2(r.right() - 7.0, r.top() + 3.0), Align2::RIGHT_TOP, &b.label, FontId::monospace(9.0), a(th.secondary, 230));
    let big = (h * 0.34).clamp(11.0, 22.0);
    p.text(
        pos2(r.right() - 7.0 + 1.0, r.top() + 14.0 + 1.0),
        Align2::RIGHT_TOP,
        format!("{:.0}%", b.pct),
        FontId::monospace(big),
        a(th.tertiary, 200),
    );
    p.text(
        pos2(r.right() - 7.0, r.top() + 14.0),
        Align2::RIGHT_TOP,
        format!("{:.0}%", b.pct),
        FontId::monospace(big),
        th.text_bright,
    );
    if h > 50.0 {
        p.text(pos2(r.right() - 7.0, r.bottom() - 3.0), Align2::RIGHT_BOTTOM, &b.sub, FontId::monospace(8.0), th.text_dim);
    }
}

// ---------------------------------------------------------------- AI Model Quota: Automotive Fuel Gauge

pub fn draw_fuel_gauge(
    p: &Painter,
    rect: Rect,
    quota: &crate::quota::ModelQuotaData,
    th: &ThemePalette,
    t: f32,
    style: &str,
) {
    let (w, h) = (rect.width(), rect.height());
    if w < 60.0 || h < 20.0 {
        return;
    }

    // Top Header: "AI FUEL TANK" on left, Claude/GPT 3P summary on right
    let header_y = rect.top() + 1.0;
    let any_low = quota.gemini_5h_pct < 15.0 || quota.gemini_weekly_pct < 15.0;
    let pump_col = if any_low {
        if ((t * 4.0) as i32) % 2 == 0 { th.redline } else { th.warning }
    } else {
        th.primary
    };

    let title = if !quota.is_connected {
        "AI FUEL [CONNECTING...]"
    } else if any_low {
        "AI FUEL [LOW RESERVE]"
    } else {
        "AI FUEL TANK"
    };
    p.text(
        pos2(rect.left() + 4.0, header_y),
        Align2::LEFT_TOP,
        title,
        FontId::monospace(9.5),
        pump_col,
    );

    let sub_3p = format!("3P: 5H {:.0}% · Wk {:.0}%", quota.claude_5h_pct, quota.claude_weekly_pct);
    p.text(
        pos2(rect.right() - 4.0, header_y),
        Align2::RIGHT_TOP,
        sub_3p,
        FontId::monospace(8.5),
        th.text_dim,
    );

    let body_top = header_y + 13.0;
    let body_rect = Rect::from_min_max(pos2(rect.left() + 2.0, body_top), pos2(rect.right() - 2.0, rect.bottom() - 2.0));
    if body_rect.height() < 12.0 {
        return;
    }

    let gap = 6.0;
    let tank_w = (body_rect.width() - gap) * 0.5;
    let left_tank = Rect::from_min_size(body_rect.min, Vec2::new(tank_w, body_rect.height()));
    let right_tank = Rect::from_min_size(pos2(body_rect.left() + tank_w + gap, body_rect.top()), Vec2::new(tank_w, body_rect.height()));

    match style {
        "twin_dial" => {
            draw_ferrari_fuel_dial(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
            draw_ferrari_fuel_dial(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
        }
        "digital_bar" => {
            draw_corvette_fuel_wedge(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
            draw_corvette_fuel_wedge(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
        }
        "vertical_tubes" => {
            draw_vertical_fuel_cell(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
            draw_vertical_fuel_cell(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
        }
        "arc_rings" => {
            draw_ring_fuel_gauge(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
            draw_ring_fuel_gauge(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
        }
        "hud_telemetry" => {
            draw_hud_fuel_gauge(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
            draw_hud_fuel_gauge(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
        }
        "block" => {
            draw_nissan_fuel_ladder(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
            draw_nissan_fuel_ladder(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
        }
        "line" => {
            draw_classic_fuel_gauge(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
            draw_classic_fuel_gauge(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
        }
        _ => {
            // "dash" / Theme-Matched Auto
            match th.id.as_str() {
                "ferrari_dash" => {
                    draw_ferrari_fuel_dial(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
                    draw_ferrari_fuel_dial(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
                }
                "corvette_red" => {
                    draw_corvette_fuel_wedge(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
                    draw_corvette_fuel_wedge(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
                }
                "nissan_300zx" => {
                    draw_nissan_fuel_ladder(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
                    draw_nissan_fuel_ladder(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
                }
                _ => {
                    draw_classic_fuel_gauge(p, left_tank, "5H SPRINT", quota.gemini_5h_pct, &quota.gemini_5h_reset, th, t);
                    draw_classic_fuel_gauge(p, right_tank, "WEEKLY MAIN", quota.gemini_weekly_pct, &quota.gemini_weekly_reset, th, t);
                }
            }
        }
    }
}

/// Ferrari Dash / Twin Analog Dial: Semicircular sweeping needle fuel dial with E .. F scale & left-side red reserve zone
fn draw_ferrari_fuel_dial(
    p: &Painter,
    r: Rect,
    label: &str,
    pct: f32,
    reset_str: &str,
    th: &ThemePalette,
    t: f32,
) {
    let (w, h) = (r.width(), r.height());
    let low_fuel = pct < 15.0;
    let rad = (h - 6.0).min(w * 0.34).clamp(10.0, 30.0);
    let cx = r.left() + rad + 8.0;
    let cy = r.bottom() - 2.0;

    let ang = |f: f32| std::f32::consts::PI * (1.0 - f.clamp(0.0, 1.0));
    let pt = |f: f32, rr: f32| {
        let a = ang(f);
        pos2(cx + rr * a.cos(), cy - rr * a.sin())
    };
    let arc_pts = |from: f32, to: f32, rr: f32| -> Vec<Pos2> {
        let steps = 20;
        (0..=steps).map(|k| pt(from + (to - from) * (k as f32 / steps as f32), rr)).collect()
    };

    // Track arcs: LEFT 0..0.18 is red reserve zone, 0.18..1.0 is normal secondary track
    p.add(PathShape::line(arc_pts(0.0, 0.18, rad), Stroke::new(2.8, a(th.redline, 110))));
    p.add(PathShape::line(arc_pts(0.18, 1.0, rad), Stroke::new(2.5, a(th.secondary, 45))));

    // Active sweep arc (healthy primary when full, warning when <35%, pulsing red when <15%)
    if pct > 0.5 {
        let sweep_f = (pct / 100.0).clamp(0.0, 1.0);
        let active_col = if low_fuel {
            if ((t * 4.0) as i32) % 2 == 0 { th.redline } else { th.warning }
        } else if pct < 35.0 {
            th.warning
        } else {
            th.primary
        };
        p.add(PathShape::line(arc_pts(0.0, sweep_f, rad), Stroke::new(2.8, active_col)));
    }

    // Tick marks: E (0.0), 1/4 (0.25), 1/2 (0.50), 3/4 (0.75), F (1.0)
    for k in 0..=4 {
        let f = k as f32 / 4.0;
        let inner = if k == 0 || k == 2 || k == 4 { rad - 5.0 } else { rad - 3.0 };
        let tcol = if k == 0 { th.redline } else { th.text_dim };
        p.line_segment([pt(f, inner), pt(f, rad)], Stroke::new(1.0, a(tcol, 190)));
    }

    // E and F letters
    p.text(pt(0.0, rad + 4.5), Align2::RIGHT_BOTTOM, "E", FontId::monospace(8.0), if low_fuel { th.redline } else { th.text_dim });
    p.text(pt(1.0, rad + 4.5), Align2::LEFT_BOTTOM, "F", FontId::monospace(8.0), th.text_dim);

    // Needle + Pivot
    let needle_val = (pct / 100.0).clamp(0.0, 1.0);
    p.line_segment([pt(needle_val, 2.0), pt(needle_val, rad - 2.5)], Stroke::new(1.8, th.tertiary));
    p.line_segment([pt(needle_val, rad - 5.0), pt(needle_val, rad - 1.5)], Stroke::new(1.8, Color32::WHITE));
    p.circle_filled(pos2(cx, cy), 2.5, th.tertiary);

    // Low Fuel Warning Lamp (pulsing LED ONLY when low)
    if low_fuel {
        let flash = ((t * 4.0) as i32) % 2 == 0;
        let lamp_col = if flash { th.redline } else { th.warning };
        let lamp_pos = pos2(cx - rad * 0.42, cy - rad * 0.58);
        p.circle_filled(lamp_pos, 4.0, a(lamp_col, 60));
        p.circle_filled(lamp_pos, 2.2, lamp_col);
    }

    // Right side: Label, Bold Percentage, Reset countdown
    let text_x = r.right() - 2.0;
    p.text(pos2(text_x, r.top() + 1.0), Align2::RIGHT_TOP, label, FontId::monospace(8.5), th.text_dim);

    let num_size = (h * 0.36).clamp(12.0, 18.0);
    let num_col = if low_fuel { th.redline } else { th.text_bright };
    p.text(
        pos2(text_x, r.center().y - 1.0),
        Align2::RIGHT_CENTER,
        format!("{:.0}%", pct),
        FontId::monospace(num_size),
        num_col,
    );

    p.text(pos2(text_x, r.bottom() - 1.0), Align2::RIGHT_BOTTOM, reset_str, FontId::monospace(8.5), th.secondary);
}

/// Twin Vertical Fuel Cells / Sight Glass: liquid fill chamber with meniscus, graduation ticks, and low-fuel glow
fn draw_vertical_fuel_cell(
    p: &Painter,
    r: Rect,
    label: &str,
    pct: f32,
    reset_str: &str,
    th: &ThemePalette,
    t: f32,
) {
    let low_fuel = pct < 15.0;
    let h = r.height();

    // Vertical glass tube on the left side of this tank half
    let tube_w = (r.width() * 0.24).clamp(16.0, 26.0);
    let tube = Rect::from_min_max(
        pos2(r.left() + 12.0, r.top() + 2.0),
        pos2(r.left() + 12.0 + tube_w, r.bottom() - 2.0),
    );

    // Glass tube housing
    p.rect_filled(tube, 3.0, a(th.graph_bg, 220));
    let border_col = if low_fuel && ((t * 4.0) as i32) % 2 == 0 { th.redline } else { th.border };
    p.rect_stroke(tube, 3.0, Stroke::new(1.1, border_col), egui::StrokeKind::Inside);

    // Reserve zone tint at bottom 15%
    let res_h = tube.height() * 0.15;
    let res_rect = Rect::from_min_max(pos2(tube.left() + 1.0, tube.bottom() - res_h), pos2(tube.right() - 1.0, tube.bottom() - 1.0));
    p.rect_filled(res_rect, 1.0, a(th.redline, 35));

    // Liquid fill from bottom up
    let inner_h = (tube.height() - 2.0).max(1.0);
    let fill_h = inner_h * (pct / 100.0).clamp(0.0, 1.0);
    if fill_h > 1.0 {
        let fill_top = tube.bottom() - 1.0 - fill_h;
        let fill_col = if low_fuel {
            if ((t * 4.0) as i32) % 2 == 0 { th.redline } else { th.warning }
        } else if pct < 35.0 {
            th.warning
        } else {
            th.primary
        };
        let fill_rect = Rect::from_min_max(pos2(tube.left() + 1.5, fill_top), pos2(tube.right() - 1.5, tube.bottom() - 1.0));
        p.rect_filled(fill_rect, 2.0, a(fill_col, 210));
        // Bright liquid meniscus highlight line at top of fill
        p.line_segment(
            [pos2(tube.left() + 2.0, fill_top), pos2(tube.right() - 2.0, fill_top)],
            Stroke::new(1.5, Color32::WHITE),
        );
    }

    // Quarter-tank graduation ticks along the glass
    for k in 1..=3 {
        let ty = tube.bottom() - tube.height() * (k as f32 / 4.0);
        let tick_w = if k == 2 { tube_w * 0.45 } else { tube_w * 0.25 };
        p.line_segment([pos2(tube.right() - tick_w, ty), pos2(tube.right() - 1.0, ty)], Stroke::new(0.8, a(th.text_bright, 120)));
    }

    // F and E labels beside the tube
    p.text(pos2(tube.left() - 3.0, tube.top()), Align2::RIGHT_TOP, "F", FontId::monospace(7.5), th.text_dim);
    p.text(pos2(tube.left() - 3.0, tube.bottom()), Align2::RIGHT_BOTTOM, "E", FontId::monospace(7.5), if low_fuel { th.redline } else { th.text_dim });

    // Right column: Label, Big %, Reset time, Low Fuel badge
    let tx = tube.right() + 7.0;
    p.text(pos2(tx, r.top() + 1.0), Align2::LEFT_TOP, label, FontId::monospace(8.5), th.text_dim);
    let num_size = (h * 0.42).clamp(13.0, 21.0);
    p.text(
        pos2(tx, r.center().y),
        Align2::LEFT_CENTER,
        format!("{:.0}%", pct),
        FontId::monospace(num_size),
        if low_fuel { th.redline } else { th.text_bright },
    );
    let sub_txt = if low_fuel {
        format!("LOW · {}", reset_str)
    } else {
        format!("RST {}", reset_str)
    };
    p.text(
        pos2(tx, r.bottom() - 1.0),
        Align2::LEFT_BOTTOM,
        sub_txt,
        FontId::monospace(8.0),
        if low_fuel { th.warning } else { th.secondary },
    );
}

/// Minimalist Ring Gauges: Antigravity-inspired circular progress rings with centered % & reset timer
fn draw_ring_fuel_gauge(
    p: &Painter,
    r: Rect,
    label: &str,
    pct: f32,
    reset_str: &str,
    th: &ThemePalette,
    t: f32,
) {
    let low_fuel = pct < 15.0;
    let h = r.height();
    let rad = ((h - 6.0) * 0.5).min(r.width() * 0.24).clamp(10.0, 24.0);
    let cx = r.left() + rad + 6.0;
    let cy = r.center().y;

    let ring_pt = |frac: f32, rr: f32| {
        // Start at 12 o'clock (-PI/2) and sweep clockwise
        let angle = -std::f32::consts::FRAC_PI_2 + frac.clamp(0.0, 1.0) * std::f32::consts::TAU;
        pos2(cx + rr * angle.cos(), cy + rr * angle.sin())
    };

    // Background ring track
    p.circle_stroke(pos2(cx, cy), rad, Stroke::new(3.0, a(th.border, 120)));

    // Active progress arc
    let sweep = (pct / 100.0).clamp(0.0, 1.0);
    let ring_col = if low_fuel {
        if ((t * 4.0) as i32) % 2 == 0 { th.redline } else { th.warning }
    } else if pct < 35.0 {
        th.warning
    } else {
        th.primary
    };
    if sweep > 0.01 {
        let steps = ((sweep * 36.0) as usize).max(4);
        let pts: Vec<Pos2> = (0..=steps).map(|k| ring_pt(sweep * (k as f32 / steps as f32), rad)).collect();
        p.add(PathShape::line(pts, Stroke::new(3.2, ring_col)));
    }

    // Center % inside ring
    let inner_font = (rad * 0.62).clamp(8.0, 13.0);
    p.text(
        pos2(cx, cy),
        Align2::CENTER_CENTER,
        format!("{:.0}", pct),
        FontId::monospace(inner_font),
        if low_fuel { th.redline } else { th.text_bright },
    );

    // Red alert dot at top of ring when low (matching Antigravity UI)
    if low_fuel {
        let dot_pos = ring_pt(0.0, rad);
        p.circle_filled(dot_pos, 3.0, a(th.redline, 70));
        p.circle_filled(dot_pos, 1.8, th.redline);
    }

    // Right side text block
    let tx = cx + rad + 8.0;
    p.text(pos2(tx, r.top() + 2.0), Align2::LEFT_TOP, label, FontId::monospace(8.5), th.text_dim);
    p.text(
        pos2(tx, r.center().y),
        Align2::LEFT_CENTER,
        format!("{:.0}% left", pct),
        FontId::monospace(10.5),
        if low_fuel { th.redline } else { th.text_bright },
    );
    p.text(
        pos2(tx, r.bottom() - 2.0),
        Align2::LEFT_BOTTOM,
        format!("Resets {}", reset_str),
        FontId::monospace(8.0),
        th.secondary,
    );
}

/// Cockpit HUD Fuel Telemetry: fighter-jet / avionics split-bar HUD with reserve zone hatching & caret
fn draw_hud_fuel_gauge(
    p: &Painter,
    r: Rect,
    label: &str,
    pct: f32,
    reset_str: &str,
    th: &ThemePalette,
    t: f32,
) {
    let low_fuel = pct < 15.0;

    // Frame with subtle corner brackets
    let bracket = 5.0;
    let br_col = a(if low_fuel { th.redline } else { th.primary }, 160);
    let st = Stroke::new(1.0, br_col);
    p.line_segment([r.left_top(), pos2(r.left() + bracket, r.top())], st);
    p.line_segment([r.left_top(), pos2(r.left(), r.top() + bracket)], st);
    p.line_segment([r.right_top(), pos2(r.right() - bracket, r.top())], st);
    p.line_segment([r.right_top(), pos2(r.right(), r.top() + bracket)], st);
    p.line_segment([r.left_bottom(), pos2(r.left() + bracket, r.bottom())], st);
    p.line_segment([r.left_bottom(), pos2(r.left(), r.bottom() - bracket)], st);
    p.line_segment([r.right_bottom(), pos2(r.right() - bracket, r.bottom())], st);
    p.line_segment([r.right_bottom(), pos2(r.right(), r.bottom() - bracket)], st);

    // Top row: label + numeric %
    p.text(pos2(r.left() + 5.0, r.top() + 2.0), Align2::LEFT_TOP, label, FontId::monospace(8.0), th.text_dim);
    let pct_col = if low_fuel {
        if ((t * 4.0) as i32) % 2 == 0 { th.redline } else { Color32::WHITE }
    } else {
        th.text_bright
    };
    p.text(pos2(r.right() - 5.0, r.top() + 1.0), Align2::RIGHT_TOP, format!("{:.0}%", pct), FontId::monospace(10.5), pct_col);

    // Horizontal HUD bar
    let bar_l = r.left() + 6.0;
    let bar_r = r.right() - 6.0;
    let bar_w = (bar_r - bar_l).max(16.0);
    let bar_cy = r.center().y + 1.0;
    let bar_h = 7.0;
    let bar_rect = Rect::from_min_max(pos2(bar_l, bar_cy - bar_h * 0.5), pos2(bar_r, bar_cy + bar_h * 0.5));

    p.rect_filled(bar_rect, 1.0, a(th.graph_bg, 220));
    p.rect_stroke(bar_rect, 1.0, Stroke::new(0.8, a(th.border, 140)), egui::StrokeKind::Inside);

    // Hatched reserve zone (left 15%)
    let res_x = bar_l + bar_w * 0.15;
    p.rect_filled(
        Rect::from_min_max(pos2(bar_l + 1.0, bar_rect.top() + 1.0), pos2(res_x, bar_rect.bottom() - 1.0)),
        0.0,
        a(th.redline, 55),
    );
    p.line_segment([pos2(res_x, bar_rect.top() - 2.0), pos2(res_x, bar_rect.bottom() + 2.0)], Stroke::new(1.0, a(th.redline, 180)));

    // Active fuel fill
    let cur_x = bar_l + bar_w * (pct / 100.0).clamp(0.0, 1.0);
    if cur_x > bar_l + 1.5 {
        let fcol = if low_fuel { th.redline } else if pct < 35.0 { th.warning } else { th.primary };
        p.rect_filled(
            Rect::from_min_max(pos2(bar_l + 1.0, bar_rect.top() + 1.5), pos2(cur_x, bar_rect.bottom() - 1.5)),
            0.5,
            a(fcol, 220),
        );
    }

    // Crisp HUD caret pointer at current level
    p.line_segment([pos2(cur_x, bar_rect.top() - 3.0), pos2(cur_x, bar_rect.bottom() + 3.0)], Stroke::new(1.5, Color32::WHITE));

    // Bottom row: E / LOW FUEL / reset countdown
    let bot_left = if low_fuel { "LOW FUEL" } else { "E       1/2" };
    p.text(
        pos2(r.left() + 5.0, r.bottom() - 2.0),
        Align2::LEFT_BOTTOM,
        bot_left,
        FontId::monospace(7.5),
        if low_fuel { th.redline } else { th.text_dim },
    );
    p.text(pos2(r.right() - 5.0, r.bottom() - 2.0), Align2::RIGHT_BOTTOM, reset_str, FontId::monospace(8.0), th.secondary);
}

/// Corvette C4: Segmented LCD digital wedge with 7-segment readout and LOW FUEL alert
fn draw_corvette_fuel_wedge(
    p: &Painter,
    r: Rect,
    label: &str,
    pct: f32,
    reset_str: &str,
    th: &ThemePalette,
    t: f32,
) {
    let (_w, h) = (r.width(), r.height());
    let low_fuel = pct < 15.0;

    // Header info: Label on left, Reset countdown on right
    p.text(pos2(r.left() + 2.0, r.top() + 1.0), Align2::LEFT_TOP, label, FontId::monospace(8.5), th.text_dim);
    p.text(pos2(r.right() - 2.0, r.top() + 1.0), Align2::RIGHT_TOP, format!("⏱ {}", reset_str), FontId::monospace(8.5), th.secondary);

    // Middle/Bottom: Digital Readout + Segmented LCD Ladder
    let bar_bottom = r.bottom() - 3.0;
    let bar_top = bar_bottom - (h * 0.38).clamp(8.0, 16.0);
    let bar_left = r.left() + 14.0;
    let bar_right = r.right() - 48.0;
    let bar_w = (bar_right - bar_left).max(20.0);

    // E and F markers
    p.text(pos2(bar_left - 4.0, (bar_top + bar_bottom) * 0.5), Align2::RIGHT_CENTER, "E", FontId::monospace(9.0), if low_fuel { th.redline } else { th.text_dim });
    p.text(pos2(bar_right + 4.0, (bar_top + bar_bottom) * 0.5), Align2::LEFT_CENTER, "F", FontId::monospace(9.0), th.text_dim);

    // Segmented blocks (12 segments)
    let seg_count = 12;
    let seg_spacing = 2.0;
    let seg_w = ((bar_w - (seg_count as f32 * seg_spacing)) / seg_count as f32).max(2.0);
    let lit_count = ((pct / 100.0) * seg_count as f32).round() as usize;

    for i in 0..seg_count {
        let x1 = bar_left + i as f32 * (seg_w + seg_spacing);
        let x2 = x1 + seg_w;
        let frac = i as f32 / seg_count as f32;

        let seg_col = if frac < 0.20 {
            th.redline
        } else if frac < 0.50 {
            th.warning
        } else {
            th.primary
        };

        let seg_r = Rect::from_min_max(pos2(x1, bar_top), pos2(x2, bar_bottom));
        if i < lit_count {
            p.rect_filled(seg_r, 1.0, seg_col);
        } else {
            p.rect_filled(seg_r, 1.0, a(seg_col, 25));
            p.rect_stroke(seg_r, 1.0, Stroke::new(0.6, a(seg_col, 50)), egui::StrokeKind::Inside);
        }
    }

    // Bold Digital Readout on far right
    let val_x = r.right() - 2.0;
    let val_y = (bar_top + bar_bottom) * 0.5;
    let num_size = (h * 0.42).clamp(13.0, 20.0);
    let txt_col = if low_fuel {
        if ((t * 4.0) as i32) % 2 == 0 { th.redline } else { Color32::WHITE }
    } else {
        th.text_bright
    };
    p.text(pos2(val_x, val_y), Align2::RIGHT_CENTER, format!("{:.0}%", pct), FontId::monospace(num_size), txt_col);
}

/// Nissan 300ZX: 80s Japanese Electronic Cockpit Segmented Ladder
fn draw_nissan_fuel_ladder(
    p: &Painter,
    r: Rect,
    label: &str,
    pct: f32,
    reset_str: &str,
    th: &ThemePalette,
    _t: f32,
) {
    let (_w, h) = (r.width(), r.height());
    let low_fuel = pct < 15.0;

    // Header
    p.text(pos2(r.left() + 2.0, r.top() + 1.0), Align2::LEFT_TOP, label, FontId::monospace(8.5), th.text_dim);
    p.text(pos2(r.right() - 2.0, r.top() + 1.0), Align2::RIGHT_TOP, format!("⏱ {}", reset_str), FontId::monospace(8.5), th.secondary);

    let bar_bottom = r.bottom() - 3.0;
    let bar_top = bar_bottom - (h * 0.35).clamp(8.0, 14.0);
    let bar_left = r.left() + 14.0;
    let bar_right = r.right() - 44.0;
    let bar_w = (bar_right - bar_left).max(20.0);

    // E and F markers
    p.text(pos2(bar_left - 3.0, (bar_top + bar_bottom) * 0.5), Align2::RIGHT_CENTER, "E", FontId::monospace(8.5), if low_fuel { th.redline } else { th.text_dim });
    p.text(pos2(bar_right + 3.0, (bar_top + bar_bottom) * 0.5), Align2::LEFT_CENTER, "F", FontId::monospace(8.5), th.text_dim);

    let seg_count = 14;
    let seg_spacing = 1.5;
    let seg_w = ((bar_w - (seg_count as f32 * seg_spacing)) / seg_count as f32).max(2.0);
    let lit_count = ((pct / 100.0) * seg_count as f32).round() as usize;

    for i in 0..seg_count {
        let x1 = bar_left + i as f32 * (seg_w + seg_spacing);
        let x2 = x1 + seg_w;
        let frac = i as f32 / seg_count as f32;
        let seg_col = if frac < 0.18 { th.redline } else if frac < 0.50 { th.warning } else { th.primary };

        let seg_r = Rect::from_min_max(pos2(x1, bar_top), pos2(x2, bar_bottom));
        if i < lit_count {
            p.rect_filled(seg_r, 0.0, seg_col);
        } else {
            p.rect_filled(seg_r, 0.0, a(seg_col, 20));
        }
    }

    // Number
    let val_x = r.right() - 2.0;
    let num_size = (h * 0.38).clamp(12.0, 18.0);
    p.text(
        pos2(val_x, (bar_top + bar_bottom) * 0.5),
        Align2::RIGHT_CENTER,
        format!("{:.0}%", pct),
        FontId::monospace(num_size),
        if low_fuel { th.redline } else { th.text_bright },
    );
}

/// Classic / Retro Amber / VFD / Default: Dual Fuel Tank with E ━━ 1/2 ━━ F graduations
fn draw_classic_fuel_gauge(
    p: &Painter,
    r: Rect,
    label: &str,
    pct: f32,
    reset_str: &str,
    th: &ThemePalette,
    t: f32,
) {
    let (_w, h) = (r.width(), r.height());
    let low_fuel = pct < 15.0;

    // Header row: Label + Reset countdown
    p.text(pos2(r.left() + 2.0, r.top() + 1.0), Align2::LEFT_TOP, label, FontId::monospace(8.5), th.text_dim);
    p.text(pos2(r.right() - 2.0, r.top() + 1.0), Align2::RIGHT_TOP, format!("⏱ {}", reset_str), FontId::monospace(8.5), th.secondary);

    // Fuel Chamber Rect
    let bar_bottom = r.bottom() - 3.0;
    let bar_top = bar_bottom - (h * 0.38).clamp(8.0, 16.0);
    let bar_left = r.left() + 14.0;
    let bar_right = r.right() - 46.0;
    let bar_w = (bar_right - bar_left).max(20.0);

    // Scale text: E on left, 1/2 in center, F on right
    p.text(pos2(bar_left - 3.0, (bar_top + bar_bottom) * 0.5), Align2::RIGHT_CENTER, "E", FontId::monospace(8.5), if low_fuel { th.redline } else { th.text_dim });
    p.text(pos2(bar_right + 3.0, (bar_top + bar_bottom) * 0.5), Align2::LEFT_CENTER, "F", FontId::monospace(8.5), th.text_dim);

    // Background Chamber
    let chamber = Rect::from_min_max(pos2(bar_left, bar_top), pos2(bar_right, bar_bottom));
    p.rect_filled(chamber, 2.0, a(th.card_bg, 180));
    p.rect_stroke(chamber, 2.0, Stroke::new(1.0, a(th.border, 120)), egui::StrokeKind::Inside);

    // Half-tank mark notch
    let mid_x = (bar_left + bar_right) * 0.5;
    p.line_segment([pos2(mid_x, bar_top - 2.0), pos2(mid_x, bar_top + 2.0)], Stroke::new(1.0, a(th.text_dim, 140)));

    // Fill level
    let fill_w = bar_w * (pct / 100.0).clamp(0.0, 1.0);
    if fill_w > 1.0 {
        let fill_r = Rect::from_min_max(pos2(bar_left + 1.0, bar_top + 1.0), pos2(bar_left + fill_w - 1.0, bar_bottom - 1.0));
        let fill_col = if low_fuel {
            let flash = ((t * 4.0) as i32) % 2 == 0;
            if flash { th.redline } else { th.warning }
        } else if pct < 35.0 {
            th.warning
        } else {
            th.primary
        };
        p.rect_filled(fill_r, 1.5, fill_col);
    }

    // Bold Digital Readout
    let val_x = r.right() - 2.0;
    let num_size = (h * 0.40).clamp(12.0, 19.0);
    p.text(
        pos2(val_x, (bar_top + bar_bottom) * 0.5),
        Align2::RIGHT_CENTER,
        format!("{:.0}%", pct),
        FontId::monospace(num_size),
        if low_fuel { th.redline } else { th.text_bright },
    );
}

