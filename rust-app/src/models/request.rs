use serde::{Deserialize, Serialize};

use super::Item;

/// PrintRequest represents the print request data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintRequest {
    pub items: Vec<Item>,
    #[serde(default)]
    pub print: bool,
    pub printer_name: Option<String>,
    pub printer_ip: Option<String>,

    // Direct IPP printing fields
    #[serde(default)]
    pub use_direct_ipp: bool,           // true = Direct IPP, false = CUPS
    pub paper_size: Option<String>,     // "A4", "B5", "A3", "Letter", etc.
    pub color_mode: Option<String>,     // "color", "monochrome"
}

/// API Response for PDF generation/printing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<usize>,
}

impl ApiResponse {
    pub fn success(message: &str) -> Self {
        Self {
            status: "success".to_string(),
            message: message.to_string(),
            items: None,
            printed: None,
            filename: None,
            printer: None,
            file_size: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: message.to_string(),
            items: None,
            printed: None,
            filename: None,
            printer: None,
            file_size: None,
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

    pub fn with_filename(mut self, filename: String) -> Self {
        self.filename = Some(filename);
        self
    }

    pub fn with_printer(mut self, printer: String) -> Self {
        self.printer = Some(printer);
        self
    }

    pub fn with_file_size(mut self, size: usize) -> Self {
        self.file_size = Some(size);
        self
    }
}
