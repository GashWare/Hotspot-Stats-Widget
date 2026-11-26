use egui::{Color32, RichText, ScrollArea, Ui};
use crate::config::ACCENT_CYAN;
use crate::logger::{AppLogger, LogEntry, LogLevel};

pub struct LogViewerUI {
    pub is_open: bool,
    filter_text: String,
    filter_level: String, // "ALL", "INFO", "SUCCESS", "WARN", "ERROR"
    auto_scroll: bool,
}

impl LogViewerUI {
    pub fn new() -> Self {
        Self {
            is_open: false,
            filter_text: String::new(),
            filter_level: "ALL".to_string(),
            auto_scroll: true,
        }
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        let viewport_id = egui::ViewportId::from_hash_of("system_troubleshooting_logs");
        let builder = egui::ViewportBuilder::default()
            .with_title("NetMonitor System & Troubleshooting Logs")
            .with_inner_size([760.0, 480.0])
            .with_min_inner_size([540.0, 320.0])
            .with_decorations(true)
            .with_resizable(true);

        let mut still_open = true;
        ctx.show_viewport_immediate(viewport_id, builder, |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                still_open = false;
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_content(ui);
            });
        });
        if !still_open {
            self.is_open = false;
        }
    }

    fn render_content(&mut self, ui: &mut Ui) {
        let logger = AppLogger::global();
        let log_path = logger.get_log_path();

        ui.horizontal(|ui| {
            ui.heading(RichText::new("SYSTEM & HOTSPOT LOGS").color(ACCENT_CYAN).strong());
            ui.label(RichText::new(format!("(File: {})", log_path)).color(Color32::GRAY).size(10.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear Buffer").clicked() {
                    logger.clear();
                }
                if ui.button("Copy All").clicked() {
                    let all_text = logger
                        .get_entries()
                        .iter()
                        .map(|e| format!("[{}] [{:?}] [{}] {}", e.timestamp, e.level, e.tag, e.message))
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.ctx().copy_text(all_text);
                }
                ui.checkbox(&mut self.auto_scroll, "Auto-Scroll");
            });
        });

        ui.add_space(4.0);

        // Filter Controls
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add(egui::TextEdit::singleline(&mut self.filter_text).hint_text("Search tag or message...").desired_width(180.0));

            ui.separator();
            ui.label("Level:");
            for level in &["ALL", "INFO", "SUCCESS", "WARN", "ERROR"] {
                if ui.selectable_label(self.filter_level == *level, *level).clicked() {
                    self.filter_level = level.to_string();
                }
            }
        });

        ui.separator();

        let entries = logger.get_entries();
        let filter = self.filter_text.to_lowercase();
        let selected_lvl = self.filter_level.clone();

        let filtered: Vec<&LogEntry> = entries
            .iter()
            .filter(|e| {
                let match_level = match selected_lvl.as_str() {
                    "INFO" => e.level == LogLevel::Info,
                    "SUCCESS" => e.level == LogLevel::Success,
                    "WARN" => e.level == LogLevel::Warning,
                    "ERROR" => e.level == LogLevel::Error,
                    _ => true,
                };
                if !match_level {
                    return false;
                }
                if !filter.is_empty() {
                    let msg_low = e.message.to_lowercase();
                    let tag_low = e.tag.to_lowercase();
                    if !msg_low.contains(&filter) && !tag_low.contains(&filter) {
                        return false;
                    }
                }
                true
            })
            .collect();

        ScrollArea::vertical()
            .stick_to_bottom(self.auto_scroll)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No log records matching current filter.").color(Color32::GRAY));
                    });
                    return;
                }

                for e in filtered {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&e.timestamp).color(Color32::from_rgb(140, 140, 140)).monospace().size(10.5));

                        let (badge_col, badge_txt) = match e.level {
                            LogLevel::Info => (Color32::from_rgb(120, 200, 255), "INFO"),
                            LogLevel::Success => (Color32::from_rgb(0, 255, 136), "OK  "),
                            LogLevel::Warning => (Color32::from_rgb(255, 200, 0), "WARN"),
                            LogLevel::Error => (Color32::from_rgb(255, 60, 90), "ERR "),
                        };

                        ui.colored_label(badge_col, RichText::new(badge_txt).monospace().strong().size(10.5));
                        ui.label(RichText::new(format!("[{}]", e.tag)).color(Color32::from_rgb(200, 150, 255)).monospace().size(10.5));
                        ui.label(RichText::new(&e.message).color(Color32::WHITE).monospace().size(10.5));
                    });
                }
            });
    }
}
