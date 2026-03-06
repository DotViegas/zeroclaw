# Publishing Guide for composio-sdk

## Important Notes About Publishing

### Wizard Instructions Feature

The wizard instructions generation feature includes the Composio Skills content **bundled directly within the SDK**.

**For published crate users:**
- The core SDK functionality (sessions, tool execution, meta tools) works perfectly
- Wizard instruction generation is fully available
- Skills content is bundled at `composio-sdk/skills/` (no external dependencies)

**For development:**
- Skills content is included in the repository at `composio-sdk/skills/`
- No build script needed - Skills are part of the source tree
- No manual cloning or downloading required
- Skills content is versioned with the SDK

## Publishing Steps

### 1. Prepare for Publishing

Make sure all changes are committed:

```bash
git add .
git commit -m "Prepare for v0.1.0 release"
```

### 2. Dry Run

Test the publishing process:

```bash
cargo publish --dry-run --allow-dirty
```

Expected output:
- Package size information
- List of included files (including `skills/` directory)
- Confirmation that Skills content is included (~33 files)

### 3. Publish to crates.io

```bash
# Login to crates.io (if not already logged in)
cargo login <your-token>

# Publish (use --allow-dirty if you have uncommitted changes)
cargo publish --allow-dirty
```

### 4. Verify Publication

After publishing:
1. Check crates.io: https://crates.io/crates/composio-sdk
2. Wait for docs.rs build: https://docs.rs/composio-sdk (may take 5-10 minutes)
3. Test installation in a new project:

```bash
cargo new test-composio
cd test-composio
cargo add composio-sdk
cargo build
```

## What's Excluded from the Package

The following are excluded from the published crate (see `Cargo.toml` `exclude` field):

- None - All content is included (Skills are bundled within the SDK)

## What's Included in the Package

The published crate includes:

- All source code (`src/`)
- Examples (`examples/`)
- Tests (`tests/`)
- Benchmarks (`benches/`)
- Documentation files (README.md, CHANGELOG.md, etc.)
- Build script (`build.rs`) - validates bundled Skills content
- Cargo configuration files
- Skills content (`skills/`) - bundled within SDK for wizard instructions

## Troubleshooting

### Error: "Source directory was modified by build.rs"

This means the build script tried to clone the Skills repository. This has been fixed in the current version - the build script now gracefully handles missing Skills.

**Solution:** Make sure you're using the updated `build.rs` that doesn't clone during publish.

### Error: "failed to verify package tarball"

This usually means files were added during the build process.

**Solution:** Use the updated build script that doesn't modify the source directory.

### Warning: "Skills repository not found"

This means the Skills content is missing from your package.

**Solution:** The Skills content should be bundled at `composio-sdk/skills/`. If missing, copy from the official repository or ensure the directory exists before publishing.

## Post-Publishing Checklist

- [ ] Verify crate appears on crates.io
- [ ] Wait for docs.rs build to complete
- [ ] Test installation in a fresh project
- [ ] Create GitHub release with same version tag
- [ ] Update README if needed
- [ ] Announce release (Discord, Twitter, etc.)

## Version Bumping for Future Releases

For future releases:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with new version section
3. Commit changes
4. Tag release: `git tag v0.x.y`
5. Push tags: `git push --tags`
6. Publish: `cargo publish --allow-dirty`
7. Create GitHub release

## Support

If you encounter issues during publishing:

1. Check this guide first
2. Review cargo publish documentation: https://doc.rust-lang.org/cargo/commands/cargo-publish.html
3. Ask in Composio Discord: https://discord.gg/composio
4. Open an issue: https://github.com/composio/composio-rust-sdk/issues
