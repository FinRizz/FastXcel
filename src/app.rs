use crate::data::{DataEngine, LoadStats};
use crate::model::{CellId, SourceColumnId, SourceRowId};
use crate::numeric::parse_numeric;
use crate::ui::state::{AppCommand, AppState};
use crate::ui::table::{cell_str, DataTableWidget};
use egui::{vec2, Color32, Context};
use polars::prelude::DataFrame;
use rfd::FileDialog;
use std::time::Instant;

pub struct UltraFastApp {
    engine: DataEngine,
    ui_state: AppState,
    last_stats: Option<LoadStats>,
    last_page: Option<(DataFrame, bool)>,
    selected_cell: Option<CellId>,
    interpretation_input: String,
}

impl UltraFastApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            engine: DataEngine::new(),
            ui_state: AppState::new(),
            last_stats: None,
            last_page: None,
            selected_cell: None,
            interpretation_input: String::new(),
        }
    }

    fn open_file(&mut self, path: &std::path::Path) {
        let t0 = Instant::now();
        match self.engine.open_path(path) {
            Ok(_stats) => {
                self.ui_state.apply(AppCommand::OpenSucceeded(path.to_path_buf()));
                self.selected_cell = None;
                self.interpretation_input.clear();
                self.last_page = None;
                self.last_stats = Some(LoadStats {
                    ms: t0.elapsed().as_millis(),
                });
            }
            Err(error) => {
                self.ui_state.apply(AppCommand::OpenFailed(error.to_string()));
                self.last_stats = None;
            }
        }
    }

    fn selected_cell_value(df: &DataFrame, cell_id: CellId) -> Option<String> {
        let row_index = usize::try_from(cell_id.row.get()).ok()?;
        let col_index = usize::try_from(cell_id.column.get()).ok()?;
        let column = df.get_columns().get(col_index)?;
        Some(cell_str(column, row_index))
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
                    if ui
                        .add_sized(btn_size, egui::Button::new("Open CSV/Parquet"))
                        .clicked()
                    {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Data", &["csv", "parquet"])
                            .pick_file()
                        {
                            self.open_file(&path);
                        }
                    }

                    ui.separator();
                    ui.label("Page size:");
                    let mut page_size = self.ui_state.page_size;
                    if ui
                        .add(
                            egui::DragValue::new(&mut page_size)
                                .range(1_000..=2_000_000)
                                .speed(1_000)
                                .min_decimals(0)
                                .max_decimals(0),
                        )
                        .changed()
                    {
                        self.ui_state.apply(AppCommand::SetPageSize(page_size));
                        self.selected_cell = None;
                        self.interpretation_input.clear();
                    }

                    ui.separator();
                    let can_go_prev = self.ui_state.current_offset > 0;
                    let can_go_next = self
                        .last_page
                        .as_ref()
                        .map(|(_, has_more)| *has_more)
                        .unwrap_or(false);
                    let nav_btn_size = vec2(80.0, 32.0);
                    if ui
                        .add_enabled(can_go_prev, egui::Button::new("Prev"))
                        .clicked()
                    {
                        self.ui_state.apply(AppCommand::PrevPage);
                        self.selected_cell = None;
                        self.interpretation_input.clear();
                    }
                    if ui
                        .add_enabled(can_go_next, egui::Button::new("Next"))
                        .clicked()
                    {
                        self.ui_state.apply(AppCommand::NextPage);
                        self.selected_cell = None;
                        self.interpretation_input.clear();
                    }

                    ui.separator();
                    ui.label("Filter:");
                    let mut filter_query = self.ui_state.filter_query.clone();
                    let filter_response = ui.add(
                        egui::TextEdit::singleline(&mut filter_query)
                            .desired_width(220.0)
                            .hint_text("Enter filter expression"),
                    );
                    let apply_clicked = ui
                        .add_sized(nav_btn_size, egui::Button::new("Apply"))
                        .on_hover_text("Example: Open > 100 & Volume > 10000")
                        .clicked();
                    if apply_clicked || filter_response.changed() {
                        self.ui_state
                            .apply(AppCommand::SetFilterQuery(filter_query.clone()));
                        self.selected_cell = None;
                        self.interpretation_input.clear();
                    }

                    if let Some(p) = &self.ui_state.open_path {
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label("Current file:");
                        ui.monospace(p.to_string_lossy());
                    }
                });
                ui.add_space(8.0);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.ui_state.last_error {
                ui.colored_label(Color32::RED, format!("Error: {error}"));
                ui.add_space(8.0);
            }

            let Some(_) = self.ui_state.open_path.as_ref() else {
                ui.centered_and_justified(|ui| {
                    ui.heading("Open a CSV/Parquet file to begin");
                });
                return;
            };

            let request_offset = self.ui_state.current_offset;
            let request_limit = self.ui_state.page_size;
            let request_filter = self.ui_state.filter_query.trim().to_owned();

            match self
                .engine
                .fetch_page(request_offset, request_limit, &request_filter)
            {
                Ok((df, has_more)) => {
                    self.last_page = Some((df, has_more));
                    self.ui_state.clear_error();
                }
                Err(error) => {
                    self.ui_state.open_failed(error.to_string());
                }
            }

            let Some((df, has_more)) = self.last_page.as_ref() else {
                ui.colored_label(Color32::RED, "No page is available to render");
                return;
            };

            if let Some(stats) = &self.last_stats {
                ui.horizontal(|ui| {
                    ui.small(format!(
                        "Loaded in ~{} ms | Columns: {} | Showing rows {}..{}{}",
                        stats.ms,
                        df.get_columns().len(),
                        self.ui_state.current_offset,
                        self.ui_state.current_offset + df.height(),
                        if *has_more { "+" } else { "" }
                    ));
                });
            }

            ui.add_space(8.0);

            let clicked_cell = DataTableWidget::new(df).show(
                ui,
                self.selected_cell.map(|cell| (cell.row.get(), cell.column.get() as usize)),
            );

            if let Some((source_row_id, column_index)) = clicked_cell {
                let cell_id = CellId::new(
                    SourceRowId::new(source_row_id),
                    SourceColumnId::new(column_index as u64),
                );
                if self.selected_cell != Some(cell_id) {
                    self.interpretation_input = self
                        .engine
                        .override_for_cell(&cell_id)
                        .map(|value| value.to_string())
                        .or_else(|| Self::selected_cell_value(df, cell_id))
                        .unwrap_or_default();
                }
                self.selected_cell = Some(cell_id);
            }

            if let Some(cell_id) = self.selected_cell {
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.label(format!(
                        "Selected cell: row {} column {}",
                        cell_id.row, cell_id.column
                    ));

                    if let Some(current_value) = Self::selected_cell_value(df, cell_id) {
                        ui.monospace(format!("Raw value: {current_value}"));
                    }

                    ui.horizontal(|ui| {
                        ui.label("Interpret as number for this session:");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.interpretation_input)
                                .desired_width(180.0),
                        );

                        let apply_clicked = ui.button("Apply").clicked();
                        let clear_clicked = ui.button("Clear").clicked();

                        if apply_clicked
                            || (response.lost_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                        {
                            let trimmed = self.interpretation_input.trim();
                            if trimmed.is_empty() {
                                self.engine.remove_override(&cell_id);
                                self.ui_state.clear_error();
                            } else if let Ok(value) = parse_numeric(trimmed) {
                                self.engine.set_override(cell_id, value);
                                self.ui_state.clear_error();
                                self.interpretation_input = value.to_string();
                            } else {
                                self.ui_state.open_failed(format!(
                                    "Invalid interpretation: {trimmed}"
                                ));
                            }
                        }

                        if clear_clicked {
                            self.engine.remove_override(&cell_id);
                            self.interpretation_input.clear();
                            self.ui_state.clear_error();
                        }
                    });
                });
            }
        });

        ctx.request_repaint();
    }
}
