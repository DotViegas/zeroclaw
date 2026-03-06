//! Wizard instruction generation module
//!
//! This module provides utilities for extracting Composio Skills content
//! and generating wizard instructions for AI agents. It integrates with the
//! official Composio Skills repository to provide production-ready guidance
//! based on best practices and anti-patterns.
//!
//! # Overview
//!
//! The wizard module consists of three main components:
//!
//! - **[`SkillsExtractor`]**: Extracts rules and best practices from the Composio Skills repository
//! - **[`WizardInstructionGenerator`]**: Generates formatted wizard instructions for AI agents
//! - **[`InstructionValidator`]**: Validates generated instructions against official patterns
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Composio Skills Repository                      │
//! │  (https://github.com/ComposioHQ/skills)                     │
//! │  - AGENTS.md (consolidated reference)                        │
//! │  - rules/tr-*.md (Tool Router rules)                        │
//! │  - rules/triggers-*.md (Trigger rules)                      │
//! └───────────────────────┬─────────────────────────────────────┘
//!                         │
//!                         │ extracts
//!                         ▼
//!                 ┌───────────────┐
//!                 │ SkillsExtractor│
//!                 └───────┬───────┘
//!                         │
//!          ┌──────────────┼──────────────┐
//!          │              │              │
//!          ▼              ▼              ▼
//!   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
//!   │  Generator  │ │  Validator  │ │   Rules     │
//!   └─────────────┘ └─────────────┘ └─────────────┘
//! ```
//!
//! # Usage Examples
//!
//! ## Basic Usage: Generate Wizard Instructions
//!
//! ```rust,no_run
//! use composio_sdk::wizard::{SkillsExtractor, WizardInstructionGenerator};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Skills are now bundled within the SDK
//! // No need to specify external path - use the bundled skills
//! let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
//! let skills = SkillsExtractor::new(skills_path);
//!
//! // Verify the Skills repository is accessible
//! skills.verify_path()?;
//!
//! // Create the instruction generator
//! let generator = WizardInstructionGenerator::new(skills);
//!
//! // Generate generic Composio instructions
//! let instructions = generator.generate_composio_instructions(None)?;
//! println!("{}", instructions);
//!
//! // Generate toolkit-specific instructions (e.g., for GitHub)
//! let github_instructions = generator.generate_composio_instructions(Some("github"))?;
//! println!("{}", github_instructions);
//! # Ok(())
//! # }
//! ```
//!
//! ## Advanced Usage: Extract and Filter Rules
//!
//! ```rust,no_run
//! use composio_sdk::wizard::{SkillsExtractor, Impact};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Skills are bundled within the SDK
//! let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
//! let skills = SkillsExtractor::new(skills_path);
//!
//! // Get all Tool Router rules
//! let tool_router_rules = skills.get_tool_router_rules()?;
//! println!("Found {} Tool Router rules", tool_router_rules.len());
//!
//! // Get all Trigger rules
//! let trigger_rules = skills.get_trigger_rules()?;
//! println!("Found {} Trigger rules", trigger_rules.len());
//!
//! // Filter rules by tag
//! let session_rules = skills.get_rules_by_tag("session")?;
//! println!("Found {} session-related rules", session_rules.len());
//!
//! // Get consolidated content from AGENTS.md
//! let consolidated = skills.get_consolidated_content()?;
//! println!("Consolidated content: {} bytes", consolidated.len());
//!
//! // Inspect individual rules
//! for rule in tool_router_rules.iter().take(5) {
//!     println!("Rule: {}", rule.title);
//!     println!("Impact: {:?}", rule.impact);
//!     println!("Tags: {:?}", rule.tags);
//!     println!("Correct examples: {}", rule.correct_examples.len());
//!     println!("Incorrect examples: {}", rule.incorrect_examples.len());
//!     println!("---");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Validation: Check Instructions Against Official Patterns
//!
//! ```rust,no_run
//! use composio_sdk::wizard::{SkillsExtractor, InstructionValidator};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Skills are bundled within the SDK
//! let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
//! let skills = SkillsExtractor::new(skills_path);
//! let validator = InstructionValidator::new(skills);
//!
//! // Validate some instructions
//! let instructions = r#"
//! # Composio Integration Guide
//!
//! Always use composio.create(user_id) to create a session.
//! Use session.tools() for native tool integration.
//! "#;
//!
//! let result = validator.validate(instructions)?;
//!
//! if result.is_valid() {
//!     println!("✓ Instructions are valid!");
//! } else {
//!     println!("✗ Validation failed:");
//!     println!("{}", result.format());
//! }
//!
//! if result.has_warnings() {
//!     println!("⚠ Warnings found:");
//!     println!("{}", result.format());
//! }
//!
//! println!("Total issues: {}", result.total_issues());
//! # Ok(())
//! # }
//! ```
//!
//! ## Working with Rules
//!
//! ```rust,no_run
//! use composio_sdk::wizard::{Rule, Impact};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load a rule from the bundled skills
//! let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
//! let rule_path = format!("{}/rules/tr-001.md", skills_path);
//! let rule = Rule::from_file(Path::new(&rule_path))?;
//!
//! println!("Title: {}", rule.title);
//! println!("Impact: {:?}", rule.impact);
//! println!("Description: {}", rule.description);
//! println!("Tags: {:?}", rule.tags);
//!
//! // Access examples
//! for (i, example) in rule.correct_examples.iter().enumerate() {
//!     println!("✅ Correct example {}: {}", i + 1, example);
//! }
//!
//! for (i, example) in rule.incorrect_examples.iter().enumerate() {
//!     println!("❌ Incorrect example {}: {}", i + 1, example);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Integration with Build Process
//!
//! The Skills content is now **bundled directly within the SDK** at `composio-sdk/skills/`.
//! This means:
//!
//! 1. No external dependencies on vendor/skills repository
//! 2. Skills content is always available at compile time
//! 3. No need to clone or download Skills repository separately
//! 4. SDK is fully self-contained
//!
//! The bundled Skills include:
//! - `AGENTS.md` - Consolidated reference (150+ KB)
//! - `SKILL.md` - Metadata
//! - `rules/*.md` - 30+ rule files with best practices
//!
//! # Skills Repository Structure
//!
//! ```text
//! composio-sdk/skills/
//! ├── AGENTS.md              # Consolidated reference (150+ KB)
//! ├── SKILL.md               # Metadata
//! └── rules/
//!     ├── tr-*.md            # Tool Router rules
//!     ├── triggers-*.md      # Trigger rules
//!     └── app-*.md           # Application rules
//! ```
//!
//! # Rule Format
//!
//! Rules are markdown files with YAML frontmatter:
//!
//! ```markdown
//! ---
//! title: Always use composio.create(user_id)
//! impact: critical
//! tags: [session, user-scoping]
//! ---
//!
//! # Description
//! Always create sessions with user_id for proper isolation.
//!
//! ## Correct ✅
//! ```python
//! session = composio.create(user_id="user_123")
//! ```
//!
//! ## Incorrect ❌
//! ```python
//! session = composio.create()  # Missing user_id
//! ```
//!
//! # Impact Levels
//!
//! Rules are categorized by impact:
//!
//! - **Critical**: Must be followed, causes failures if violated
//! - **High**: Should be followed, causes issues if violated
//! - **Medium**: Recommended, improves quality
//! - **Low**: Optional, nice to have
//!
//! # Error Handling
//!
//! All operations return `Result<T, SkillsError>` for proper error handling:
//!
//! ```rust,no_run
//! use composio_sdk::wizard::{SkillsExtractor, SkillsError};
//!
//! # fn main() {
//! // Skills are bundled within the SDK
//! let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
//! let skills = SkillsExtractor::new(skills_path);
//!
//! match skills.verify_path() {
//!     Ok(_) => println!("Skills repository found"),
//!     Err(SkillsError::PathNotFound(path)) => {
//!         eprintln!("Skills repository not found at: {}", path.display());
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! # }
//! ```

mod generator;
mod skills;
mod validator;

pub use generator::WizardInstructionGenerator;
pub use skills::{Impact, Rule, SkillsExtractor, SkillsError};
pub use validator::{InstructionValidator, ValidationResult};

/// Generate wizard instructions for Composio integration
///
/// This is a convenience function that creates a SkillsExtractor and
/// WizardInstructionGenerator, then generates comprehensive wizard instructions
/// for AI agents using Composio Skills content.
///
/// # Arguments
///
/// * `toolkit` - Optional toolkit name for context-aware instructions (e.g., "github", "gmail", "slack")
///
/// # Returns
///
/// A formatted markdown string with wizard instructions, or an error if the Skills
/// repository is not accessible or parsing fails.
///
/// # Examples
///
/// ## Generate Generic Instructions
///
/// ```no_run
/// use composio_sdk::wizard::generate_wizard_instructions;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Generate generic Composio instructions
/// let instructions = generate_wizard_instructions(None)?;
/// println!("{}", instructions);
/// # Ok(())
/// # }
/// ```
///
/// ## Generate Toolkit-Specific Instructions
///
/// ```no_run
/// use composio_sdk::wizard::generate_wizard_instructions;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Generate GitHub-specific instructions
/// let github_instructions = generate_wizard_instructions(Some("github"))?;
/// println!("{}", github_instructions);
///
/// // Generate Gmail-specific instructions
/// let gmail_instructions = generate_wizard_instructions(Some("gmail"))?;
/// println!("{}", gmail_instructions);
///
/// // Generate Slack-specific instructions
/// let slack_instructions = generate_wizard_instructions(Some("slack"))?;
/// println!("{}", slack_instructions);
/// # Ok(())
/// # }
/// ```
///
/// # Skills Repository
///
/// This function uses the Skills content bundled within the SDK at
/// `composio-sdk/skills/`. The Skills are included at compile time,
/// making the SDK fully self-contained.
///
/// If the Skills directory is not found, the function will return a
/// `SkillsError::PathNotFound` error.
///
/// # Generated Content
///
/// The generated instructions include:
///
/// - **Overview**: Introduction from AGENTS.md consolidated reference
/// - **Critical Rules**: Must-follow rules with CRITICAL impact
/// - **Session Management**: Best practices for session creation and management
/// - **Authentication**: Patterns for in-chat and manual authentication
/// - **Toolkit-Specific Guidance**: Context-aware rules for the specified toolkit (if provided)
///
/// Each rule includes:
/// - Description and impact level
/// - Correct examples (✅)
/// - Incorrect examples (❌)
/// - Relevant tags
///
/// # Supported Toolkits
///
/// Common toolkits include:
/// - `github` - GitHub integration
/// - `gmail` - Gmail integration
/// - `slack` - Slack integration
/// - `jira` - Jira integration
/// - `notion` - Notion integration
/// - And 900+ more toolkits
///
/// For unknown toolkits, generic instructions are provided with a note that
/// no toolkit-specific rules were found.
///
/// # Error Handling
///
/// ```no_run
/// use composio_sdk::wizard::{generate_wizard_instructions, SkillsError};
///
/// # fn main() {
/// match generate_wizard_instructions(Some("github")) {
///     Ok(instructions) => {
///         println!("Generated {} bytes of instructions", instructions.len());
///         println!("{}", instructions);
///     }
///     Err(SkillsError::PathNotFound(path)) => {
///         eprintln!("Skills repository not found at: {}", path.display());
///         eprintln!("Run the build script to download it automatically.");
///     }
///     Err(e) => {
///         eprintln!("Error generating instructions: {}", e);
///     }
/// }
/// # }
/// ```
pub fn generate_wizard_instructions(toolkit: Option<&str>) -> Result<String, SkillsError> {
    // Skills are now bundled within the SDK at compile time
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    
    // Create SkillsExtractor
    let skills = SkillsExtractor::new(skills_path);
    
    // Verify the Skills repository is accessible
    skills.verify_path()?;
    
    // Create WizardInstructionGenerator
    let generator = WizardInstructionGenerator::new(skills);
    
    // Generate instructions
    generator.generate_composio_instructions(toolkit)
}
