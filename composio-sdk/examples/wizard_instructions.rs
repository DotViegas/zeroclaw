//! Example: Generate wizard instructions for Composio integration
//!
//! This example demonstrates how to generate wizard instructions for AI agents
//! using the Composio Skills repository. Instructions include best practices,
//! critical rules, and toolkit-specific guidance.
//!
//! Run with:
//! ```bash
//! cargo run --example wizard_instructions
//! ```

use composio_sdk::wizard::generate_wizard_instructions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Composio Wizard Instructions Generator ===\n");

    // Example 1: Generate generic Composio instructions
    println!("1. Generating generic Composio instructions...\n");
    match generate_wizard_instructions(None) {
        Ok(instructions) => {
            println!("✓ Generated {} bytes of generic instructions\n", instructions.len());
            println!("{}\n", instructions);
            println!("{}", "=".repeat(80));
        }
        Err(e) => {
            eprintln!("✗ Error generating generic instructions: {}", e);
            eprintln!("  Make sure the Skills content is available (should be bundled with SDK)");
            eprintln!("  If building from source, ensure composio-sdk/skills/ directory exists.\n");
        }
    }

    // Example 2: Generate GitHub-specific instructions
    println!("\n2. Generating GitHub-specific instructions...\n");
    match generate_wizard_instructions(Some("github")) {
        Ok(instructions) => {
            println!("✓ Generated {} bytes of GitHub-specific instructions\n", instructions.len());
            println!("{}\n", instructions);
            println!("{}", "=".repeat(80));
        }
        Err(e) => {
            eprintln!("✗ Error generating GitHub instructions: {}", e);
        }
    }

    // Example 3: Generate Gmail-specific instructions
    println!("\n3. Generating Gmail-specific instructions...\n");
    match generate_wizard_instructions(Some("gmail")) {
        Ok(instructions) => {
            println!("✓ Generated {} bytes of Gmail-specific instructions\n", instructions.len());
            // Print first 500 characters as preview
            let preview = if instructions.len() > 500 {
                format!("{}...\n\n[Truncated for brevity]", &instructions[..500])
            } else {
                instructions.clone()
            };
            println!("{}\n", preview);
            println!("{}", "=".repeat(80));
        }
        Err(e) => {
            eprintln!("✗ Error generating Gmail instructions: {}", e);
        }
    }

    // Example 4: Generate Slack-specific instructions
    println!("\n4. Generating Slack-specific instructions...\n");
    match generate_wizard_instructions(Some("slack")) {
        Ok(instructions) => {
            println!("✓ Generated {} bytes of Slack-specific instructions\n", instructions.len());
            // Print first 500 characters as preview
            let preview = if instructions.len() > 500 {
                format!("{}...\n\n[Truncated for brevity]", &instructions[..500])
            } else {
                instructions.clone()
            };
            println!("{}\n", preview);
            println!("{}", "=".repeat(80));
        }
        Err(e) => {
            eprintln!("✗ Error generating Slack instructions: {}", e);
        }
    }

    // Example 5: Generate instructions for unknown toolkit
    println!("\n5. Generating instructions for unknown toolkit (example-toolkit)...\n");
    match generate_wizard_instructions(Some("example-toolkit")) {
        Ok(instructions) => {
            println!("✓ Generated {} bytes of instructions\n", instructions.len());
            println!("Note: For unknown toolkits, generic instructions are provided");
            println!("with a note that no toolkit-specific rules were found.\n");
            // Print first 500 characters as preview
            let preview = if instructions.len() > 500 {
                format!("{}...\n\n[Truncated for brevity]", &instructions[..500])
            } else {
                instructions.clone()
            };
            println!("{}\n", preview);
        }
        Err(e) => {
            eprintln!("✗ Error generating instructions: {}", e);
        }
    }

    println!("\n=== Summary ===");
    println!("The wizard instructions include:");
    println!("  • Overview from AGENTS.md");
    println!("  • Critical rules (MUST follow)");
    println!("  • Session management patterns");
    println!("  • Authentication patterns");
    println!("  • Toolkit-specific guidance (when available)");
    println!("  • Correct examples (✅)");
    println!("  • Incorrect examples (❌)");

    Ok(())
}
