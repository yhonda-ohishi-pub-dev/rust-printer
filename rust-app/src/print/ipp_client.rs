use anyhow::{Context, Result};
use ipp::prelude::*;
use std::io::Cursor;

/// CUPS Printer client using IPP protocol
pub struct CupsPrinter {
    server: String,
    port: u16,
}

impl CupsPrinter {
    /// Create a new CUPS printer client
    pub fn new(server: &str, port: u16) -> Self {
        Self {
            server: server.to_string(),
            port,
        }
    }

    /// Get the IPP URI for a printer
    fn get_printer_uri(&self, printer_name: &str) -> String {
        format!(
            "ipp://{}:{}/printers/{}",
            self.server, self.port, printer_name
        )
    }

    /// Print PDF data to the specified printer
    pub async fn print(
        &self,
        pdf_data: Vec<u8>,
        job_name: &str,
        printer_name: &str,
    ) -> Result<u32> {
        let uri: Uri = self
            .get_printer_uri(printer_name)
            .parse()
            .context("Failed to parse printer URI")?;

        tracing::info!("Printing to: {}", uri);

        // Build IPP print-job operation with Cursor for Read trait
        let payload = IppPayload::new(Cursor::new(pdf_data));
        let operation = IppOperationBuilder::print_job(uri.clone(), payload)
            .attribute(IppAttribute::new(
                "job-name",
                IppValue::NameWithoutLanguage(job_name.to_string()),
            ))
            .build();

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

            tracing::info!("Print job submitted successfully, job-id: {}", job_id);
            Ok(job_id)
        } else {
            let error_msg = format!("Print job failed with status: {:?}", status);
            tracing::error!("{}", error_msg);
            anyhow::bail!(error_msg)
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
