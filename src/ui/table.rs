use egui_extras::{Column, TableBuilder};
use polars::prelude::*;

pub struct DataTableWidget<'a> {
    df: &'a DataFrame,
}

impl<'a> DataTableWidget<'a> {
    pub fn new(df: &'a DataFrame) -> Self {
        Self { df }
    }

    pub fn show(&self, ui: &mut egui::Ui, selected: Option<(u64, usize)>) -> Option<(u64, usize)> {
        let cols = self.df.get_columns();
        if cols.is_empty() {
            ui.label("No columns to display");
            return None;
        }

        let mut clicked = None;
        egui::ScrollArea::horizontal()
            .id_salt("data_table_horizontal_scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let mut table = TableBuilder::new(ui)
                    .id_salt("data_table")
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(64.0).at_least(48.0))
                    .resizable(true)
                    .auto_shrink([false, true])
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::INFINITY);

                for _ in cols.iter().skip(1) {
                    table = table.column(Column::initial(160.0).at_least(100.0));
                }

                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("#");
                        });
                        for c in cols.iter().skip(1) {
                            header.col(|ui| {
                                ui.strong(c.name().to_string());
                            });
                        }
                    })
                    .body(|body| {
                        let n = self.df.height();
                        body.rows(18.0, n, |mut row| {
                            let r = row.index();
                            let source_row_id =
                                source_row_id_value(&cols[0], r).unwrap_or(r as u64);

                            row.col(|ui| {
                                ui.monospace(source_row_id.to_string());
                            });

                            for (index, c) in cols.iter().enumerate().skip(1) {
                                let is_selected = selected == Some((source_row_id, index));
                                row.col(|ui| {
                                    let response = ui.selectable_label(is_selected, cell_str(c, r));
                                    if response.clicked() {
                                        clicked = Some((source_row_id, index));
                                    }
                                });
                            }
                        });
                    });
            });

        clicked
    }
}

pub fn source_row_id_value(s: &Series, row: usize) -> Option<u64> {
    match s.dtype() {
        DataType::UInt64 => s.u64().ok().and_then(|ca| ca.get(row)),
        DataType::UInt32 => s.u32().ok().and_then(|ca| ca.get(row)).map(u64::from),
        DataType::Int64 => s
            .i64()
            .ok()
            .and_then(|ca| ca.get(row))
            .and_then(|v| u64::try_from(v).ok()),
        DataType::Int32 => s
            .i32()
            .ok()
            .and_then(|ca| ca.get(row))
            .and_then(|v| u64::try_from(v).ok()),
        DataType::String => s
            .str()
            .ok()
            .and_then(|ca| ca.get(row))
            .and_then(|v| v.trim().parse::<u64>().ok()),
        _ => None,
    }
}

pub fn cell_str(s: &Series, row: usize) -> String {
    match s.dtype() {
        DataType::Datetime(unit, timezone) => s
            .datetime()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(|value| format_timestamp(value, *unit, timezone.as_deref()))
            .unwrap_or_default(),
        DataType::Int64 => s
            .i64()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(|v| v.to_string())
            .unwrap_or_default(),
        DataType::Int32 => s
            .i32()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(|v| v.to_string())
            .unwrap_or_default(),
        DataType::UInt64 => s
            .u64()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(|v| v.to_string())
            .unwrap_or_default(),
        DataType::UInt32 => s
            .u32()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(|v| v.to_string())
            .unwrap_or_default(),
        DataType::Float64 => s
            .f64()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(format_float)
            .unwrap_or_default(),
        DataType::Float32 => s
            .f32()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(|v| format_float(v as f64))
            .unwrap_or_default(),
        DataType::Boolean => s
            .bool()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(|v| v.to_string())
            .unwrap_or_default(),
        DataType::String => s
            .str()
            .ok()
            .and_then(|ca| ca.get(row))
            .map(|v| v.to_string())
            .unwrap_or_default(),
        _ => s.get(row).ok().map(|v| v.to_string()).unwrap_or_default(),
    }
}

fn format_float(v: f64) -> String {
    if v.abs() >= 1_000_000.0 {
        format!("{:.3e}", v)
    } else {
        format!("{:.6}", v)
    }
}

fn format_timestamp(value: i64, unit: TimeUnit, timezone: Option<&str>) -> String {
    let per_second = match unit {
        TimeUnit::Nanoseconds => 1_000_000_000,
        TimeUnit::Microseconds => 1_000_000,
        TimeUnit::Milliseconds => 1_000,
    };
    let seconds = value.div_euclid(per_second);
    let nanos = (value.rem_euclid(per_second) * (1_000_000_000 / per_second)) as u32;
    let Some(utc) = chrono::DateTime::from_timestamp(seconds, nanos) else {
        return format!("{value} {unit:?} ({})", timezone.unwrap_or("no timezone"));
    };
    match timezone {
        None => utc.naive_utc().to_string(),
        Some(zone) => {
            if let Ok(offset) = zone.parse::<chrono::FixedOffset>() {
                return utc.with_timezone(&offset).to_rfc3339();
            }
            if let Ok(tz) = zone.parse::<chrono_tz::Tz>() {
                return utc.with_timezone(&tz).to_rfc3339();
            }
            format!("{} [unrecognized timezone: {zone}]", utc.to_rfc3339())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_preserve_precision_and_convert_named_timezones() {
        assert_eq!(
            format_timestamp(1, TimeUnit::Microseconds, None),
            "1970-01-01 00:00:00.000001"
        );
        assert_eq!(
            format_timestamp(0, TimeUnit::Milliseconds, Some("Asia/Kolkata")),
            "1970-01-01T05:30:00+05:30"
        );
        assert_eq!(
            format_timestamp(0, TimeUnit::Milliseconds, Some("America/New_York")),
            "1969-12-31T19:00:00-05:00"
        );
        assert_eq!(
            format_timestamp(0, TimeUnit::Milliseconds, Some("-03:30")),
            "1969-12-31T20:30:00-03:30"
        );
    }
}
