use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::models::{ApiResponse, Item, PrintRequest};
use crate::AppState;

/// GET /health - Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "rust-pdf-printer",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// GET / - API information
pub async fn api_info() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "Rust PDF Printer API",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "/": "API information",
            "/health": "Health check",
            "/generate-pdf": "Generate PDF from JSON (POST)",
            "/print-pdf": "Generate and print PDF (POST)",
            "/print": "Print existing PDF file (POST multipart)"
        }
    }))
}

/// POST /generate-pdf - Generate PDF from JSON data
pub async fn generate_pdf(
    State(state): State<Arc<AppState>>,
    Json(items): Json<Vec<Item>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiResponse>)> {
    if items.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("No items provided")),
        ));
    }

    let pdf_data = state
        .pdf_generator
        .generate(&items)
        .map_err(|e| {
            tracing::error!("PDF generation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("PDF generation failed: {}", e))),
            )
        })?;

    // Return PDF as binary response
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/pdf")],
        pdf_data,
    ))
}

/// POST /print-pdf - Generate and print PDF
pub async fn print_pdf(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PrintRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
    if request.items.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("No items provided")),
        ));
    }

    let items_count = request.items.len();

    // Generate PDF
    let pdf_data = state
        .pdf_generator
        .generate(&request.items)
        .map_err(|e| {
            tracing::error!("PDF generation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("PDF generation failed: {}", e))),
            )
        })?;

    // If print is requested, send to CUPS
    if request.print {
        let printer_name = request
            .printer_name
            .as_deref()
            .unwrap_or(&state.default_printer);

        state
            .cups_printer
            .print(pdf_data, "travel-expense-report", printer_name)
            .await
            .map_err(|e| {
                tracing::error!("Print failed: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(&format!("Print failed: {}", e))),
                )
            })?;

        Ok(Json(
            ApiResponse::success("PDF generated and printed successfully")
                .with_items(items_count)
                .with_printed(true)
                .with_printer(printer_name.to_string()),
        ))
    } else {
        Ok(Json(
            ApiResponse::success("PDF generated successfully")
                .with_items(items_count)
                .with_printed(false),
        ))
    }
}

/// POST /print - Print existing PDF file (multipart form)
pub async fn print_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut printer_name: Option<String> = None;

    // Parse multipart form
    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "document" => {
                filename = field.file_name().map(|s| s.to_string());
                if let Ok(bytes) = field.bytes().await {
                    pdf_data = Some(bytes.to_vec());
                }
            }
            "printer" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        printer_name = Some(value);
                    }
                }
            }
            _ => {}
        }
    }

    let pdf_data = pdf_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("No document field in form")),
        )
    })?;

    let file_size = pdf_data.len();
    let filename = filename.unwrap_or_else(|| "document.pdf".to_string());
    let printer = printer_name
        .as_deref()
        .unwrap_or(&state.default_printer);

    // Send to CUPS
    state
        .cups_printer
        .print(pdf_data, &filename, printer)
        .await
        .map_err(|e| {
            tracing::error!("Print failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Print failed: {}", e))),
            )
        })?;

    Ok(Json(
        ApiResponse::success("PDF printed successfully")
            .with_printed(true)
            .with_filename(filename)
            .with_printer(printer.to_string())
            .with_file_size(file_size),
    ))
}
