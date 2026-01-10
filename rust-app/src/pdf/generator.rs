use anyhow::Result;
use printpdf::*;

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

        // Create PDF document
        let mut doc = PdfDocument::new("Travel Expense Report");

        // Parse and add font
        let font = ParsedFont::from_bytes(&self.font_bytes, 0, &mut Vec::new())
            .ok_or_else(|| anyhow::anyhow!("Failed to parse font"))?;
        let font_id = doc.add_font(&font);

        // Build pages
        let mut pages = Vec::new();
        for item in items {
            let ops = self.build_page_ops(&font_id, item)?;
            let page = PdfPage::new(
                Mm(self.layout.page_width as f32),
                Mm(self.layout.page_height as f32),
                ops,
            );
            pages.push(page);
        }

        // Save to bytes (subset_fonts: true by default in PdfSaveOptions)
        let pdf_bytes = doc.with_pages(pages).save(
            &PdfSaveOptions::default(),
            &mut Vec::new(),
        );

        Ok(pdf_bytes)
    }

    /// Build operations for a single page
    fn build_page_ops(&self, font_id: &FontId, item: &Item) -> Result<Vec<Op>> {
        let mut ops = Vec::new();

        // Draw page border
        ops.extend(self.build_border_ops());

        // Draw approval table (right top)
        ops.extend(self.build_approval_table_ops(font_id));

        // Draw base data (title, date, etc.)
        ops.extend(self.build_base_data_ops(font_id, item));

        // Draw basic info table
        ops.extend(self.build_basic_info_table_ops(font_id));

        // Draw main data table
        ops.extend(self.build_main_data_table_ops(font_id));

        // Draw summary table
        ops.extend(self.build_summary_table_ops(font_id));

        // Print item data
        ops.extend(self.build_item_ops(font_id, item));

        Ok(ops)
    }

    /// Build page border operations
    fn build_border_ops(&self) -> Vec<Op> {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 15.0) as f32;
        let end_x = (self.layout.page_width - 10.0) as f32;
        let end_y = 10.0_f32;

        let mut ops = vec![
            Op::SetOutlineThickness { pt: Pt(0.5) },
            Op::SetOutlineColor { col: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)) },
        ];

        // Draw border rectangle using lines
        ops.extend(self.line_segment_ops(start_x, start_y, end_x, start_y));
        ops.extend(self.line_segment_ops(end_x, start_y, end_x, end_y));
        ops.extend(self.line_segment_ops(end_x, end_y, start_x, end_y));
        ops.extend(self.line_segment_ops(start_x, end_y, start_x, start_y));

        ops
    }

    /// Build approval table operations
    fn build_approval_table_ops(&self, font_id: &FontId) -> Vec<Op> {
        let start_x = 155.0_f32;
        let start_y = (self.layout.page_height - 25.0) as f32;
        let col_width = 15.0_f32;
        let row_height1 = 5.0_f32;
        let row_height2 = 15.0_f32;

        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];

        let headers = ["社　長", "会　計", "所　属"];
        for (i, header) in headers.iter().enumerate() {
            let x = start_x + (i as f32) * col_width;

            // Header cell
            ops.extend(self.rect_ops(x, start_y, col_width, row_height1));
            ops.extend(self.text_centered_ops(font_id, 9.0, header, x, start_y - 4.0, col_width));

            // Data cell (empty)
            ops.extend(self.rect_ops(x, start_y - row_height1, col_width, row_height2));
        }

        ops
    }

    /// Build base data operations (title, date, office)
    fn build_base_data_ops(&self, font_id: &FontId, item: &Item) -> Vec<Op> {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 15.0) as f32;
        let mut ops = Vec::new();

        // Title
        let title = "出 張 旅 費 日 当 駐 車 料 込 精 算 書";
        ops.extend(self.text_ops(font_id, 14.0, title, start_x + 13.0, start_y - 5.0));

        // Title underlines (2 lines) - 「精算書」の下で止める
        let title_width = 81.0; // 固定幅で調整 (1cm短く)
        ops.extend(self.line_segment_ops(
            start_x + 13.0,
            start_y - 6.0,
            start_x + 13.0 + title_width,
            start_y - 6.0,
        ));
        ops.extend(self.line_segment_ops(
            start_x + 13.0,
            start_y - 7.0,
            start_x + 13.0 + title_width,
            start_y - 7.0,
        ));

        // Settlement date
        if let Some(ref pay_day) = item.pay_day {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(pay_day, "%Y-%m-%d") {
                let pay_day_str = date.format("清算日　%Y年 %m月 %d日").to_string();
                ops.extend(self.text_ops(font_id, 9.0, &pay_day_str, start_x + 110.0, start_y - 5.0));
            }
        }

        // Office (right side)
        if let Some(ref office) = item.office {
            let text_width = self.estimate_text_width(office, 10.0);
            ops.extend(self.text_ops(
                font_id,
                10.0,
                office,
                start_x + 190.0 - text_width - 2.0,
                start_y - 5.0,
            ));
        }

        ops
    }

    /// Build basic info table operations
    fn build_basic_info_table_ops(&self, font_id: &FontId) -> Vec<Op> {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 30.0) as f32;

        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];

        // Departure/Return labels (left side)
        let row_height = 3.5_f32;
        let diff_start_y = 3.0_f32;
        ops.extend(self.text_ops(font_id, 9.0, "出発", start_x + 1.0, start_y - diff_start_y));
        ops.extend(self.text_ops(
            font_id,
            9.0,
            "　　月　　日",
            start_x + 2.0,
            start_y - diff_start_y - row_height,
        ));
        ops.extend(self.text_ops(
            font_id,
            9.0,
            "帰着",
            start_x + 1.0,
            start_y - diff_start_y - row_height * 2.0,
        ));
        ops.extend(self.text_ops(
            font_id,
            9.0,
            "　　月　　日",
            start_x + 2.0,
            start_y - diff_start_y - row_height * 3.0,
        ));

        // Table headers
        let headers = ["", "出張目的", "車両No.", "氏　名", "サイン"];
        let col_widths: [f32; 5] = [31.0, 25.0, 28.75, 30.0, 30.25]; // サイン列調整

        let mut current_x = start_x;
        for (i, header) in headers.iter().enumerate() {
            ops.extend(self.rect_ops(current_x, start_y, col_widths[i], 15.0));
            if !header.is_empty() {
                ops.extend(self.text_ops(font_id, 9.0, header, current_x + 1.0, start_y - 4.0));
            }
            current_x += col_widths[i];
        }

        ops
    }

    /// Build main data table operations
    fn build_main_data_table_ops(&self, font_id: &FontId) -> Vec<Op> {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 45.0) as f32;

        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];

        // 日付列の幅を2mm広げて右線を右にずらす (10.0→12.0)
        let col_widths: [f32; 9] = [12.0, 17.0, 40.0, 30.0, 15.0, 15.0, 15.0, 25.0, 21.0];
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
            ops.extend(self.rect_ops(current_x, start_y, col_widths[i], header_height));
            // 列ごとのオフセット
            let text_offset = match i {
                4 => 2.0,  // 交通機関
                5 => 1.0,  // 運賃
                6 => 2.0,  // 特別料金
                7 => 1.0,  // 旅費日当
                _ => 0.0,
            };
            ops.extend(self.text_centered_ops(
                font_id,
                8.0,
                header,
                current_x + text_offset,
                start_y - 3.0,
                col_widths[i],
            ));
            current_x += col_widths[i];
        }

        // Data rows (7 rows)
        for row in 0..7 {
            current_x = start_x;
            let current_y = start_y - header_height - (row as f32) * row_height;

            for (col, &width) in col_widths.iter().enumerate() {
                if col == 2 {
                    // 摘要欄は左右の線のみ
                    ops.extend(self.line_segment_ops(current_x, current_y, current_x, current_y - row_height));
                    ops.extend(self.line_segment_ops(
                        current_x + width,
                        current_y,
                        current_x + width,
                        current_y - row_height,
                    ));
                } else {
                    ops.extend(self.rect_ops(current_x, current_y, width, row_height));
                }
                current_x += width;
            }
        }

        ops
    }

    /// Build summary table operations
    fn build_summary_table_ops(&self, font_id: &FontId) -> Vec<Op> {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 119.0) as f32;

        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];

        let col_widths = [145.0_f32, 45.0_f32];
        let row_height = 19.0_f32;
        let headers = ["備考", "計"];

        let mut current_x = start_x;
        for (i, header) in headers.iter().enumerate() {
            ops.extend(self.rect_ops(current_x, start_y, col_widths[i], row_height));
            ops.extend(self.text_ops(font_id, 8.0, header, current_x + 2.0, start_y - 4.0));
            current_x += col_widths[i];
        }

        ops
    }

    /// Build item data operations
    fn build_item_ops(&self, font_id: &FontId, item: &Item) -> Vec<Op> {
        let start_x = 14.0_f32;
        let start_y = (self.layout.page_height - 36.8) as f32;
        let mut ops = Vec::new();

        // Start date
        if let Some(ref start_date) = item.start_date {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d") {
                let date_str = date.format("%m　 %d").to_string();
                ops.extend(self.text_ops(font_id, 10.0, &date_str, start_x, start_y));
            }
        }

        // End date
        if let Some(ref end_date) = item.end_date {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d") {
                let date_str = date.format("%m　 %d").to_string();
                ops.extend(self.text_ops(font_id, 10.0, &date_str, start_x, start_y - 7.0));
            }
        }

        // Purpose
        if let Some(ref purpose) = item.purpose {
            ops.extend(self.text_ops(font_id, 10.0, purpose, start_x + 32.0, start_y - 7.0));
        }

        // Car
        if !item.car.is_empty() {
            ops.extend(self.text_ops(font_id, 10.0, &item.car, start_x + 52.0, start_y - 7.0));
        }

        // Name
        if !item.name.is_empty() {
            ops.extend(self.text_ops(font_id, 10.0, &item.name, start_x + 85.0, start_y - 7.0));
        }

        // Total price in summary table (計セルの右側)
        let price_str = format_price(item.price);
        let text_width = self.estimate_text_width(&price_str, 10.0);
        let summary_table_y = (self.layout.page_height - 119.0) as f32;
        let summary_cell_x = 10.0 + 145.0; // 計セルの開始位置
        let summary_cell_width = 45.0_f32;
        ops.extend(self.text_ops(
            font_id,
            10.0,
            &price_str,
            summary_cell_x + summary_cell_width - text_width - 2.0 - 10.0, // 1cm左
            summary_table_y - 12.0 + 10.0 - 4.0 + 2.0, // 2mm上
        ));

        // Print ryohi items
        ops.extend(self.build_ryohi_ops(font_id, &item.ryohi));

        ops
    }

    /// Build ryohi items operations
    fn build_ryohi_ops(
        &self,
        font_id: &FontId,
        ryohi_list: &[crate::models::Ryohi],
    ) -> Vec<Op> {
        let start_x = 10.0_f32;
        let start_y = (self.layout.page_height - 47.0) as f32;
        // 日付列の幅を2mm広げて右線を右にずらす (10.0→12.0)
        let col_widths: [f32; 9] = [12.0, 17.0, 40.0, 30.0, 15.0, 15.0, 15.0, 25.0, 21.0];
        let row_height = 10.0_f32;

        let mut ops = Vec::new();
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

                // Date (2mm右にオフセット)
                if row < print_data.date_lines.len() && !print_data.date_lines[row].is_empty() {
                    let date = &print_data.date_lines[row];
                    ops.extend(self.text_centered_ops(
                        font_id,
                        10.0,
                        date,
                        current_x + 2.0,
                        current_y - 6.0,
                        col_widths[0],
                    ));
                }
                current_x += col_widths[0];

                // Dest
                if row < print_data.dest_lines.len() && !print_data.dest_lines[row].is_empty() {
                    let dest = &print_data.dest_lines[row];
                    ops.extend(self.text_centered_ops(
                        font_id,
                        10.0,
                        dest,
                        current_x,
                        current_y - 6.0,
                        col_widths[1],
                    ));
                }
                current_x += col_widths[1];

                // Detail
                if row < print_data.detail_lines.len() && !print_data.detail_lines[row].is_empty() {
                    let detail = &print_data.detail_lines[row];
                    ops.extend(self.text_ops(font_id, 10.0, detail, current_x + 1.0, current_y - 6.0));
                }
                current_x += col_widths[2];

                // Kukan
                if row < print_data.kukan_lines.len() && !print_data.kukan_lines[row].is_empty() {
                    let kukan = &print_data.kukan_lines[row];
                    ops.extend(self.text_ops(font_id, 10.0, kukan, current_x + 1.0, current_y - 6.0));
                }
                current_x += col_widths[3];

                // Skip 交通機関, 運賃, 特別料金
                current_x += col_widths[4] + col_widths[5] + col_widths[6];

                // Price
                if row < print_data.price_lines.len() && !print_data.price_lines[row].is_empty() {
                    let price_str = &print_data.price_lines[row];
                    let text_width = self.estimate_text_width(price_str, 10.0);
                    ops.extend(self.text_ops(
                        font_id,
                        10.0,
                        price_str,
                        current_x + col_widths[7] - text_width - 1.0,
                        current_y - 6.0,
                    ));
                }
                current_x += col_widths[7];

                // Vol
                if row < print_data.vol_lines.len() && !print_data.vol_lines[row].is_empty() {
                    let vol_str = &print_data.vol_lines[row];
                    let text_width = self.estimate_text_width(vol_str, 10.0);
                    ops.extend(self.text_ops(
                        font_id,
                        10.0,
                        vol_str,
                        current_x + col_widths[8] - text_width - 1.0,
                        current_y - 6.0,
                    ));
                }

                drawn_rows += 1;
            }

            current_row += drawn_rows;
        }

        ops
    }

    // Helper methods

    fn rect_ops(&self, x: f32, y: f32, width: f32, height: f32) -> Vec<Op> {
        // Draw rectangle using 4 lines
        let mut ops = Vec::new();
        ops.extend(self.line_segment_ops(x, y, x + width, y));
        ops.extend(self.line_segment_ops(x + width, y, x + width, y - height));
        ops.extend(self.line_segment_ops(x + width, y - height, x, y - height));
        ops.extend(self.line_segment_ops(x, y - height, x, y));
        ops
    }

    fn line_segment_ops(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> Vec<Op> {
        let points = vec![
            LinePoint { p: Point::new(Mm(x1), Mm(y1)), bezier: false },
            LinePoint { p: Point::new(Mm(x2), Mm(y2)), bezier: false },
        ];
        let line = Line {
            points,
            is_closed: false,
        };
        vec![Op::DrawLine { line }]
    }

    fn text_ops(&self, font_id: &FontId, size: f32, text: &str, x: f32, y: f32) -> Vec<Op> {
        vec![
            Op::StartTextSection,
            Op::SetFontSize {
                font: font_id.clone(),
                size: Pt(size),
            },
            Op::SetTextCursor { pos: Point::new(Mm(x), Mm(y)) },
            Op::WriteText {
                font: font_id.clone(),
                items: vec![TextItem::Text(text.to_string())],
            },
            Op::EndTextSection,
        ]
    }

    fn text_centered_ops(
        &self,
        font_id: &FontId,
        size: f32,
        text: &str,
        x: f32,
        y: f32,
        width: f32,
    ) -> Vec<Op> {
        let text_width = self.estimate_text_width(text, size);
        let centered_x = x + (width - text_width) / 2.0;
        self.text_ops(font_id, size, text, centered_x, y)
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
