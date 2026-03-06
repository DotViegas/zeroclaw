//! Workbench Executor Implementation
//!
//! Hybrid implementation: Rust wrapper + remote Python execution.
//! The workbench provides a persistent Python sandbox for bulk operations,
//! data analysis, and complex workflows that would overflow the context window.

use crate::client::ComposioClient;
use crate::error::ComposioError;
use crate::models::MetaToolSlug;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Workbench execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchResult {
    /// Execution output
    pub output: String,
    
    /// Whether execution was successful
    pub successful: bool,
    
    /// Error message (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Session ID used
    pub session_id: String,
    
    /// Files created/modified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
}

/// Pandas operation types
#[derive(Debug, Clone)]
pub enum PandasOperation {
    /// Read CSV from URL
    ReadCsv { url: String },
    
    /// Filter rows by column value
    FilterRows { column: String, value: String },
    
    /// Group by column
    GroupBy { column: String },
    
    /// Aggregate with operation
    Aggregate { column: String, operation: String },
    
    /// Sort by column
    SortBy { column: String, ascending: bool },
    
    /// Custom pandas code
    Custom { code: String },
}

/// Excel operation types
#[derive(Debug, Clone)]
pub enum ExcelOperation {
    /// Read Excel file
    Read { s3_url: String },
    
    /// Edit Excel file (preserves existing content)
    Edit {
        s3_url: String,
        operations: Vec<String>,
        upload_tool: String,
        file_path: String,
    },
    
    /// Add rows to Excel
    AddRows {
        s3_url: String,
        rows: Vec<Vec<String>>,
        upload_tool: String,
        file_path: String,
    },
}

/// Workbench executor
pub struct WorkbenchExecutor {
    client: Arc<ComposioClient>,
    session_id: String,
}

impl WorkbenchExecutor {
    /// Create a new workbench executor
    ///
    /// # Arguments
    ///
    /// * `client` - Composio client instance
    /// * `session_id` - Session ID for workbench context
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::{ComposioClient, meta_tools::WorkbenchExecutor};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your-api-key")
    ///     .build()?;
    ///
    /// let executor = WorkbenchExecutor::new(Arc::new(client), "session_123");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: Arc<ComposioClient>, session_id: impl Into<String>) -> Self {
        Self {
            client,
            session_id: session_id.into(),
        }
    }

    /// Execute Python code in the workbench
    ///
    /// # Arguments
    ///
    /// * `code` - Python code to execute
    ///
    /// # Returns
    ///
    /// Workbench execution result
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::WorkbenchExecutor};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let executor = WorkbenchExecutor::new(client, "session_123");
    ///
    /// let code = r#"
    /// import pandas as pd
    /// df = pd.DataFrame({'a': [1, 2, 3], 'b': [4, 5, 6]})
    /// print(df.describe())
    /// "#;
    ///
    /// let result = executor.execute_python(code).await?;
    /// println!("Output: {}", result.output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_python(&self, code: &str) -> Result<WorkbenchResult, ComposioError> {
        // Validate Python syntax (basic check)
        self.validate_python_syntax(code)?;

        // Execute via COMPOSIO_REMOTE_WORKBENCH meta tool
        let url = format!(
            "{}/tool_router/session/{}/execute_meta",
            self.client.base_url(),
            self.session_id
        );

        let response = self
            .client
            .http_client()
            .post(&url)
            .json(&serde_json::json!({
                "tool_slug": MetaToolSlug::ComposioRemoteWorkbench,
                "arguments": {
                    "code": code,
                    "session_id": self.session_id,
                }
            }))
            .send()
            .await
            .map_err(|e| ComposioError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ComposioError::ApiError {
                status: status.as_u16(),
                message: error_text,
                request_id: None,
                suggested_fix: None,
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ComposioError::SerializationError(e.to_string()))?;

        // Parse workbench result
        let result = WorkbenchResult {
            output: data["data"]["output"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            successful: data["data"]["successful"].as_bool().unwrap_or(false),
            error: data["data"]["error"].as_str().map(|s| s.to_string()),
            session_id: self.session_id.clone(),
            files: data["data"]["files"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                }),
        };

        Ok(result)
    }

    /// Generate Python code for pandas operations
    ///
    /// # Arguments
    ///
    /// * `operation` - Pandas operation to generate code for
    ///
    /// # Returns
    ///
    /// Python code string
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::{WorkbenchExecutor, PandasOperation}};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let executor = WorkbenchExecutor::new(client, "session_123");
    ///
    /// let code = executor.generate_pandas_code(PandasOperation::ReadCsv {
    ///     url: "https://example.com/data.csv".to_string(),
    /// });
    ///
    /// let result = executor.execute_python(&code).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn generate_pandas_code(&self, operation: PandasOperation) -> String {
        match operation {
            PandasOperation::ReadCsv { url } => {
                format!(
                    r#"
import pandas as pd
import requests

# Download CSV
response = requests.get("{}")
df = pd.read_csv(response.content)
print(df.head())
print(f"\nShape: {{df.shape}}")
print(f"Columns: {{df.columns.tolist()}}")
"#,
                    url
                )
            }
            PandasOperation::FilterRows { column, value } => {
                format!(
                    r#"
# Filter dataframe
filtered = df[df['{}'] == '{}']
print(f"Found {{len(filtered)}} rows")
print(filtered)
"#,
                    column, value
                )
            }
            PandasOperation::GroupBy { column } => {
                format!(
                    r#"
# Group by column
grouped = df.groupby('{}')
print(grouped.size())
"#,
                    column
                )
            }
            PandasOperation::Aggregate { column, operation } => {
                format!(
                    r#"
# Aggregate
result = df['{}'].{}()
print(f"{} of {}: {{result}}")
"#,
                    column, operation, operation, column
                )
            }
            PandasOperation::SortBy { column, ascending } => {
                format!(
                    r#"
# Sort by column
sorted_df = df.sort_values('{}', ascending={})
print(sorted_df.head())
"#,
                    column, ascending
                )
            }
            PandasOperation::Custom { code } => code,
        }
    }

    /// Generate Python code for Excel operations
    ///
    /// # Arguments
    ///
    /// * `operation` - Excel operation to generate code for
    ///
    /// # Returns
    ///
    /// Python code string
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::{WorkbenchExecutor, ExcelOperation}};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let executor = WorkbenchExecutor::new(client, "session_123");
    ///
    /// let code = executor.generate_excel_code(ExcelOperation::Read {
    ///     s3_url: "https://s3.amazonaws.com/bucket/file.xlsx".to_string(),
    /// });
    ///
    /// let result = executor.execute_python(&code).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn generate_excel_code(&self, operation: ExcelOperation) -> String {
        match operation {
            ExcelOperation::Read { s3_url } => {
                format!(
                    r#"
import openpyxl
import requests

# Download Excel file
response = requests.get('{}')
with open('temp.xlsx', 'wb') as f:
    f.write(response.content)

# Load workbook
wb = openpyxl.load_workbook('temp.xlsx')
ws = wb.active

# Print content
print(f"Sheet: {{ws.title}}")
print(f"Dimensions: {{ws.dimensions}}")
print("\nFirst 10 rows:")
for i, row in enumerate(ws.iter_rows(values_only=True), 1):
    if i > 10:
        break
    print(row)
"#,
                    s3_url
                )
            }
            ExcelOperation::Edit {
                s3_url,
                operations,
                upload_tool,
                file_path,
            } => {
                let ops_code = operations.join("\n");
                format!(
                    r#"
import openpyxl
import requests

# Download existing file
response = requests.get('{}')
with open('temp.xlsx', 'wb') as f:
    f.write(response.content)

# Load and edit
wb = openpyxl.load_workbook('temp.xlsx')
ws = wb.active

# Apply operations
{}

# Save
wb.save('temp.xlsx')

# Upload back
with open('temp.xlsx', 'rb') as f:
    result = run_composio_tool('{}', {{
        'path': '{}',
        'content': f.read()
    }})
print(result)
"#,
                    s3_url, ops_code, upload_tool, file_path
                )
            }
            ExcelOperation::AddRows {
                s3_url,
                rows,
                upload_tool,
                file_path,
            } => {
                let rows_code = rows
                    .iter()
                    .map(|row| format!("ws.append({:?})", row))
                    .collect::<Vec<_>>()
                    .join("\n");

                format!(
                    r#"
import openpyxl
import requests

# Download existing file
response = requests.get('{}')
with open('temp.xlsx', 'wb') as f:
    f.write(response.content)

# Load workbook
wb = openpyxl.load_workbook('temp.xlsx')
ws = wb.active

# Add new rows
{}

# Save
wb.save('temp.xlsx')

# Upload back
with open('temp.xlsx', 'rb') as f:
    result = run_composio_tool('{}', {{
        'path': '{}',
        'content': f.read()
    }})
print(result)
"#,
                    s3_url, rows_code, upload_tool, file_path
                )
            }
        }
    }

    /// Validate Python syntax (basic check)
    fn validate_python_syntax(&self, code: &str) -> Result<(), ComposioError> {
        // Basic validation: check for common syntax errors
        if code.trim().is_empty() {
            return Err(ComposioError::ValidationError(
                "Python code cannot be empty".to_string(),
            ));
        }

        // Check for balanced parentheses, brackets, braces
        let mut paren_count = 0;
        let mut bracket_count = 0;
        let mut brace_count = 0;

        for ch in code.chars() {
            match ch {
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                '[' => bracket_count += 1,
                ']' => bracket_count -= 1,
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                _ => {}
            }
        }

        if paren_count != 0 {
            return Err(ComposioError::ValidationError(
                "Unbalanced parentheses in Python code".to_string(),
            ));
        }

        if bracket_count != 0 {
            return Err(ComposioError::ValidationError(
                "Unbalanced brackets in Python code".to_string(),
            ));
        }

        if brace_count != 0 {
            return Err(ComposioError::ValidationError(
                "Unbalanced braces in Python code".to_string(),
            ));
        }

        Ok(())
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pandas_read_csv_code_generation() {
        let executor = WorkbenchExecutor::new(
            Arc::new(ComposioClient::builder().api_key("test").build().unwrap()),
            "session_123",
        );

        let code = executor.generate_pandas_code(PandasOperation::ReadCsv {
            url: "https://example.com/data.csv".to_string(),
        });

        assert!(code.contains("import pandas as pd"));
        assert!(code.contains("requests.get"));
        assert!(code.contains("https://example.com/data.csv"));
        assert!(code.contains("pd.read_csv"));
    }

    #[test]
    fn test_pandas_filter_code_generation() {
        let executor = WorkbenchExecutor::new(
            Arc::new(ComposioClient::builder().api_key("test").build().unwrap()),
            "session_123",
        );

        let code = executor.generate_pandas_code(PandasOperation::FilterRows {
            column: "age".to_string(),
            value: "25".to_string(),
        });

        assert!(code.contains("df['age']"));
        assert!(code.contains("== '25'"));
    }

    #[test]
    fn test_excel_read_code_generation() {
        let executor = WorkbenchExecutor::new(
            Arc::new(ComposioClient::builder().api_key("test").build().unwrap()),
            "session_123",
        );

        let code = executor.generate_excel_code(ExcelOperation::Read {
            s3_url: "https://s3.amazonaws.com/bucket/file.xlsx".to_string(),
        });

        assert!(code.contains("import openpyxl"));
        assert!(code.contains("requests.get"));
        assert!(code.contains("load_workbook"));
    }

    #[test]
    fn test_python_syntax_validation_empty() {
        let executor = WorkbenchExecutor::new(
            Arc::new(ComposioClient::builder().api_key("test").build().unwrap()),
            "session_123",
        );

        let result = executor.validate_python_syntax("");
        assert!(result.is_err());
    }

    #[test]
    fn test_python_syntax_validation_unbalanced_parens() {
        let executor = WorkbenchExecutor::new(
            Arc::new(ComposioClient::builder().api_key("test").build().unwrap()),
            "session_123",
        );

        let result = executor.validate_python_syntax("print('hello'");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parentheses"));
    }

    #[test]
    fn test_python_syntax_validation_valid() {
        let executor = WorkbenchExecutor::new(
            Arc::new(ComposioClient::builder().api_key("test").build().unwrap()),
            "session_123",
        );

        let result = executor.validate_python_syntax("print('hello')");
        assert!(result.is_ok());
    }
}
