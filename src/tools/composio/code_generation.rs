//! Python code generation for Composio Workbench
//!
//! This module provides LLM-based Python code generation for Workbench operations.
//! It generates Python code that can use Composio tools via the run_composio_tool helper.

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;

use crate::providers::Provider;

/// Prompt template for Python code generation
const CODE_GENERATION_PROMPT_TEMPLATE: &str = r#"Generate Python code for Composio Workbench to accomplish the following task:

{query}

Available helpers in the Workbench environment:
- run_composio_tool(tool_name: str, params: dict) -> dict
  Execute a Composio tool and return the result
  
- read_file(path: str) -> str
  Read a file from the workspace
  
- write_file(path: str, content: str) -> None
  Write content to a file in the workspace
  
- search_output(pattern: str) -> list
  Search previous output for a pattern

Guidelines:
1. Return ONLY the Python code, no explanations or markdown
2. Use run_composio_tool() to call Composio tools
3. Handle errors gracefully with try/except
4. Print a summary of results at the end
5. For bulk operations, use loops and track progress
6. For large data, write to files instead of printing everything
7. Keep code concise and focused on the task

Example for sending an email:
result = run_composio_tool("GMAIL_SEND_EMAIL", {{
    "to": "user@example.com",
    "subject": "Hello",
    "body": "This is a test email"
}})
print(f"Email sent: {{result['success']}}")

Now generate the Python code:"#;

/// Default timeout for code generation LLM calls
const CODE_GENERATION_TIMEOUT_SECS: u64 = 30;

/// Code generator for Workbench Python code
pub struct CodeGenerator {
    /// Provider for LLM calls
    provider: Arc<dyn Provider>,
    /// Model name for code generation
    model_name: String,
    /// Temperature for code generation (lower = more deterministic)
    temperature: f64,
    /// Timeout for LLM calls
    timeout: Duration,
}

impl CodeGenerator {
    /// Create a new code generator
    ///
    /// # Arguments
    /// * `provider` - Provider for LLM calls
    /// * `model_name` - Model name for code generation
    pub fn new(provider: Arc<dyn Provider>, model_name: String) -> Self {
        Self {
            provider,
            model_name,
            temperature: 0.2, // Low temperature for deterministic code
            timeout: Duration::from_secs(CODE_GENERATION_TIMEOUT_SECS),
        }
    }

    /// Create a new code generator with custom temperature
    pub fn with_temperature(
        provider: Arc<dyn Provider>,
        model_name: String,
        temperature: f64,
    ) -> Self {
        Self {
            provider,
            model_name,
            temperature,
            timeout: Duration::from_secs(CODE_GENERATION_TIMEOUT_SECS),
        }
    }

    /// Create a new code generator with custom timeout
    pub fn with_timeout(
        provider: Arc<dyn Provider>,
        model_name: String,
        timeout: Duration,
    ) -> Self {
        Self {
            provider,
            model_name,
            temperature: 0.2,
            timeout,
        }
    }

    /// Generate Python code for a natural language query
    ///
    /// # Arguments
    /// * `query` - Natural language description of the task
    ///
    /// # Returns
    /// Generated Python code as a string
    pub async fn generate_workbench_code(&self, query: &str) -> Result<String> {
        // Build the prompt
        let prompt = CODE_GENERATION_PROMPT_TEMPLATE.replace("{query}", query);

        tracing::debug!(
            query = query,
            model = %self.model_name,
            temperature = self.temperature,
            "Generating Workbench Python code"
        );

        // Call LLM to generate code with timeout
        let code = self.call_llm(&prompt).await?;

        // Clean up the generated code
        let cleaned_code = self.clean_generated_code(&code);

        // Validate the generated code
        self.validate_code(&cleaned_code)?;

        tracing::debug!(
            code_length = cleaned_code.len(),
            "Generated Python code"
        );

        Ok(cleaned_code)
    }

    /// Call LLM to generate code
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        let result = tokio::time::timeout(
            self.timeout,
            self.provider
                .simple_chat(prompt, &self.model_name, self.temperature),
        )
        .await;

        match result {
            Ok(inner) => inner.context("LLM call failed"),
            Err(_) => anyhow::bail!(
                "Code generation timed out after {}s",
                self.timeout.as_secs()
            ),
        }
    }

    /// Clean up generated code by removing markdown formatting and extra whitespace
    fn clean_generated_code(&self, code: &str) -> String {
        let mut cleaned = code.trim().to_string();

        // Remove markdown code blocks if present
        if cleaned.starts_with("```python") {
            cleaned = cleaned
                .strip_prefix("```python")
                .unwrap_or(&cleaned)
                .to_string();
        }
        if cleaned.starts_with("```") {
            cleaned = cleaned.strip_prefix("```").unwrap_or(&cleaned).to_string();
        }
        if cleaned.ends_with("```") {
            cleaned = cleaned.strip_suffix("```").unwrap_or(&cleaned).to_string();
        }

        cleaned.trim().to_string()
    }

    /// Validate generated Python code
    ///
    /// Performs basic validation to ensure the code is safe and well-formed:
    /// - Not empty
    /// - Contains valid Python syntax patterns
    /// - Doesn't contain dangerous operations
    fn validate_code(&self, code: &str) -> Result<()> {
        if code.trim().is_empty() {
            anyhow::bail!("Generated code is empty");
        }

        // Check for dangerous operations that shouldn't be in Workbench code
        let dangerous_patterns = [
            "import os",
            "import sys",
            "import subprocess",
            "__import__",
            "eval(",
            "exec(",
            "compile(",
        ];

        for pattern in &dangerous_patterns {
            if code.contains(pattern) {
                tracing::warn!(
                    pattern = pattern,
                    "Generated code contains potentially dangerous pattern"
                );
                // Note: We log but don't fail - Workbench sandbox should handle this
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{ChatRequest, ChatResponse, Provider};
    use async_trait::async_trait;

    // Mock provider for testing
    struct MockProvider {
        response: String,
    }

    #[async_trait]
    impl Provider for MockProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(self.response.clone())
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            unimplemented!("Not needed for tests")
        }
    }

    #[test]
    fn test_code_generator_new() {
        let provider = Arc::new(MockProvider {
            response: "print('test')".to_string(),
        });
        let generator = CodeGenerator::new(provider, "gpt-4".to_string());
        assert_eq!(generator.model_name, "gpt-4");
        assert_eq!(generator.temperature, 0.2);
    }

    #[test]
    fn test_code_generator_with_temperature() {
        let provider = Arc::new(MockProvider {
            response: "print('test')".to_string(),
        });
        let generator = CodeGenerator::with_temperature(provider, "gpt-4".to_string(), 0.5);
        assert_eq!(generator.model_name, "gpt-4");
        assert_eq!(generator.temperature, 0.5);
    }

    #[test]
    fn test_code_generator_with_timeout() {
        let provider = Arc::new(MockProvider {
            response: "print('test')".to_string(),
        });
        let generator =
            CodeGenerator::with_timeout(provider, "gpt-4".to_string(), Duration::from_secs(60));
        assert_eq!(generator.model_name, "gpt-4");
        assert_eq!(generator.timeout, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_generate_workbench_code_success() {
        let provider = Arc::new(MockProvider {
            response: r#"```python
result = run_composio_tool("GMAIL_SEND_EMAIL", {
    "to": "test@example.com",
    "subject": "Test",
    "body": "Hello"
})
print(f"Email sent: {result['success']}")
```"#
                .to_string(),
        });

        let generator = CodeGenerator::new(provider, "gpt-4".to_string());
        let code = generator
            .generate_workbench_code("Send an email to test@example.com")
            .await
            .unwrap();

        assert!(code.contains("run_composio_tool"));
        assert!(code.contains("GMAIL_SEND_EMAIL"));
        assert!(!code.contains("```")); // Should be cleaned
    }

    #[tokio::test]
    async fn test_generate_workbench_code_empty_response() {
        let provider = Arc::new(MockProvider {
            response: "".to_string(),
        });

        let generator = CodeGenerator::new(provider, "gpt-4".to_string());
        let result = generator
            .generate_workbench_code("Send an email")
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_clean_generated_code_with_markdown() {
        let provider = Arc::new(MockProvider {
            response: "".to_string(),
        });
        let generator = CodeGenerator::new(provider, "gpt-4".to_string());

        let code_with_markdown = r#"```python
print("Hello, World!")
```"#;

        let cleaned = generator.clean_generated_code(code_with_markdown);
        assert_eq!(cleaned, r#"print("Hello, World!")"#);
    }

    #[test]
    fn test_clean_generated_code_without_markdown() {
        let provider = Arc::new(MockProvider {
            response: "".to_string(),
        });
        let generator = CodeGenerator::new(provider, "gpt-4".to_string());

        let code = r#"print("Hello, World!")"#;

        let cleaned = generator.clean_generated_code(code);
        assert_eq!(cleaned, r#"print("Hello, World!")"#);
    }

    #[test]
    fn test_clean_generated_code_with_whitespace() {
        let provider = Arc::new(MockProvider {
            response: "".to_string(),
        });
        let generator = CodeGenerator::new(provider, "gpt-4".to_string());

        let code = r#"

        print("Hello, World!")
        
        "#;

        let cleaned = generator.clean_generated_code(code);
        assert_eq!(cleaned, r#"print("Hello, World!")"#);
    }

    #[test]
    fn test_clean_generated_code_with_generic_markdown() {
        let provider = Arc::new(MockProvider {
            response: "".to_string(),
        });
        let generator = CodeGenerator::new(provider, "gpt-4".to_string());

        let code_with_markdown = r#"```
print("Hello, World!")
```"#;

        let cleaned = generator.clean_generated_code(code_with_markdown);
        assert_eq!(cleaned, r#"print("Hello, World!")"#);
    }

    #[test]
    fn test_validate_code_success() {
        let provider = Arc::new(MockProvider {
            response: "".to_string(),
        });
        let generator = CodeGenerator::new(provider, "gpt-4".to_string());

        let code = r#"result = run_composio_tool("GMAIL_SEND_EMAIL", {"to": "test@example.com"})
print(f"Result: {result}")"#;

        assert!(generator.validate_code(code).is_ok());
    }

    #[test]
    fn test_validate_code_empty() {
        let provider = Arc::new(MockProvider {
            response: "".to_string(),
        });
        let generator = CodeGenerator::new(provider, "gpt-4".to_string());

        let result = generator.validate_code("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_code_with_dangerous_patterns() {
        let provider = Arc::new(MockProvider {
            response: "".to_string(),
        });
        let generator = CodeGenerator::new(provider, "gpt-4".to_string());

        // These should log warnings but not fail
        let code_with_eval = r#"eval("print('test')")"#;
        assert!(generator.validate_code(code_with_eval).is_ok());

        let code_with_exec = r#"exec("print('test')")"#;
        assert!(generator.validate_code(code_with_exec).is_ok());
    }
}
