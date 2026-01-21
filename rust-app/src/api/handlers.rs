use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::jobs::PrintJob;
use crate::models::{ApiResponse, Item, PrintRequest, ShidoshoRequest, ShidoshoResponse};
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
            "/print": "Print existing PDF file (POST multipart)",
            "/print-async": "Print existing PDF file asynchronously (POST multipart)",
            "/print-shidosho": "Generate and print Shidosho PDF (POST)",
            "/jobs": "Get all jobs (GET)",
            "/job/:id": "Get job status (GET) / Cancel job (DELETE)"
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

    // If print is requested, send to printer
    if request.print {
        // Determine print mode
        let mode = if request.use_direct_ipp {
            // Direct IPP mode (for PX-M650F etc.)
            let document_format = request
                .document_format
                .as_deref()
                .and_then(crate::print::DocumentFormat::from_str);
            crate::print::PrintMode::DirectIpp {
                ipp_path: "/ipp/print".to_string(),
                paper_size: request.paper_size.clone(),
                color_mode: request.color_mode.clone(),
                document_format,
            }
        } else {
            // RAW mode (default, port 9100)
            crate::print::PrintMode::Raw
        };

        let printer_ip = request
            .printer_ip
            .as_deref()
            .unwrap_or(&state.printer_ip);

        // Create printer client with the target IP
        let printer = crate::print::IppPrinter::new(printer_ip, 0);

        printer
            .print_with_mode(pdf_data, "travel-expense-report", mode.clone())
            .await
            .map_err(|e| {
                tracing::error!("Print failed: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(&format!("Print failed: {}", e))),
                )
            })?;

        let printer_info = match mode {
            crate::print::PrintMode::Raw => format!("{} (RAW)", printer_ip),
            crate::print::PrintMode::DirectIpp { .. } => format!("{} (Direct IPP)", printer_ip),
        };

        Ok(Json(
            ApiResponse::success("PDF generated and printed successfully")
                .with_items(items_count)
                .with_printed(true)
                .with_printer(printer_info),
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
    tracing::info!("Received /print request (sync)");
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut printer_ip: Option<String> = None;
    let mut use_direct_ipp: bool = false;
    let mut paper_size: Option<String> = None;
    let mut color_mode: Option<String> = None;
    let mut document_format: Option<String> = None;

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
            "printerIp" | "printer_ip" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        printer_ip = Some(value);
                    }
                }
            }
            "useDirectIpp" | "use_direct_ipp" => {
                if let Ok(value) = field.text().await {
                    use_direct_ipp = value.parse().unwrap_or(false);
                }
            }
            "paperSize" | "paper_size" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        paper_size = Some(value);
                    }
                }
            }
            "colorMode" | "color_mode" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        color_mode = Some(value);
                    }
                }
            }
            "documentFormat" | "document_format" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        document_format = Some(value);
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

    // Determine print mode
    let mode = if use_direct_ipp {
        let doc_format = document_format
            .as_deref()
            .and_then(crate::print::DocumentFormat::from_str);
        crate::print::PrintMode::DirectIpp {
            ipp_path: "/ipp/print".to_string(),
            paper_size,
            color_mode,
            document_format: doc_format,
        }
    } else {
        crate::print::PrintMode::Raw
    };

    let printer_ip = printer_ip
        .as_deref()
        .unwrap_or(&state.printer_ip);

    // Create printer client and send
    let printer = crate::print::IppPrinter::new(printer_ip, 0);

    printer
        .print_with_mode(pdf_data, &filename, mode.clone())
        .await
        .map_err(|e| {
            tracing::error!("Print failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Print failed: {}", e))),
            )
        })?;

    let printer_info = match mode {
        crate::print::PrintMode::Raw => format!("{} (RAW)", printer_ip),
        crate::print::PrintMode::DirectIpp { .. } => format!("{} (Direct IPP)", printer_ip),
    };

    Ok(Json(
        ApiResponse::success("PDF printed successfully")
            .with_printed(true)
            .with_filename(filename)
            .with_printer(printer_info)
            .with_file_size(file_size),
    ))
}

/// POST /print-async - Print existing PDF file asynchronously (multipart form)
/// Returns immediately with job_id, print job runs in background
pub async fn print_file_async(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiResponse>)> {
    tracing::info!("Received /print-async request (async)");
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut printer_ip: Option<String> = None;
    let mut use_direct_ipp: bool = false;
    let mut paper_size: Option<String> = None;
    let mut color_mode: Option<String> = None;
    let mut document_format: Option<String> = None;

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
            "printerIp" | "printer_ip" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        printer_ip = Some(value);
                    }
                }
            }
            "useDirectIpp" | "use_direct_ipp" => {
                if let Ok(value) = field.text().await {
                    use_direct_ipp = value.parse().unwrap_or(false);
                }
            }
            "paperSize" | "paper_size" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        paper_size = Some(value);
                    }
                }
            }
            "colorMode" | "color_mode" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        color_mode = Some(value);
                    }
                }
            }
            "documentFormat" | "document_format" => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        document_format = Some(value);
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

    // Create a job
    let job = state.job_store.create_job().await;
    let job_id = job.id.clone();

    // Determine print mode
    let mode = if use_direct_ipp {
        let doc_format = document_format
            .as_deref()
            .and_then(crate::print::DocumentFormat::from_str);
        crate::print::PrintMode::DirectIpp {
            ipp_path: "/ipp/print".to_string(),
            paper_size,
            color_mode,
            document_format: doc_format,
        }
    } else {
        crate::print::PrintMode::Raw
    };

    let printer_ip_str = printer_ip
        .clone()
        .unwrap_or_else(|| state.printer_ip.clone());

    let print_mode_str = match &mode {
        crate::print::PrintMode::Raw => "RAW".to_string(),
        crate::print::PrintMode::DirectIpp { .. } => "Direct IPP".to_string(),
    };

    // Guess printer name from IP (can be extended with actual discovery)
    let printer_name = guess_printer_name(&printer_ip_str);

    // Update job metadata
    state.job_store.update_metadata(
        &job_id,
        Some(filename.clone()),
        Some(printer_ip_str.clone()),
        printer_name,
        Some(print_mode_str),
        Some(file_size),
    ).await;

    // Clone what we need for the spawned task
    let job_store = state.job_store.clone();
    let job_store_for_cancel = state.job_store.clone();
    let job_id_clone = job_id.clone();
    let job_id_for_cancel = job_id.clone();

    // Spawn background task for printing
    tokio::spawn(async move {
        job_store.set_processing(&job_id_clone).await;

        let printer = crate::print::IppPrinter::new(&printer_ip_str, 0);

        // Create cancellation check closure
        let is_cancelled = move || {
            // Use blocking check - we're in an async context but the closure is sync
            futures::executor::block_on(job_store_for_cancel.is_cancelled(&job_id_for_cancel))
        };

        match printer.print_with_mode_cancellable(pdf_data, &filename, mode, is_cancelled).await {
            Ok(_) => {
                tracing::info!("Async print job {} completed successfully", job_id_clone);
                job_store.set_completed(&job_id_clone, "PDF printed successfully").await;
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                if error_msg.contains("cancelled") {
                    tracing::info!("Async print job {} was cancelled", job_id_clone);
                    // Job already marked as cancelled by cancel_job endpoint
                } else {
                    tracing::error!("Async print job {} failed: {}", job_id_clone, e);
                    job_store.set_failed(&job_id_clone, &format!("Print failed: {}", e)).await;
                }
            }
        }
    });

    tracing::info!("Async print job {} created, processing in background", job_id);

    Ok(Json(serde_json::json!({
        "status": "accepted",
        "job_id": job_id,
        "message": "Print job accepted, processing in background"
    })))
}

/// GET /job/:id - Get job status
pub async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<PrintJob>, (StatusCode, Json<ApiResponse>)> {
    match state.job_store.get_job(&job_id).await {
        Some(job) => Ok(Json(job)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(&format!("Job {} not found", job_id))),
        )),
    }
}

/// GET /jobs - Get all jobs
pub async fn get_all_jobs(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let jobs = state.job_store.get_all_jobs().await;
    Json(serde_json::json!({
        "jobs": jobs,
        "total": jobs.len()
    }))
}

/// DELETE /job/:id - Cancel a job
pub async fn cancel_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiResponse>)> {
    match state.job_store.cancel_job(&job_id).await {
        Some(job) => {
            tracing::info!("Job {} cancelled by user", job_id);
            Ok(Json(serde_json::json!({
                "status": "cancelled",
                "job": job
            })))
        }
        None => {
            // Check if job exists but can't be cancelled
            match state.job_store.get_job(&job_id).await {
                Some(job) => Err((
                    StatusCode::CONFLICT,
                    Json(ApiResponse::error(&format!(
                        "Job {} cannot be cancelled (status: {:?})",
                        job_id, job.status
                    ))),
                )),
                None => Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error(&format!("Job {} not found", job_id))),
                )),
            }
        }
    }
}

/// Guess printer name from IP address
/// This uses a simple mapping; can be extended with actual printer discovery (IPP Get-Printer-Attributes)
fn guess_printer_name(ip: &str) -> Option<String> {
    // Known printer mappings (configure via environment or config file in the future)
    let known_printers: &[(&str, &str)] = &[
        // Epson printers (Direct IPP, port 631)
        ("192.168.11.100", "Epson PX-M650F"),
        ("192.168.11.101", "Epson"),
        // Canon printers (RAW, port 9100)
        ("192.168.11.200", "Canon LBP221"),
        ("192.168.11.201", "Canon"),
    ];

    for (known_ip, name) in known_printers {
        if ip == *known_ip {
            return Some(name.to_string());
        }
    }

    // Try to guess from IP pattern or return None
    None
}

/// POST /print-shidosho - Generate and print Shidosho PDF
pub async fn print_shidosho(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ShidoshoRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ShidoshoResponse>)> {
    // pages か summary_pages のどちらかが必要
    if request.pages.is_empty() && request.summary_pages.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ShidoshoResponse::error("No pages or summary_pages provided")),
        ));
    }

    let items_count = request.pages.len() + request.summary_pages.len();

    // Generate PDF
    let pdf_data = state
        .shidosho_generator
        .generate(&request.title, &request.pages, &request.summary_pages)
        .map_err(|e| {
            tracing::error!("Shidosho PDF generation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ShidoshoResponse::error(&format!("PDF generation failed: {}", e))),
            )
        })?;

    // If print is not requested, return PDF directly
    if !request.print {
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/pdf")],
            pdf_data,
        ).into_response());
    }

    // Print mode
    let mode = if request.use_direct_ipp {
        crate::print::PrintMode::DirectIpp {
            ipp_path: "/ipp/print".to_string(),
            paper_size: request.paper_size.clone(),
            color_mode: request.color_mode.clone(),
            document_format: None,
        }
    } else {
        crate::print::PrintMode::Raw
    };

    let printer_ip = request
        .printer_ip
        .as_deref()
        .unwrap_or(&state.printer_ip)
        .to_string();

    let printer_info = match &mode {
        crate::print::PrintMode::Raw => format!("{} (RAW async)", printer_ip),
        crate::print::PrintMode::DirectIpp { .. } => format!("{} (Direct IPP)", printer_ip),
    };

    // For RAW mode (Canon), run async and return immediately
    if matches!(mode, crate::print::PrintMode::Raw) {
        let printer_ip_clone = printer_ip.clone();
        tokio::spawn(async move {
            let printer = crate::print::IppPrinter::new(&printer_ip_clone, 0);
            if let Err(e) = printer
                .print_with_mode(pdf_data, "shidosho-report", crate::print::PrintMode::Raw)
                .await
            {
                tracing::error!("Async shidosho print failed: {}", e);
            } else {
                tracing::info!("Async shidosho print completed to {}", printer_ip_clone);
            }
        });

        return Ok(Json(
            ShidoshoResponse::success("Shidosho PDF generated, printing in background")
                .with_items(items_count)
                .with_printed(true)
                .with_printer(printer_info),
        ).into_response());
    }

    // For Direct IPP mode (Epson), wait for completion
    let printer = crate::print::IppPrinter::new(&printer_ip, 0);

    printer
        .print_with_mode(pdf_data, "shidosho-report", mode)
        .await
        .map_err(|e| {
            tracing::error!("Shidosho print failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ShidoshoResponse::error(&format!("Print failed: {}", e))),
            )
        })?;

    Ok(Json(
        ShidoshoResponse::success("Shidosho PDF generated and printed successfully")
            .with_items(items_count)
            .with_printed(true)
            .with_printer(printer_info),
    ).into_response())
}
