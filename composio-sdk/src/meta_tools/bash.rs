//! Bash Executor Implementation
//!
//! Native Rust implementation of COMPOSIO_REMOTE_BASH_TOOL meta tool.
//! Executes bash commands in an isolated sandbox environment.

use crate::error::ComposioError;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Bash execution result
#[derive(Debug, Clone)]
pub struct BashResult {
    /// Standard output
    pub stdout: String,
    
    /// Standard error
    pub stderr: String,
    
    /// Exit code
    pub exit_code: i32,
    
    /// Execution time in milliseconds
    pub execution_time_ms: u128,
}

/// Bash executor with sandboxing
pub struct BashExecutor {
    /// Sandbox directory for command execution
    sandbox_dir: PathBuf,
    
    /// Timeout in seconds (default: 30)
    timeout_secs: u64,
    
    /// Environment variables
    env_vars: Vec<(String, String)>,
}

impl BashExecutor {
    /// Create a new bash executor with default sandbox
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::meta_tools::BashExecutor;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = BashExecutor::new();
    /// let result = executor.execute("ls -la").await?;
    /// println!("Output: {}", result.stdout);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Self {
        Self {
            sandbox_dir: std::env::temp_dir().join("composio_sandbox"),
            timeout_secs: 30,
            env_vars: Vec::new(),
        }
    }

    /// Create a bash executor with custom sandbox directory
    ///
    /// # Arguments
    ///
    /// * `sandbox_dir` - Directory to use as sandbox
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::meta_tools::BashExecutor;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = BashExecutor::with_sandbox(PathBuf::from("/tmp/my_sandbox"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_sandbox(sandbox_dir: PathBuf) -> Self {
        Self {
            sandbox_dir,
            timeout_secs: 30,
            env_vars: Vec::new(),
        }
    }

    /// Set execution timeout
    ///
    /// # Arguments
    ///
    /// * `timeout_secs` - Timeout in seconds
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::meta_tools::BashExecutor;
    ///
    /// let executor = BashExecutor::new().timeout(60);
    /// ```
    pub fn timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Add environment variable
    ///
    /// # Arguments
    ///
    /// * `key` - Variable name
    /// * `value` - Variable value
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::meta_tools::BashExecutor;
    ///
    /// let executor = BashExecutor::new()
    ///     .env("PATH", "/usr/local/bin:/usr/bin")
    ///     .env("HOME", "/tmp");
    /// ```
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    /// Execute a bash command
    ///
    /// # Arguments
    ///
    /// * `command` - Bash command to execute
    ///
    /// # Returns
    ///
    /// Bash execution result with stdout, stderr, and exit code
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::meta_tools::BashExecutor;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = BashExecutor::new();
    /// let result = executor.execute("echo 'Hello, World!'").await?;
    ///
    /// println!("Output: {}", result.stdout);
    /// println!("Exit code: {}", result.exit_code);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(&self, command: &str) -> Result<BashResult, ComposioError> {
        // Ensure sandbox directory exists
        if !self.sandbox_dir.exists() {
            tokio::fs::create_dir_all(&self.sandbox_dir)
                .await
                .map_err(|e| ComposioError::ExecutionError(format!("Failed to create sandbox: {}", e)))?;
        }

        let start_time = std::time::Instant::now();

        // Build command
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&self.sandbox_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Execute with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            cmd.output(),
        )
        .await
        .map_err(|_| {
            ComposioError::ExecutionError(format!(
                "Command timed out after {} seconds",
                self.timeout_secs
            ))
        })?
        .map_err(|e| ComposioError::ExecutionError(format!("Failed to execute command: {}", e)))?;

        let execution_time_ms = start_time.elapsed().as_millis();

        Ok(BashResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            execution_time_ms,
        })
    }

    /// Execute multiple commands sequentially
    ///
    /// # Arguments
    ///
    /// * `commands` - Vector of bash commands
    ///
    /// # Returns
    ///
    /// Vector of bash results (one per command)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::meta_tools::BashExecutor;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = BashExecutor::new();
    /// let results = executor.execute_batch(vec![
    ///     "echo 'Step 1'",
    ///     "echo 'Step 2'",
    ///     "echo 'Step 3'",
    /// ]).await?;
    ///
    /// for (i, result) in results.iter().enumerate() {
    ///     println!("Command {}: {}", i + 1, result.stdout);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_batch(&self, commands: Vec<&str>) -> Result<Vec<BashResult>, ComposioError> {
        let mut results = Vec::new();

        for command in commands {
            let result = self.execute(command).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get sandbox directory path
    pub fn sandbox_dir(&self) -> &PathBuf {
        &self.sandbox_dir
    }
}

impl Default for BashExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bash_executor_echo() {
        let executor = BashExecutor::new();
        let result = executor.execute("echo 'Hello, World!'").await.unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("Hello, World!"));
        assert!(result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_bash_executor_with_error() {
        let executor = BashExecutor::new();
        let result = executor.execute("ls /nonexistent_directory").await.unwrap();

        assert_ne!(result.exit_code, 0);
        assert!(!result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_bash_executor_with_env() {
        let executor = BashExecutor::new().env("TEST_VAR", "test_value");
        let result = executor.execute("echo $TEST_VAR").await.unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("test_value"));
    }

    #[tokio::test]
    async fn test_bash_executor_batch() {
        let executor = BashExecutor::new();
        let results = executor
            .execute_batch(vec!["echo 'Step 1'", "echo 'Step 2'", "echo 'Step 3'"])
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        assert!(results[0].stdout.contains("Step 1"));
        assert!(results[1].stdout.contains("Step 2"));
        assert!(results[2].stdout.contains("Step 3"));
    }

    #[tokio::test]
    async fn test_bash_executor_timeout() {
        let executor = BashExecutor::new().timeout(1);
        let result = executor.execute("sleep 5").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[test]
    fn test_bash_result_clone() {
        let result = BashResult {
            stdout: "output".to_string(),
            stderr: "error".to_string(),
            exit_code: 0,
            execution_time_ms: 100,
        };

        let cloned = result.clone();
        assert_eq!(cloned.stdout, "output");
        assert_eq!(cloned.exit_code, 0);
    }
}
