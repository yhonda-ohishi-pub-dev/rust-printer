use serde::{Deserialize, Serialize};

/// Ryohi represents the expense data structure (旅費明細)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Ryohi {
    pub date: Option<String>,
    #[serde(default)]
    pub date_ar: Vec<String>,
    pub dest: Option<String>,
    #[serde(default)]
    pub dest_ar: Vec<String>,
    #[serde(default)]
    pub detail: Vec<String>,
    pub kukan: Option<String>,
    #[serde(default)]
    pub kukan_sprit: Vec<String>,
    pub price: Option<i32>,
    #[serde(default)]
    pub price_ar: Vec<i32>,
    pub vol: Option<f64>,
    #[serde(default)]
    pub vol_ar: Vec<f64>,
    #[serde(default)]
    pub print_detail: Vec<String>,
    #[serde(default)]
    pub print_detail_row: i32,
    #[serde(default)]
    pub print_kukan: Vec<String>,
    #[serde(default)]
    pub print_kukan_row: i32,
    #[serde(default)]
    pub max_row: i32,
    #[serde(default)]
    pub page_count: i32,
}

/// Item represents the main item data structure (旅費精算書1件)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub car: String,
    pub name: String,
    pub purpose: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default)]
    pub price: i32,
    pub tax: Option<f64>,
    pub description: Option<String>,
    #[serde(default)]
    pub ryohi: Vec<Ryohi>,
    pub office: Option<String>,
    pub pay_day: Option<String>,
}

/// Format price with comma separator (例: 12345 -> "12,345")
pub fn format_price(price: i32) -> String {
    if price == 0 {
        return String::new();
    }

    let is_negative = price < 0;
    let mut num = price.abs();
    let mut digits = Vec::new();

    while num > 0 {
        digits.push((num % 10) as u8 + b'0');
        num /= 10;
    }

    if digits.is_empty() {
        return "0".to_string();
    }

    digits.reverse();

    let mut result = String::new();
    if is_negative {
        result.push('-');
    }

    for (i, &digit) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(digit as char);
    }

    result
}

/// Parse date string and return formatted date (例: "2024-01-15" -> "01　15")
pub fn parse_date(date_str: &str) -> Option<String> {
    if date_str.is_empty() {
        return None;
    }

    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    Some(date.format("%m　%d").to_string())
}

/// Parse pay day and return formatted date (例: "2024-01-15" -> "2024  01　15")
pub fn parse_pay_day(date_str: &str) -> Option<String> {
    if date_str.is_empty() {
        return None;
    }

    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    Some(date.format("%Y  %m　%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_price() {
        assert_eq!(format_price(0), "");
        assert_eq!(format_price(100), "100");
        assert_eq!(format_price(1000), "1,000");
        assert_eq!(format_price(12345), "12,345");
        assert_eq!(format_price(1234567), "1,234,567");
        assert_eq!(format_price(-1234), "-1,234");
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(parse_date("2024-01-15"), Some("01　15".to_string()));
        assert_eq!(parse_date(""), None);
    }

    #[test]
    fn test_parse_pay_day() {
        assert_eq!(
            parse_pay_day("2024-01-15"),
            Some("2024  01　15".to_string())
        );
        assert_eq!(parse_pay_day(""), None);
    }
}
