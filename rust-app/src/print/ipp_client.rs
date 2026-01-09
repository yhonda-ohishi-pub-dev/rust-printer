use anyhow::{Context, Result};
use ipp::prelude::*;
use std::io::Cursor;

/// Print mode enum to distinguish between CUPS and Direct IPP printing
#[derive(Debug, Clone)]
pub enum PrintMode {
    /// CUPS-based printing (via CUPS server)
    Cups {
        printer_name: String,
    },
    /// Direct IPP printing (directly to printer)
    DirectIpp {
        printer_ip: String,
        ipp_path: String,
        paper_size: Option<String>,
        color_mode: Option<String>,
    },
}

/// IPP Printer client supporting both CUPS and direct IPP protocols
pub struct IppPrinter {
    server: String,
    port: u16,
}

/// Map paper size to IPP media keyword
fn map_paper_size(size: &str) -> String {
    match size.to_lowercase().as_str() {
        "a4" => "iso_a4_210x297mm".to_string(),
        "a3" => "iso_a3_297x420mm".to_string(),
        "b5" => "iso_b5_176x250mm".to_string(),
        "letter" => "na_letter_8.5x11in".to_string(),
        "legal" => "na_legal_8.5x14in".to_string(),
        _ => "iso_a4_210x297mm".to_string(), // default to A4
    }
}

/// Map color mode to IPP keyword
fn map_color_mode(mode: &str) -> String {
    match mode.to_lowercase().as_str() {
        "color" => "color".to_string(),
        "monochrome" | "mono" | "bw" | "black-and-white" => "monochrome".to_string(),
        _ => "auto".to_string(), // default to auto
    }
}

impl IppPrinter {
    /// Create a new IPP printer client
    pub fn new(server: &str, port: u16) -> Self {
        Self {
            server: server.to_string(),
            port,
        }
    }

    /// Build IPP URI based on print mode
    fn build_uri(&self, mode: &PrintMode) -> String {
        match mode {
            PrintMode::Cups { printer_name } => {
                format!(
                    "ipp://{}:{}/printers/{}",
                    self.server, self.port, printer_name
                )
            }
            PrintMode::DirectIpp { printer_ip, ipp_path, .. } => {
                format!("ipp://{}:{}{}", printer_ip, self.port, ipp_path)
            }
        }
    }

    /// Build IPP attributes for print job
    fn build_attributes(&self, job_name: &str, mode: &PrintMode) -> Vec<IppAttribute> {
        let mut attributes = vec![IppAttribute::new(
            "job-name",
            IppValue::NameWithoutLanguage(job_name.to_string()),
        )];

        // Add attributes for Direct IPP mode
        if let PrintMode::DirectIpp {
            paper_size,
            color_mode,
            ..
        } = mode
        {
            // Add paper size attribute
            if let Some(size) = paper_size {
                attributes.push(IppAttribute::new(
                    "media",
                    IppValue::Keyword(map_paper_size(size)),
                ));
            }

            // Add color mode attribute
            if let Some(color) = color_mode {
                attributes.push(IppAttribute::new(
                    "print-color-mode",
                    IppValue::Keyword(map_color_mode(color)),
                ));
            }
        }

        attributes
    }

    /// Get the IPP URI for a printer (legacy method for CUPS)
    fn get_printer_uri(&self, printer_name: &str) -> String {
        format!(
            "ipp://{}:{}/printers/{}",
            self.server, self.port, printer_name
        )
    }

    /// Print PDF data with specified print mode (main method)
    pub async fn print_with_mode(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        mode: PrintMode,
    ) -> Result<u32> {
        let uri_string = self.build_uri(&mode);

        let uri: Uri = uri_string
            .parse()
            .context("Failed to parse printer URI")?;

        // Log which mode we're using
        match &mode {
            PrintMode::Cups { printer_name } => {
                tracing::info!("CUPS印刷: {} ({})", printer_name, uri);
            }
            PrintMode::DirectIpp {
                printer_ip,
                paper_size,
                color_mode,
                ..
            } => {
                tracing::info!(
                    "IPP直接印刷: {} (用紙: {:?}, カラー: {:?})",
                    printer_ip,
                    paper_size,
                    color_mode
                );
            }
        }

        // Build IPP attributes
        let attributes = self.build_attributes(job_name, &mode);

        // Build IPP print-job operation
        let payload = IppPayload::new(Cursor::new(pdf_data));
        let mut operation_builder = IppOperationBuilder::print_job(uri.clone(), payload);

        // Add all attributes
        for attr in attributes {
            operation_builder = operation_builder.attribute(attr);
        }

        let operation = operation_builder.build();

        // Create async IPP client
        let client = AsyncIppClient::new(uri);

        // Send print request
        let response = client
            .send(operation)
            .await
            .context("Failed to send print job")?;

        // Check response status
        let status = response.header().status_code();
        if status.is_success() {
            // Extract job-id from response
            let job_id = response
                .attributes()
                .groups()
                .iter()
                .find_map(|g| g.attributes().get("job-id"))
                .and_then(|a| a.value().as_integer())
                .copied()
                .unwrap_or(0) as u32;

            tracing::info!("印刷ジョブ送信成功, job-id: {}", job_id);
            Ok(job_id)
        } else {
            let error_msg = format!("印刷失敗: {:?}", status);
            tracing::error!("{}", error_msg);
            anyhow::bail!(error_msg)
        }
    }

    /// Print PDF data to the specified printer (backward compatibility)
    pub async fn print(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        printer_name: &str,
    ) -> Result<u32> {
        self.print_with_mode(
            pdf_data,
            job_name,
            PrintMode::Cups {
                printer_name: printer_name.to_string(),
            },
        )
        .await
    }

    /// Print PDF data to the specified printer with custom IP (backward compatibility)
    pub async fn print_with_ip(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        printer_name: &str,
        printer_ip: Option<&str>,
    ) -> Result<u32> {
        let mode = PrintMode::Cups {
            printer_name: printer_name.to_string(),
        };

        // If printer_ip is specified, we need to temporarily override
        // This maintains backward compatibility with existing code
        if let Some(ip) = printer_ip {
            // Create a temporary IppPrinter with the custom IP
            let temp_printer = IppPrinter::new(ip, self.port);
            temp_printer.print_with_mode(pdf_data, job_name, mode).await
        } else {
            self.print_with_mode(pdf_data, job_name, mode).await
        }
    }

    /// Get printer status
    #[allow(dead_code)]
    pub async fn get_printer_status(&self, printer_name: &str) -> Result<String> {
        let uri: Uri = self
            .get_printer_uri(printer_name)
            .parse()
            .context("Failed to parse printer URI")?;

        let operation = IppOperationBuilder::get_printer_attributes(uri.clone()).build();

        let client = AsyncIppClient::new(uri);
        let response = client
            .send(operation)
            .await
            .context("Failed to get printer attributes")?;

        if response.header().status_code().is_success() {
            let state = response
                .attributes()
                .groups()
                .iter()
                .find_map(|g| g.attributes().get("printer-state"))
                .map(|a| format!("{:?}", a.value()))
                .unwrap_or_else(|| "unknown".to_string());

            Ok(state)
        } else {
            anyhow::bail!(
                "Failed to get printer status: {:?}",
                response.header().status_code()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_uri_cups_mode() {
        let printer = IppPrinter::new("cups-server", 631);
        let mode = PrintMode::Cups {
            printer_name: "Canon_LBP221".to_string(),
        };
        let uri = printer.build_uri(&mode);
        assert_eq!(uri, "ipp://cups-server:631/printers/Canon_LBP221");
    }

    #[test]
    fn test_build_uri_direct_ipp_mode() {
        let printer = IppPrinter::new("localhost", 631);
        let mode = PrintMode::DirectIpp {
            printer_ip: "192.168.1.200".to_string(),
            ipp_path: "/ipp/print".to_string(),
            paper_size: None,
            color_mode: None,
        };
        let uri = printer.build_uri(&mode);
        assert_eq!(uri, "ipp://192.168.1.200:631/ipp/print");
    }

    #[test]
    fn test_map_paper_size_a4() {
        assert_eq!(map_paper_size("A4"), "iso_a4_210x297mm");
        assert_eq!(map_paper_size("a4"), "iso_a4_210x297mm");
    }

    #[test]
    fn test_map_paper_size_b5() {
        assert_eq!(map_paper_size("B5"), "iso_b5_176x250mm");
        assert_eq!(map_paper_size("b5"), "iso_b5_176x250mm");
    }

    #[test]
    fn test_map_paper_size_a3() {
        assert_eq!(map_paper_size("A3"), "iso_a3_297x420mm");
    }

    #[test]
    fn test_map_paper_size_letter() {
        assert_eq!(map_paper_size("letter"), "na_letter_8.5x11in");
        assert_eq!(map_paper_size("Letter"), "na_letter_8.5x11in");
    }

    #[test]
    fn test_map_paper_size_default() {
        assert_eq!(map_paper_size("unknown"), "iso_a4_210x297mm");
        assert_eq!(map_paper_size(""), "iso_a4_210x297mm");
    }

    #[test]
    fn test_map_color_mode_color() {
        assert_eq!(map_color_mode("color"), "color");
        assert_eq!(map_color_mode("Color"), "color");
    }

    #[test]
    fn test_map_color_mode_monochrome() {
        assert_eq!(map_color_mode("monochrome"), "monochrome");
        assert_eq!(map_color_mode("Monochrome"), "monochrome");
        assert_eq!(map_color_mode("mono"), "monochrome");
        assert_eq!(map_color_mode("bw"), "monochrome");
        assert_eq!(map_color_mode("black-and-white"), "monochrome");
    }

    #[test]
    fn test_map_color_mode_default() {
        assert_eq!(map_color_mode("unknown"), "auto");
        assert_eq!(map_color_mode(""), "auto");
    }

    #[test]
    fn test_build_attributes_cups_mode() {
        let printer = IppPrinter::new("cups-server", 631);
        let mode = PrintMode::Cups {
            printer_name: "Canon_LBP221".to_string(),
        };
        let attributes = printer.build_attributes("test-job", &mode);

        // CUPS mode should only have job-name attribute
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name(), "job-name");
    }

    #[test]
    fn test_build_attributes_direct_ipp_with_options() {
        let printer = IppPrinter::new("localhost", 631);
        let mode = PrintMode::DirectIpp {
            printer_ip: "192.168.1.200".to_string(),
            ipp_path: "/ipp/print".to_string(),
            paper_size: Some("A4".to_string()),
            color_mode: Some("color".to_string()),
        };
        let attributes = printer.build_attributes("test-job", &mode);

        // Direct IPP mode with options should have 3 attributes
        assert_eq!(attributes.len(), 3);
        assert_eq!(attributes[0].name(), "job-name");
        assert_eq!(attributes[1].name(), "media");
        assert_eq!(attributes[2].name(), "print-color-mode");
    }

    #[test]
    fn test_build_attributes_direct_ipp_no_options() {
        let printer = IppPrinter::new("localhost", 631);
        let mode = PrintMode::DirectIpp {
            printer_ip: "192.168.1.200".to_string(),
            ipp_path: "/ipp/print".to_string(),
            paper_size: None,
            color_mode: None,
        };
        let attributes = printer.build_attributes("test-job", &mode);

        // Direct IPP mode without options should only have job-name
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name(), "job-name");
    }
}

