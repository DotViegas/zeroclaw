use std::fs;
use std::path::Path;

fn main() {
    // Skills content is now bundled within the SDK at composio-sdk/skills/
    let skills_path = Path::new("skills");
    
    // Verify Skills content is accessible
    if !skills_path.exists() {
        panic!(
            "Skills directory not found at {}. \
            The bundled Skills content is required for wizard functionality.",
            skills_path.display()
        );
    }
    
    println!("cargo:warning=Bundled Skills content found at skills/");
    
    // Verify Skills content structure
    verify_skills_content();
    
    // Add rerun-if-changed directives for Skills content
    add_rerun_directives();
}

fn verify_skills_content() {
    let skills_path = Path::new("skills");
    
    // Check for key files
    let agents_md = skills_path.join("AGENTS.md");
    let skill_md = skills_path.join("SKILL.md");
    let rules_dir = skills_path.join("rules");
    
    if !agents_md.exists() {
        panic!(
            "Skills content verification failed: AGENTS.md not found at {}. \
            The bundled Skills content may be incomplete.",
            agents_md.display()
        );
    }
    
    if !skill_md.exists() {
        println!(
            "cargo:warning=SKILL.md not found at {} (optional)",
            skill_md.display()
        );
    }
    
    if !rules_dir.exists() {
        panic!(
            "Skills content verification failed: rules directory not found at {}. \
            The bundled Skills content may be incomplete.",
            rules_dir.display()
        );
    }
    
    // Check for rule files
    let rule_files: Vec<_> = fs::read_dir(&rules_dir)
        .expect("Failed to read rules directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            // Skip template files
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with('_') {
                    return false;
                }
            }
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "md")
                .unwrap_or(false)
        })
        .collect();
    
    if rule_files.is_empty() {
        panic!(
            "Skills content verification failed: No markdown files found in rules directory. \
            The bundled Skills content may be incomplete."
        );
    }
    
    println!("cargo:warning=Bundled Skills content verified successfully");
    println!("cargo:warning=Found {} rule files", rule_files.len());
}

fn add_rerun_directives() {
    let skills_path = Path::new("skills");
    
    // Rerun if the entire skills directory changes
    println!("cargo:rerun-if-changed=skills");
    
    // Rerun if AGENTS.md changes
    let agents_md = skills_path.join("AGENTS.md");
    if agents_md.exists() {
        println!("cargo:rerun-if-changed={}", agents_md.display());
    }
    
    // Rerun if SKILL.md changes
    let skill_md = skills_path.join("SKILL.md");
    if skill_md.exists() {
        println!("cargo:rerun-if-changed={}", skill_md.display());
    }
    
    // Rerun if rules directory changes
    let rules_dir = skills_path.join("rules");
    if rules_dir.exists() {
        println!("cargo:rerun-if-changed={}", rules_dir.display());
        
        // Add directives for individual rule files
        if let Ok(entries) = fs::read_dir(&rules_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }
    
    // Rerun if build.rs itself changes
    println!("cargo:rerun-if-changed=build.rs");
}
