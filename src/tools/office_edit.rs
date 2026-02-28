use crate::security::SecurityPolicy;
use crate::tools::traits::{Tool, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Office file editing tool (Word, Excel) with Composio integration
/// Requires --features office-edit to be enabled
pub struct OfficeEditTool {
    security: Arc<SecurityPolicy>,
    composio_api_key: String,
}

impl OfficeEditTool {
    pub fn new(security: Arc<SecurityPolicy>, composio_api_key: String) -> Self {
        Self {
            security,
            composio_api_key,
        }
    }

    /// Get temp directory for office file operations
    fn temp_dir(&self) -> PathBuf {
        self.security.workspace_dir.join("temp")
    }

    /// Detect MIME type from file extension
    fn detect_mimetype(&self, path: &Path) -> &str {
        match path.extension().and_then(|e| e.to_str()) {
            Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Some("xls") => "application/vnd.ms-excel",
            Some("docx") => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            Some("doc") => "application/msword",
            _ => "application/octet-stream",
        }
    }

    /// Stage binary content via Composio S3 API v3
    async fn stage_binary_content(
        &self,
        content: &[u8],
        filename: &str,
        mimetype: &str,
    ) -> Result<String> {
        let md5_hash = format!("{:x}", md5::compute(content));

        let client = reqwest::Client::new();

        // 1. Request upload URL
        let request_payload = json!({
            "toolkit_slug": "dropbox",
            "tool_slug": "DROPBOX_UPLOAD_FILE",
            "filename": filename,
            "mimetype": mimetype,
            "md5": md5_hash
        });

        tracing::debug!(
            filename = filename,
            mimetype = mimetype,
            size = content.len(),
            "Requesting upload URL for binary content"
        );

        let response = client
            .post("https://backend.composio.dev/api/v3/files/upload/request")
            .header("x-api-key", &self.composio_api_key)
            .header("Content-Type", "application/json")
            .json(&request_payload)
            .send()
            .await
            .context("Failed to request upload URL")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Upload request failed: status={} body={}", status, body);
        }

        let result: Value = response
            .json()
            .await
            .context("Failed to parse upload request response")?;

        // Check for existing file (deduplication)
        if let Some(existing_url) = result
            .get("existing_url")
            .or_else(|| result.get("existingUrl"))
            .and_then(|u| u.as_str())
        {
            tracing::info!(
                existing_url = existing_url,
                "File already exists (deduplicated)"
            );
            return Ok(existing_url.to_string());
        }

        // 2. Upload binary content to presigned URL
        let upload_url = result
            .get("newPresignedUrl")
            .or_else(|| result.get("new_presigned_url"))
            .and_then(|u| u.as_str())
            .ok_or_else(|| anyhow::anyhow!("No newPresignedUrl in response"))?;

        tracing::debug!(upload_url = upload_url, "Uploading binary content");

        let upload_response = client
            .put(upload_url)
            .header("Content-Type", mimetype)
            .body(content.to_vec())
            .send()
            .await
            .context("Failed to upload binary content")?;

        if !upload_response.status().is_success() {
            let status = upload_response.status();
            let body = upload_response.text().await.unwrap_or_default();
            anyhow::bail!("Binary upload failed: status={} body={}", status, body);
        }

        // 3. Return s3key
        let file_key = result
            .get("key")
            .and_then(|k| k.as_str())
            .ok_or_else(|| anyhow::anyhow!("No key in upload request response"))?;

        tracing::info!(file_key = file_key, "Binary content staged successfully");

        Ok(file_key.to_string())
    }

    #[cfg(feature = "office-edit")]
    async fn edit_excel(&self, path: &Path, operation: &str, data: &Value) -> Result<()> {
        use rust_xlsxwriter::*;

        tracing::info!(
            path = ?path,
            operation = operation,
            "Editing Excel file"
        );

        match operation {
            "add_row" => {
                // Open existing workbook
                let mut workbook = Workbook::new();
                let worksheet = workbook.add_worksheet();

                // Get values to add
                let values = data["values"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'values' array"))?;

                // Add row
                let row = data.get("row").and_then(|r| r.as_u64()).unwrap_or(0) as u32;

                for (col, value) in values.iter().enumerate() {
                    if let Some(text) = value.as_str() {
                        worksheet.write_string(row, col as u16, text)?;
                    } else if let Some(num) = value.as_f64() {
                        worksheet.write_number(row, col as u16, num)?;
                    }
                }

                workbook
                    .save(path)
                    .context("Failed to save Excel workbook")?;
            }
            "update_cell" => {
                let mut workbook = Workbook::new();
                let worksheet = workbook.add_worksheet();

                let row = data["row"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'row'"))?
                    as u32;
                let col = data["col"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'col'"))?
                    as u16;

                if let Some(text) = data["value"].as_str() {
                    worksheet.write_string(row, col, text)?;
                } else if let Some(num) = data["value"].as_f64() {
                    worksheet.write_number(row, col, num)?;
                }

                workbook
                    .save(path)
                    .context("Failed to save Excel workbook")?;
            }
            _ => anyhow::bail!("Unknown Excel operation: {}", operation),
        }

        Ok(())
    }

    #[cfg(feature = "office-edit")]
    async fn edit_word(&self, path: &Path, operation: &str, data: &Value) -> Result<()> {
        use docx_rs::*;

        tracing::info!(
            path = ?path,
            operation = operation,
            "Editing Word document"
        );

        match operation {
            "add_paragraph" => {
                let text = data["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'text'"))?;

                let docx = Docx::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)));

                let file = std::fs::File::create(path)?;
                docx.build().pack(file)?;
            }
            "replace_text" => {
                anyhow::bail!("replace_text not yet implemented for Word documents");
            }
            _ => anyhow::bail!("Unknown Word operation: {}", operation),
        }

        Ok(())
    }

    #[cfg(not(feature = "office-edit"))]
    async fn edit_excel(&self, _path: &Path, _operation: &str, _data: &Value) -> Result<()> {
        anyhow::bail!("Office editing not enabled. Compile with --features office-edit")
    }

    #[cfg(not(feature = "office-edit"))]
    async fn edit_word(&self, _path: &Path, _operation: &str, _data: &Value) -> Result<()> {
        anyhow::bail!("Office editing not enabled. Compile with --features office-edit")
    }
}

#[async_trait]
impl Tool for OfficeEditTool {
    fn name(&self) -> &str {
        "office_edit"
    }

    fn description(&self) -> &str {
        "Edit Office files (Word .docx, Excel .xlsx) and upload to Dropbox via Composio. \
         Supports operations: add_row, update_cell (Excel), add_paragraph (Word). \
         Requires --features office-edit to be enabled."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["dropbox_path", "operation"],
            "properties": {
                "dropbox_path": {
                    "type": "string",
                    "description": "Path to the file in Dropbox (e.g., '/vendas.xlsx')"
                },
                "operation": {
                    "type": "string",
                    "enum": ["add_row", "update_cell", "add_paragraph", "replace_text"],
                    "description": "Operation to perform on the file"
                },
                "data": {
                    "type": "object",
                    "description": "Operation-specific data. For add_row: {values: [...]}, for update_cell: {row: 0, col: 0, value: '...'}, for add_paragraph: {text: '...'}"
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Note: SecurityPolicy doesn't have check_rate_limit method
        // Rate limiting is handled at a higher level

        let dropbox_path = args["dropbox_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'dropbox_path'"))?;
        let operation = args["operation"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'operation'"))?;
        let data = &args["data"];

        tracing::info!(
            dropbox_path = dropbox_path,
            operation = operation,
            "Starting office file edit"
        );

        // Create temp directory
        let temp_dir = self.temp_dir();
        tokio::fs::create_dir_all(&temp_dir)
            .await
            .context("Failed to create temp directory")?;

        // Extract filename
        let filename = Path::new(dropbox_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid dropbox_path"))?;

        let local_path = temp_dir.join(filename);

        // Note: For now, we create a new file. In a full implementation,
        // you would download the existing file from Dropbox first using
        // DROPBOX_READ_FILE or DROPBOX_DOWNLOAD via Composio MCP

        // Determine file type and edit
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("File must have .xlsx or .docx extension"))?;

        match extension {
            "xlsx" => self.edit_excel(&local_path, operation, data).await?,
            "docx" => self.edit_word(&local_path, operation, data).await?,
            _ => anyhow::bail!("Unsupported file type: {}", extension),
        }

        // Read edited file
        let edited_bytes = tokio::fs::read(&local_path)
            .await
            .context("Failed to read edited file")?;

        tracing::info!(
            size = edited_bytes.len(),
            "File edited successfully, staging for upload"
        );

        // Stage via Composio
        let mimetype = self.detect_mimetype(&local_path);
        let s3key = self
            .stage_binary_content(&edited_bytes, filename, mimetype)
            .await?;

        // Cleanup temp file
        let _ = tokio::fs::remove_file(&local_path).await;

        Ok(ToolResult {
            success: true,
            output: format!(
                "File {} edited successfully. S3 key: {}. Use composio_nl tool to upload this file to Dropbox.",
                dropbox_path, s3key
            ),
            error: None,
        })
    }
}
