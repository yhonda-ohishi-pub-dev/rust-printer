use anyhow::{Context, Result};
use std::process::Command;
use tempfile::TempDir;

/// Raster output format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RasterFormat {
    /// PWG Raster (image/pwg-raster)
    Pwg,
    /// URF/Apple Raster (image/urf)
    Urf,
}

/// Convert PDF to PWG/URF Raster format
///
/// Uses external tools: pdftoppm (Poppler) and ppm2pwg
pub struct PwgConverter;

impl PwgConverter {
    /// Convert PDF data to PWG Raster format
    pub fn convert(
        pdf_data: &[u8],
        resolution: u32,
        paper_size: Option<&str>,
    ) -> Result<Vec<u8>> {
        Self::convert_with_format(pdf_data, resolution, paper_size, RasterFormat::Pwg)
    }

    /// Convert PDF data to URF (Apple Raster) format
    pub fn convert_to_urf(
        pdf_data: &[u8],
        resolution: u32,
        paper_size: Option<&str>,
    ) -> Result<Vec<u8>> {
        Self::convert_with_format(pdf_data, resolution, paper_size, RasterFormat::Urf)
    }

    /// Convert PDF data to specified raster format
    ///
    /// # Arguments
    /// * `pdf_data` - Raw PDF bytes
    /// * `resolution` - DPI resolution (default: 300)
    /// * `paper_size` - IPP media name (e.g., "iso_a4_210x297mm")
    /// * `format` - Output format (PWG or URF)
    ///
    /// # Returns
    /// Raster data as bytes
    pub fn convert_with_format(
        pdf_data: &[u8],
        resolution: u32,
        paper_size: Option<&str>,
        format: RasterFormat,
    ) -> Result<Vec<u8>> {
        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let pdf_path = temp_dir.path().join("input.pdf");
        let ppm_prefix = temp_dir.path().join("page");
        let output_ext = match format {
            RasterFormat::Pwg => "pwg",
            RasterFormat::Urf => "urf",
        };
        let output_path = temp_dir.path().join(format!("output.{}", output_ext));

        // Write PDF to temp file
        std::fs::write(&pdf_path, pdf_data).context("Failed to write PDF to temp file")?;

        // PDF → PPM using pdftoppm
        let pdftoppm_output = Command::new("pdftoppm")
            .arg("-r")
            .arg(resolution.to_string())
            .arg(&pdf_path)
            .arg(&ppm_prefix)
            .output()
            .context("Failed to execute pdftoppm")?;

        if !pdftoppm_output.status.success() {
            let stderr = String::from_utf8_lossy(&pdftoppm_output.stderr);
            anyhow::bail!("pdftoppm failed: {}", stderr);
        }

        // Find generated PPM files
        let ppm_files = Self::find_ppm_files(temp_dir.path())?;
        if ppm_files.is_empty() {
            anyhow::bail!("No PPM files generated from PDF");
        }

        let num_pages = ppm_files.len();
        tracing::info!("PDF converted to {} PPM page(s)", num_pages);

        // Concatenate all PPM files for multi-page support
        let concatenated_ppm = Self::concatenate_ppm_files(&ppm_files)?;
        let concatenated_ppm_path = temp_dir.path().join("concatenated.ppm");
        std::fs::write(&concatenated_ppm_path, &concatenated_ppm)
            .context("Failed to write concatenated PPM")?;

        // PPM → PWG/URF using ppm2pwg
        let mut ppm2pwg_cmd = Command::new("ppm2pwg");
        ppm2pwg_cmd
            .arg("-r")
            .arg(resolution.to_string())
            .arg("-f")
            .arg(output_ext)
            .arg("--num-pages")
            .arg(num_pages.to_string());

        if let Some(size) = paper_size {
            ppm2pwg_cmd.arg("--paper-size").arg(size);
        }

        ppm2pwg_cmd.arg(&concatenated_ppm_path).arg(&output_path);

        let ppm2pwg_output = ppm2pwg_cmd
            .output()
            .context("Failed to execute ppm2pwg")?;

        if !ppm2pwg_output.status.success() {
            let stderr = String::from_utf8_lossy(&ppm2pwg_output.stderr);
            anyhow::bail!("ppm2pwg failed: {}", stderr);
        }

        // Read output
        let raster_data = std::fs::read(&output_path).context("Failed to read raster output")?;

        tracing::info!(
            "{:?} conversion complete: {} bytes (from {} bytes PDF)",
            format,
            raster_data.len(),
            pdf_data.len()
        );

        Ok(raster_data)
    }

    /// Find all PPM files in directory (sorted by name for page order)
    fn find_ppm_files(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
        let mut ppm_files: Vec<_> = std::fs::read_dir(dir)
            .context("Failed to read temp directory")?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .map(|ext| ext == "ppm")
                    .unwrap_or(false)
            })
            .collect();

        ppm_files.sort();
        Ok(ppm_files)
    }

    /// Concatenate multiple PPM files into a single stream
    /// PPM format allows multiple images in sequence (PNM stream)
    fn concatenate_ppm_files(ppm_files: &[std::path::PathBuf]) -> Result<Vec<u8>> {
        let mut concatenated = Vec::new();
        for (i, ppm_path) in ppm_files.iter().enumerate() {
            let ppm_data = std::fs::read(ppm_path)
                .with_context(|| format!("Failed to read PPM file: {:?}", ppm_path))?;
            tracing::debug!("Page {} PPM size: {} bytes", i + 1, ppm_data.len());
            concatenated.extend_from_slice(&ppm_data);
        }
        tracing::info!(
            "Concatenated {} PPM files into {} bytes",
            ppm_files.len(),
            concatenated.len()
        );
        Ok(concatenated)
    }

    /// Check if conversion tools are available
    pub fn check_dependencies() -> Result<()> {
        // Check pdftoppm
        let pdftoppm = Command::new("pdftoppm")
            .arg("-v")
            .output();

        if pdftoppm.is_err() {
            anyhow::bail!("pdftoppm not found. Install poppler-utils: apt install poppler-utils");
        }

        // Check ppm2pwg
        let ppm2pwg = Command::new("ppm2pwg")
            .arg("--help")
            .output();

        if ppm2pwg.is_err() {
            anyhow::bail!("ppm2pwg not found. Build from: https://github.com/attah/ppm2pwg");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_dependencies() {
        // This test will pass if the tools are installed
        let result = PwgConverter::check_dependencies();
        println!("Dependencies check: {:?}", result);
    }
}
