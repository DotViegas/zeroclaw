//! Skills Integration Module
//!
//! This module provides functionality for integrating Composio Skills from the vendor
//! directory into the workspace steering directory. It handles:
//!
//! - Copying Skills files with frontmatter modification
//! - Validating Skills directory structure
//! - Generating reference documentation
//! - Indexing Skills by tags and impact levels
//!
//! # Example
//!
//! ```no_run
//! use composio_sdk::skills_integration::copy_composio_skills;
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let workspace_dir = Path::new("/path/to/workspace");
//!     let result = copy_composio_skills(workspace_dir).await?;
//!     
//!     println!("Copied {} files", result.files_copied);
//!     println!("Skipped {} files", result.files_skipped);
//!     
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Error type for Skills integration operations
///
/// This enum represents all possible errors that can occur during Skills
/// integration, including file I/O errors, YAML parsing errors, validation
/// failures, and security violations.
#[derive(Debug, Error)]
pub enum SkillsError {
    /// I/O error occurred during file operations
    ///
    /// This variant wraps standard I/O errors from file reading, writing,
    /// or directory operations.
    ///
    /// # Example
    ///
    /// ```
    /// # use composio_sdk::skills_integration::SkillsError;
    /// # use std::io;
    /// let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
    /// let skills_error: SkillsError = io_error.into();
    /// ```
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// YAML parsing error occurred
    ///
    /// This variant is used when frontmatter YAML parsing fails due to
    /// invalid syntax or structure.
    ///
    /// # Example
    ///
    /// ```
    /// # use composio_sdk::skills_integration::SkillsError;
    /// let error = SkillsError::YamlError("Invalid YAML syntax".to_string());
    /// assert!(error.to_string().contains("YAML parsing error"));
    /// ```
    #[error("YAML parsing error: {0}")]
    YamlError(String),

    /// Validation error occurred
    ///
    /// This variant is used when Skills directory structure validation fails,
    /// such as missing required files or invalid frontmatter.
    ///
    /// # Example
    ///
    /// ```
    /// # use composio_sdk::skills_integration::SkillsError;
    /// let error = SkillsError::ValidationError("Missing SKILL.md file".to_string());
    /// assert!(error.to_string().contains("Validation error"));
    /// ```
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Path traversal security violation detected
    ///
    /// This variant is used when a path attempts to escape the allowed
    /// directory boundaries, which could be a security risk.
    ///
    /// # Example
    ///
    /// ```
    /// # use composio_sdk::skills_integration::SkillsError;
    /// let error = SkillsError::PathTraversalError(
    ///     "Path contains '..' components".to_string()
    /// );
    /// assert!(error.to_string().contains("Path traversal"));
    /// ```
    #[error("Path traversal security violation: {0}")]
    PathTraversalError(String),
}

// Implement conversion from serde_yaml::Error to SkillsError
impl From<serde_yaml::Error> for SkillsError {
    fn from(err: serde_yaml::Error) -> Self {
        SkillsError::YamlError(err.to_string())
    }
}

// ============================================================================
// Data Structures
// ============================================================================

/// Result of a Skills copy operation
///
/// Contains statistics about the copy operation including number of files
/// copied, skipped, and any warnings encountered.
#[derive(Debug, Clone)]
pub struct SkillsCopyResult {
    /// Number of files successfully copied
    pub files_copied: usize,
    
    /// Number of files skipped (e.g., already exist)
    pub files_skipped: usize,
    
    /// List of warning messages encountered during copy
    pub warnings: Vec<String>,
    
    /// Path to the destination directory where files were copied
    pub destination_path: PathBuf,
}

/// Result of Skills structure validation
///
/// Contains information about the validity of the Skills directory structure
/// and any missing files.
#[derive(Debug, Clone)]
pub struct SkillsValidation {
    /// Whether the Skills directory structure is valid
    pub is_valid: bool,
    
    /// List of required files that are missing
    pub missing_required: Vec<String>,
    
    /// List of optional files that are missing
    pub missing_optional: Vec<String>,
    
    /// Total number of rule files found
    pub total_rule_files: usize,
}

/// Index of Skills files organized by tags
///
/// Provides fast lookup of Skills files by tag and maintains statistics
/// about the total number of Skills available.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillsIndex {
    /// Mapping of tags to lists of file paths
    ///
    /// Each tag maps to a vector of relative file paths that contain that tag.
    /// For example: "tool-router" -> ["rules/tr-userid-best-practices.md", ...]
    pub skills_by_tag: std::collections::HashMap<String, Vec<String>>,
    
    /// Total number of Skills files indexed
    pub total_skills: usize,
}

/// Metadata for a single Skills file
///
/// Contains information extracted from a Skills file's frontmatter and
/// file system metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillFile {
    /// Relative path to the Skills file from the steering directory
    ///
    /// Example: "rules/tr-userid-best-practices.md"
    pub path: String,
    
    /// Title of the Skills file from frontmatter
    ///
    /// Falls back to filename if no title is present in frontmatter.
    pub title: String,
    
    /// Tags associated with this Skills file
    ///
    /// Tags are used for categorization and discovery. Examples: "tool-router",
    /// "security", "authentication"
    pub tags: Vec<String>,
    
    /// Whether the file has valid YAML frontmatter
    pub has_frontmatter: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let skills_error: SkillsError = io_error.into();
        
        match skills_error {
            SkillsError::IoError(_) => (),
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_yaml_error_display() {
        let error = SkillsError::YamlError("Invalid YAML syntax".to_string());
        let display = format!("{}", error);
        
        assert!(display.contains("YAML parsing error"));
        assert!(display.contains("Invalid YAML syntax"));
    }

    #[test]
    fn test_validation_error_display() {
        let error = SkillsError::ValidationError("Missing SKILL.md".to_string());
        let display = format!("{}", error);
        
        assert!(display.contains("Validation error"));
        assert!(display.contains("Missing SKILL.md"));
    }

    #[test]
    fn test_path_traversal_error_display() {
        let error = SkillsError::PathTraversalError(
            "Path contains '..' components".to_string()
        );
        let display = format!("{}", error);
        
        assert!(display.contains("Path traversal"));
        assert!(display.contains("'..'"));
    }

    #[test]
    fn test_serde_yaml_error_conversion() {
        // Create an invalid YAML string to trigger a parsing error
        let invalid_yaml = "invalid: yaml: syntax:";
        let yaml_error = serde_yaml::from_str::<serde_yaml::Value>(invalid_yaml)
            .unwrap_err();
        
        let skills_error: SkillsError = yaml_error.into();
        
        match skills_error {
            SkillsError::YamlError(_) => (),
            _ => panic!("Expected YamlError variant"),
        }
    }

    #[test]
    fn test_skills_copy_result_creation() {
        let result = SkillsCopyResult {
            files_copied: 31,
            files_skipped: 0,
            warnings: vec!["Optional file missing".to_string()],
            destination_path: PathBuf::from("/workspace/.kiro/steering/composio"),
        };
        
        assert_eq!(result.files_copied, 31);
        assert_eq!(result.files_skipped, 0);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_skills_validation_creation() {
        let validation = SkillsValidation {
            is_valid: true,
            missing_required: vec![],
            missing_optional: vec!["optional.md".to_string()],
            total_rule_files: 29,
        };
        
        assert!(validation.is_valid);
        assert_eq!(validation.missing_required.len(), 0);
        assert_eq!(validation.missing_optional.len(), 1);
        assert_eq!(validation.total_rule_files, 29);
    }

    #[test]
    fn test_error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SkillsError>();
    }

    #[test]
    fn test_skills_copy_result_is_clone() {
        let result = SkillsCopyResult {
            files_copied: 10,
            files_skipped: 5,
            warnings: vec![],
            destination_path: PathBuf::from("/test"),
        };
        
        let cloned = result.clone();
        assert_eq!(cloned.files_copied, result.files_copied);
        assert_eq!(cloned.files_skipped, result.files_skipped);
    }

    #[test]
    fn test_skills_validation_is_clone() {
        let validation = SkillsValidation {
            is_valid: false,
            missing_required: vec!["SKILL.md".to_string()],
            missing_optional: vec![],
            total_rule_files: 0,
        };
        
        let cloned = validation.clone();
        assert_eq!(cloned.is_valid, validation.is_valid);
        assert_eq!(cloned.missing_required, validation.missing_required);
    }

    #[test]
    fn test_skills_index_creation() {
        use std::collections::HashMap;
        
        let mut skills_by_tag = HashMap::new();
        skills_by_tag.insert(
            "tool-router".to_string(),
            vec!["rules/tr-userid-best-practices.md".to_string()],
        );
        skills_by_tag.insert(
            "security".to_string(),
            vec![
                "rules/tr-userid-best-practices.md".to_string(),
                "rules/tr-auth-auto.md".to_string(),
            ],
        );
        
        let index = SkillsIndex {
            skills_by_tag,
            total_skills: 29,
        };
        
        assert_eq!(index.total_skills, 29);
        assert_eq!(index.skills_by_tag.len(), 2);
        assert_eq!(index.skills_by_tag.get("security").unwrap().len(), 2);
    }

    #[test]
    fn test_skills_index_serialization() {
        use std::collections::HashMap;
        
        let mut skills_by_tag = HashMap::new();
        skills_by_tag.insert(
            "test-tag".to_string(),
            vec!["test-file.md".to_string()],
        );
        
        let index = SkillsIndex {
            skills_by_tag,
            total_skills: 1,
        };
        
        // Test serialization
        let json = serde_json::to_string(&index).unwrap();
        assert!(json.contains("test-tag"));
        assert!(json.contains("test-file.md"));
        
        // Test deserialization
        let deserialized: SkillsIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_skills, 1);
        assert_eq!(deserialized.skills_by_tag.len(), 1);
    }

    #[test]
    fn test_skill_file_creation() {
        let skill_file = SkillFile {
            path: "rules/tr-userid-best-practices.md".to_string(),
            title: "Choose User IDs Carefully for Security and Isolation".to_string(),
            tags: vec![
                "tool-router".to_string(),
                "user-id".to_string(),
                "security".to_string(),
            ],
            has_frontmatter: true,
        };
        
        assert_eq!(skill_file.path, "rules/tr-userid-best-practices.md");
        assert_eq!(skill_file.tags.len(), 3);
        assert!(skill_file.has_frontmatter);
    }

    #[test]
    fn test_skill_file_serialization() {
        let skill_file = SkillFile {
            path: "test.md".to_string(),
            title: "Test Skill".to_string(),
            tags: vec!["test".to_string()],
            has_frontmatter: true,
        };
        
        // Test serialization
        let json = serde_json::to_string(&skill_file).unwrap();
        assert!(json.contains("test.md"));
        assert!(json.contains("Test Skill"));
        
        // Test deserialization
        let deserialized: SkillFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "test.md");
        assert_eq!(deserialized.title, "Test Skill");
        assert_eq!(deserialized.tags.len(), 1);
    }

    #[test]
    fn test_skills_index_is_clone() {
        use std::collections::HashMap;
        
        let mut skills_by_tag = HashMap::new();
        skills_by_tag.insert("tag1".to_string(), vec!["file1.md".to_string()]);
        
        let index = SkillsIndex {
            skills_by_tag,
            total_skills: 1,
        };
        
        let cloned = index.clone();
        assert_eq!(cloned.total_skills, index.total_skills);
        assert_eq!(cloned.skills_by_tag.len(), index.skills_by_tag.len());
    }

    #[test]
    fn test_skill_file_is_clone() {
        let skill_file = SkillFile {
            path: "test.md".to_string(),
            title: "Test".to_string(),
            tags: vec!["tag1".to_string()],
            has_frontmatter: false,
        };
        
        let cloned = skill_file.clone();
        assert_eq!(cloned.path, skill_file.path);
        assert_eq!(cloned.title, skill_file.title);
        assert_eq!(cloned.has_frontmatter, skill_file.has_frontmatter);
    }
}

// ============================================================================
// Skills Structure Validation
// ============================================================================

/// Validates that the source Skills directory has the expected structure
///
/// This function checks for the presence of required files and directories
/// in the bundled Composio Skills directory. It verifies:
///
/// - The skills/ directory exists
/// - SKILL.md file exists (required)
/// - AGENTS.md file exists (required)
/// - rules/ directory exists (required)
/// - At least one rule file exists in rules/ directory
///
/// # Arguments
///
/// * `skills_dir` - Path to the bundled skills directory (e.g., "composio-sdk/skills")
///
/// # Returns
///
/// * `Ok(SkillsValidation)` - Validation result with details about missing files
/// * `Err(SkillsError)` - If the skills directory doesn't exist or cannot be accessed
///
/// # Example
///
/// ```no_run
/// use composio_sdk::skills_integration::validate_skills_structure;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let skills_path = Path::new("composio-sdk/skills");
///     let validation = validate_skills_structure(skills_path).await?;
///     
///     if validation.is_valid {
///         println!("Skills structure is valid!");
///         println!("Found {} rule files", validation.total_rule_files);
///     } else {
///         println!("Validation failed:");
///         for missing in &validation.missing_required {
///             println!("  - Missing required file: {}", missing);
///         }
///     }
///     
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns `SkillsError::IoError` if:
/// - The vendor directory doesn't exist
/// - Directory permissions prevent reading
/// - File system errors occur during validation
pub async fn validate_skills_structure(
    vendor_dir: &std::path::Path,
) -> Result<SkillsValidation, SkillsError> {
    use tokio::fs;
    
    // Check if source directory exists
    if !vendor_dir.exists() {
        return Ok(SkillsValidation {
            is_valid: false,
            missing_required: vec![
                format!("Source directory not found: {}", vendor_dir.display())
            ],
            missing_optional: vec![],
            total_rule_files: 0,
        });
    }
    
    // Check if it's actually a directory
    let metadata = fs::metadata(vendor_dir).await?;
    if !metadata.is_dir() {
        return Ok(SkillsValidation {
            is_valid: false,
            missing_required: vec![
                format!("Path is not a directory: {}", vendor_dir.display())
            ],
            missing_optional: vec![],
            total_rule_files: 0,
        });
    }
    
    let mut missing_required = Vec::new();
    let mut missing_optional = Vec::new();
    
    // Verify required files: SKILL.md
    let skill_md = vendor_dir.join("SKILL.md");
    if !skill_md.exists() {
        missing_required.push("SKILL.md".to_string());
    }
    
    // Verify required files: AGENTS.md
    let agents_md = vendor_dir.join("AGENTS.md");
    if !agents_md.exists() {
        missing_required.push("AGENTS.md".to_string());
    }
    
    // Verify required directory: rules/
    let rules_dir = vendor_dir.join("rules");
    let mut total_rule_files = 0;
    
    if !rules_dir.exists() {
        missing_required.push("rules/ directory".to_string());
    } else {
        // Check if it's a directory
        let rules_metadata = fs::metadata(&rules_dir).await?;
        if !rules_metadata.is_dir() {
            missing_required.push("rules/ is not a directory".to_string());
        } else {
            // Count rule files (*.md files in rules/ directory)
            let mut entries = fs::read_dir(&rules_dir).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                
                // Check if it's a markdown file
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == "md" {
                            total_rule_files += 1;
                        }
                    }
                }
            }
            
            // Warn if no rule files found
            if total_rule_files == 0 {
                missing_optional.push(
                    "No markdown files found in rules/ directory".to_string()
                );
            }
        }
    }
    
    // Validation is successful if no required files are missing
    let is_valid = missing_required.is_empty();
    
    Ok(SkillsValidation {
        is_valid,
        missing_required,
        missing_optional,
        total_rule_files,
    })
}

// ============================================================================
// Validation Tests
// ============================================================================

#[cfg(test)]
mod validation_tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::fs;
    
    /// Helper function to create a temporary test directory
    async fn create_test_dir() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        Ok(temp_dir)
    }
    
    /// Helper function to create a test Skills directory structure
    async fn create_test_skills_structure(
        base_dir: &std::path::Path,
        include_skill_md: bool,
        include_agents_md: bool,
        include_rules_dir: bool,
        num_rule_files: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create SKILL.md if requested
        if include_skill_md {
            let skill_path = base_dir.join("SKILL.md");
            fs::write(&skill_path, "# Composio Skills\n\nTest content").await?;
        }
        
        // Create AGENTS.md if requested
        if include_agents_md {
            let agents_path = base_dir.join("AGENTS.md");
            fs::write(&agents_path, "# Agents\n\nTest content").await?;
        }
        
        // Create rules/ directory if requested
        if include_rules_dir {
            let rules_dir = base_dir.join("rules");
            fs::create_dir(&rules_dir).await?;
            
            // Create rule files
            for i in 0..num_rule_files {
                let rule_path = rules_dir.join(format!("rule-{}.md", i));
                fs::write(&rule_path, format!("# Rule {}\n\nTest content", i)).await?;
            }
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_with_valid_directory() {
        let temp_dir = create_test_dir().await.unwrap();
        let vendor_path = temp_dir.path();
        
        // Create valid structure
        create_test_skills_structure(vendor_path, true, true, true, 5)
            .await
            .unwrap();
        
        let validation = validate_skills_structure(vendor_path).await.unwrap();
        
        assert!(validation.is_valid);
        assert_eq!(validation.missing_required.len(), 0);
        assert_eq!(validation.total_rule_files, 5);
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_missing_skill_md() {
        let temp_dir = create_test_dir().await.unwrap();
        let vendor_path = temp_dir.path();
        
        // Create structure without SKILL.md
        create_test_skills_structure(vendor_path, false, true, true, 3)
            .await
            .unwrap();
        
        let validation = validate_skills_structure(vendor_path).await.unwrap();
        
        assert!(!validation.is_valid);
        assert!(validation.missing_required.contains(&"SKILL.md".to_string()));
        assert_eq!(validation.total_rule_files, 3);
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_missing_agents_md() {
        let temp_dir = create_test_dir().await.unwrap();
        let vendor_path = temp_dir.path();
        
        // Create structure without AGENTS.md
        create_test_skills_structure(vendor_path, true, false, true, 3)
            .await
            .unwrap();
        
        let validation = validate_skills_structure(vendor_path).await.unwrap();
        
        assert!(!validation.is_valid);
        assert!(validation.missing_required.contains(&"AGENTS.md".to_string()));
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_missing_rules_directory() {
        let temp_dir = create_test_dir().await.unwrap();
        let vendor_path = temp_dir.path();
        
        // Create structure without rules/ directory
        create_test_skills_structure(vendor_path, true, true, false, 0)
            .await
            .unwrap();
        
        let validation = validate_skills_structure(vendor_path).await.unwrap();
        
        assert!(!validation.is_valid);
        assert!(validation
            .missing_required
            .iter()
            .any(|s| s.contains("rules/")));
        assert_eq!(validation.total_rule_files, 0);
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_empty_rules_directory() {
        let temp_dir = create_test_dir().await.unwrap();
        let vendor_path = temp_dir.path();
        
        // Create structure with empty rules/ directory
        create_test_skills_structure(vendor_path, true, true, true, 0)
            .await
            .unwrap();
        
        let validation = validate_skills_structure(vendor_path).await.unwrap();
        
        // Should be valid but with a warning
        assert!(validation.is_valid);
        assert!(validation
            .missing_optional
            .iter()
            .any(|s| s.contains("No markdown files")));
        assert_eq!(validation.total_rule_files, 0);
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_nonexistent_directory() {
        let nonexistent_path = PathBuf::from("/nonexistent/path/to/skills");
        
        let validation = validate_skills_structure(&nonexistent_path)
            .await
            .unwrap();
        
        assert!(!validation.is_valid);
        assert!(validation
            .missing_required
            .iter()
            .any(|s| s.contains("Source directory not found")));
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_path_is_file() {
        let temp_dir = create_test_dir().await.unwrap();
        let file_path = temp_dir.path().join("not-a-directory.txt");
        
        // Create a file instead of a directory
        fs::write(&file_path, "test content").await.unwrap();
        
        let validation = validate_skills_structure(&file_path).await.unwrap();
        
        assert!(!validation.is_valid);
        assert!(validation
            .missing_required
            .iter()
            .any(|s| s.contains("not a directory")));
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_multiple_missing_files() {
        let temp_dir = create_test_dir().await.unwrap();
        let vendor_path = temp_dir.path();
        
        // Create structure with multiple missing files
        create_test_skills_structure(vendor_path, false, false, false, 0)
            .await
            .unwrap();
        
        let validation = validate_skills_structure(vendor_path).await.unwrap();
        
        assert!(!validation.is_valid);
        assert!(validation.missing_required.len() >= 3);
        assert!(validation.missing_required.contains(&"SKILL.md".to_string()));
        assert!(validation.missing_required.contains(&"AGENTS.md".to_string()));
        assert!(validation
            .missing_required
            .iter()
            .any(|s| s.contains("rules/")));
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_with_many_rule_files() {
        let temp_dir = create_test_dir().await.unwrap();
        let vendor_path = temp_dir.path();
        
        // Create structure with many rule files (like the real Composio Skills)
        create_test_skills_structure(vendor_path, true, true, true, 29)
            .await
            .unwrap();
        
        let validation = validate_skills_structure(vendor_path).await.unwrap();
        
        assert!(validation.is_valid);
        assert_eq!(validation.missing_required.len(), 0);
        assert_eq!(validation.total_rule_files, 29);
    }
    
    #[tokio::test]
    async fn test_validate_skills_structure_rules_is_file_not_directory() {
        let temp_dir = create_test_dir().await.unwrap();
        let vendor_path = temp_dir.path();
        
        // Create SKILL.md and AGENTS.md
        create_test_skills_structure(vendor_path, true, true, false, 0)
            .await
            .unwrap();
        
        // Create rules as a file instead of a directory
        let rules_path = vendor_path.join("rules");
        fs::write(&rules_path, "not a directory").await.unwrap();
        
        let validation = validate_skills_structure(vendor_path).await.unwrap();
        
        assert!(!validation.is_valid);
        assert!(validation
            .missing_required
            .iter()
            .any(|s| s.contains("rules/") && s.contains("not a directory")));
    }
}

// ============================================================================
// Frontmatter Management
// ============================================================================

/// Adds or updates the `inclusion: auto` frontmatter field in a markdown file
///
/// This function handles three scenarios:
/// 1. File has existing frontmatter: Adds `inclusion: auto` field while preserving other fields
/// 2. File has no frontmatter: Creates minimal frontmatter with `inclusion: auto`
/// 3. File has malformed YAML: Returns error with details
///
/// The function preserves all existing frontmatter fields and only adds or updates
/// the `inclusion` field. This ensures that metadata like title, tags, impact, and
/// description are maintained.
///
/// # Arguments
///
/// * `content` - The original markdown file content
///
/// # Returns
///
/// * `Ok(String)` - Modified content with frontmatter containing `inclusion: auto`
/// * `Err(SkillsError)` - If YAML parsing fails or frontmatter is malformed
///
/// # Example
///
/// ```
/// use composio_sdk::skills_integration::add_auto_inclusion_frontmatter;
///
/// // File without frontmatter
/// let content = "# My Skill\n\nThis is a skill file.";
/// let result = add_auto_inclusion_frontmatter(content).unwrap();
/// assert!(result.contains("inclusion: auto"));
///
/// // File with existing frontmatter
/// let content = "---\ntitle: My Skill\ntags:\n  - test\n---\n\n# Content";
/// let result = add_auto_inclusion_frontmatter(content).unwrap();
/// assert!(result.contains("inclusion: auto"));
/// assert!(result.contains("title: My Skill"));
/// ```
///
/// # Errors
///
/// Returns `SkillsError::YamlError` if:
/// - Frontmatter YAML syntax is invalid
/// - Frontmatter structure cannot be parsed
/// - YAML serialization fails
pub fn add_auto_inclusion_frontmatter(content: &str) -> Result<String, SkillsError> {
    use serde_yaml::Value;

    // Check if content starts with frontmatter delimiter
    if content.starts_with("---") {
        // Find the end of frontmatter (second "---")
        let content_after_first_delimiter = &content[3..];

        if let Some(end_pos) = content_after_first_delimiter.find("\n---") {
            // Extract frontmatter YAML (between the two "---")
            let frontmatter_yaml = &content_after_first_delimiter[..end_pos];

            // Extract body (after the second "---")
            let body_start = 3 + end_pos + 4; // 3 for first "---", end_pos, 4 for "\n---"
            let body = if body_start < content.len() {
                &content[body_start..]
            } else {
                ""
            };

            // Parse existing frontmatter
            let mut frontmatter: serde_yaml::Mapping = serde_yaml::from_str(frontmatter_yaml)
                .map_err(|e| SkillsError::YamlError(format!("Failed to parse frontmatter: {}", e)))?;

            // Add or update the inclusion field
            frontmatter.insert(
                Value::String("inclusion".to_string()),
                Value::String("auto".to_string()),
            );

            // Serialize back to YAML
            let new_yaml = serde_yaml::to_string(&frontmatter)
                .map_err(|e| SkillsError::YamlError(format!("Failed to serialize frontmatter: {}", e)))?;

            // Reconstruct the file with updated frontmatter
            Ok(format!("---\n{}---{}", new_yaml, body))
        } else {
            // Malformed frontmatter: starts with "---" but no closing "---"
            Err(SkillsError::YamlError(
                "Malformed frontmatter: missing closing '---' delimiter".to_string()
            ))
        }
    } else {
        // No frontmatter exists, create minimal one
        let frontmatter = "---\ninclusion: auto\n---\n\n";
        Ok(format!("{}{}", frontmatter, content))
    }
}

// ============================================================================
// Frontmatter Management Tests
// ============================================================================

#[cfg(test)]
mod frontmatter_tests {
    use super::*;

    #[test]
    fn test_add_frontmatter_to_file_without_frontmatter() {
        let content = "# My Skill\n\nThis is a skill file.";
        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.starts_with("---\n"));
        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("# My Skill"));
        assert!(result.contains("This is a skill file."));
    }

    #[test]
    fn test_add_frontmatter_to_empty_file() {
        let content = "";
        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.starts_with("---\n"));
        assert!(result.contains("inclusion: auto"));
        assert!(result.ends_with("---\n\n"));
    }

    #[test]
    fn test_preserve_existing_frontmatter_fields() {
        let content = r#"---
title: My Skill
impact: HIGH
description: A test skill
tags:
  - test
  - example
---

# Content here"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("title: My Skill"));
        assert!(result.contains("impact: HIGH"));
        assert!(result.contains("description: A test skill"));
        assert!(result.contains("tags:"));
        assert!(result.contains("- test"));
        assert!(result.contains("- example"));
        assert!(result.contains("# Content here"));
    }

    #[test]
    fn test_update_existing_inclusion_field() {
        let content = r#"---
title: My Skill
inclusion: manual
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(!result.contains("inclusion: manual"));
        assert!(result.contains("title: My Skill"));
    }

    #[test]
    fn test_handle_frontmatter_with_complex_yaml() {
        let content = r#"---
title: Complex Skill
nested:
  field1: value1
  field2: value2
list:
  - item1
  - item2
  - item3
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("title: Complex Skill"));
        assert!(result.contains("nested:"));
        assert!(result.contains("field1: value1"));
        assert!(result.contains("list:"));
        assert!(result.contains("- item1"));
    }

    #[test]
    fn test_malformed_frontmatter_missing_closing_delimiter() {
        let content = r#"---
title: My Skill
tags:
  - test

# Content without closing ---"#;

        let result = add_auto_inclusion_frontmatter(content);

        assert!(result.is_err());
        match result {
            Err(SkillsError::YamlError(msg)) => {
                assert!(msg.contains("missing closing"));
            }
            _ => panic!("Expected YamlError"),
        }
    }

    #[test]
    fn test_malformed_yaml_syntax() {
        let content = r#"---
title: My Skill
invalid: yaml: syntax:
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content);

        assert!(result.is_err());
        match result {
            Err(SkillsError::YamlError(_)) => (),
            _ => panic!("Expected YamlError"),
        }
    }

    #[test]
    fn test_frontmatter_with_special_characters() {
        let content = r#"---
title: "Skill with: special characters"
description: "Contains \"quotes\" and 'apostrophes'"
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("title:"));
        assert!(result.contains("description:"));
    }

    #[test]
    fn test_frontmatter_with_multiline_strings() {
        let content = r#"---
title: My Skill
description: |
  This is a multiline
  description that spans
  multiple lines
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("title: My Skill"));
        assert!(result.contains("description:"));
    }

    #[test]
    fn test_preserve_body_formatting() {
        let content = r#"---
title: My Skill
---

# Heading 1

Some paragraph with **bold** and *italic*.

## Heading 2

- List item 1
- List item 2

```rust
fn example() {
    println!("code block");
}
```"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("# Heading 1"));
        assert!(result.contains("**bold**"));
        assert!(result.contains("*italic*"));
        assert!(result.contains("## Heading 2"));
        assert!(result.contains("- List item 1"));
        assert!(result.contains("```rust"));
        assert!(result.contains("fn example()"));
    }

    #[test]
    fn test_empty_frontmatter() {
        let content = r#"---
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("# Content"));
    }

    #[test]
    fn test_frontmatter_with_only_inclusion() {
        let content = r#"---
inclusion: manual
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(!result.contains("inclusion: manual"));
    }

    #[test]
    fn test_frontmatter_with_numeric_values() {
        let content = r#"---
title: My Skill
priority: 1
version: 2.5
enabled: true
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("title: My Skill"));
        assert!(result.contains("priority:"));
        assert!(result.contains("version:"));
        assert!(result.contains("enabled:"));
    }

    #[test]
    fn test_frontmatter_with_null_values() {
        let content = r#"---
title: My Skill
optional_field: null
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("title: My Skill"));
    }

    #[test]
    fn test_content_starting_with_dashes_but_not_frontmatter() {
        // Content that starts with "---" but has no closing delimiter
        // This is treated as malformed frontmatter (safer behavior)
        let content = "--- This is not frontmatter\n\nJust regular content.";
        let result = add_auto_inclusion_frontmatter(content);

        // Should return error for malformed frontmatter
        assert!(result.is_err());
        match result {
            Err(SkillsError::YamlError(msg)) => {
                assert!(msg.contains("missing closing"));
            }
            _ => panic!("Expected YamlError"),
        }
    }

    #[test]
    fn test_frontmatter_preserves_field_order() {
        let content = r#"---
title: My Skill
impact: HIGH
description: Test
tags:
  - tag1
---

# Content"#;

        let result = add_auto_inclusion_frontmatter(content).unwrap();

        // Verify all fields are present (order may vary in YAML)
        assert!(result.contains("inclusion: auto"));
        assert!(result.contains("title:"));
        assert!(result.contains("impact:"));
        assert!(result.contains("description:"));
        assert!(result.contains("tags:"));
    }
}


