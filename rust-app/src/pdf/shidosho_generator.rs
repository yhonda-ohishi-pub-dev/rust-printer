use anyhow::Result;
use printpdf::*;

use crate::models::shidosho::{ShidoshoPage, SummaryPage, ViolationDetail};

/// 指導書PDF生成器
pub struct ShidoshoPdfGenerator {
    font_bytes: Vec<u8>,
    page_width: f64,
    page_height: f64,
}

impl ShidoshoPdfGenerator {
    pub fn new(font_bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            font_bytes,
            page_width: 210.0,  // A5 横向き (幅)
            page_height: 148.0, // A5 横向き (高さ)
        })
    }

    /// 指導書PDFを生成
    pub fn generate(&self, title: &str, pages: &[ShidoshoPage], summary_pages: &[SummaryPage]) -> Result<Vec<u8>> {
        if pages.is_empty() {
            anyhow::bail!("No pages to generate");
        }

        let mut doc = PdfDocument::new(title);
        let font = ParsedFont::from_bytes(&self.font_bytes, 0, &mut Vec::new())
            .ok_or_else(|| anyhow::anyhow!("Failed to parse font"))?;
        let font_id = doc.add_font(&font);

        let mut pdf_pages = Vec::new();

        // 指導書ページ: 1件ずつA5横向きで配置
        for page_data in pages {
            let ops = self.build_page_ops(&font_id, page_data)?;
            let page = PdfPage::new(
                Mm(self.page_width as f32),
                Mm(self.page_height as f32),
                ops,
            );
            pdf_pages.push(page);
        }

        // 一覧表ページ (各会社ごとに1ページ)
        for summary in summary_pages {
            let summary_ops = self.build_summary_page_ops(&font_id, summary)?;
            let summary_page = PdfPage::new(
                Mm(self.page_width as f32),
                Mm(self.page_height as f32),
                summary_ops,
            );
            pdf_pages.push(summary_page);
        }

        let pdf_bytes = doc.with_pages(pdf_pages).save(
            &PdfSaveOptions::default(),
            &mut Vec::new(),
        );

        Ok(pdf_bytes)
    }

    /// 1ページ分の描画オペレーション (A5横向き1件)
    fn build_page_ops(&self, font_id: &FontId, page: &ShidoshoPage) -> Result<Vec<Op>> {
        let mut ops = Vec::new();

        // ページ罫線
        ops.extend(self.build_frame_ops());

        // 指導書内容を描画
        ops.extend(self.build_shidosho_ops(font_id, page)?);

        Ok(ops)
    }

    /// 枠線描画 (A5横向き: 210x148mm)
    fn build_frame_ops(&self) -> Vec<Op> {
        let mut ops = vec![
            Op::SetOutlineThickness { pt: Pt(0.5) },
            Op::SetOutlineColor { col: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)) },
        ];

        let top = 131.0;    // 上端から17mm余白 (148 - 17 = 131)
        let bottom = 10.0;  // 下端から10mm余白
        let left = 10.0;    // 左端から10mm余白
        let right = 200.0;  // 右端から10mm余白

        // 外枠
        ops.extend(self.line_segment_ops(left, top, right, top));       // 上
        ops.extend(self.line_segment_ops(left, bottom, right, bottom)); // 下
        ops.extend(self.line_segment_ops(left, top, left, bottom));     // 左
        ops.extend(self.line_segment_ops(right, top, right, bottom));   // 右

        ops
    }

    /// 1件分の指導書描画 (A5横向き: 210x148mm)
    /// レイアウト: 上から順に配置
    /// - 日付(左上) / 会社名(右上)
    /// - タイトル / 運行管理・配車(右)
    /// - 「下記の内容について...」
    /// - 基本情報テーブル / 署名欄(右)
    /// - 行程 / 違反内容サマリ / 指導者コメント(右)
    /// - 違反詳細(行程の下から)
    fn build_shidosho_ops(&self, font_id: &FontId, page: &ShidoshoPage) -> Result<Vec<Op>> {
        let mut ops = Vec::new();

        // Y座標: 上から下へ (148mmがページ高さ、上端=148, 下端=0)
        // 日付・会社名は枠線(131mm)のすぐ上に表示
        let date_y = 132.0_f32;  // 枠線(131mm)のすぐ上

        // 日付 (左上) - 枠線の上
        ops.extend(self.text_ops(font_id, 9.0, &page.date, 11.0, date_y));
        // 会社名 (右上) - 枠線の上
        ops.extend(self.text_ops(font_id, 9.0, &page.firm_name, 150.0, date_y));

        // タイトル - 枠線(131mm)のすぐ下
        let title_y = 126.0_f32;
        ops.extend(self.text_ops(font_id, 14.0, "運行記録計による指導記録", 11.0, title_y));

        // 押印欄 (運行管理・配車) - 枠線上端に合わせる
        ops.extend(self.build_stamp_area_ops(font_id, 131.0)?);

        // 基本情報テーブル (ヘッダー行) - タイトルのすぐ下 (Y=125)
        let header_y = 125.0_f32;
        ops.extend(self.build_basic_info_ops(font_id, page, header_y)?);

        // 署名欄 (基本情報の右、押印欄と同じY座標から)
        ops.extend(self.build_signature_ops(font_id, 131.0)?);

        // 「下記の内容について、指導を受け、承知いたしました。」- 基本情報テーブルのすぐ下
        // 基本情報テーブルは2行 (row_height=5.0 x 2 = 10mm)、下端は header_y - 10 = 115
        let shita_y = header_y - 12.0 - 1.0;  // 125 - 12 - 1 = 112 (テーブル下端から1mm下)
        ops.extend(self.text_ops(font_id, 9.0, "下記の内容について、指導を受け、承知いたしました。", 11.0, shita_y));

        let mut y = shita_y - 5.0;  // 108

        // 行程 / 違反内容サマリ / 指導者コメント を横に並べる
        let content_y = y;

        // 行程 (左)
        ops.extend(self.build_itinerary_ops(font_id, page, content_y)?);

        // 違反内容サマリ (中央)
        ops.extend(self.build_violations_ops(font_id, page, content_y)?);

        // 指導者コメント欄 (右)
        ops.extend(self.build_comment_area_ops(font_id, content_y)?);

        // 違反詳細 (違反サマリの下)
        let violations_count = page.violations.len();
        let details_y = content_y - 4.0 - (violations_count as f32 + 1.0) * 4.0;
        ops.extend(self.build_violation_details_ops(font_id, page, details_y)?);

        Ok(ops)
    }

    /// 基本情報テーブル (4列: 運行課/乗務員/運行開始/車番)
    fn build_basic_info_ops(&self, font_id: &FontId, page: &ShidoshoPage, start_y: f32) -> Result<Vec<Op>> {
        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];
        let row_height = 5.0_f32;
        let start_x = 11.0_f32;

        // 列幅を個別設定
        let col_widths: [f32; 4] = [24.0, 24.0, 24.0, 28.0];  // 運行課, 乗務員, 運行開始, 車番

        let headers = ["運行課", "乗務員", "運行開始", "車番"];
        let values = [
            page.driver_bunrui.as_str(),
            page.driver_name.as_str(),
            page.syukko_date.as_str(),
            page.car_name.as_str(),
        ];

        // ヘッダー行
        let mut x = start_x;
        for (i, header) in headers.iter().enumerate() {
            ops.extend(self.rect_ops(x, start_y, col_widths[i], row_height));
            ops.extend(self.text_centered_ops(font_id, 9.0, header, x, start_y - 3.8, col_widths[i]));
            x += col_widths[i];
        }

        // データ行
        let mut x = start_x;
        for (i, value) in values.iter().enumerate() {
            ops.extend(self.rect_ops(x, start_y - row_height, col_widths[i], row_height));
            ops.extend(self.text_centered_ops(font_id, 9.0, value, x, start_y - row_height - 3.8, col_widths[i]));
            x += col_widths[i];
        }

        Ok(ops)
    }

    /// 押印欄 (右上に配置、枠線右端に合わせる、正方形)
    fn build_stamp_area_ops(&self, font_id: &FontId, start_y: f32) -> Result<Vec<Op>> {
        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];
        let col_width = 17.0_f32;
        let start_x = 200.0 - col_width * 2.0;  // 枠線右端(200mm)に合わせる
        let header_height = 4.0_f32;
        let cell_height = 17.0_f32;  // 正方形: 17mm x 17mm (headerは別)

        // 運行管理
        ops.extend(self.rect_ops(start_x, start_y, col_width, header_height));
        ops.extend(self.text_centered_ops(font_id, 9.0, "運行管理", start_x, start_y - 3.0, col_width));
        ops.extend(self.rect_ops(start_x, start_y - header_height, col_width, cell_height));

        // 配車
        ops.extend(self.rect_ops(start_x + col_width, start_y, col_width, header_height));
        ops.extend(self.text_centered_ops(font_id, 9.0, "配車", start_x + col_width, start_y - 3.0, col_width));
        ops.extend(self.rect_ops(start_x + col_width, start_y - header_height, col_width, cell_height));

        Ok(ops)
    }

    /// 署名欄 (基本情報テーブルの右、押印欄の標題下線〜下線に合わせる)
    fn build_signature_ops(&self, font_id: &FontId, start_y: f32) -> Result<Vec<Op>> {
        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];
        let start_x = 112.0_f32;  // 基本情報右端(111) + 1mm余白
        let width = 53.0_f32;     // 押印欄(166) - 1mm余白 - 112 = 53
        // 押印欄の標題下線(start_y - 4)から下線(start_y - 21)に合わせる
        let sig_start_y = start_y - 4.0;  // 押印欄標題の下線位置
        let height = 17.0_f32;    // 押印セルと同じ高さ

        ops.extend(self.rect_filled_gray_ops(start_x, sig_start_y, width, height));
        ops.extend(self.text_ops(font_id, 9.0, "署名", start_x + 1.0, sig_start_y - 3.5));

        Ok(ops)
    }

    /// 行程 (左側)
    fn build_itinerary_ops(&self, font_id: &FontId, page: &ShidoshoPage, start_y: f32) -> Result<Vec<Op>> {
        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];
        let start_x = 11.0_f32;
        let row_height = 4.5_f32;
        let max_rows = 18;  // A5に収まる行数
        let loc_width = 28.0_f32;  // 場所名の列幅
        let type_width = 8.0_f32;  // 種別の列幅

        // ヘッダー
        ops.extend(self.rect_ops(start_x, start_y, loc_width + type_width, row_height));
        ops.extend(self.text_centered_ops(font_id, 9.0, "行程", start_x, start_y - 3.5, loc_width + type_width));

        let display_count = page.itinerary.len().min(max_rows);
        for (i, item) in page.itinerary.iter().take(max_rows).enumerate() {
            let y = start_y - row_height - (i as f32) * row_height;
            ops.extend(self.rect_ops(start_x, y, loc_width, row_height));
            // 場所名を短縮
            let location = Self::truncate_text(&item.location, 8);
            ops.extend(self.text_ops(font_id, 9.0, &location, start_x + 0.5, y - 3.5));
            ops.extend(self.rect_ops(start_x + loc_width, y, type_width, row_height));
            ops.extend(self.text_centered_ops(font_id, 9.0, &item.item_type, start_x + loc_width, y - 3.5, type_width));
        }

        // 「他N件」表示
        if page.itinerary.len() > max_rows {
            let y = start_y - row_height - (display_count as f32) * row_height;
            let text = format!("他{}件", page.itinerary.len() - max_rows);
            ops.extend(self.text_ops(font_id, 9.0, &text, start_x, y - 3.5));
        }

        Ok(ops)
    }

    /// 違反内容サマリ (中央)
    fn build_violations_ops(&self, font_id: &FontId, page: &ShidoshoPage, start_y: f32) -> Result<Vec<Op>> {
        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];
        let start_x = 48.0_f32;  // 行程の右 (11 + 28 + 8 = 47, + 1mm余白)
        let col_width = 35.0_f32;  // 違反内容・諸元の列幅
        let row_height = 4.5_f32;

        // ヘッダー
        ops.extend(self.rect_ops(start_x, start_y, col_width, row_height));
        ops.extend(self.text_centered_ops(font_id, 9.0, "違反内容", start_x, start_y - 3.5, col_width));
        ops.extend(self.rect_ops(start_x + col_width, start_y, col_width, row_height));
        ops.extend(self.text_centered_ops(font_id, 9.0, "諸元", start_x + col_width, start_y - 3.5, col_width));

        let mut current_y = start_y - row_height;

        // 違反内容を表示 (PHPと同じ: str_replace("道", "", str_replace("オーバー", "超過", $f_key)))
        let violation_keys = [
            ("高速道速度オーバー回数", "高速速度超過回数", "回"),
            ("高速道速度オーバー最大値", "高速速度超過最大値", "km/h"),
            ("専用道速度オーバー回数", "専用速度超過回数", "回"),
            ("専用道速度オーバー最大値", "専用速度超過最大値", "km/h"),
            ("一般道速度オーバー回数", "一般速度超過回数", "回"),
            ("一般道速度オーバー最大値", "一般速度超過最大値", "km/h"),
            ("連続運転回数", "連続運転回数", "回"),
            ("連続運転最大値", "連続運転最大値", ""),
        ];

        for (key, label, unit) in violation_keys.iter() {
            if let Some(value) = page.violations.get(*key) {
                let value_str = if *key == "連続運転最大値" {
                    Self::format_minutes(*value as i32)
                } else if unit.is_empty() {
                    format!("{}", value)
                } else {
                    format!("{}{}", value, unit)
                };
                ops.extend(self.build_violation_summary_row(font_id, label, &value_str, start_x, current_y, col_width));
                current_y -= row_height;
            }
        }

        Ok(ops)
    }

    fn build_violation_summary_row(&self, font_id: &FontId, label: &str, value: &str, start_x: f32, y: f32, col_width: f32) -> Vec<Op> {
        let mut ops = Vec::new();
        let row_height = 4.5_f32;

        ops.extend(self.rect_ops(start_x, y, col_width, row_height));
        ops.extend(self.text_centered_ops(font_id, 9.0, label, start_x, y - 3.5, col_width));
        ops.extend(self.rect_ops(start_x + col_width, y, col_width, row_height));
        ops.extend(self.text_centered_ops(font_id, 9.0, value, start_x + col_width, y - 3.5, col_width));

        ops
    }

    /// 違反詳細リスト (違反サマリの下)
    fn build_violation_details_ops(&self, font_id: &FontId, page: &ShidoshoPage, start_y: f32) -> Result<Vec<Op>> {
        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];
        let start_x = 48.0_f32;  // 違反サマリと同じX位置
        let row_height = 4.5_f32;
        let max_rows = 12;

        // 全ての違反詳細を1つのリストにまとめる
        let mut all_details: Vec<&ViolationDetail> = Vec::new();
        for details in page.violation_details.values() {
            all_details.extend(details.iter());
        }

        // 列幅定義 (9pt: 日本語=3.15mm/文字, ASCII=2.25mm/文字)
        let type_w = 8.0_f32;      // 種別: 2文字
        let loc_w = 29.0_f32;      // 地点: 9文字 (truncate)
        let time_w = 18.0_f32;     // 日時: 8文字
        let speed_w = 18.0_f32;    // 速度: 8文字
        let interval_w = 10.0_f32; // 区間時間: 4文字 (3桁+分)

        for (i, detail) in all_details.iter().take(max_rows).enumerate() {
            let y = start_y - (i as f32) * row_height;
            let mut col_x = start_x;

            // 種別
            ops.extend(self.rect_ops(col_x, y, type_w, row_height));
            ops.extend(self.text_centered_ops(font_id, 9.0, &detail.detail_type, col_x, y - 3.5, type_w));
            col_x += type_w;

            // 開始地点 (短縮)
            let start_loc = Self::truncate_text(&detail.start_location, 9);
            ops.extend(self.rect_ops(col_x, y, loc_w, row_height));
            ops.extend(self.text_ops(font_id, 9.0, &start_loc, col_x + 0.5, y - 3.5));
            col_x += loc_w;

            // 終了地点 (短縮)
            let end_loc = Self::truncate_text(&detail.end_location, 9);
            ops.extend(self.rect_ops(col_x, y, loc_w, row_height));
            ops.extend(self.text_ops(font_id, 9.0, &end_loc, col_x + 0.5, y - 3.5));
            col_x += loc_w;

            // 日時
            ops.extend(self.rect_ops(col_x, y, time_w, row_height));
            ops.extend(self.text_centered_ops(font_id, 9.0, &detail.start_time, col_x, y - 3.5, time_w));
            col_x += time_w;

            // 速度 or 連続時間
            if detail.detail_type == "連続" {
                if let Some(ref duration) = detail.duration {
                    ops.extend(self.rect_ops(col_x, y, speed_w + interval_w, row_height));
                    ops.extend(self.text_right_ops(font_id, 9.0, duration, col_x, y - 3.5, speed_w + interval_w));
                }
            } else {
                if let Some(ref speed) = detail.speed {
                    ops.extend(self.rect_ops(col_x, y, speed_w, row_height));
                    ops.extend(self.text_right_ops(font_id, 9.0, speed, col_x, y - 3.5, speed_w));
                }
                col_x += speed_w;
                // 区間時間
                if let Some(ref interval) = detail.interval_time {
                    ops.extend(self.rect_ops(col_x, y, interval_w, row_height));
                    ops.extend(self.text_right_ops(font_id, 9.0, interval, col_x, y - 3.5, interval_w));
                }
            }
        }

        // 「他N件」表示
        if all_details.len() > max_rows {
            let y = start_y - (max_rows as f32) * row_height;
            let text = format!("他{}件", all_details.len() - max_rows);
            ops.extend(self.text_ops(font_id, 9.0, &text, start_x + 40.0, y - 3.5));
        }

        Ok(ops)
    }

    /// 指導者コメント欄 (右側、押印欄の下から枠線下端まで)
    fn build_comment_area_ops(&self, font_id: &FontId, _start_y: f32) -> Result<Vec<Op>> {
        let mut ops = vec![Op::SetOutlineThickness { pt: Pt(0.2) }];
        // 押印欄の下端: 131 - 4(header) - 17(cell) = 110mm
        let comment_top = 110.0_f32;
        let col_width = 17.0_f32;
        let start_x = 200.0 - col_width * 2.0;  // 押印欄と同じX位置（枠線右端に合わせる）
        let width = col_width * 2.0;  // 押印欄と同じ幅
        let height = comment_top - 10.0;  // 枠線下端(10mm)まで

        ops.extend(self.rect_filled_gray_ops(start_x, comment_top, width, height));
        ops.extend(self.text_ops(font_id, 9.0, "指導者コメント", start_x + 1.0, comment_top - 3.5));

        Ok(ops)
    }

    /// 会社別一覧表ページ (A5横向き1会社)
    fn build_summary_page_ops(&self, font_id: &FontId, summary: &SummaryPage) -> Result<Vec<Op>> {
        let mut ops = vec![
            Op::SetOutlineThickness { pt: Pt(0.5) },
            Op::SetOutlineColor { col: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)) },
        ];

        let base_y = 138.0;

        // 会社名と日付
        ops.extend(self.text_ops(font_id, 9.0, &summary.firm_name, 120.0, base_y - 3.0));
        ops.extend(self.text_ops(font_id, 9.0, &summary.date, 165.0, base_y - 3.0));

        // タイトル
        ops.extend(self.text_ops(font_id, 14.0, "運行記録計による指導記録", 15.0, base_y - 12.0));

        // テーブルヘッダー
        let headers = ["運行課", "氏名", "出庫日時", "帰庫日時", "コメント", "署名"];
        let col_widths: [f32; 6] = [20.0, 28.0, 28.0, 28.0, 28.0, 28.0];
        let row_height = 4.5_f32;
        let header_y = base_y - 20.0;
        let mut x = 20.0_f32;

        ops.push(Op::SetOutlineThickness { pt: Pt(0.2) });

        for (i, header) in headers.iter().enumerate() {
            ops.extend(self.rect_ops(x, header_y, col_widths[i], row_height));
            ops.extend(self.text_centered_ops(font_id, 9.0, header, x, header_y - 3.5, col_widths[i]));
            x += col_widths[i];
        }

        // データ行
        for (row_idx, row) in summary.rows.iter().enumerate() {
            let y = header_y - row_height - (row_idx as f32) * row_height;
            let mut x = 20.0_f32;

            let values = [
                row.driver_bunrui.as_str(),
                row.driver_name.as_str(),
                row.syukko_datetime.as_str(),
                row.kiko_datetime.as_str(),
                "", // コメント
                "", // 署名
            ];

            for (i, value) in values.iter().enumerate() {
                ops.extend(self.rect_ops(x, y, col_widths[i], row_height));
                ops.extend(self.text_centered_ops(font_id, 9.0, value, x, y - 3.5, col_widths[i]));
                x += col_widths[i];
            }
        }

        Ok(ops)
    }

    // ヘルパー関数

    fn truncate_text(text: &str, max_chars: usize) -> String {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() <= max_chars {
            text.to_string()
        } else {
            chars[..max_chars].iter().collect()
        }
    }

    fn format_minutes(minutes: i32) -> String {
        if minutes >= 60 {
            format!("{}時間{}分", minutes / 60, minutes % 60)
        } else {
            format!("{}分", minutes)
        }
    }

    fn rect_ops(&self, x: f32, y: f32, width: f32, height: f32) -> Vec<Op> {
        let mut ops = Vec::new();
        ops.extend(self.line_segment_ops(x, y, x + width, y));
        ops.extend(self.line_segment_ops(x + width, y, x + width, y - height));
        ops.extend(self.line_segment_ops(x + width, y - height, x, y - height));
        ops.extend(self.line_segment_ops(x, y - height, x, y));
        ops
    }

    /// グレー背景の矩形 (PHP: setFillColor(240) = 240/255 ≈ 0.94)
    fn rect_filled_gray_ops(&self, x: f32, y: f32, width: f32, height: f32) -> Vec<Op> {
        let gray = 0.94;  // PHP: 240/255
        let points = vec![
            LinePoint { p: Point::new(Mm(x), Mm(y)), bezier: false },
            LinePoint { p: Point::new(Mm(x + width), Mm(y)), bezier: false },
            LinePoint { p: Point::new(Mm(x + width), Mm(y - height)), bezier: false },
            LinePoint { p: Point::new(Mm(x), Mm(y - height)), bezier: false },
        ];
        let polygon = Polygon {
            rings: vec![PolygonRing { points }],
            mode: PaintMode::FillStroke,
            winding_order: WindingOrder::NonZero,
        };
        vec![
            Op::SetFillColor { col: Color::Rgb(Rgb::new(gray, gray, gray, None)) },
            Op::SetOutlineColor { col: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)) },
            Op::DrawPolygon { polygon },
            // 塗りつぶし色を黒に戻す（テキスト用）
            Op::SetFillColor { col: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)) },
        ]
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

    /// グレーテキスト (PHP: setFillColor(240) = 240/255 ≈ 0.94)
    fn text_gray_ops(&self, font_id: &FontId, size: f32, text: &str, x: f32, y: f32) -> Vec<Op> {
        let gray = 0.75;  // 少し濃いめのグレー (0.94だと薄すぎる)
        vec![
            Op::StartTextSection,
            Op::SetFillColor { col: Color::Rgb(Rgb::new(gray, gray, gray, None)) },
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
            // 黒に戻す
            Op::SetFillColor { col: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)) },
        ]
    }

    fn text_centered_ops(&self, font_id: &FontId, size: f32, text: &str, x: f32, y: f32, width: f32) -> Vec<Op> {
        let text_width = self.estimate_text_width(text, size);
        let centered_x = x + (width - text_width) / 2.0;
        tracing::debug!("text_centered: '{}' x={} width={} text_width={} centered_x={}", text, x, width, text_width, centered_x);
        self.text_ops(font_id, size, text, centered_x, y)
    }

    fn text_right_ops(&self, font_id: &FontId, size: f32, text: &str, x: f32, y: f32, width: f32) -> Vec<Op> {
        let text_width = self.estimate_text_width(text, size);
        let right_x = x + width - text_width - 2.5;  // 2.5mm右余白
        tracing::debug!("text_right: '{}' x={} width={} text_width={} right_x={}", text, x, width, text_width, right_x);
        self.text_ops(font_id, size, text, right_x, y)
    }

    fn estimate_text_width(&self, text: &str, size: f32) -> f32 {
        // printpdf: 1pt = 0.3528mm
        let mut width = 0.0_f32;
        for c in text.chars() {
            if c.is_ascii() {
                width += size * 0.18;  // ASCII
            } else {
                width += size * 0.32;  // 日本語
            }
        }
        width
    }
}
