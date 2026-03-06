//! Instruction validator for wizard instructions
//!
//! This module validates generated wizard instructions against Composio Skills
//! best practices, checking for anti-patterns and missing critical rules.

use super::skills::{Impact, Rule, SkillsExtractor, SkillsError};

/// Result of instruction validation
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// Validation errors (must be fixed)
    pub errors: Vec<String>,
    /// Validation warnings (should be addressed)
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add an error to the validation result
    ///
    /// # Arguments
    ///
    /// * `error` - Error message to add
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Add a warning to the validation result
    ///
    /// # Arguments
    ///
    /// * `warning` - Warning message to add
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get total number of issues (errors + warnings)
    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    /// Format validation result as a human-readable string
    pub fn format(&self) -> String {
        let mut output = String::new();

        if !self.errors.is_empty() {
            output.push_str(&format!("❌ {} Error(s):\n", self.errors.len()));
            for (i, error) in self.errors.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, error));
            }
            output.push('\n');
        }

        if !self.warnings.is_empty() {
            output.push_str(&format!("⚠️  {} Warning(s):\n", self.warnings.len()));
            for (i, warning) in self.warnings.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, warning));
            }
            output.push('\n');
        }

        if self.is_valid() && !self.has_warnings() {
            output.push_str("✅ Validation passed with no issues\n");
        }

        output
    }
}

/// Validator for wizard instructions
#[derive(Debug, Clone)]
pub struct InstructionValidator {
    skills: SkillsExtractor,
}

impl InstructionValidator {
    /// Create a new instruction validator
    ///
    /// # Arguments
    ///
    /// * `skills` - SkillsExtractor instance for accessing Skills content
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::{SkillsExtractor, InstructionValidator};
    ///
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let skills = SkillsExtractor::new(skills_path);
    /// let validator = InstructionValidator::new(skills);
    /// ```
    pub fn new(skills: SkillsExtractor) -> Self {
        Self { skills }
    }

    /// Validate wizard instructions against Skills best practices
    ///
    /// Checks for:
    /// - Anti-patterns (❌ examples) in instructions
    /// - Missing critical rules
    /// - Missing high-priority rules (warnings)
    ///
    /// # Arguments
    ///
    /// * `instructions` - The wizard instructions to validate
    ///
    /// # Returns
    ///
    /// A ValidationResult containing errors and warnings
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::{SkillsExtractor, InstructionValidator};
    ///
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let skills = SkillsExtractor::new(skills_path);
    /// let validator = InstructionValidator::new(skills);
    ///
    /// let instructions = "let session = client.create_session(\"default\");";
    /// let result = validator.validate(instructions).unwrap();
    ///
    /// if !result.is_valid() {
    ///     println!("{}", result.format());
    /// }
    /// ```
    pub fn validate(&self, instructions: &str) -> Result<ValidationResult, SkillsError> {
        let mut result = ValidationResult::new();

        // Get all rules from Skills
        let all_rules = self.skills.get_all_rules()?;

        // Check for anti-patterns
        self.check_anti_patterns(instructions, &all_rules, &mut result);

        // Check for missing critical rules
        self.check_missing_critical_rules(instructions, &all_rules, &mut result);

        // Check for missing high-priority rules (warnings)
        self.check_missing_high_priority_rules(instructions, &all_rules, &mut result);

        Ok(result)
    }

    /// Check for anti-patterns (❌ examples) in instructions
    fn check_anti_patterns(
        &self,
        instructions: &str,
        rules: &[Rule],
        result: &mut ValidationResult,
    ) {
        for rule in rules {
            // Check each incorrect example
            for incorrect_example in &rule.incorrect_examples {
                // Normalize whitespace for comparison
                let normalized_example = Self::normalize_code(incorrect_example);
                let normalized_instructions = Self::normalize_code(instructions);

                // Check if the anti-pattern appears in instructions
                if Self::contains_pattern(&normalized_instructions, &normalized_example) {
                    result.add_error(format!(
                        "Anti-pattern detected from rule '{}' ({}): Found code matching incorrect example",
                        rule.title,
                        rule.impact.as_str()
                    ));
                }
            }

            // Check for common anti-pattern keywords
            self.check_keyword_anti_patterns(instructions, rule, result);
        }
    }

    /// Check for keyword-based anti-patterns
    fn check_keyword_anti_patterns(
        &self,
        instructions: &str,
        rule: &Rule,
        result: &mut ValidationResult,
    ) {
        // Common anti-patterns to check
        let anti_patterns = [
            ("\"default\"", "Using 'default' as user_id"),
            ("entity_id", "Using deprecated 'entity_id' instead of 'user_id'"),
            ("actions", "Using deprecated 'actions' instead of 'tools'"),
        ];

        for (pattern, description) in &anti_patterns {
            if instructions.contains(pattern) && rule.impact == Impact::Critical {
                // Only report if it's in a critical rule context
                if rule.content.contains(pattern) || rule.incorrect_examples.iter().any(|ex| ex.contains(pattern)) {
                    result.add_error(format!(
                        "Anti-pattern detected: {} (from rule '{}')",
                        description, rule.title
                    ));
                }
            }
        }
    }

    /// Check for missing critical rules
    fn check_missing_critical_rules(
        &self,
        instructions: &str,
        rules: &[Rule],
        result: &mut ValidationResult,
    ) {
        let critical_rules: Vec<&Rule> = rules
            .iter()
            .filter(|r| r.impact == Impact::Critical)
            .collect();

        for rule in critical_rules {
            // Check if the rule title or key concepts are mentioned
            let is_mentioned = Self::is_rule_mentioned(instructions, rule);

            if !is_mentioned {
                result.add_error(format!(
                    "Missing critical rule: '{}' - {}",
                    rule.title, rule.description
                ));
            }
        }
    }

    /// Check for missing high-priority rules (warnings)
    fn check_missing_high_priority_rules(
        &self,
        instructions: &str,
        rules: &[Rule],
        result: &mut ValidationResult,
    ) {
        let high_priority_rules: Vec<&Rule> = rules
            .iter()
            .filter(|r| r.impact == Impact::High)
            .collect();

        for rule in high_priority_rules {
            let is_mentioned = Self::is_rule_mentioned(instructions, rule);

            if !is_mentioned {
                result.add_warning(format!(
                    "Missing high-priority rule: '{}' - {}",
                    rule.title, rule.description
                ));
            }
        }
    }

    /// Check if a rule is mentioned in instructions
    fn is_rule_mentioned(instructions: &str, rule: &Rule) -> bool {
        let instructions_lower = instructions.to_lowercase();

        // Check if title is mentioned
        if instructions_lower.contains(&rule.title.to_lowercase()) {
            return true;
        }

        // Check if any correct examples are present
        for correct_example in &rule.correct_examples {
            let normalized_example = Self::normalize_code(correct_example);
            let normalized_instructions = Self::normalize_code(instructions);

            if Self::contains_pattern(&normalized_instructions, &normalized_example) {
                return true;
            }
        }

        // Check if key tags are mentioned
        for tag in &rule.tags {
            if instructions_lower.contains(&tag.to_lowercase()) {
                return true;
            }
        }

        false
    }

    /// Normalize code by removing extra whitespace and comments
    fn normalize_code(code: &str) -> String {
        code.lines()
            .map(|line| {
                // Remove comments
                let line = if let Some(pos) = line.find("//") {
                    &line[..pos]
                } else {
                    line
                };

                // Trim and normalize whitespace
                line.split_whitespace().collect::<Vec<_>>().join(" ")
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if instructions contain a pattern (fuzzy matching)
    fn contains_pattern(instructions: &str, pattern: &str) -> bool {
        // Simple substring matching for now
        // Could be enhanced with more sophisticated pattern matching
        instructions.contains(pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_validator() -> InstructionValidator {
        let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
        let skills = SkillsExtractor::new(skills_path);
        InstructionValidator::new(skills)
    }

    #[test]
    fn test_validation_result_new() {
        let result = ValidationResult::new();
        assert!(result.is_valid());
        assert!(!result.has_warnings());
        assert_eq!(result.total_issues(), 0);
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::new();
        result.add_error("Test error");

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Test error");
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::new();
        result.add_warning("Test warning");

        assert!(result.is_valid()); // Still valid with only warnings
        assert!(result.has_warnings());
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Test warning");
    }

    #[test]
    fn test_validation_result_format() {
        let mut result = ValidationResult::new();
        result.add_error("Error 1");
        result.add_error("Error 2");
        result.add_warning("Warning 1");

        let formatted = result.format();

        assert!(formatted.contains("❌ 2 Error(s)"));
        assert!(formatted.contains("Error 1"));
        assert!(formatted.contains("Error 2"));
        assert!(formatted.contains("⚠️  1 Warning(s)"));
        assert!(formatted.contains("Warning 1"));
    }

    #[test]
    fn test_validation_result_format_success() {
        let result = ValidationResult::new();
        let formatted = result.format();

        assert!(formatted.contains("✅ Validation passed with no issues"));
    }

    #[test]
    fn test_normalize_code() {
        let code = r#"
            let session = client.create_session("user_123");  // Create session
            let tools = session.tools();
        "#;

        let normalized = InstructionValidator::normalize_code(code);

        assert!(!normalized.contains("//"));
        assert!(normalized.contains("create_session"));
        assert!(normalized.contains("user_123"));
    }

    #[test]
    fn test_contains_pattern() {
        let instructions = "let session = client.create_session(\"user_123\");";
        let pattern = "create_session(\"user_123\")";

        assert!(InstructionValidator::contains_pattern(instructions, pattern));
    }

    #[test]
    fn test_contains_pattern_not_found() {
        let instructions = "let session = client.create_session(\"user_123\");";
        let pattern = "create_session(\"default\")";

        assert!(!InstructionValidator::contains_pattern(instructions, pattern));
    }

    #[test]
    fn test_validator_creation() {
        let validator = create_test_validator();
        assert!(std::mem::size_of_val(&validator) > 0);
    }

    #[test]
    #[ignore] // Requires Skills repository to be present
    fn test_validate_with_anti_pattern() {
        let validator = create_test_validator();

        let instructions = r#"
            // Bad example - using "default" as user_id
            let session = client.create_session("default");
        "#;

        let result = validator.validate(instructions);

        if let Ok(validation) = result {
            // Should detect the "default" anti-pattern
            assert!(!validation.is_valid() || validation.has_warnings());
        }
    }

    #[test]
    #[ignore] // Requires Skills repository to be present
    fn test_validate_correct_pattern() {
        let validator = create_test_validator();

        let instructions = r#"
            # Composio Wizard Instructions

            ## Session Management

            Always create sessions with a valid user_id:

            ```rust
            let session = client.create_session("user_123");
            ```

            ## Authentication

            Use in-chat authentication for dynamic auth flows.
        "#;

        let result = validator.validate(instructions);

        if let Ok(validation) = result {
            // Should pass validation or have minimal warnings
            println!("{}", validation.format());
        }
    }

    #[test]
    fn test_is_rule_mentioned_by_title() {
        let instructions = "This document covers Session Management best practices.";

        let rule = Rule {
            title: "Session Management".to_string(),
            impact: Impact::Critical,
            description: "Best practices".to_string(),
            tags: vec![],
            content: String::new(),
            correct_examples: vec![],
            incorrect_examples: vec![],
        };

        assert!(InstructionValidator::is_rule_mentioned(instructions, &rule));
    }

    #[test]
    fn test_is_rule_mentioned_by_tag() {
        let instructions = "This document covers sessions and authentication.";

        let rule = Rule {
            title: "Some Rule".to_string(),
            impact: Impact::High,
            description: "Description".to_string(),
            tags: vec!["sessions".to_string()],
            content: String::new(),
            correct_examples: vec![],
            incorrect_examples: vec![],
        };

        assert!(InstructionValidator::is_rule_mentioned(instructions, &rule));
    }

    #[test]
    fn test_is_rule_not_mentioned() {
        let instructions = "This document covers something else entirely.";

        let rule = Rule {
            title: "Session Management".to_string(),
            impact: Impact::Critical,
            description: "Best practices".to_string(),
            tags: vec!["sessions".to_string()],
            content: String::new(),
            correct_examples: vec![],
            incorrect_examples: vec![],
        };

        assert!(!InstructionValidator::is_rule_mentioned(instructions, &rule));
    }

    #[test]
    fn test_check_keyword_anti_patterns() {
        let validator = create_test_validator();
        let mut result = ValidationResult::new();

        let instructions = r#"
            let session = client.create_session("default");
            let entity_id = "user_123";
        "#;

        let rule = Rule {
            title: "User ID Best Practices".to_string(),
            impact: Impact::Critical,
            description: "Never use default".to_string(),
            tags: vec![],
            content: "Never use \"default\" as user_id".to_string(),
            correct_examples: vec![],
            incorrect_examples: vec!["create_session(\"default\")".to_string()],
        };

        validator.check_keyword_anti_patterns(instructions, &rule, &mut result);

        // Should detect anti-patterns
        assert!(!result.errors.is_empty());
    }
}
