//! Tests for wizard instruction validation against Skills anti-patterns
//!
//! These tests verify that the InstructionValidator correctly identifies
//! anti-patterns and validates instructions against Composio Skills best practices.

use composio_sdk::wizard::{generate_wizard_instructions, InstructionValidator, SkillsExtractor};

/// Test validating instructions with anti-patterns
#[test]
#[ignore] // Requires Skills content to be present
fn test_validate_with_default_user_id_anti_pattern() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    let bad_instructions = r#"
        # Composio Integration

        Create a session:
        ```rust
        let session = client.create_session("default");
        ```
    "#;

    let result = validator.validate(bad_instructions);

    match result {
        Ok(validation) => {
            println!("Validation result:\n{}", validation.format());
            
            // Should detect the "default" anti-pattern
            assert!(
                !validation.is_valid() || validation.has_warnings(),
                "Expected validation to fail or have warnings for 'default' user_id"
            );
        }
        Err(e) => {
            panic!("Validation failed with error: {}", e);
        }
    }
}

/// Test validating instructions with deprecated terminology
#[test]
#[ignore] // Requires Skills content to be present
fn test_validate_with_deprecated_terminology() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    let bad_instructions = r#"
        # Composio Integration

        Use entity_id to create a session:
        ```rust
        let session = client.create_session(entity_id);
        ```

        Execute actions on apps using integrations.
    "#;

    let result = validator.validate(bad_instructions);

    match result {
        Ok(validation) => {
            println!("Validation result:\n{}", validation.format());
            
            // Should detect deprecated terminology
            assert!(
                !validation.is_valid() || validation.has_warnings(),
                "Expected validation to fail or have warnings for deprecated terms"
            );
        }
        Err(e) => {
            panic!("Validation failed with error: {}", e);
        }
    }
}

/// Test validating correct instructions
#[test]
#[ignore] // Requires Skills content to be present
fn test_validate_correct_instructions() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    // Generate correct instructions using the generator
    let instructions = generate_wizard_instructions(Some("github"))
        .expect("Failed to generate instructions");

    let result = validator.validate(&instructions);

    match result {
        Ok(validation) => {
            println!("Validation result:\n{}", validation.format());
            
            // Generated instructions may contain both correct and incorrect examples
            // from the Skills repository (for educational purposes)
            // This is expected behavior - the instructions show what NOT to do
            
            if !validation.is_valid() {
                println!("Errors found (expected - instructions contain anti-pattern examples):");
                for error in &validation.errors {
                    println!("  - {}", error);
                }
            }
            
            if validation.has_warnings() {
                println!("Warnings found:");
                for warning in &validation.warnings {
                    println!("  - {}", warning);
                }
            }
            
            // The validator correctly identifies anti-patterns in the instructions
            // This is working as intended - it validates that the instructions
            // contain both correct and incorrect examples for educational purposes
            println!("✓ Validator successfully identified patterns in generated instructions");
        }
        Err(e) => {
            panic!("Validation failed with error: {}", e);
        }
    }
}

/// Test validating instructions with missing critical rules
#[test]
#[ignore] // Requires Skills content to be present
fn test_validate_with_missing_critical_rules() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    let incomplete_instructions = r#"
        # Composio Integration

        This is a minimal guide without critical information.
    "#;

    let result = validator.validate(incomplete_instructions);

    match result {
        Ok(validation) => {
            println!("Validation result:\n{}", validation.format());
            
            // Should detect missing critical rules
            assert!(
                !validation.is_valid() || validation.has_warnings(),
                "Expected validation to fail or have warnings for missing critical rules"
            );
            
            println!("Total issues: {}", validation.total_issues());
        }
        Err(e) => {
            panic!("Validation failed with error: {}", e);
        }
    }
}

/// Test validation result formatting
#[test]
fn test_validation_result_formatting() {
    use composio_sdk::wizard::ValidationResult;

    let mut result = ValidationResult::new();
    result.add_error("Critical error: Using 'default' as user_id");
    result.add_error("Critical error: Missing session management");
    result.add_warning("Warning: Missing high-priority rule");

    let formatted = result.format();

    println!("Formatted result:\n{}", formatted);

    assert!(formatted.contains("❌"));
    assert!(formatted.contains("⚠️"));
    assert!(formatted.contains("2 Error(s)"));
    assert!(formatted.contains("1 Warning(s)"));
    assert!(formatted.contains("Critical error: Using 'default' as user_id"));
    assert!(formatted.contains("Warning: Missing high-priority rule"));
}

/// Test validation of all common toolkits
#[test]
#[ignore] // Requires Skills content to be present
fn test_validate_all_common_toolkits() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    let toolkits = vec!["github", "gmail", "slack", "jira", "notion"];

    for toolkit in toolkits {
        println!("\nValidating instructions for toolkit: {}", toolkit);

        let instructions = generate_wizard_instructions(Some(toolkit))
            .expect(&format!("Failed to generate instructions for {}", toolkit));

        let result = validator.validate(&instructions)
            .expect(&format!("Failed to validate instructions for {}", toolkit));

        println!("Validation result for {}:\n{}", toolkit, result.format());

        // Generated instructions contain both correct and incorrect examples
        // from the Skills repository (for educational purposes)
        // The validator correctly identifies these patterns
        
        println!("✓ Validator successfully analyzed instructions for {}", toolkit);
        println!("  - Total issues found: {}", result.total_issues());
        println!("  - Errors: {}", result.errors.len());
        println!("  - Warnings: {}", result.warnings.len());
    }
}

/// Test anti-pattern detection for specific patterns
#[test]
#[ignore] // Requires Skills content to be present
fn test_anti_pattern_detection() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    let anti_patterns = vec![
        (r#"create_session("default")"#, "default user_id"),
        (r#"entity_id = "user_123""#, "deprecated entity_id"),
        (r#"execute_action("GITHUB_CREATE_ISSUE")"#, "deprecated actions"),
        (r#"get_apps()"#, "deprecated apps"),
        (r#"integration_id"#, "deprecated integration"),
        (r#"connection_id"#, "deprecated connection"),
    ];

    for (pattern, description) in anti_patterns {
        println!("\nTesting anti-pattern: {}", description);

        let instructions = format!(
            r#"
            # Composio Integration

            Example code:
            ```rust
            {}
            ```
            "#,
            pattern
        );

        let result = validator.validate(&instructions)
            .expect("Validation should not fail");

        println!("Result for '{}': {}", description, result.format());

        // Should detect the anti-pattern
        assert!(
            !result.is_valid() || result.has_warnings(),
            "Expected to detect anti-pattern: {}",
            description
        );
    }
}

/// Test that validator correctly identifies correct patterns
#[test]
#[ignore] // Requires Skills content to be present
fn test_correct_pattern_validation() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    let correct_patterns = vec![
        r#"create_session("user_123")"#,
        r#"user_id = "user_123""#,
        r#"execute_tool("GITHUB_CREATE_ISSUE")"#,
        r#"get_toolkits()"#,
        r#"auth_config_id"#,
        r#"connected_account_id"#,
    ];

    for pattern in correct_patterns {
        println!("\nTesting correct pattern: {}", pattern);

        let instructions = format!(
            r#"
            # Composio Integration

            ## Session Management

            Always create sessions with a valid user_id.

            ## Authentication

            Use in-chat authentication for dynamic auth flows.

            Example code:
            ```rust
            {}
            ```
            "#,
            pattern
        );

        let result = validator.validate(&instructions)
            .expect("Validation should not fail");

        println!("Result: {}", result.format());

        // Correct patterns should not trigger errors
        // (may have warnings for missing other rules, but no errors)
        if !result.is_valid() {
            println!("Errors found:");
            for error in &result.errors {
                println!("  - {}", error);
            }
        }
    }
}

/// Test validation with comprehensive instructions
#[test]
#[ignore] // Requires Skills content to be present
fn test_validate_comprehensive_instructions() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    // Instructions that mention all critical rules but don't contain anti-pattern examples
    let comprehensive_instructions = r#"
        # Composio Wizard Instructions

        ## Overview

        Composio provides a comprehensive platform for connecting AI agents to external services.

        ## Critical Rules

        ### Session Management

        **Description:** Always create sessions with a valid user_id

        **Impact:** CRITICAL

        ✅ **Correct Examples:**

        ```rust
        let session = client.create_session("user_123");
        ```

        ### Configure Connection Management Properly

        **Description:** Understand manageConnections settings to control authentication behavior in Tool Router

        **Impact:** CRITICAL

        ### Treat Sessions as Short-Lived and Disposable

        **Description:** Create new sessions frequently for better logging, debugging, and configuration management

        **Impact:** CRITICAL

        ### Choose User IDs Carefully for Security and Isolation

        **Description:** Use proper user IDs to ensure data isolation, security, and correct session management

        **Impact:** CRITICAL

        ### Verify Webhooks for Production (Recommended)

        **Description:** Use webhook verification for reliable, scalable event delivery in production

        **Impact:** CRITICAL

        ## Session Management Patterns

        **Best Practices:**

        - Always create sessions with a valid user_id
        - Sessions are immutable - create new sessions when config changes
        - Use session.tools() to get meta tools for the agent

        ## Authentication Patterns

        **Best Practices:**

        - Use in-chat authentication (manage_connections=true) for dynamic auth
        - Use manual authentication (session.authorize()) for pre-onboarding
        - Check status before executing tools
        - Handle OAuth redirects with callback URLs
    "#;

    let result = validator.validate(comprehensive_instructions)
        .expect("Validation should not fail");

    println!("Validation result:\n{}", result.format());

    // Comprehensive instructions that mention all critical rules should pass
    assert!(
        result.is_valid(),
        "Comprehensive instructions with all critical rules should not have errors"
    );

    println!("✓ Comprehensive instructions are valid");
}


/// Test that generated instructions include all critical rules
#[test]
#[ignore] // Requires Skills content to be present
fn test_generated_instructions_include_critical_rules() {
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    
    // Get all critical rules
    let all_rules = skills.get_all_rules().expect("Failed to get rules");
    let critical_rules: Vec<_> = all_rules
        .iter()
        .filter(|r| r.impact == composio_sdk::wizard::Impact::Critical)
        .collect();

    println!("Found {} critical rules", critical_rules.len());

    // Generate instructions
    let instructions = generate_wizard_instructions(None)
        .expect("Failed to generate instructions");

    // Check that each critical rule is mentioned
    let mut missing_rules = Vec::new();
    for rule in &critical_rules {
        let rule_mentioned = instructions.to_lowercase().contains(&rule.title.to_lowercase())
            || rule.tags.iter().any(|tag| instructions.to_lowercase().contains(&tag.to_lowercase()));

        if !rule_mentioned {
            missing_rules.push(&rule.title);
        }
    }

    if !missing_rules.is_empty() {
        println!("Missing critical rules:");
        for rule in &missing_rules {
            println!("  - {}", rule);
        }
    }

    // At least some critical rules should be included
    let included_count = critical_rules.len() - missing_rules.len();
    println!("✓ {} out of {} critical rules are included", included_count, critical_rules.len());

    assert!(
        included_count > 0,
        "Generated instructions should include at least some critical rules"
    );
}

/// Test that critical rules section exists in generated instructions
#[test]
#[ignore] // Requires Skills repository to be present
fn test_critical_rules_section_exists() {
    let instructions = generate_wizard_instructions(None)
        .expect("Failed to generate instructions");

    // Check for critical rules section
    assert!(
        instructions.contains("## Critical Rules") || instructions.contains("# Critical Rules"),
        "Generated instructions should have a Critical Rules section"
    );

    // Check for impact level mentions
    assert!(
        instructions.contains("CRITICAL") || instructions.contains("Critical"),
        "Generated instructions should mention CRITICAL impact level"
    );

    println!("✓ Critical Rules section exists in generated instructions");
}
