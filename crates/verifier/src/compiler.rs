use anyhow::Result;
use regex::Regex;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub suggestion: Option<String>,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.file, self.line, self.column, self.message
        )?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n  suggestion: {}", suggestion)?;
        }
        Ok(())
    }
}

pub enum CompileResult {
    Success,
    Errors(Vec<CompileError>),
}

pub struct CompileChecker {
    project_dir: std::path::PathBuf,
}

impl CompileChecker {
    pub fn new(project_dir: &Path) -> Self {
        Self {
            project_dir: project_dir.to_path_buf(),
        }
    }

    pub fn check(&self) -> Result<CompileResult> {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--message-format=short")
            .current_dir(&self.project_dir)
            .output()?;

        if output.status.success() {
            return Ok(CompileResult::Success);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let errors = parse_compiler_errors(&stderr);

        if errors.is_empty() {
            Ok(CompileResult::Errors(vec![CompileError {
                file: String::new(),
                line: 0,
                column: 0,
                message: stderr.to_string(),
                suggestion: None,
            }]))
        } else {
            Ok(CompileResult::Errors(errors))
        }
    }

    pub fn check_single_file(&self, file_path: &Path) -> Result<CompileResult> {
        // For single file checks, we create a temporary Cargo project
        let temp_dir = std::env::temp_dir().join("cpp_to_rust_check");
        std::fs::create_dir_all(&temp_dir)?;

        let cargo_toml = r#"[package]
name = "check-temp"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "check"
path = "main.rs"
"#;

        std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;
        std::fs::copy(file_path, temp_dir.join("main.rs"))?;

        let checker = CompileChecker::new(&temp_dir);
        let result = checker.check();

        let _ = std::fs::remove_dir_all(&temp_dir);
        result
    }

    pub fn format_errors(errors: &[CompileError]) -> String {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

fn parse_compiler_errors(stderr: &str) -> Vec<CompileError> {
    let mut errors = Vec::new();
    let error_re = Regex::new(r"(?m)^error(?:\[E\d+\])?: (.+)\n\s*--> (.+):(\d+):(\d+)").unwrap();
    let suggestion_re = Regex::new(r"(?m)help: (.+)").unwrap();

    for cap in error_re.captures_iter(stderr) {
        let message = cap[1].to_string();
        let file = cap[2].to_string();
        let line: usize = cap[3].parse().unwrap_or(0);
        let column: usize = cap[4].parse().unwrap_or(0);

        // Look for suggestions near this error
        let error_end = cap.get(0).unwrap().end();
        let remaining = &stderr[error_end..];
        let suggestion = suggestion_re.captures(remaining).map(|s| s[1].to_string());

        errors.push(CompileError {
            file,
            line,
            column,
            message,
            suggestion,
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compiler_errors() {
        let stderr = r#"error[E0308]: mismatched types
 --> src/main.rs:5:12
  |
5 |     let x: i32 = "hello";
  |            ---   ^^^^^^^ expected `i32`, found `&str`
  |            |
  |            expected due to this
  help: try this instead

error[E0425]: cannot find value `y` in this scope
 --> src/main.rs:10:5
  |
10 |     y
   |     ^ not found in this scope
"#;

        let errors = parse_compiler_errors(stderr);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].line, 5);
        assert!(errors[0].message.contains("mismatched types"));
        assert!(errors[0].suggestion.is_some());
        assert_eq!(errors[1].line, 10);
    }

    #[test]
    fn test_parse_no_errors() {
        let stderr = "   Compiling my-project v0.1.0\n    Finished dev [unoptimized + debuginfo]\n";
        let errors = parse_compiler_errors(stderr);
        assert!(errors.is_empty());
    }
}
