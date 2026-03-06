//! Integration tests for wizard instructions generation
//!
//! These tests verify that the wizard instructions generation function works
//! correctly with actual Skills content from the Composio Skills repository.

use composio_sdk::wizard::{generate_wizard_instructions, SkillsError};

/// Test generating generic Composio instructions
#[test]
#[ignore] // Requires Skills repository to be present
fn test_generate_generic_instructions() {
    let result = generate_wizard_instructions(None);

    match result {
        Ok(instructions) => {
            // Verify basic structure
            assert!(instructions.contains("# Composio Wizard Instructions"));
            assert!(instructions.contains("## Overview"));
            assert!(instructions.contains("## Critical Rules"));
            assert!(instructions.contains("## Session Management Patterns"));
            assert!(instructions.contains("## Authentication Patterns"));

            // Verify content is substantial
            assert!(instructions.len() > 500, "Instructions should be substantial");

            println!("✓ Generated {} bytes of generic instructions", instructions.len());
        }
        Err(SkillsError::PathNotFound(path)) => {
            eprintln!("⚠ Skills repository not found at: {}", path.display());
            eprintln!("  Run `cargo build` to download it automatically.");
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test generating GitHub-specific instructions
#[test]
#[ignore] // Requires Skills repository to be present
fn test_generate_github_instructions() {
    let result = generate_wizard_instructions(Some("github"));

    match result {
        Ok(instructions) => {
            // Verify basic structure
            assert!(instructions.contains("# Composio Wizard Instructions"));
            assert!(instructions.contains("**Context:** Using toolkit `github`"));
            assert!(instructions.contains("## Toolkit-Specific Guidance: github"));

            // Verify content is substantial
            assert!(instructions.len() > 500, "Instructions should be substantial");

            println!("✓ Generated {} bytes of GitHub-specific instructions", instructions.len());
        }
        Err(SkillsError::PathNotFound(path)) => {
            eprintln!("⚠ Skills repository not found at: {}", path.display());
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test generating Gmail-specific instructions
#[test]
#[ignore] // Requires Skills repository to be present
fn test_generate_gmail_instructions() {
    let result = generate_wizard_instructions(Some("gmail"));

    match result {
        Ok(instructions) => {
            // Verify basic structure
            assert!(instructions.contains("# Composio Wizard Instructions"));
            assert!(instructions.contains("**Context:** Using toolkit `gmail`"));
            assert!(instructions.contains("## Toolkit-Specific Guidance: gmail"));

            println!("✓ Generated {} bytes of Gmail-specific instructions", instructions.len());
        }
        Err(SkillsError::PathNotFound(path)) => {
            eprintln!("⚠ Skills repository not found at: {}", path.display());
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test generating Slack-specific instructions
#[test]
#[ignore] // Requires Skills repository to be present
fn test_generate_slack_instructions() {
    let result = generate_wizard_instructions(Some("slack"));

    match result {
        Ok(instructions) => {
            // Verify basic structure
            assert!(instructions.contains("# Composio Wizard Instructions"));
            assert!(instructions.contains("**Context:** Using toolkit `slack`"));
            assert!(instructions.contains("## Toolkit-Specific Guidance: slack"));

            println!("✓ Generated {} bytes of Slack-specific instructions", instructions.len());
        }
        Err(SkillsError::PathNotFound(path)) => {
            eprintln!("⚠ Skills repository not found at: {}", path.display());
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test generating instructions for unknown toolkit
#[test]
#[ignore] // Requires Skills repository to be present
fn test_generate_unknown_toolkit_instructions() {
    let result = generate_wizard_instructions(Some("unknown-toolkit-xyz"));

    match result {
        Ok(instructions) => {
            // Verify basic structure
            assert!(instructions.contains("# Composio Wizard Instructions"));
            assert!(instructions.contains("**Context:** Using toolkit `unknown-toolkit-xyz`"));
            assert!(instructions.contains("## Toolkit-Specific Guidance: unknown-toolkit-xyz"));

            // Should contain fallback message for unknown toolkit
            assert!(
                instructions.contains("No specific rules found") ||
                instructions.contains("Use general best practices")
            );

            println!("✓ Generated {} bytes of instructions for unknown toolkit", instructions.len());
        }
        Err(SkillsError::PathNotFound(path)) => {
            eprintln!("⚠ Skills repository not found at: {}", path.display());
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test that instructions contain correct examples
#[test]
#[ignore] // Requires Skills repository to be present
fn test_instructions_contain_correct_examples() {
    let result = generate_wizard_instructions(None);

    match result {
        Ok(instructions) => {
            // Should contain correct example markers
            assert!(instructions.contains("✅"));

            println!("✓ Instructions contain correct examples");
        }
        Err(SkillsError::PathNotFound(_)) => {
            eprintln!("⚠ Skills repository not found");
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test that instructions contain incorrect examples
#[test]
#[ignore] // Requires Skills repository to be present
fn test_instructions_contain_incorrect_examples() {
    let result = generate_wizard_instructions(None);

    match result {
        Ok(instructions) => {
            // Should contain incorrect example markers
            assert!(instructions.contains("❌"));

            println!("✓ Instructions contain incorrect examples");
        }
        Err(SkillsError::PathNotFound(_)) => {
            eprintln!("⚠ Skills repository not found");
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test that instructions contain impact levels
#[test]
#[ignore] // Requires Skills repository to be present
fn test_instructions_contain_impact_levels() {
    let result = generate_wizard_instructions(None);

    match result {
        Ok(instructions) => {
            // Should contain impact level indicators
            assert!(
                instructions.contains("**Impact:**") ||
                instructions.contains("CRITICAL") ||
                instructions.contains("HIGH")
            );

            println!("✓ Instructions contain impact levels");
        }
        Err(SkillsError::PathNotFound(_)) => {
            eprintln!("⚠ Skills repository not found");
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test error handling when Skills repository is not found
#[test]
fn test_error_when_skills_not_found() {
    // This test doesn't require the Skills repository
    // It tests the error handling when the path doesn't exist

    use composio_sdk::wizard::{SkillsExtractor, WizardInstructionGenerator};

    let skills = SkillsExtractor::new("/nonexistent/path/to/skills");
    
    // Verify path should fail
    let verify_result = skills.verify_path();
    assert!(verify_result.is_err());
    
    if let Err(SkillsError::PathNotFound(path)) = verify_result {
        assert_eq!(path.to_str().unwrap(), "/nonexistent/path/to/skills");
        println!("✓ Correctly handles missing Skills repository");
    } else {
        panic!("Expected PathNotFound error from verify_path");
    }
    
    // Generator should also fail when trying to generate
    let generator = WizardInstructionGenerator::new(skills);
    let result = generator.generate_composio_instructions(None);

    // Should return PathNotFound error
    assert!(result.is_err());

    if let Err(SkillsError::PathNotFound(_)) = result {
        println!("✓ Generator correctly handles missing Skills repository");
    } else {
        panic!("Expected PathNotFound error from generator");
    }
}

/// Test that multiple toolkit instructions can be generated
#[test]
#[ignore] // Requires Skills repository to be present
fn test_generate_multiple_toolkit_instructions() {
    let toolkits = vec!["github", "gmail", "slack", "jira", "notion"];

    for toolkit in toolkits {
        let result = generate_wizard_instructions(Some(toolkit));

        match result {
            Ok(instructions) => {
                assert!(instructions.contains("# Composio Wizard Instructions"));
                assert!(instructions.contains(&format!("**Context:** Using toolkit `{}`", toolkit)));
                println!("✓ Generated instructions for toolkit: {}", toolkit);
            }
            Err(SkillsError::PathNotFound(_)) => {
                eprintln!("⚠ Skills repository not found");
                panic!("Skills repository required for this test");
            }
            Err(e) => {
                panic!("Unexpected error for toolkit {}: {}", toolkit, e);
            }
        }
    }
}

/// Test that instructions are valid markdown
#[test]
#[ignore] // Requires Skills repository to be present
fn test_instructions_are_valid_markdown() {
    let result = generate_wizard_instructions(None);

    match result {
        Ok(instructions) => {
            // Check for markdown headers
            assert!(instructions.contains("# "));
            assert!(instructions.contains("## "));

            // Check for markdown formatting
            assert!(instructions.contains("**"));

            // Check for code blocks
            assert!(instructions.contains("```"));

            println!("✓ Instructions are valid markdown");
        }
        Err(SkillsError::PathNotFound(_)) => {
            eprintln!("⚠ Skills repository not found");
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test that instructions contain session management guidance
#[test]
#[ignore] // Requires Skills repository to be present
fn test_instructions_contain_session_management() {
    let result = generate_wizard_instructions(None);

    match result {
        Ok(instructions) => {
            // Should contain session-related content
            assert!(
                instructions.to_lowercase().contains("session") ||
                instructions.contains("Session Management")
            );

            println!("✓ Instructions contain session management guidance");
        }
        Err(SkillsError::PathNotFound(_)) => {
            eprintln!("⚠ Skills repository not found");
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

/// Test that instructions contain authentication guidance
#[test]
#[ignore] // Requires Skills repository to be present
fn test_instructions_contain_authentication() {
    let result = generate_wizard_instructions(None);

    match result {
        Ok(instructions) => {
            // Should contain authentication-related content
            assert!(
                instructions.to_lowercase().contains("authentication") ||
                instructions.to_lowercase().contains("auth")
            );

            println!("✓ Instructions contain authentication guidance");
        }
        Err(SkillsError::PathNotFound(_)) => {
            eprintln!("⚠ Skills repository not found");
            panic!("Skills repository required for this test");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}
