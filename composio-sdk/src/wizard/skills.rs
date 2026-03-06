//! Skills extraction utilities for wizard instruction generation
//!
//! This module provides functionality to extract and parse Composio Skills
//! content from the official Skills repository.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Errors that can occur during skills extraction
#[derive(Debug, thiserror::Error)]
pub enum SkillsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse frontmatter: {0}")]
    FrontmatterParse(String),

    #[error("Invalid impact level: {0}")]
    InvalidImpact(String),

    #[error("Skills path not found: {0}")]
    PathNotFound(PathBuf),
}

/// Impact level of a rule
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Impact {
    Critical,
    High,
    Medium,
    Low,
}

impl Impact {
    /// Parse impact from string
    pub fn from_str(s: &str) -> Result<Self, SkillsError> {
        match s.to_uppercase().as_str() {
            "CRITICAL" => Ok(Impact::Critical),
            "HIGH" => Ok(Impact::High),
            "MEDIUM" => Ok(Impact::Medium),
            "LOW" => Ok(Impact::Low),
            _ => Err(SkillsError::InvalidImpact(s.to_string())),
        }
    }

    /// Convert impact to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Impact::Critical => "CRITICAL",
            Impact::High => "HIGH",
            Impact::Medium => "MEDIUM",
            Impact::Low => "LOW",
        }
    }
}

/// A rule extracted from the Skills repository
#[derive(Debug, Clone)]
pub struct Rule {
    pub title: String,
    pub impact: Impact,
    pub description: String,
    pub tags: Vec<String>,
    pub content: String,
    pub correct_examples: Vec<String>,
    pub incorrect_examples: Vec<String>,
}

impl Rule {
    /// Parse a rule from a markdown file
    pub fn from_file(path: &Path) -> Result<Self, SkillsError> {
        let content = fs::read_to_string(path)?;
        Self::from_content(&content)
    }

    /// Parse a rule from markdown content
    pub fn from_content(content: &str) -> Result<Self, SkillsError> {
        let (frontmatter, body) = Self::parse_markdown(content)?;

        let title = frontmatter
            .get("title")
            .cloned()
            .unwrap_or_else(|| "Untitled".to_string());

        let impact = frontmatter
            .get("impact")
            .map(|s| Impact::from_str(s))
            .transpose()?
            .unwrap_or(Impact::Medium);

        let description = frontmatter
            .get("description")
            .cloned()
            .unwrap_or_default();

        let tags = frontmatter
            .get("tags")
            .map(|t| {
                t.trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let correct_examples = Self::extract_examples(&body, "✅");
        let incorrect_examples = Self::extract_examples(&body, "❌");

        Ok(Self {
            title,
            impact,
            description,
            tags,
            content: body,
            correct_examples,
            incorrect_examples,
        })
    }

    /// Parse markdown with frontmatter
    fn parse_markdown(content: &str) -> Result<(HashMap<String, String>, String), SkillsError> {
        let mut frontmatter = HashMap::new();
        let body;

        let lines: Vec<&str> = content.lines().collect();
        let mut in_frontmatter = false;
        let mut frontmatter_end = 0;

        // Check if content starts with frontmatter
        if lines.first().map(|l| l.trim()) == Some("---") {
            in_frontmatter = true;
            frontmatter_end = 1;

            // Parse frontmatter
            for (i, line) in lines.iter().enumerate().skip(1) {
                if line.trim() == "---" {
                    frontmatter_end = i + 1;
                    break;
                }

                if let Some((key, value)) = line.split_once(':') {
                    frontmatter.insert(
                        key.trim().to_string(),
                        value.trim().to_string(),
                    );
                }
            }
        }

        // Extract body
        if in_frontmatter && frontmatter_end < lines.len() {
            body = lines[frontmatter_end..].join("\n");
        } else {
            body = content.to_string();
        }

        Ok((frontmatter, body))
    }

    /// Extract code examples following a marker (✅ or ❌)
    fn extract_examples(content: &str, marker: &str) -> Vec<String> {
        let mut examples = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Look for marker
            if line.contains(marker) {
                // Look for code block after marker
                let mut j = i + 1;
                while j < lines.len() {
                    let next_line = lines[j].trim();

                    // Found code block start
                    if next_line.starts_with("```") {
                        let mut code = String::new();
                        j += 1;

                        // Extract code until closing ```
                        while j < lines.len() {
                            let code_line = lines[j];
                            if code_line.trim().starts_with("```") {
                                break;
                            }
                            code.push_str(code_line);
                            code.push('\n');
                            j += 1;
                        }

                        if !code.trim().is_empty() {
                            examples.push(code.trim().to_string());
                        }
                        break;
                    }

                    // Stop if we hit another marker or section
                    if next_line.contains("✅") || next_line.contains("❌") || next_line.starts_with("##") {
                        break;
                    }

                    j += 1;
                }
            }

            i += 1;
        }

        examples
    }
}

/// Extractor for Composio Skills content
#[derive(Debug, Clone)]
pub struct SkillsExtractor {
    skills_path: PathBuf,
}

impl SkillsExtractor {
    /// Create a new skills extractor
    ///
    /// # Arguments
    ///
    /// * `skills_path` - Path to the bundled Skills directory (e.g., "composio-sdk/skills")
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::SkillsExtractor;
    ///
    /// // Skills are bundled within the SDK
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let extractor = SkillsExtractor::new(skills_path);
    /// ```
    pub fn new(skills_path: impl Into<PathBuf>) -> Self {
        Self {
            skills_path: skills_path.into(),
        }
    }

    /// Verify that the skills path exists
    pub fn verify_path(&self) -> Result<(), SkillsError> {
        if !self.skills_path.exists() {
            return Err(SkillsError::PathNotFound(self.skills_path.clone()));
        }
        Ok(())
    }

    /// Extract Tool Router rules (tr-*.md files)
    ///
    /// # Returns
    ///
    /// A vector of rules extracted from files matching the pattern `tr-*.md`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::SkillsExtractor;
    ///
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let extractor = SkillsExtractor::new(skills_path);
    /// let rules = extractor.get_tool_router_rules().unwrap();
    /// println!("Found {} Tool Router rules", rules.len());
    /// ```
    pub fn get_tool_router_rules(&self) -> Result<Vec<Rule>, SkillsError> {
        let rules_dir = self.skills_path.join("rules");
        self.get_rules_by_prefix(&rules_dir, "tr-")
    }

    /// Extract Trigger rules (triggers-*.md files)
    ///
    /// # Returns
    ///
    /// A vector of rules extracted from files matching the pattern `triggers-*.md`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::SkillsExtractor;
    ///
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let extractor = SkillsExtractor::new(skills_path);
    /// let rules = extractor.get_trigger_rules().unwrap();
    /// println!("Found {} Trigger rules", rules.len());
    /// ```
    pub fn get_trigger_rules(&self) -> Result<Vec<Rule>, SkillsError> {
        let rules_dir = self.skills_path.join("rules");
        self.get_rules_by_prefix(&rules_dir, "triggers-")
    }

    /// Extract rules by filename prefix
    fn get_rules_by_prefix(&self, rules_dir: &Path, prefix: &str) -> Result<Vec<Rule>, SkillsError> {
        let mut rules = Vec::new();

        if !rules_dir.exists() {
            return Ok(rules);
        }

        for entry in fs::read_dir(rules_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    // Skip template files
                    if filename.starts_with('_') {
                        continue;
                    }
                    
                    if filename.starts_with(prefix) {
                        match Rule::from_file(&path) {
                            Ok(rule) => rules.push(rule),
                            Err(e) => {
                                eprintln!("Warning: Failed to parse rule from {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(rules)
    }

    /// Extract rules filtered by tag
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag to filter by (e.g., "sessions", "authentication")
    ///
    /// # Returns
    ///
    /// A vector of rules that contain the specified tag
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::SkillsExtractor;
    ///
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let extractor = SkillsExtractor::new(skills_path);
    /// let session_rules = extractor.get_rules_by_tag("sessions").unwrap();
    /// ```
    pub fn get_rules_by_tag(&self, tag: &str) -> Result<Vec<Rule>, SkillsError> {
        let all_rules = self.get_all_rules()?;
        Ok(all_rules
            .into_iter()
            .filter(|r| r.tags.iter().any(|t| t == tag))
            .collect())
    }

    /// Get all rules from the rules directory
    pub fn get_all_rules(&self) -> Result<Vec<Rule>, SkillsError> {
        let rules_dir = self.skills_path.join("rules");
        let mut rules = Vec::new();

        if !rules_dir.exists() {
            return Ok(rules);
        }

        for entry in fs::read_dir(rules_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Skip template files
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    if filename.starts_with('_') {
                        continue;
                    }
                }
                
                match Rule::from_file(&path) {
                    Ok(rule) => rules.push(rule),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse rule from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(rules)
    }

    /// Get consolidated AGENTS.md content
    ///
    /// # Returns
    ///
    /// The full content of the AGENTS.md file (150+ KB consolidated reference)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::SkillsExtractor;
    ///
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let extractor = SkillsExtractor::new(skills_path);
    /// let content = extractor.get_consolidated_content().unwrap();
    /// println!("AGENTS.md size: {} bytes", content.len());
    /// ```
    pub fn get_consolidated_content(&self) -> Result<String, SkillsError> {
        let agents_path = self.skills_path.join("AGENTS.md");
        Ok(fs::read_to_string(agents_path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impact_from_str() {
        assert_eq!(Impact::from_str("CRITICAL").unwrap(), Impact::Critical);
        assert_eq!(Impact::from_str("critical").unwrap(), Impact::Critical);
        assert_eq!(Impact::from_str("HIGH").unwrap(), Impact::High);
        assert_eq!(Impact::from_str("MEDIUM").unwrap(), Impact::Medium);
        assert_eq!(Impact::from_str("LOW").unwrap(), Impact::Low);
        assert!(Impact::from_str("INVALID").is_err());
    }

    #[test]
    fn test_impact_as_str() {
        assert_eq!(Impact::Critical.as_str(), "CRITICAL");
        assert_eq!(Impact::High.as_str(), "HIGH");
        assert_eq!(Impact::Medium.as_str(), "MEDIUM");
        assert_eq!(Impact::Low.as_str(), "LOW");
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
title: Test Rule
impact: CRITICAL
description: A test rule
tags: [tool-router, sessions]
---

# Content

This is the body."#;

        let (frontmatter, body) = Rule::parse_markdown(content).unwrap();

        assert_eq!(frontmatter.get("title"), Some(&"Test Rule".to_string()));
        assert_eq!(frontmatter.get("impact"), Some(&"CRITICAL".to_string()));
        assert_eq!(frontmatter.get("description"), Some(&"A test rule".to_string()));
        assert!(body.contains("# Content"));
        assert!(body.contains("This is the body."));
    }

    #[test]
    fn test_extract_examples() {
        let content = r#"
## Examples

✅ **Correct:**

```rust
let session = client.create_session("user_123");
```

❌ **Incorrect:**

```rust
let session = client.create_session("default");
```
"#;

        let correct = Rule::extract_examples(content, "✅");
        let incorrect = Rule::extract_examples(content, "❌");

        assert_eq!(correct.len(), 1);
        assert!(correct[0].contains("user_123"));

        assert_eq!(incorrect.len(), 1);
        assert!(incorrect[0].contains("default"));
    }

    #[test]
    fn test_rule_from_content() {
        let content = r#"---
title: Session Management
impact: CRITICAL
description: Best practices for session management
tags: [tool-router, sessions]
---

# Session Management

✅ **Correct:**

```rust
let session = client.create_session("user_123");
```

❌ **Incorrect:**

```rust
let session = client.create_session("default");
```
"#;

        let rule = Rule::from_content(content).unwrap();

        assert_eq!(rule.title, "Session Management");
        assert_eq!(rule.impact, Impact::Critical);
        assert_eq!(rule.description, "Best practices for session management");
        assert_eq!(rule.tags, vec!["tool-router", "sessions"]);
        assert_eq!(rule.correct_examples.len(), 1);
        assert_eq!(rule.incorrect_examples.len(), 1);
    }
}
