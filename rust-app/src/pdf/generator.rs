use anyhow::{Context, Result};
use printpdf::*;
use std::io::BufWriter;

use super::layout::{prepare_ryohi_for_print, PageLayout};
use crate::models::{format_price, Item};

/// PDF Generator for travel expense reports
pub struct PdfGenerator {
    font_bytes: Vec<u8>,
    layout: PageLayout,
}

impl PdfGenerator {
    /// Create a new PDF generator with embedded font
    pub fn new(font_bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            font_bytes,
            layout: PageLayout::default(),
        })
    }

    /// Generate PDF from items and return as bytes
    pub fn generate(&self, items: &[Item]) -> Result<Vec<u8>> {
        if items.is_empty() {
            anyhow::bail!("No items to generate");
        }

        // Create PDF document (A5 Landscape: 210mm x 148mm)
        let (doc, page1, layer1) = PdfDocument::new(
            "Travel Expense Report",
            Mm(self.layout.page_width as f32),
            Mm(self.layout.page_height as f32),
            "Layer 1",
        );

        // Add font
        let font = doc
            .add_external_font(std::io::Cursor::new(&self.font_bytes))
            .context("Failed to add font")?;

        // Process first item on the first page
        let current_layer = doc.get_page(page1).get_layer(layer1);
        self.render_page(&current_layer, &font, &items[0])?;

        // Add pages for remaining items
        for item in items.iter().skip(1) {
            let (page, layer) = doc.add_page(
                Mm(self.layout.page_width as f32),
                Mm(self.layout.page_height as f32),
                "Layer 1",
            );
            let current_layer = doc.get_page(page).get_layer(layer);
            self.render_page(&current_layer, &font, item)?;
        }

        // Save to bytes
        let mut buffer = Vec::new();
        doc.save(&mut BufWriter::new(std::io::Cursor::new(&mut buffer)))
            .context("Failed to save PDF")?;

        Ok(buffer)
    }

    /// Render a single page
    fn render_page(
        &self,
        layer: &PdfLayerReference,
        font: &IndirectFontRef,
        item: &Item,
    ) -> Result<()> {
        // Draw page border
        self.draw_border(layer);

        // Draw approval table (right top)
        self.draw_approval_table(layer, font);

        // Draw base data (title, date, etc.)
        self.draw_base_data(layer, font, item);

        // Draw basic info table
        self.draw_basic_info_table(layer, font);

        // Draw main data table
        self.draw_main_data_table(layer, font);

        // Draw summary table
        self.draw_summary_table(layer, font);

        // Print item data
        self.print_item(layer, font, item);

        Ok(())
    }

    /// Draw page border
    fn draw_border(&self, layer: &PdfLayerReference) {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 15.0) as f32;
        let end_x = (self.layout.page_width - 10.0) as f32;
        let end_y = 10.0_f32;

        layer.set_outline_thickness(0.5);
        layer.set_outline_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));

        // Draw border rectangle using lines
        self.draw_line_segment(layer, start_x, start_y, end_x, start_y);
        self.draw_line_segment(layer, end_x, start_y, end_x, end_y);
        self.draw_line_segment(layer, end_x, end_y, start_x, end_y);
        self.draw_line_segment(layer, start_x, end_y, start_x, start_y);
    }

    /// Draw approval table
    fn draw_approval_table(&self, layer: &PdfLayerReference, font: &IndirectFontRef) {
        let start_x = 155.0_f32;
        let start_y = (self.layout.page_height - 25.0) as f32;
        let col_width = 15.0_f32;
        let row_height1 = 5.0_f32;
        let row_height2 = 15.0_f32;

        layer.set_outline_thickness(0.2);

        let headers = ["社　長", "会　計", "所　属"];
        for (i, header) in headers.iter().enumerate() {
            let x = start_x + (i as f32) * col_width;

            // Header cell
            self.draw_rect(layer, x, start_y, col_width, row_height1);
            self.draw_text_centered(layer, font, 9.0, header, x, start_y - 4.0, col_width);

            // Data cell (empty)
            self.draw_rect(layer, x, start_y - row_height1, col_width, row_height2);
        }
    }

    /// Draw base data (title, date, office)
    fn draw_base_data(&self, layer: &PdfLayerReference, font: &IndirectFontRef, item: &Item) {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 15.0) as f32;

        // Title
        let title = "出 張 旅 費 日 当 駐 車 料 込 精 算 書";
        layer.use_text(title.to_string(), 14.0, Mm(start_x + 13.0), Mm(start_y - 5.0), font);

        // Title underlines (2 lines)
        let title_width = self.estimate_text_width(title, 14.0);
        self.draw_line_segment(
            layer,
            start_x + 13.0,
            start_y - 6.0,
            start_x + 15.0 + title_width,
            start_y - 6.0,
        );
        self.draw_line_segment(
            layer,
            start_x + 13.0,
            start_y - 7.0,
            start_x + 15.0 + title_width,
            start_y - 7.0,
        );

        // Settlement date
        if let Some(ref pay_day) = item.pay_day {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(pay_day, "%Y-%m-%d") {
                let pay_day_str = date.format("清算日　%Y年 %m月 %d日").to_string();
                layer.use_text(pay_day_str, 9.0, Mm(start_x + 100.0), Mm(start_y - 5.0), font);
            }
        }

        // Office (right side)
        if let Some(ref office) = item.office {
            let text_width = self.estimate_text_width(office, 10.0);
            layer.use_text(
                office.clone(),
                10.0,
                Mm(start_x + 190.0 - text_width - 2.0),
                Mm(start_y - 5.0),
                font,
            );
        }
    }

    /// Draw basic info table
    fn draw_basic_info_table(&self, layer: &PdfLayerReference, font: &IndirectFontRef) {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 30.0) as f32;

        layer.set_outline_thickness(0.2);

        // Departure/Return labels (left side)
        let row_height = 3.5_f32;
        let diff_start_y = 3.0_f32;
        layer.use_text(
            "出発".to_string(),
            9.0,
            Mm(start_x + 1.0),
            Mm(start_y - diff_start_y),
            font,
        );
        layer.use_text(
            "　　月　　日".to_string(),
            9.0,
            Mm(start_x + 2.0),
            Mm(start_y - diff_start_y - row_height),
            font,
        );
        layer.use_text(
            "帰着".to_string(),
            9.0,
            Mm(start_x + 1.0),
            Mm(start_y - diff_start_y - row_height * 2.0),
            font,
        );
        layer.use_text(
            "　　月　　日".to_string(),
            9.0,
            Mm(start_x + 2.0),
            Mm(start_y - diff_start_y - row_height * 3.0),
            font,
        );

        // Table headers
        let headers = ["", "出張目的", "車両No.", "氏　名", "サイン"];
        let col_widths: [f32; 5] = [31.0, 25.0, 28.75, 30.0, 30.0];

        let mut current_x = start_x;
        for (i, header) in headers.iter().enumerate() {
            self.draw_rect(layer, current_x, start_y, col_widths[i], 15.0);
            if !header.is_empty() {
                layer.use_text(header.to_string(), 9.0, Mm(current_x + 1.0), Mm(start_y - 4.0), font);
            }
            current_x += col_widths[i];
        }
    }

    /// Draw main data table
    fn draw_main_data_table(&self, layer: &PdfLayerReference, font: &IndirectFontRef) {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 45.0) as f32;

        layer.set_outline_thickness(0.2);

        let col_widths: [f32; 9] = [10.0, 17.0, 40.0, 30.0, 15.0, 15.0, 15.0, 25.0, 23.0];
        let row_height = 10.0_f32;
        let header_height = 4.0_f32;

        // Headers
        let headers = [
            "日付",
            "行　先",
            "摘　　要",
            "区　　間",
            "交通機関",
            "運　賃",
            "特別料金",
            "旅費日当",
            "計",
        ];

        let mut current_x = start_x;
        for (i, header) in headers.iter().enumerate() {
            self.draw_rect(layer, current_x, start_y, col_widths[i], header_height);
            self.draw_text_centered(
                layer,
                font,
                8.0,
                header,
                current_x,
                start_y - 3.0,
                col_widths[i],
            );
            current_x += col_widths[i];
        }

        // Data rows (7 rows)
        for row in 0..7 {
            current_x = start_x;
            let current_y = start_y - header_height - (row as f32) * row_height;

            for (col, &width) in col_widths.iter().enumerate() {
                if col == 2 {
                    // 摘要欄は左右の線のみ
                    self.draw_line_segment(layer, current_x, current_y, current_x, current_y - row_height);
                    self.draw_line_segment(
                        layer,
                        current_x + width,
                        current_y,
                        current_x + width,
                        current_y - row_height,
                    );
                } else {
                    self.draw_rect(layer, current_x, current_y, width, row_height);
                }
                current_x += width;
            }
        }
    }

    /// Draw summary table
    fn draw_summary_table(&self, layer: &PdfLayerReference, font: &IndirectFontRef) {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 119.0) as f32;

        layer.set_outline_thickness(0.2);

        let col_widths = [145.0_f32, 45.0_f32];
        let row_height = 19.0_f32;
        let headers = ["備考", "計"];

        let mut current_x = start_x;
        for (i, header) in headers.iter().enumerate() {
            self.draw_rect(layer, current_x, start_y, col_widths[i], row_height);
            layer.use_text(header.to_string(), 8.0, Mm(current_x + 2.0), Mm(start_y - 4.0), font);
            current_x += col_widths[i];
        }
    }

    /// Print item data
    fn print_item(&self, layer: &PdfLayerReference, font: &IndirectFontRef, item: &Item) {
        let start_x = 14.0_f32;
        let start_y = (self.layout.page_height - 36.8) as f32;

        // Start date
        if let Some(ref start_date) = item.start_date {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d") {
                let date_str = date.format("%m　 %d").to_string();
                layer.use_text(date_str, 10.0, Mm(start_x), Mm(start_y), font);
            }
        }

        // End date
        if let Some(ref end_date) = item.end_date {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d") {
                let date_str = date.format("%m　 %d").to_string();
                layer.use_text(date_str, 10.0, Mm(start_x), Mm(start_y - 7.0), font);
            }
        }

        // Purpose
        if let Some(ref purpose) = item.purpose {
            layer.use_text(purpose.clone(), 10.0, Mm(start_x + 32.0), Mm(start_y - 7.0), font);
        }

        // Car
        if !item.car.is_empty() {
            layer.use_text(item.car.clone(), 10.0, Mm(start_x + 52.0), Mm(start_y - 7.0), font);
        }

        // Name
        if !item.name.is_empty() {
            layer.use_text(item.name.clone(), 10.0, Mm(start_x + 85.0), Mm(start_y - 7.0), font);
        }

        // Total price (upper total field)
        let price_str = format_price(item.price);
        let text_width = self.estimate_text_width(&price_str, 12.0);
        layer.use_text(
            price_str,
            12.0,
            Mm((self.layout.right_margin as f32) - text_width - 5.0),
            Mm((self.layout.top_margin - 12.0) as f32),
            font,
        );

        // Print ryohi items
        self.print_ryohi_items(layer, font, &item.ryohi);
    }

    /// Print ryohi items
    fn print_ryohi_items(
        &self,
        layer: &PdfLayerReference,
        font: &IndirectFontRef,
        ryohi_list: &[crate::models::Ryohi],
    ) {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 47.0) as f32;
        let col_widths: [f32; 9] = [10.0, 17.0, 40.0, 30.0, 15.0, 15.0, 15.0, 25.0, 23.0];
        let row_height = 10.0_f32;

        let mut current_row = 0_usize;

        for ryohi in ryohi_list {
            if current_row >= 14 {
                break;
            }

            let print_data = prepare_ryohi_for_print(ryohi, 10, 22);
            let remaining_rows = 14 - current_row;
            let actual_rows = print_data.max_rows.min(remaining_rows);

            let mut drawn_rows = 0_usize;

            for row in 0..actual_rows {
                if !print_data.has_content_in_row(row) {
                    continue;
                }

                let logical_row = current_row + drawn_rows;
                let physical_row = logical_row / 2;
                let sub_row = logical_row % 2;
                let y_offset = (sub_row as f32) * 5.0;

                let current_y = start_y - (physical_row as f32) * row_height - y_offset;
                let mut current_x = start_x;

                // Date
                if row < print_data.date_lines.len() && !print_data.date_lines[row].is_empty() {
                    let date = &print_data.date_lines[row];
                    self.draw_text_centered(
                        layer,
                        font,
                        10.0,
                        date,
                        current_x,
                        current_y - 6.0,
                        col_widths[0],
                    );
                }
                current_x += col_widths[0];

                // Dest
                if row < print_data.dest_lines.len() && !print_data.dest_lines[row].is_empty() {
                    let dest = &print_data.dest_lines[row];
                    self.draw_text_centered(
                        layer,
                        font,
                        10.0,
                        dest,
                        current_x,
                        current_y - 6.0,
                        col_widths[1],
                    );
                }
                current_x += col_widths[1];

                // Detail
                if row < print_data.detail_lines.len() && !print_data.detail_lines[row].is_empty() {
                    let detail = print_data.detail_lines[row].clone();
                    layer.use_text(detail, 10.0, Mm(current_x + 1.0), Mm(current_y - 6.0), font);
                }
                current_x += col_widths[2];

                // Kukan
                if row < print_data.kukan_lines.len() && !print_data.kukan_lines[row].is_empty() {
                    let kukan = print_data.kukan_lines[row].clone();
                    layer.use_text(kukan, 10.0, Mm(current_x + 1.0), Mm(current_y - 6.0), font);
                }
                current_x += col_widths[3];

                // Skip 交通機関, 運賃, 特別料金
                current_x += col_widths[4] + col_widths[5] + col_widths[6];

                // Price
                if row < print_data.price_lines.len() && !print_data.price_lines[row].is_empty() {
                    let price_str = print_data.price_lines[row].clone();
                    let text_width = self.estimate_text_width(&price_str, 10.0);
                    layer.use_text(
                        price_str,
                        10.0,
                        Mm(current_x + col_widths[7] - text_width - 1.0),
                        Mm(current_y - 6.0),
                        font,
                    );
                }
                current_x += col_widths[7];

                // Vol
                if row < print_data.vol_lines.len() && !print_data.vol_lines[row].is_empty() {
                    let vol_str = print_data.vol_lines[row].clone();
                    let text_width = self.estimate_text_width(&vol_str, 10.0);
                    layer.use_text(
                        vol_str,
                        10.0,
                        Mm(current_x + col_widths[8] - text_width - 1.0),
                        Mm(current_y - 6.0),
                        font,
                    );
                }

                drawn_rows += 1;
            }

            current_row += drawn_rows;
        }
    }

    // Helper methods

    fn draw_rect(&self, layer: &PdfLayerReference, x: f32, y: f32, width: f32, height: f32) {
        // Draw rectangle using 4 lines
        self.draw_line_segment(layer, x, y, x + width, y);
        self.draw_line_segment(layer, x + width, y, x + width, y - height);
        self.draw_line_segment(layer, x + width, y - height, x, y - height);
        self.draw_line_segment(layer, x, y - height, x, y);
    }

    fn draw_line_segment(&self, layer: &PdfLayerReference, x1: f32, y1: f32, x2: f32, y2: f32) {
        let points = vec![
            (Point::new(Mm(x1), Mm(y1)), false),
            (Point::new(Mm(x2), Mm(y2)), false),
        ];
        let line = Line {
            points,
            is_closed: false,
        };
        layer.add_line(line);
    }

    fn draw_text_centered(
        &self,
        layer: &PdfLayerReference,
        font: &IndirectFontRef,
        size: f32,
        text: &str,
        x: f32,
        y: f32,
        width: f32,
    ) {
        let text_width = self.estimate_text_width(text, size);
        let centered_x = x + (width - text_width) / 2.0;
        layer.use_text(text.to_string(), size, Mm(centered_x), Mm(y), font);
    }

    fn estimate_text_width(&self, text: &str, size: f32) -> f32 {
        // Rough estimation: Japanese chars ~= size * 0.5mm, ASCII ~= size * 0.3mm
        let mut width = 0.0_f32;
        for c in text.chars() {
            if c.is_ascii() {
                width += size * 0.3;
            } else {
                width += size * 0.5;
            }
        }
        width
    }
}
