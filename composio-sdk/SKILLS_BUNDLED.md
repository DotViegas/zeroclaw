# Bundled Skills Content

## Overview

The Composio Rust SDK now includes **bundled Skills content** directly within the SDK package. This makes the SDK fully self-contained with no external dependencies on the vendor/skills repository.

## Structure

```
composio-sdk/
├── skills/                    # Bundled Skills content
│   ├── AGENTS.md             # Consolidated reference (150+ KB)
│   ├── SKILL.md              # Metadata
│   └── rules/                # Best practices and rules
│       ├── tr-*.md           # Tool Router rules
│       ├── triggers-*.md     # Trigger rules
│       └── app-*.md          # Application rules
├── src/
│   └── wizard/               # Wizard module (uses bundled skills)
│       ├── mod.rs
│       ├── skills.rs         # Skills extraction
│       ├── generator.rs      # Instruction generation
│       └── validator.rs      # Instruction validation
└── ...
```

## Benefits

### 1. Self-Contained SDK
- ✅ No external dependencies on vendor/skills repository
- ✅ Skills content always available at compile time
- ✅ No need to clone or download Skills repository separately
- ✅ Single package installation

### 2. Simplified Usage
Users only need to install the SDK:

```toml
[dependencies]
composio-sdk = "0.1.0"
```

No need to:
- Clone the Skills repository
- Set up vendor/skills directory
- Manage external dependencies

### 3. Guaranteed Availability
- Skills content is always present
- No runtime errors due to missing Skills files
- Consistent behavior across all installations

### 4. Version Control
- Skills content versioned with SDK
- Specific SDK version = specific Skills version
- No version mismatch issues

## Usage

### Generating Wizard Instructions

```rust
use composio_sdk::wizard::generate_wizard_instructions;

// Generate generic instructions
let instructions = generate_wizard_instructions(None)?;
println!("{}", instructions);

// Generate toolkit-specific instructions
let github_instructions = generate_wizard_instructions(Some("github"))?;
println!("{}", github_instructions);
```

The function automatically uses the bundled Skills content at compile time.

### Advanced Usage

```rust
use composio_sdk::wizard::{SkillsExtractor, WizardInstructionGenerator};

// Skills path is resolved at compile time
let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
let skills = SkillsExtractor::new(skills_path);

// Extract specific rules
let tool_router_rules = skills.get_tool_router_rules()?;
let trigger_rules = skills.get_trigger_rules()?;
let session_rules = skills.get_rules_by_tag("sessions")?;

// Generate custom instructions
let generator = WizardInstructionGenerator::new(skills);
let instructions = generator.generate_composio_instructions(Some("slack"))?;
```

## Content Included

### AGENTS.md
- Consolidated reference documentation (150+ KB)
- Comprehensive guide to Composio integration
- Best practices and patterns
- Common pitfalls and solutions

### SKILL.md
- Metadata about the Skills content
- Version information
- Content structure

### Rules Directory
30+ rule files covering:

#### Tool Router Rules (tr-*.md)
- Session management (`tr-session-*.md`)
- Authentication (`tr-auth-*.md`)
- Framework integration (`tr-framework-*.md`)
- Toolkit queries (`tr-toolkit-query.md`)
- User ID best practices (`tr-userid-best-practices.md`)
- MCP vs Native tools (`tr-mcp-vs-native.md`)
- Building chat UI (`tr-building-chat-ui.md`)

#### Trigger Rules (triggers-*.md)
- Creating triggers (`triggers-create.md`)
- Managing triggers (`triggers-manage.md`)
- Subscribing to events (`triggers-subscribe.md`)
- Webhook handling (`triggers-webhook.md`)

#### Application Rules (app-*.md)
- Auth configs (`app-auth-configs.md`)
- Connected accounts (`app-connected-accounts.md`)
- Custom tools (`app-custom-tools.md`)
- Tool execution (`app-execute-tools.md`)
- Fetching tools (`app-fetch-tools.md`)
- Tool modifiers (`app-modifiers.md`)
- Tool versions (`app-tool-versions.md`)
- Toolkits (`app-toolkits.md`)
- User context (`app-user-context.md`)
- Auth popup UI (`app-auth-popup-ui.md`)

#### Setup Rules
- API keys (`setup-api-keys.md`)
- Composio CLI (`composio-cli.md`)

## Rule Format

Each rule file contains:

```markdown
---
title: Rule Title
impact: CRITICAL|HIGH|MEDIUM|LOW
tags: [tag1, tag2, tag3]
description: Brief description
---

# Rule Content

Detailed explanation of the rule.

## Correct ✅

```language
// Correct example
```

## Incorrect ❌

```language
// Incorrect example
```
```

## Migration from External Skills

If you were previously using external vendor/skills:

### Before
```rust
// Required external vendor/skills repository
let skills = SkillsExtractor::new("vendor/skills/skills/composio");
```

### After
```rust
// Uses bundled skills (no external dependency)
let skills_path = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
let skills = SkillsExtractor::new(skills_path);

// Or use the convenience function
let instructions = generate_wizard_instructions(Some("github"))?;
```

## Publishing

When publishing to crates.io, the Skills content is automatically included:

```toml
# Cargo.toml
[package]
name = "composio-sdk"
# Skills directory is included by default
# No need to specify in 'include' field
```

The bundled Skills add approximately:
- **Size**: ~200 KB to the published crate
- **Files**: 33 files (AGENTS.md, SKILL.md, 31 rule files)

## Updating Skills Content

To update the bundled Skills content:

1. Copy updated files from the official Skills repository:
   ```bash
   cp -r vendor/skills/skills/composio/* composio-sdk/skills/
   ```

2. Verify the update:
   ```bash
   cargo test --package composio-sdk
   ```

3. Commit the changes:
   ```bash
   git add composio-sdk/skills/
   git commit -m "Update bundled Skills content"
   ```

## Examples

See the examples directory:
- [`examples/wizard_instructions.rs`](examples/wizard_instructions.rs) - Generate wizard instructions
- [`examples/skills_extraction.rs`](examples/skills_extraction.rs) - Extract and filter rules

Run examples:
```bash
cargo run --example wizard_instructions
cargo run --example skills_extraction
```

## FAQ

### Q: Do I need to download the Skills repository separately?
**A:** No, Skills content is bundled within the SDK.

### Q: Can I use custom Skills content?
**A:** Yes, you can create a `SkillsExtractor` with a custom path pointing to your own Skills directory.

### Q: How often is the Skills content updated?
**A:** Skills content is updated with each SDK release. Check the CHANGELOG for updates.

### Q: What if I don't need the wizard functionality?
**A:** The wizard module is optional. If you don't use it, the bundled Skills content has minimal impact on your binary size due to Rust's dead code elimination.

### Q: Can I contribute to the Skills content?
**A:** Yes! Skills content comes from the official [Composio Skills repository](https://github.com/ComposioHQ/skills). Contribute there, and updates will be included in future SDK releases.

## Support

For issues or questions:
- [GitHub Issues](https://github.com/composio/composio-rust-sdk/issues)
- [Discord Community](https://discord.gg/composio)
- [Documentation](https://docs.composio.dev)
