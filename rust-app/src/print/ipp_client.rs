use anyhow::{Context, Result};
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

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
    },
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
        match &mode {
            PrintMode::Raw => {
                self.print_raw(pdf_data, job_name, &self.printer_ip).await?;
                Ok(0)
            }
            PrintMode::DirectIpp {
                ipp_path,
                paper_size,
                color_mode,
            } => {
                self.print_ipp(
                    pdf_data,
                    job_name,
                    &self.printer_ip,
                    ipp_path,
                    paper_size.as_deref(),
                    color_mode.as_deref(),
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

    /// Print via Direct IPP (port 631) - for PX-M650F etc.
    async fn print_ipp(
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

        tracing::info!(
            "IPP直接印刷: {} (用紙: {:?}, カラー: {:?})",
            uri,
            paper_size,
            color_mode
        );

        // Build IPP attributes
        let payload = IppPayload::new(Cursor::new(pdf_data));
        let mut builder = IppOperationBuilder::print_job(uri.clone(), payload)
            .attribute(IppAttribute::new(
                "job-name",
                IppValue::NameWithoutLanguage(job_name.to_string()),
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
        let client = AsyncIppClient::new(uri);

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
}

/// Map paper size to IPP media keyword and detect if envelope
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
        // Japanese envelopes
        "naga3" | "cho3" | "長3" => "om_cho-3_120x235mm".to_string(),
        "naga4" | "cho4" | "長4" => "om_cho-4_90x205mm".to_string(),
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
        assert_eq!(map_paper_size("naga3"), ("om_cho-3_120x235mm".to_string(), true));
        assert_eq!(map_paper_size("cho3"), ("om_cho-3_120x235mm".to_string(), true));
    }

    #[test]
    fn test_map_color_mode() {
        assert_eq!(map_color_mode("color"), "color");
        assert_eq!(map_color_mode("mono"), "monochrome");
        assert_eq!(map_color_mode("bw"), "monochrome");
    }
}
