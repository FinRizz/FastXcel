use crate::data::{DataEngine, LoadStats};
use crate::ui::table::DataTableWidget;
use egui::{vec2, Color32, Context};  // Add vec2 import
use rfd::FileDialog;
use std::path::PathBuf;
use std::time::Instant;

pub struct UltraFastApp {
    engine: DataEngine,
    // UI state
    page_size: usize,
    current_offset: usize,
    open_path: Option<PathBuf>,
    last_stats: Option<LoadStats>,
    filter_query: String,
}

impl UltraFastApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            engine: DataEngine::new(),
            page_size: 100_000, // adjust for your RAM/latency tradeoff
            current_offset: 0,
            open_path: None,
            last_stats: None,
            filter_query: String::new(),
        }
    }
}

impl eframe::App for UltraFastApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar")
            .min_height(60.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.spacing_mut().button_padding = vec2(12.0, 6.0);

                    let btn_size = vec2(150.0, 32.0);
                    if ui.add_sized(btn_size, egui::Button::new("📂 Open CSV/Parquet")).clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Data", &[("csv"), ("parquet")])
                            .pick_file()
                        {
                            let t0 = Instant::now();
                            match self.engine.open_path(&path) {
                                Ok(stats) => {
                                    self.open_path = Some(path);
                                    self.current_offset = 0;
                                    self.last_stats = Some(LoadStats { ms: t0.elapsed().as_millis() as u128, ..stats });
                                }
                                Err(e) => {
                                    self.open_path = None;
                                    self.last_stats = None;
                                    ui.colored_label(Color32::RED, format!("Error: {e}"));
                                }
                            }
                        }
                    }

                    ui.separator();
                    ui.label("Page size:");
                    ui.add(egui::DragValue::new(&mut self.page_size)
                        .range(1000..=2_000_000)  // Changed from clamp_range
                        .speed(1000)
                        .prefix("📊 ")
                        .min_decimals(0)
                        .max_decimals(0)
                    );

                    ui.separator();
                    let nav_btn_size = vec2(80.0, 32.0);
                    if ui.add_sized(nav_btn_size, egui::Button::new("⬅️ Prev")).clicked() 
                        && self.current_offset >= self.page_size 
                    {
                        self.current_offset -= self.page_size;
                    }
                    if ui.add_sized(nav_btn_size, egui::Button::new("Next ➡️")).clicked() {
                        self.current_offset = self.current_offset.saturating_add(self.page_size);
                    }

                    ui.separator();
                    ui.label("Filter:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.filter_query)
                            .desired_width(200.0)
                            .hint_text("Enter filter expression...")
                    );
                    if ui.add_sized(nav_btn_size, egui::Button::new("🔍 Apply"))
                        .on_hover_text("Polars expression, e.g. `Open > 100 & Volume > 10000`")
                        .clicked() 
                    {
                        // just trigger rerender; filter applied in draw below
                    }

                    if let Some(p) = &self.open_path {
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label("📄 Current file:");
                        ui.monospace(p.to_string_lossy());
                    }
                });
                ui.add_space(8.0);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.open_path.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.heading("Open a CSV/Parquet file to begin");
                });
                return;
            }

            // page data
            let (df, more_rows) = match self.engine.fetch_page(self.current_offset, self.page_size, self.filter_query.trim()) {
                Ok(r) => r,
                Err(e) => {
                    ui.colored_label(Color32::RED, format!("Load error: {e}"));
                    return;
                }
            };

            if let Some(stats) = &self.last_stats {
                ui.horizontal(|ui| {
                    ui.small(format!("Loaded in ~{} ms | Columns: {} | Showing rows {}..{}{}",
                        stats.ms,
                        df.get_columns().len(),
                        self.current_offset,
                        self.current_offset + df.height(),
                        if more_rows { "+" } else { "" }
                    ));
                });
            }

            // Table
            DataTableWidget::new(&df).show(ui);
        });

        ctx.request_repaint(); // keep smooth scrolling
    }
}