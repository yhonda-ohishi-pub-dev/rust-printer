use anyhow::{Context, Result};
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

use super::pwg_converter::PwgConverter;

/// Print mode: RAW (port 9100) or Direct IPP (port 631)
#[derive(Debug, Clone)]
pub enum PrintMode {
    /// RAW TCP printing (port 9100) - simple, direct PDF send
    Raw,
    /// Direct IPP printing (port 631) - for printers like PX-M650F
    DirectIpp {
        ipp_path: String,
        paper_size: Option<String>,
        color_mode: Option<String>,
        document_format: Option<DocumentFormat>,
    },
}

/// Document format for IPP printing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DocumentFormat {
    /// application/pdf - for printers that support PDF natively
    Pdf,
    /// image/pwg-raster - PWG Raster format
    PwgRaster,
    /// image/urf - Apple Raster format (AirPrint compatible)
    Urf,
}

impl DocumentFormat {
    /// Parse from string (e.g., "pdf", "urf", "pwg")
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pdf" => Some(DocumentFormat::Pdf),
            "urf" | "apple" | "airprint" => Some(DocumentFormat::Urf),
            "pwg" | "pwg-raster" => Some(DocumentFormat::PwgRaster),
            _ => None,
        }
    }
}

/// Printer client supporting both RAW TCP and Direct IPP
pub struct IppPrinter {
    printer_ip: String,
}

impl IppPrinter {
    /// Create a new printer client
    pub fn new(printer_ip: &str, _port: u16) -> Self {
        Self {
            printer_ip: printer_ip.to_string(),
        }
    }

    /// Print with specified mode
    pub async fn print_with_mode(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        mode: PrintMode,
    ) -> Result<u32> {
        self.print_with_mode_cancellable(pdf_data, job_name, mode, || false).await
    }

    /// Print with specified mode, with cancellation check
    pub async fn print_with_mode_cancellable<F>(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        mode: PrintMode,
        is_cancelled: F,
    ) -> Result<u32>
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        match &mode {
            PrintMode::Raw => {
                self.print_raw(pdf_data, job_name, &self.printer_ip).await?;
                Ok(0)
            }
            PrintMode::DirectIpp {
                ipp_path,
                paper_size,
                color_mode,
                document_format,
            } => {
                self.print_ipp_cancellable(
                    pdf_data,
                    job_name,
                    &self.printer_ip,
                    ipp_path,
                    paper_size.as_deref(),
                    color_mode.as_deref(),
                    *document_format,
                    is_cancelled,
                )
                .await
            }
        }
    }

    /// Print via RAW TCP (port 9100)
    async fn print_raw(&self, pdf_data: Vec<u8>, job_name: &str, printer_ip: &str) -> Result<()> {
        let addr = format!("{}:9100", printer_ip);
        let job_name = job_name.to_string();

        tracing::info!(
            "RAW印刷: {} ({}bytes, job: {})",
            addr,
            pdf_data.len(),
            job_name
        );

        tokio::task::spawn_blocking(move || {
            let mut stream = TcpStream::connect(&addr)
                .with_context(|| format!("Failed to connect to printer at {}", addr))?;

            stream.set_write_timeout(Some(Duration::from_secs(30)))?;
            stream.write_all(&pdf_data)?;
            stream.flush()?;
            stream.shutdown(Shutdown::Both)?;

            tracing::info!("RAW印刷完了: {}", job_name);
            Ok::<_, anyhow::Error>(())
        })
        .await
        .context("Task join error")??;

        Ok(())
    }

    /// Print via Direct IPP (port 631) - for PX-M650F etc. with cancellation support
    async fn print_ipp_cancellable<F>(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        printer_ip: &str,
        ipp_path: &str,
        paper_size: Option<&str>,
        color_mode: Option<&str>,
        document_format: Option<DocumentFormat>,
        is_cancelled: F,
    ) -> Result<u32>
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        // Default to URF (Apple Raster) for Epson inkjets - better compatibility than PWG
        // Use PDF for Canon laser printers (LBP221 etc.)
        let format = document_format.unwrap_or(DocumentFormat::Urf);

        // Send pages one by one with cancellation check
        self.print_ipp_with_format_cancellable(
            pdf_data,
            job_name,
            printer_ip,
            ipp_path,
            paper_size,
            color_mode,
            format,
            is_cancelled,
        )
        .await
    }

    /// Print via Direct IPP with specified document format and cancellation support
    pub async fn print_ipp_with_format_cancellable<F>(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        printer_ip: &str,
        ipp_path: &str,
        paper_size: Option<&str>,
        color_mode: Option<&str>,
        format: DocumentFormat,
        is_cancelled: F,
    ) -> Result<u32>
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        use ipp::prelude::*;
        use std::io::Cursor;

        let uri_string = format!("ipp://{}:631{}", printer_ip, ipp_path);
        let uri: Uri = uri_string.parse().context("Failed to parse printer URI")?;

        // Determine document format and convert if necessary
        let (print_data, mime_type) = match format {
            DocumentFormat::Pdf => {
                tracing::info!("Using PDF format (application/pdf)");
                (pdf_data, "application/pdf")
            }
            DocumentFormat::PwgRaster => {
                tracing::info!("Converting PDF to PWG Raster format");
                let media = paper_size.map(|s| map_paper_size(s).0);
                let pwg_data = tokio::task::spawn_blocking(move || {
                    PwgConverter::convert(&pdf_data, 300, media.as_deref())
                })
                .await
                .context("PWG conversion task failed")??;
                (pwg_data, "image/pwg-raster")
            }
            DocumentFormat::Urf => {
                // For URF, send page by page to avoid timeout on large documents
                tracing::info!("Converting PDF to URF (Apple Raster) format - page by page");
                return self
                    .print_ipp_pages_cancellable(
                        pdf_data,
                        job_name,
                        printer_ip,
                        ipp_path,
                        paper_size,
                        color_mode,
                        is_cancelled,
                    )
                    .await;
            }
        };

        tracing::info!(
            "IPP直接印刷: {} (用紙: {:?}, カラー: {:?}, format: {})",
            uri,
            paper_size,
            color_mode,
            mime_type
        );

        // Build IPP attributes
        let payload = IppPayload::new(Cursor::new(print_data));
        let mut builder = IppOperationBuilder::print_job(uri.clone(), payload)
            .attribute(IppAttribute::new(
                "job-name",
                IppValue::NameWithoutLanguage(job_name.to_string()),
            ))
            .attribute(IppAttribute::new(
                "document-format",
                IppValue::MimeMediaType(mime_type.to_string()),
            ));

        if let Some(size) = paper_size {
            let (media, is_envelope) = map_paper_size(size);
            builder = builder.attribute(IppAttribute::new(
                "media",
                IppValue::Keyword(media),
            ));
            if is_envelope {
                builder = builder.attribute(IppAttribute::new(
                    "media-type",
                    IppValue::Keyword("envelope".to_string()),
                ));
                tracing::info!("封筒モード: media-type=envelope を設定");
            }
        }

        if let Some(color) = color_mode {
            builder = builder.attribute(IppAttribute::new(
                "print-color-mode",
                IppValue::Keyword(map_color_mode(color)),
            ));
        }

        let operation = builder.build();

        // Configure client with extended timeout for large documents
        let timeout_secs = std::env::var("IPP_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120u64); // Default: 2 minutes

        let client = AsyncIppClient::builder(uri)
            .request_timeout(Duration::from_secs(timeout_secs))
            .build();

        let response = client
            .send(operation)
            .await
            .map_err(|e| {
                tracing::error!("IPP送信エラー: {:?}", e);
                e
            })
            .context("Failed to send print job")?;

        let status = response.header().status_code();
        if status.is_success() {
            let job_id = response
                .attributes()
                .groups()
                .iter()
                .find_map(|g| g.attributes().get("job-id"))
                .and_then(|a| a.value().as_integer())
                .copied()
                .unwrap_or(0) as u32;

            tracing::info!("IPP印刷成功, job-id: {}", job_id);
            Ok(job_id)
        } else {
            anyhow::bail!("印刷失敗: {:?}", status)
        }
    }

    /// Print multi-page URF as separate Print-Job requests (one per page)
    /// This works for printers that don't support multiple-document-jobs
    pub async fn print_ipp_multi_document(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        printer_ip: &str,
        ipp_path: &str,
        paper_size: Option<&str>,
        color_mode: Option<&str>,
    ) -> Result<u32> {
        use ipp::prelude::*;
        use std::io::Cursor;

        let uri_string = format!("ipp://{}:631{}", printer_ip, ipp_path);
        let uri: Uri = uri_string.parse().context("Failed to parse printer URI")?;

        // Convert PDF to per-page URF
        let media = paper_size.map(|s| map_paper_size(s).0);
        let urf_pages = tokio::task::spawn_blocking(move || {
            PwgConverter::convert_to_urf_pages(&pdf_data, 300, media.as_deref())
        })
        .await
        .context("URF conversion task failed")??;

        let num_pages = urf_pages.len();
        tracing::info!(
            "Printing {} pages as separate jobs to {}",
            num_pages,
            uri
        );

        let timeout_secs = std::env::var("IPP_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120u64);

        let client = AsyncIppClient::builder(uri.clone())
            .request_timeout(Duration::from_secs(timeout_secs))
            .build();

        let mut last_job_id = 0u32;

        // Send each page as a separate Print-Job
        // Don't wait for busy - just send all jobs and let the printer queue them
        for (i, urf_data) in urf_pages.into_iter().enumerate() {
            let page_job_name = format!("{} (page {})", job_name, i + 1);
            tracing::info!(
                "Sending page {}/{} ({} bytes)",
                i + 1,
                num_pages,
                urf_data.len()
            );

            let payload = IppPayload::new(Cursor::new(urf_data));
            let mut builder = IppOperationBuilder::print_job(uri.clone(), payload)
                .attribute(IppAttribute::new(
                    "job-name",
                    IppValue::NameWithoutLanguage(page_job_name.clone()),
                ))
                .attribute(IppAttribute::new(
                    "document-format",
                    IppValue::MimeMediaType("image/urf".to_string()),
                ));

            if let Some(size) = paper_size {
                let (media_keyword, is_envelope) = map_paper_size(size);
                builder = builder.attribute(IppAttribute::new(
                    "media",
                    IppValue::Keyword(media_keyword),
                ));
                if is_envelope {
                    builder = builder.attribute(IppAttribute::new(
                        "media-type",
                        IppValue::Keyword("envelope".to_string()),
                    ));
                }
            }

            if let Some(color) = color_mode {
                builder = builder.attribute(IppAttribute::new(
                    "print-color-mode",
                    IppValue::Keyword(map_color_mode(color)),
                ));
            }

            let operation = builder.build();

            let response: IppRequestResponse = client
                .send(operation)
                .await
                .with_context(|| format!("Failed to send page {}", i + 1))?;

            let status = response.header().status_code();
            if status.is_success() {
                last_job_id = response
                    .attributes()
                    .groups()
                    .iter()
                    .find_map(|g| g.attributes().get("job-id"))
                    .and_then(|a| a.value().as_integer())
                    .copied()
                    .unwrap_or(0) as u32;
                tracing::info!("Page {}/{} sent, job-id: {}", i + 1, num_pages, last_job_id);
            } else if status == StatusCode::ServerErrorBusy {
                // Printer busy but might still accept job - log and continue
                tracing::warn!("Page {}/{} got ServerErrorBusy - printer may not queue jobs", i + 1, num_pages);
                anyhow::bail!("Printer returned ServerErrorBusy for page {} - printer does not support job queuing while busy", i + 1);
            } else {
                anyhow::bail!("Print-Job failed for page {}: {:?}", i + 1, status);
            }
        }

        tracing::info!("All {} pages sent successfully", num_pages);
        Ok(last_job_id)
    }

    /// Print URF pages one by one, waiting for each job to complete before sending the next
    /// This avoids timeout issues with large multi-page documents
    /// Supports cancellation check between pages
    async fn print_ipp_pages_cancellable<F>(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        printer_ip: &str,
        ipp_path: &str,
        paper_size: Option<&str>,
        color_mode: Option<&str>,
        is_cancelled: F,
    ) -> Result<u32>
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        use ipp::prelude::*;
        use std::io::Cursor;

        let uri_string = format!("ipp://{}:631{}", printer_ip, ipp_path);
        let uri: Uri = uri_string.parse().context("Failed to parse printer URI")?;

        // Convert PDF to per-page URF
        let media = paper_size.map(|s| map_paper_size(s).0);
        let urf_pages = tokio::task::spawn_blocking(move || {
            PwgConverter::convert_to_urf_pages(&pdf_data, 300, media.as_deref())
        })
        .await
        .context("URF conversion task failed")??;

        let num_pages = urf_pages.len();
        tracing::info!(
            "Printing {} pages one by one to {} (waiting for each to complete)",
            num_pages,
            uri
        );

        let timeout_secs = std::env::var("IPP_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120u64);

        let client = AsyncIppClient::builder(uri.clone())
            .request_timeout(Duration::from_secs(timeout_secs))
            .build();

        let mut last_job_id = 0u32;

        // Use index-based iteration to allow retry with same data
        for i in 0..num_pages {
            // Check for cancellation before sending each page
            if is_cancelled() {
                tracing::info!("Job cancelled, stopping at page {}/{}", i + 1, num_pages);
                anyhow::bail!("Job cancelled by user (stopped at page {}/{})", i + 1, num_pages);
            }

            let urf_data = &urf_pages[i];
            let page_job_name = format!("{} (page {})", job_name, i + 1);
            tracing::info!(
                "Sending page {}/{} ({} bytes)",
                i + 1,
                num_pages,
                urf_data.len()
            );

            // Try to send page with retry on ServerErrorBusy
            let max_retries = 10;
            let mut attempt = 0;
            let mut page_job_id = 0u32;

            loop {
                attempt += 1;

                let payload = IppPayload::new(Cursor::new(urf_data.clone()));
                let mut builder = IppOperationBuilder::print_job(uri.clone(), payload)
                    .attribute(IppAttribute::new(
                        "job-name",
                        IppValue::NameWithoutLanguage(page_job_name.clone()),
                    ))
                    .attribute(IppAttribute::new(
                        "document-format",
                        IppValue::MimeMediaType("image/urf".to_string()),
                    ));

                if let Some(size) = paper_size {
                    let (media_keyword, is_envelope) = map_paper_size(size);
                    builder = builder.attribute(IppAttribute::new(
                        "media",
                        IppValue::Keyword(media_keyword),
                    ));
                    if is_envelope {
                        builder = builder.attribute(IppAttribute::new(
                            "media-type",
                            IppValue::Keyword("envelope".to_string()),
                        ));
                    }
                }

                if let Some(color) = color_mode {
                    builder = builder.attribute(IppAttribute::new(
                        "print-color-mode",
                        IppValue::Keyword(map_color_mode(color)),
                    ));
                }

                let operation = builder.build();

                let response: IppRequestResponse = client
                    .send(operation)
                    .await
                    .with_context(|| format!("Failed to send page {}", i + 1))?;

                let status = response.header().status_code();
                if status.is_success() {
                    page_job_id = response
                        .attributes()
                        .groups()
                        .iter()
                        .find_map(|g| g.attributes().get("job-id"))
                        .and_then(|a| a.value().as_integer())
                        .copied()
                        .unwrap_or(0) as u32;
                    tracing::info!("Page {}/{} accepted, job-id: {}", i + 1, num_pages, page_job_id);
                    break;
                } else if status == StatusCode::ServerErrorBusy {
                    if attempt >= max_retries {
                        anyhow::bail!("Printer busy for page {} after {} retries", i + 1, max_retries);
                    }
                    let wait_secs = std::cmp::min(5 * attempt, 30) as u64;
                    tracing::warn!(
                        "Page {}/{} got ServerErrorBusy (attempt {}/{}) - waiting {} seconds",
                        i + 1, num_pages, attempt, max_retries, wait_secs
                    );
                    tokio::time::sleep(Duration::from_secs(wait_secs)).await;

                    // Check for cancellation during retry wait
                    if is_cancelled() {
                        anyhow::bail!("Job cancelled by user during retry (page {})", i + 1);
                    }
                    // Continue to retry
                } else {
                    anyhow::bail!("Print-Job failed for page {}: {:?}", i + 1, status);
                }
            }

            last_job_id = page_job_id;

            // Wait for job to complete before sending next page
            if i + 1 < num_pages {
                self.wait_for_job_complete(&client, &uri, last_job_id, timeout_secs).await?;
            }
        }

        tracing::info!("All {} pages sent successfully", num_pages);
        Ok(last_job_id)
    }

    /// Wait for a job to complete (or at least be accepted into printer queue)
    async fn wait_for_job_complete(
        &self,
        client: &ipp::prelude::AsyncIppClient,
        uri: &ipp::prelude::Uri,
        job_id: u32,
        timeout_secs: u64,
    ) -> Result<()> {
        use ipp::prelude::*;

        let start = std::time::Instant::now();
        let max_wait = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > max_wait {
                tracing::warn!("Timeout waiting for job {} - proceeding anyway", job_id);
                return Ok(());
            }

            // Get job attributes with individual timeout to prevent hanging
            let operation = IppOperationBuilder::get_job_attributes(uri.clone(), job_id as i32)
                .build();

            match tokio::time::timeout(Duration::from_secs(10), client.send(operation)).await {
                Ok(Ok(response)) => {
                    if let Some(state) = response
                        .attributes()
                        .groups()
                        .iter()
                        .find_map(|g: &IppAttributeGroup| g.attributes().get("job-state"))
                        .and_then(|a: &IppAttribute| a.value().as_enum())
                    {
                        // Job states: 3=pending, 4=pending-held, 5=processing, 6=processing-stopped,
                        // 7=canceled, 8=aborted, 9=completed
                        let state = *state;
                        tracing::debug!("Job {} state: {}", job_id, state);

                        if state >= 7 {
                            // Job finished (canceled, aborted, or completed)
                            tracing::info!("Job {} finished with state {}", job_id, state);
                            return Ok(());
                        }
                        // For Epson printers, wait until job is fully completed
                        // They don't support queuing multiple jobs while one is processing
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to get job {} status: {:?}", job_id, e);
                }
                Err(_) => {
                    tracing::warn!("Get-Job-Attributes timeout for job {}", job_id);
                }
            }

            // Wait before polling again
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

fn map_paper_size(size: &str) -> (String, bool) {
    let lower = size.to_lowercase();
    let is_envelope = matches!(
        lower.as_str(),
        "naga3" | "cho3" | "長3" | "naga4" | "cho4" | "長4"
    ) || lower.contains("cho-") || lower.contains("envelope");

    let media = match lower.as_str() {
        "a4" => "iso_a4_210x297mm".to_string(),
        "a3" => "iso_a3_297x420mm".to_string(),
        "a5" => "iso_a5_148x210mm".to_string(),
        "b5" => "iso_b5_176x250mm".to_string(),
        "letter" => "na_letter_8.5x11in".to_string(),
        "legal" => "na_legal_8.5x14in".to_string(),
        // Japanese envelopes (use jpn_ prefix for Epson printers)
        "naga3" | "cho3" | "長3" => "jpn_chou3_120x235mm".to_string(),
        "naga4" | "cho4" | "長4" => "jpn_chou4_90x205mm".to_string(),
        // If already in IPP format (contains underscore), use as-is
        s if s.contains('_') => s.to_string(),
        _ => "iso_a4_210x297mm".to_string(),
    };

    (media, is_envelope)
}

/// Map color mode to IPP keyword
fn map_color_mode(mode: &str) -> String {
    match mode.to_lowercase().as_str() {
        "color" => "color".to_string(),
        "monochrome" | "mono" | "bw" | "black-and-white" => "monochrome".to_string(),
        _ => "auto".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_paper_size() {
        assert_eq!(map_paper_size("A4"), ("iso_a4_210x297mm".to_string(), false));
        assert_eq!(map_paper_size("a4"), ("iso_a4_210x297mm".to_string(), false));
        assert_eq!(map_paper_size("B5"), ("iso_b5_176x250mm".to_string(), false));
        assert_eq!(map_paper_size("naga3"), ("jpn_chou3_120x235mm".to_string(), true));
        assert_eq!(map_paper_size("cho3"), ("jpn_chou3_120x235mm".to_string(), true));
    }

    #[test]
    fn test_map_color_mode() {
        assert_eq!(map_color_mode("color"), "color");
        assert_eq!(map_color_mode("mono"), "monochrome");
        assert_eq!(map_color_mode("bw"), "monochrome");
    }

    #[test]
    fn test_document_format_from_str() {
        assert_eq!(DocumentFormat::from_str("pdf"), Some(DocumentFormat::Pdf));
        assert_eq!(DocumentFormat::from_str("PDF"), Some(DocumentFormat::Pdf));
        assert_eq!(DocumentFormat::from_str("urf"), Some(DocumentFormat::Urf));
        assert_eq!(DocumentFormat::from_str("URF"), Some(DocumentFormat::Urf));
        assert_eq!(DocumentFormat::from_str("pwg"), Some(DocumentFormat::PwgRaster));
        assert_eq!(DocumentFormat::from_str("pwg-raster"), Some(DocumentFormat::PwgRaster));
        assert_eq!(DocumentFormat::from_str("unknown"), None);
    }
}
