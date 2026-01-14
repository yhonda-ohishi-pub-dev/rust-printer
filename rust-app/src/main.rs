mod api;
mod jobs;
mod models;
mod pdf;
mod print;

use std::sync::Arc;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::create_router;
use jobs::JobStore;
use pdf::PdfGenerator;
use print::IppPrinter;

/// Application state shared across handlers
pub struct AppState {
    pub pdf_generator: PdfGenerator,
    pub ipp_printer: IppPrinter,
    pub printer_ip: String,
    pub job_store: JobStore,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_pdf_printer=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let printer_ip = std::env::var("PRINTER_IP").unwrap_or_else(|_| "172.18.21.60".to_string());
    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string());

    // Load Japanese font
    let font_path =
        std::env::var("FONT_PATH").unwrap_or_else(|_| "/app/fonts/ipaexg.ttf".to_string());

    let font_bytes = std::fs::read(&font_path).unwrap_or_else(|e| {
        tracing::warn!("Failed to load font from {}: {}. Using fallback.", font_path, e);
        // Fallback: try current directory
        std::fs::read("fonts/ipaexg.ttf").unwrap_or_else(|_| {
            tracing::error!("No font file found. PDF generation will fail.");
            Vec::new()
        })
    });

    if font_bytes.is_empty() {
        tracing::error!("Font file is empty or not found. Please provide a valid TTF font.");
    }

    // Create PDF generator
    let pdf_generator = PdfGenerator::new(font_bytes)?;

    // Create printer client (uses RAW TCP port 9100 by default)
    let ipp_printer = IppPrinter::new(&printer_ip, 9100);

    // Create job store for async printing
    let job_store = JobStore::new();

    // Create app state
    let state = Arc::new(AppState {
        pdf_generator,
        ipp_printer,
        printer_ip: printer_ip.clone(),
        job_store,
    });

    // Create router
    let app = create_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!("Server listening on {}", listen_addr);
    tracing::info!("Default printer IP: {} (RAW port 9100)", printer_ip);

    axum::serve(listener, app).await?;

    Ok(())
}
