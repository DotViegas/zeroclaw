//! Wizard instruction generator for AI agents
//!
//! This module generates comprehensive wizard instructions for AI agents
//! using Composio Skills content. Instructions include best practices,
//! critical rules, and context-aware guidance.

use super::skills::{Impact, Rule, SkillsExtractor, SkillsError};

/// Generator for wizard instructions
#[derive(Debug, Clone)]
pub struct WizardInstructionGenerator {
    skills: SkillsExtractor,
}

impl WizardInstructionGenerator {
    /// Create a new wizard instruction generator
    ///
    /// # Arguments
    ///
    /// * `skills` - SkillsExtractor instance for accessing Skills content
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::{SkillsExtractor, WizardInstructionGenerator};
    ///
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let skills = SkillsExtractor::new(skills_path);
    /// let generator = WizardInstructionGenerator::new(skills);
    /// ```
    pub fn new(skills: SkillsExtractor) -> Self {
        Self { skills }
    }

    /// Generate comprehensive Composio wizard instructions
    ///
    /// Generates a complete set of instructions including:
    /// - Overview from AGENTS.md
    /// - Critical Tool Router rules
    /// - Session management patterns
    /// - Authentication patterns
    /// - Correct and incorrect examples
    ///
    /// # Arguments
    ///
    /// * `toolkit` - Optional toolkit name for context-aware instructions
    ///
    /// # Returns
    ///
    /// A formatted markdown string with wizard instructions
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::wizard::{SkillsExtractor, WizardInstructionGenerator};
    ///
    /// let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    /// let skills = SkillsExtractor::new(skills_path);
    /// let generator = WizardInstructionGenerator::new(skills);
    /// let instructions = generator.generate_composio_instructions(Some("github")).unwrap();
    /// println!("{}", instructions);
    /// ```
    pub fn generate_composio_instructions(&self, toolkit: Option<&str>) -> Result<String, SkillsError> {
        let mut output = String::new();

        // Add header
        output.push_str("# Composio Wizard Instructions\n\n");

        if let Some(tk) = toolkit {
            output.push_str(&format!("**Context:** Using toolkit `{}`\n\n", tk));
        }

        // Add overview section from AGENTS.md
        output.push_str(&self.generate_overview_section()?);

        // Add critical Tool Router rules
        output.push_str(&self.generate_critical_rules_section()?);

        // Add session management patterns
        output.push_str(&self.generate_session_management_section()?);

        // Add authentication patterns
        output.push_str(&self.generate_authentication_section()?);

        // Add toolkit-specific guidance if provided
        if let Some(tk) = toolkit {
            output.push_str(&self.generate_toolkit_specific_section(tk)?);
        }

        Ok(output)
    }

    /// Generate overview section from AGENTS.md
    fn generate_overview_section(&self) -> Result<String, SkillsError> {
        let mut section = String::new();

        section.push_str("## Overview\n\n");

        // Verify path before attempting to read
        self.skills.verify_path()?;

        // Get consolidated content from AGENTS.md
        match self.skills.get_consolidated_content() {
            Ok(content) => {
                // Extract first few paragraphs as overview (limit to ~500 chars)
                let lines: Vec<&str> = content.lines().collect();
                let mut overview = String::new();
                let mut char_count = 0;

                for line in lines.iter().take(50) {
                    if line.starts_with('#') && !overview.is_empty() {
                        break; // Stop at first heading after content
                    }
                    if !line.trim().is_empty() {
                        overview.push_str(line);
                        overview.push('\n');
                        char_count += line.len();

                        if char_count > 500 {
                            break;
                        }
                    }
                }

                if !overview.is_empty() {
                    section.push_str(&overview);
                    section.push_str("\n\n");
                } else {
                    section.push_str("Composio provides a comprehensive platform for connecting AI agents to external services.\n\n");
                }
            }
            Err(_) => {
                // Fallback if AGENTS.md is not available
                section.push_str("Composio provides a comprehensive platform for connecting AI agents to external services.\n");
                section.push_str("Use sessions for user-scoped tool execution, meta tools for discovery, and proper authentication patterns.\n\n");
            }
        }

        Ok(section)
    }

    /// Generate critical rules section
    fn generate_critical_rules_section(&self) -> Result<String, SkillsError> {
        let mut section = String::new();

        section.push_str("## Critical Rules\n\n");
        section.push_str("These rules are **CRITICAL** and must be followed to ensure correct behavior:\n\n");

        // Get all Tool Router rules
        let rules = self.skills.get_tool_router_rules()?;

        // Filter for critical impact
        let critical_rules: Vec<&Rule> = rules
            .iter()
            .filter(|r| r.impact == Impact::Critical)
            .collect();

        if critical_rules.is_empty() {
            section.push_str("*No critical rules found. Ensure Skills repository is properly configured.*\n\n");
        } else {
            for (i, rule) in critical_rules.iter().enumerate() {
                section.push_str(&format!("### {}.{} {}\n\n", i + 1, " ", rule.title));
                section.push_str(&self.format_rule(rule));
                section.push('\n');
            }
        }

        Ok(section)
    }

    /// Generate session management patterns section
    fn generate_session_management_section(&self) -> Result<String, SkillsError> {
        let mut section = String::new();

        section.push_str("## Session Management Patterns\n\n");

        // Get rules tagged with "sessions"
        let session_rules = self.skills.get_rules_by_tag("sessions")?;

        if session_rules.is_empty() {
            // Fallback content
            section.push_str("**Best Practices:**\n\n");
            section.push_str("- Always create sessions with a valid user_id\n");
            section.push_str("- Never use \"default\" as a user_id in production\n");
            section.push_str("- Sessions are immutable - create new sessions when config changes\n");
            section.push_str("- Use session.tools() to get meta tools for the agent\n\n");
        } else {
            for rule in session_rules.iter() {
                section.push_str(&format!("### {}\n\n", rule.title));
                section.push_str(&self.format_rule(rule));
                section.push('\n');
            }
        }

        Ok(section)
    }

    /// Generate authentication patterns section
    fn generate_authentication_section(&self) -> Result<String, SkillsError> {
        let mut section = String::new();

        section.push_str("## Authentication Patterns\n\n");

        // Get rules tagged with "authentication"
        let auth_rules = self.skills.get_rules_by_tag("authentication")?;

        if auth_rules.is_empty() {
            // Fallback content
            section.push_str("**Best Practices:**\n\n");
            section.push_str("- Use in-chat authentication (manage_connections=true) for dynamic auth\n");
            section.push_str("- Use manual authentication (session.authorize()) for pre-onboarding\n");
            section.push_str("- Check connection status before executing tools\n");
            section.push_str("- Handle OAuth redirects with callback URLs\n\n");
        } else {
            for rule in auth_rules.iter() {
                section.push_str(&format!("### {}\n\n", rule.title));
                section.push_str(&self.format_rule(rule));
                section.push('\n');
            }
        }

        Ok(section)
    }

    /// Generate toolkit-specific guidance section
    fn generate_toolkit_specific_section(&self, toolkit: &str) -> Result<String, SkillsError> {
        let mut section = String::new();

        section.push_str(&format!("## Toolkit-Specific Guidance: {}\n\n", toolkit));

        // Get rules tagged with the toolkit name
        let toolkit_rules = self.skills.get_rules_by_tag(toolkit)?;

        if toolkit_rules.is_empty() {
            section.push_str(&format!(
                "*No specific rules found for toolkit `{}`. Use general best practices.*\n\n",
                toolkit
            ));
        } else {
            for rule in toolkit_rules.iter() {
                section.push_str(&format!("### {}\n\n", rule.title));
                section.push_str(&self.format_rule(rule));
                section.push('\n');
            }
        }

        Ok(section)
    }

    /// Format a rule with description and examples
    ///
    /// Formats a rule with:
    /// - Description
    /// - Correct examples (✅)
    /// - Incorrect examples (❌)
    ///
    /// # Arguments
    ///
    /// * `rule` - The rule to format
    ///
    /// # Returns
    ///
    /// A formatted markdown string
    fn format_rule(&self, rule: &Rule) -> String {
        let mut output = String::new();

        // Add description
        if !rule.description.is_empty() {
            output.push_str(&format!("**Description:** {}\n\n", rule.description));
        }

        // Add impact level
        output.push_str(&format!("**Impact:** {}\n\n", rule.impact.as_str()));

        // Add tags if present
        if !rule.tags.is_empty() {
            output.push_str(&format!("**Tags:** {}\n\n", rule.tags.join(", ")));
        }

        // Add correct examples
        if !rule.correct_examples.is_empty() {
            output.push_str("✅ **Correct Examples:**\n\n");
            for example in &rule.correct_examples {
                output.push_str("```\n");
                output.push_str(example);
                output.push_str("\n```\n\n");
            }
        }

        // Add incorrect examples
        if !rule.incorrect_examples.is_empty() {
            output.push_str("❌ **Incorrect Examples:**\n\n");
            for example in &rule.incorrect_examples {
                output.push_str("```\n");
                output.push_str(example);
                output.push_str("\n```\n\n");
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_generator() -> WizardInstructionGenerator {
        let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
        let skills = SkillsExtractor::new(skills_path);
        WizardInstructionGenerator::new(skills)
    }

    #[test]
    fn test_generator_creation() {
        let generator = create_test_generator();
        assert!(std::mem::size_of_val(&generator) > 0);
    }

    #[test]
    fn test_format_rule() {
        let generator = create_test_generator();

        let rule = Rule {
            title: "Test Rule".to_string(),
            impact: Impact::Critical,
            description: "A test rule for formatting".to_string(),
            tags: vec!["test".to_string(), "example".to_string()],
            content: "Test content".to_string(),
            correct_examples: vec!["let x = 1;".to_string()],
            incorrect_examples: vec!["let x = \"default\";".to_string()],
        };

        let formatted = generator.format_rule(&rule);

        assert!(formatted.contains("**Description:**"));
        assert!(formatted.contains("**Impact:** CRITICAL"));
        assert!(formatted.contains("**Tags:** test, example"));
        assert!(formatted.contains("✅ **Correct Examples:**"));
        assert!(formatted.contains("❌ **Incorrect Examples:**"));
        assert!(formatted.contains("let x = 1;"));
        assert!(formatted.contains("let x = \"default\";"));
    }

    #[test]
    fn test_format_rule_minimal() {
        let generator = create_test_generator();

        let rule = Rule {
            title: "Minimal Rule".to_string(),
            impact: Impact::Low,
            description: String::new(),
            tags: Vec::new(),
            content: String::new(),
            correct_examples: Vec::new(),
            incorrect_examples: Vec::new(),
        };

        let formatted = generator.format_rule(&rule);

        assert!(formatted.contains("**Impact:** LOW"));
        assert!(!formatted.contains("**Description:**"));
        assert!(!formatted.contains("**Tags:**"));
        assert!(!formatted.contains("✅"));
        assert!(!formatted.contains("❌"));
    }

    #[test]
    #[ignore] // Requires Skills repository to be present
    fn test_generate_composio_instructions() {
        let generator = create_test_generator();

        let instructions = generator.generate_composio_instructions(None);

        if let Ok(content) = instructions {
            assert!(content.contains("# Composio Wizard Instructions"));
            assert!(content.contains("## Overview"));
            assert!(content.contains("## Critical Rules"));
            assert!(content.contains("## Session Management Patterns"));
            assert!(content.contains("## Authentication Patterns"));
        }
    }

    #[test]
    #[ignore] // Requires Skills repository to be present
    fn test_generate_with_toolkit() {
        let generator = create_test_generator();

        let instructions = generator.generate_composio_instructions(Some("github"));

        if let Ok(content) = instructions {
            assert!(content.contains("**Context:** Using toolkit `github`"));
            assert!(content.contains("## Toolkit-Specific Guidance: github"));
        }
    }

    #[test]
    fn test_generate_overview_section_fallback() {
        let generator = create_test_generator();

        // This should use fallback content if AGENTS.md is not available
        let section = generator.generate_overview_section();

        assert!(section.is_ok());
        let content = section.unwrap();
        assert!(content.contains("## Overview"));
        assert!(content.contains("Composio"));
    }

    #[test]
    fn test_generate_critical_rules_section() {
        let generator = create_test_generator();

        let section = generator.generate_critical_rules_section();

        assert!(section.is_ok());
        let content = section.unwrap();
        assert!(content.contains("## Critical Rules"));
    }

    #[test]
    fn test_generate_session_management_section() {
        let generator = create_test_generator();

        let section = generator.generate_session_management_section();

        assert!(section.is_ok());
        let content = section.unwrap();
        assert!(content.contains("## Session Management Patterns"));
    }

    #[test]
    fn test_generate_authentication_section() {
        let generator = create_test_generator();

        let section = generator.generate_authentication_section();

        assert!(section.is_ok());
        let content = section.unwrap();
        assert!(content.contains("## Authentication Patterns"));
    }

    #[test]
    fn test_generate_toolkit_specific_section() {
        let generator = create_test_generator();

        let section = generator.generate_toolkit_specific_section("github");

        assert!(section.is_ok());
        let content = section.unwrap();
        assert!(content.contains("## Toolkit-Specific Guidance: github"));
    }
}
