# Wizard Instruction Generation Process

This document describes the wizard instruction generation process for the Composio Rust SDK, which uses the official Composio Skills repository to generate production-ready guidance for AI agents.

## Overview

The wizard instruction generation system extracts best practices, critical rules, and anti-patterns from the [Composio Skills repository](https://github.com/ComposioHQ/skills) and formats them into comprehensive markdown instructions for AI agents.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              Composio Skills Repository                      │
│  (https://github.com/ComposioHQ/skills)                     │
│  - AGENTS.md (consolidated reference)                        │
│  - rules/tr-*.md (Tool Router rules)                        │
│  - rules/triggers-*.md (Trigger rules)                      │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        │ extracts
                        ▼
                ┌───────────────┐
                │ SkillsExtractor│
                └───────┬───────┘
                        │
         ┌──────────────┼──────────────┐
         │              │              │
         ▼              ▼              ▼
  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
  │  Generator  │ │  Validator  │ │   Rules     │
  └─────────────┘ └─────────────┘ └─────────────┘
```

## Components

### 1. SkillsExtractor (`src/wizard/skills.rs`)

Extracts and parses rules from the Skills repository.

**Key Features:**
- Parses markdown files with YAML frontmatter
- Extracts correct (✅) and incorrect (❌) examples
- Filters rules by tag, impact level, or prefix
- Reads consolidated AGENTS.md content

**Usage:**
```rust
use composio_sdk::wizard::SkillsExtractor;

// Skills are bundled within the SDK
let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
let skills = SkillsExtractor::new(skills_path);
skills.verify_path()?;

// Get Tool Router rules
let tr_rules = skills.get_tool_router_rules()?;

// Get rules by tag
let session_rules = skills.get_rules_by_tag("sessions")?;

// Get consolidated content
let agents_md = skills.get_consolidated_content()?;
```

### 2. WizardInstructionGenerator (`src/wizard/generator.rs`)

Generates formatted wizard instructions from extracted rules.

**Key Features:**
- Generates overview from AGENTS.md
- Includes critical rules section
- Adds session management patterns
- Adds authentication patterns
- Supports toolkit-specific guidance

**Usage:**
```rust
use composio_sdk::wizard::{SkillsExtractor, WizardInstructionGenerator};

// Skills are bundled within the SDK
let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
let skills = SkillsExtractor::new(skills_path);
let generator = WizardInstructionGenerator::new(skills);

// Generate generic instructions
let instructions = generator.generate_composio_instructions(None)?;

// Generate toolkit-specific instructions
let github_instructions = generator.generate_composio_instructions(Some("github"))?;
```

### 3. InstructionValidator (`src/wizard/validator.rs`)

Validates instructions against Skills best practices.

**Key Features:**
- Detects anti-patterns (❌ examples)
- Checks for missing critical rules
- Checks for missing high-priority rules (warnings)
- Provides detailed validation results

**Usage:**
```rust
use composio_sdk::wizard::{SkillsExtractor, InstructionValidator};

// Skills are bundled within the SDK
let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
let skills = SkillsExtractor::new(skills_path);
let validator = InstructionValidator::new(skills);

let result = validator.validate(instructions)?;

if !result.is_valid() {
    println!("{}", result.format());
}
```

### 4. Convenience Function (`src/wizard/mod.rs`)

High-level API for generating instructions.

**Usage:**
```rust
use composio_sdk::wizard::generate_wizard_instructions;

// Generate generic instructions
let instructions = generate_wizard_instructions(None)?;

// Generate toolkit-specific instructions
let github_instructions = generate_wizard_instructions(Some("github"))?;
let gmail_instructions = generate_wizard_instructions(Some("gmail"))?;
let slack_instructions = generate_wizard_instructions(Some("slack"))?;
```

## Rule Format

Rules are markdown files with YAML frontmatter:

```markdown
---
title: Always use composio.create(user_id)
impact: critical
tags: [session, user-scoping]
---

# Description
Always create sessions with user_id for proper isolation.

## Correct ✅
```python
session = composio.create(user_id="user_123")
```

## Incorrect ❌
```python
session = composio.create()  # Missing user_id
```
```

## Impact Levels

Rules are categorized by impact:

- **CRITICAL**: Must be followed, causes failures if violated
- **HIGH**: Should be followed, causes issues if violated
- **MEDIUM**: Recommended, improves quality
- **LOW**: Optional, nice to have

## Generated Instruction Structure

Generated instructions follow this structure:

```markdown
# Composio Wizard Instructions

**Context:** Using toolkit `github` (if toolkit-specific)

## Overview
[Content from AGENTS.md]

## Critical Rules
[Rules with CRITICAL impact]

### 1. Rule Title
**Description:** Rule description
**Impact:** CRITICAL
**Tags:** tag1, tag2

✅ **Correct Examples:**
[Code examples]

❌ **Incorrect Examples:**
[Code examples]

## Session Management Patterns
[Rules tagged with "sessions"]

## Authentication Patterns
[Rules tagged with "authentication"]

## Toolkit-Specific Guidance: github
[Rules tagged with toolkit name]
```

## Build Integration

The Skills content is **bundled directly within the SDK** at `composio-sdk/skills/`.

The build script (`build.rs`) verifies the bundled Skills content:

1. Checks if `skills/` directory exists
2. Verifies required files (AGENTS.md, SKILL.md, rules/)
3. Validates at least 29 rule files are present
4. Sets up rerun triggers for Skills content changes

No external downloads or cloning required - Skills are part of the SDK source tree.

## Testing

### Unit Tests

Located in `src/wizard/*.rs` files:
- Rule parsing tests
- Example extraction tests
- Validation logic tests
- Formatting tests

### Integration Tests

Located in `tests/wizard_instructions_test.rs`:
- Generic instruction generation
- Toolkit-specific instruction generation
- Multiple toolkit generation
- Content validation (correct/incorrect examples, impact levels)
- Markdown validation
- Session management and authentication content

Located in `tests/wizard_validation_test.rs`:
- Anti-pattern detection
- Deprecated terminology detection
- Missing critical rules detection
- Validation result formatting
- Comprehensive instruction validation

### Running Tests

```bash
# Run all wizard tests (requires Skills repository)
cargo test --test wizard_instructions_test -- --ignored
cargo test --test wizard_validation_test -- --ignored

# Run specific test
cargo test --test wizard_instructions_test test_generate_github_instructions -- --ignored
```

## Examples

### Example 1: Generate Instructions for GitHub

```rust
use composio_sdk::wizard::generate_wizard_instructions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instructions = generate_wizard_instructions(Some("github"))?;
    println!("{}", instructions);
    Ok(())
}
```

### Example 2: Validate Custom Instructions

```rust
use composio_sdk::wizard::{SkillsExtractor, InstructionValidator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Skills are bundled within the SDK
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);
    let validator = InstructionValidator::new(skills);

    let my_instructions = r#"
        # My Composio Guide
        
        Create a session:
        ```rust
        let session = client.create_session("default");
        ```
    "#;

    let result = validator.validate(my_instructions)?;

    if !result.is_valid() {
        println!("Validation failed:");
        println!("{}", result.format());
    }

    Ok(())
}
```

### Example 3: Extract Specific Rules

```rust
use composio_sdk::wizard::{SkillsExtractor, Impact};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Skills are bundled within the SDK
    let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
    let skills = SkillsExtractor::new(skills_path);

    // Get all critical rules
    let all_rules = skills.get_all_rules()?;
    let critical_rules: Vec<_> = all_rules
        .iter()
        .filter(|r| r.impact == Impact::Critical)
        .collect();

    println!("Found {} critical rules:", critical_rules.len());
    for rule in critical_rules {
        println!("  - {}", rule.title);
    }

    Ok(())
}
```

## Validation Process

The validation process checks instructions for:

1. **Anti-patterns**: Code matching incorrect (❌) examples from rules
2. **Keyword anti-patterns**: Deprecated terminology (entity_id, actions, etc.)
3. **Missing critical rules**: Rules with CRITICAL impact not mentioned
4. **Missing high-priority rules**: Rules with HIGH impact not mentioned (warnings)

### Validation Result

```rust
pub struct ValidationResult {
    pub errors: Vec<String>,      // Must be fixed
    pub warnings: Vec<String>,     // Should be addressed
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool;
    pub fn has_warnings(&self) -> bool;
    pub fn total_issues(&self) -> usize;
    pub fn format(&self) -> String;
}
```

## Best Practices

### For Instruction Generation

1. **Always verify Skills path** before generating instructions
2. **Use toolkit-specific generation** when possible for better context
3. **Handle errors gracefully** - Skills repository may not be available
4. **Cache generated instructions** if generating frequently

### For Validation

1. **Validate before deployment** to catch anti-patterns early
2. **Review warnings** even if validation passes
3. **Update instructions** when new critical rules are added
4. **Test with actual Skills content** using `--ignored` flag

### For Rule Extraction

1. **Filter by impact level** to focus on critical rules
2. **Use tags** to find related rules
3. **Check both correct and incorrect examples** for completeness
4. **Parse frontmatter** to get metadata

## Troubleshooting

### Skills Repository Not Found

**Error:** `SkillsError::PathNotFound`

**Solution:**
1. Verify Skills content exists: `ls composio-sdk/skills/`
2. If building from source, ensure the skills/ directory is present
3. The Skills content should be bundled with the SDK automatically

### Validation Detects False Positives

**Issue:** Validator flags correct code as anti-pattern

**Solution:**
1. Check if the code matches an incorrect example exactly
2. Review the rule's incorrect examples
3. Adjust code to avoid pattern matching
4. Consider if it's actually an anti-pattern

### Generated Instructions Too Large

**Issue:** Instructions exceed context window

**Solution:**
1. Use toolkit-specific generation to reduce size
2. Filter rules by impact level (CRITICAL only)
3. Customize generator to include fewer sections
4. Extract only relevant rules for your use case

## Performance Considerations

- **Skills extraction**: ~10-50ms for 30 rule files
- **Instruction generation**: ~50-200ms including extraction
- **Validation**: ~10-50ms for typical instructions
- **Memory usage**: ~1-2 MB for Skills content in memory

## Future Enhancements

Potential improvements for the wizard instruction system:

1. **Caching**: Cache parsed rules to avoid re-parsing
2. **Incremental updates**: Detect Skills repository changes
3. **Custom templates**: Allow custom instruction templates
4. **Multi-language support**: Generate instructions in multiple languages
5. **Interactive mode**: CLI tool for exploring rules
6. **Rule search**: Full-text search across rules
7. **Diff detection**: Compare instruction versions
8. **Auto-fix suggestions**: Suggest fixes for anti-patterns

## References

- [Composio Skills Repository](https://github.com/ComposioHQ/skills)
- [Composio Documentation](https://docs.composio.dev)
- [Tool Router API](https://docs.composio.dev/api-reference/tool-router)
- [Composio Best Practices](https://docs.composio.dev/guides)

## Support

For issues or questions:
- Check the [Skills repository](https://github.com/ComposioHQ/skills) for rule updates
- Review the [Composio documentation](https://docs.composio.dev)
- Open an issue in the SDK repository
- Contact Composio support

## License

The wizard instruction generation system is part of the Composio Rust SDK and follows the same license. The Composio Skills repository has its own license - see the Skills repository for details.
