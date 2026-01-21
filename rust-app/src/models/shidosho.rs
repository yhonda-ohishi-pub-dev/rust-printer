use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 指導書作成リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShidoshoRequest {
    pub document_type: String,
    pub title: String,
    pub pages: Vec<ShidoshoPage>,
    pub summary_pages: Vec<SummaryPage>,

    /// 印刷するか
    #[serde(default)]
    pub print: bool,
    /// プリンタIP
    pub printer_ip: Option<String>,
    /// Direct IPP使用
    #[serde(default)]
    pub use_direct_ipp: bool,
    /// 用紙サイズ
    pub paper_size: Option<String>,
    /// カラーモード
    pub color_mode: Option<String>,
}

/// 指導書1ページ分のデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShidoshoPage {
    /// 日付 "26/01/17"
    pub date: String,
    /// 会社名
    pub firm_name: String,
    /// 運行課
    pub driver_bunrui: String,
    /// 乗務員名
    pub driver_name: String,
    /// 出庫日 "26年01月14日"
    pub syukko_date: String,
    /// 車番
    pub car_name: String,
    /// 行程
    #[serde(default)]
    pub itinerary: Vec<ItineraryItem>,
    /// 違反サマリ (キー: "高速道速度オーバー最大値" など)
    #[serde(default)]
    pub violations: HashMap<String, f64>,
    /// 違反詳細 (キー: "高速道速度オーバー" など)
    #[serde(default)]
    pub violation_details: HashMap<String, Vec<ViolationDetail>>,
}

/// 行程項目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItineraryItem {
    /// 場所
    pub location: String,
    /// 種別 "始", "積", "降", "終"
    #[serde(rename = "type")]
    pub item_type: String,
}

/// 違反詳細
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationDetail {
    /// 種別 "高速", "専用", "一般", "連続"
    #[serde(rename = "type")]
    pub detail_type: String,
    /// 開始地点
    pub start_location: String,
    /// 終了地点
    pub end_location: String,
    /// 開始時刻 "16 04:16"
    pub start_time: String,
    /// 速度 "92.9km/h" または連続運転時間
    #[serde(default)]
    pub speed: Option<String>,
    /// 区間時間 "19分"
    #[serde(default)]
    pub interval_time: Option<String>,
    /// 連続運転時間 (連続運転の場合)
    #[serde(default)]
    pub duration: Option<String>,
}

/// 一覧表ページ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryPage {
    pub firm_name: String,
    pub date: String,
    pub rows: Vec<SummaryRow>,
}

/// 一覧表の行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRow {
    pub driver_bunrui: String,
    pub driver_name: String,
    pub syukko_datetime: String,
    pub kiko_datetime: String,
}

/// 指導書APIレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShidoshoResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printer: Option<String>,
}

impl ShidoshoResponse {
    pub fn success(message: &str) -> Self {
        Self {
            status: "success".to_string(),
            message: message.to_string(),
            items: None,
            printed: None,
            printer: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: message.to_string(),
            items: None,
            printed: None,
            printer: None,
        }
    }

    pub fn with_items(mut self, items: usize) -> Self {
        self.items = Some(items);
        self
    }

    pub fn with_printed(mut self, printed: bool) -> Self {
        self.printed = Some(printed);
        self
    }

    pub fn with_printer(mut self, printer: String) -> Self {
        self.printer = Some(printer);
        self
    }
}
