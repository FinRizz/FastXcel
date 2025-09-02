use egui_extras::{Column, TableBuilder};
use polars::prelude::*;

pub struct DataTableWidget<'a> {
    df: &'a DataFrame,
}

impl<'a> DataTableWidget<'a> {
    pub fn new(df: &'a DataFrame) -> Self { Self { df } }

    pub fn show(&self, ui: &mut egui::Ui) {
        let cols = self.df.get_columns();
        let mut table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto()) // first column can be row index
            .resizable(true)
            .auto_shrink([false, true])
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY);

        for _ in cols.iter() { table = table.column(Column::remainder()); }

        table
            .header(20.0, |mut header| {
                header.col(|ui| { ui.strong("#"); });
                for c in cols.iter() {
                    header.col(|ui| { ui.strong(c.name().to_string()); });
                }
            })
            .body(|body| {
                let n = self.df.height();
                body.rows(18.0, n, |mut row| {
                    let r = row.index();
                    row.col(|ui| { ui.monospace(format!("{}", r)); });
                    for c in cols.iter() {
                        row.col(|ui| {
                            ui.label(cell_str(c, r));
                        });
                    }
                });
            });
    }
}

fn cell_str(s: &Series, row: usize) -> String {
    match s.dtype() {
        DataType::Int64 => s.i64().ok().and_then(|ca| ca.get(row)).map(|v| v.to_string()).unwrap_or_default(),
        DataType::Int32 => s.i32().ok().and_then(|ca| ca.get(row)).map(|v| v.to_string()).unwrap_or_default(),
        DataType::UInt64 => s.u64().ok().and_then(|ca| ca.get(row)).map(|v| v.to_string()).unwrap_or_default(),
        DataType::UInt32 => s.u32().ok().and_then(|ca| ca.get(row)).map(|v| v.to_string()).unwrap_or_default(),
        DataType::Float64 => s.f64().ok().and_then(|ca| ca.get(row)).map(|v| format_float(v)).unwrap_or_default(),
        DataType::Float32 => s.f32().ok().and_then(|ca| ca.get(row)).map(|v| format_float(v as f64)).unwrap_or_default(),
        DataType::Boolean => s.bool().ok().and_then(|ca| ca.get(row)).map(|v| v.to_string()).unwrap_or_default(),
        DataType::String => s.str().ok().and_then(|ca| ca.get(row)).map(|v| v.to_string()).unwrap_or_default(),
        _ => s.get(row).ok().map(|v| v.to_string()).unwrap_or_default(),
    }
}

fn format_float(v: f64) -> String {
    if v.abs() >= 1_000_000.0 { format!("{:.3e}", v) }
    else { format!("{:.6}", v) }
}