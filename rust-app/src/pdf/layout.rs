/// Page layout constants for A5 Landscape (210mm x 148mm)
#[derive(Debug, Clone)]
pub struct PageLayout {
    /// Page width in mm (A5 landscape)
    pub page_width: f64,
    /// Page height in mm (A5 landscape)
    pub page_height: f64,
    /// Left margin
    pub left_margin: f64,
    /// Top margin (from top)
    pub top_margin: f64,
    /// Right margin
    pub right_margin: f64,
    /// Bottom margin
    pub bottom_margin: f64,
    /// Column widths for basic info table
    pub basic_col_widths: [f64; 5],
    /// Column widths for main data table
    pub main_col_widths: [f64; 9],
    /// Row height for basic info table
    pub row_height: f64,
    /// Table top Y position
    pub table_top_y: f64,
}

impl Default for PageLayout {
    fn default() -> Self {
        Self {
            page_width: 210.0,
            page_height: 148.0,
            left_margin: 10.0,
            top_margin: 138.0,
            right_margin: 200.0,
            bottom_margin: 10.0,
            // Basic info table columns: [出発帰着, 出張目的, 車両No., 氏名, サイン]
            basic_col_widths: [31.0, 25.0, 28.75, 30.0, 30.0],
            // Main data table columns: [日付, 行先, 摘要, 区間, 交通機関, 運賃, 特別料金, 旅費日当, 計]
            main_col_widths: [10.0, 17.0, 40.0, 30.0, 15.0, 15.0, 15.0, 25.0, 23.0],
            row_height: 15.0,
            table_top_y: 95.0,
        }
    }
}

/// Text wrapping result
#[derive(Debug, Clone)]
pub struct TextWrapResult {
    pub lines: Vec<String>,
    pub row_count: usize,
}

/// Wrap detail text with maximum length per line
pub fn wrap_detail(details: &[String], max_len: usize) -> TextWrapResult {
    if details.is_empty() {
        return TextWrapResult {
            lines: vec![],
            row_count: 0,
        };
    }

    let mut result = Vec::new();
    let mut current_line = String::new();

    for detail in details {
        let separator = if current_line.is_empty() { "" } else { "、" };
        let new_line_length =
            current_line.chars().count() + separator.chars().count() + detail.chars().count();

        if new_line_length <= max_len {
            current_line.push_str(separator);
            current_line.push_str(detail);
        } else {
            if !current_line.is_empty() {
                result.push(current_line);
                current_line = String::new();
            }

            if detail.chars().count() > max_len {
                current_line = detail.chars().take(max_len).collect();
            } else {
                current_line = detail.clone();
            }
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    // Filter empty lines
    let filtered: Vec<String> = result
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect();

    let row_count = filtered.len();
    TextWrapResult {
        lines: filtered,
        row_count,
    }
}

/// Wrap kukan (route) text with maximum length per line
pub fn wrap_kukan(kukan: &str, max_len: usize) -> TextWrapResult {
    if kukan.is_empty() {
        return TextWrapResult {
            lines: vec![String::new()],
            row_count: 1,
        };
    }

    // Replace special strings
    let mut kukan = kukan.replace("_九州外空車適用", "　九州外空車適用");
    kukan = kukan.replace("適用*   追加", "適用*　追加");
    // Convert half-width space to full-width
    kukan = kukan.replace(' ', "　");

    // Split by delimiters (full-width space, ｜, etc.)
    let re = regex::Regex::new(r"[　｜]| \||\|").unwrap();
    let parts: Vec<&str> = re.split(&kukan).collect();

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_count = 0;

    for part in parts {
        let part_len = part.chars().count();

        if current_count != 0 && current_count + part_len == max_len {
            current_line.push_str(part);
            result.push(current_line);
            current_line = String::new();
            current_count = 0;
        } else if part_len == max_len && current_line.is_empty() {
            result.push(part.to_string());
            current_count = 0;
        } else if part_len > max_len {
            result.push("exceed*".to_string());
            current_count = 0;
        } else if current_count + part_len + 1 > max_len {
            if !current_line.is_empty() {
                result.push(current_line);
            }
            current_line = format!("{}　", part);
            current_count = part_len + 1;
        } else {
            current_count += part_len + 1;
            current_line.push_str(part);
            current_line.push('　');
        }
    }

    if current_count != 0 {
        result.push(current_line);
    }

    // Trim full-width spaces
    let result: Vec<String> = result
        .into_iter()
        .map(|line| {
            let line = line.replace(' ', "　");
            line.trim_start_matches('　')
                .trim_end_matches('　')
                .to_string()
        })
        .collect();

    let row_count = result.len();
    TextWrapResult {
        lines: result,
        row_count,
    }
}

/// Ryohi print data for rendering
#[derive(Debug, Clone)]
pub struct RyohiPrintData {
    pub date_lines: Vec<String>,
    pub dest_lines: Vec<String>,
    pub detail_lines: Vec<String>,
    pub kukan_lines: Vec<String>,
    pub price_lines: Vec<String>,
    pub vol_lines: Vec<String>,
    pub max_rows: usize,
}

impl RyohiPrintData {
    /// Check if there is content in the specified row
    pub fn has_content_in_row(&self, row: usize) -> bool {
        if row >= self.date_lines.len()
            && row >= self.dest_lines.len()
            && row >= self.detail_lines.len()
            && row >= self.kukan_lines.len()
            && row >= self.price_lines.len()
            && row >= self.vol_lines.len()
        {
            return false;
        }

        self.date_lines
            .get(row)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
            || self
                .dest_lines
                .get(row)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            || self
                .detail_lines
                .get(row)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            || self
                .kukan_lines
                .get(row)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            || self
                .price_lines
                .get(row)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            || self
                .vol_lines
                .get(row)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
    }
}

/// Extend array to max rows with empty strings
fn extend_to_max_rows(lines: &[String], max_rows: usize) -> Vec<String> {
    let filtered: Vec<String> = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .cloned()
        .collect();

    let mut result = vec![String::new(); max_rows];
    for (i, line) in filtered.into_iter().enumerate() {
        if i < max_rows {
            result[i] = line;
        }
    }
    result
}

/// Align single values to max rows
fn align_rows(
    date: Option<&str>,
    dest: Option<&str>,
    price: Option<i32>,
    vol: Option<f64>,
    max_rows: usize,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut date_arr = vec![String::new(); max_rows];
    let mut dest_arr = vec![String::new(); max_rows];
    let mut price_arr = vec![String::new(); max_rows];
    let mut vol_arr = vec![String::new(); max_rows];

    // First row gets actual values
    if let Some(d) = date {
        // Convert YYYY-MM-DD to MM/DD
        if d.len() >= 10 && d.chars().nth(4) == Some('-') && d.chars().nth(7) == Some('-') {
            let month = &d[5..7];
            let day = &d[8..10];
            date_arr[0] = format!("{}/{}", month, day);
        } else {
            date_arr[0] = d.to_string();
        }
    }

    if let Some(d) = dest {
        dest_arr[0] = d.to_string();
    }

    if let Some(p) = price {
        price_arr[0] = crate::models::item::format_price(p);
    }

    if let Some(v) = vol {
        vol_arr[0] = format!("{:.1}", v);
    }

    (date_arr, dest_arr, price_arr, vol_arr)
}

use crate::models::Ryohi;

/// Prepare Ryohi data for printing
pub fn prepare_ryohi_for_print(
    ryohi: &Ryohi,
    max_detail_len: usize,
    max_kukan_len: usize,
) -> RyohiPrintData {
    // Wrap detail
    let detail_result = if !ryohi.detail.is_empty() {
        wrap_detail(&ryohi.detail, max_detail_len)
    } else {
        TextWrapResult {
            lines: vec![String::new()],
            row_count: 1,
        }
    };

    // Wrap kukan
    let kukan_result = if let Some(ref k) = ryohi.kukan {
        wrap_kukan(k, max_kukan_len)
    } else {
        TextWrapResult {
            lines: vec![String::new()],
            row_count: 1,
        }
    };

    // Determine max rows
    let max_rows = detail_result.row_count.max(kukan_result.row_count);

    // Align other data to max rows
    let (date_lines, dest_lines, price_lines, vol_lines) = align_rows(
        ryohi.date.as_deref(),
        ryohi.dest.as_deref(),
        ryohi.price,
        ryohi.vol,
        max_rows,
    );

    // Extend all arrays to max rows
    let detail_lines = extend_to_max_rows(&detail_result.lines, max_rows);
    let kukan_lines = extend_to_max_rows(&kukan_result.lines, max_rows);

    RyohiPrintData {
        date_lines,
        dest_lines,
        detail_lines,
        kukan_lines,
        price_lines,
        vol_lines,
        max_rows,
    }
}
