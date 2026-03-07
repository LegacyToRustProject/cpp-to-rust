use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub struct ComparisonResult {
    pub matches: bool,
    pub c_output: String,
    pub rust_output: String,
    pub diff: Option<String>,
}

pub struct OutputComparator {
    cc: String, // C/C++ compiler command (gcc, g++, clang, clang++)
}

impl OutputComparator {
    pub fn new(cc: Option<String>) -> Self {
        Self {
            cc: cc.unwrap_or_else(|| "gcc".to_string()),
        }
    }

    pub async fn compare(
        &self,
        c_source: &Path,
        rust_binary: &Path,
        stdin_input: Option<&str>,
    ) -> Result<ComparisonResult> {
        // Compile C/C++ source to temporary binary
        let temp_binary = std::env::temp_dir().join("cpp_to_rust_compare_c");
        let compile_status = Command::new(&self.cc)
            .arg(c_source)
            .arg("-o")
            .arg(&temp_binary)
            .arg("-lm") // link math library
            .status()?;

        if !compile_status.success() {
            bail!("Failed to compile C/C++ source: {}", c_source.display());
        }

        // Run C/C++ binary
        let c_output = run_binary(&temp_binary, stdin_input)?;

        // Run Rust binary
        let rust_output = run_binary(rust_binary, stdin_input)?;

        // Clean up
        let _ = std::fs::remove_file(&temp_binary);

        let matches = c_output == rust_output;
        let diff = if matches {
            None
        } else {
            Some(generate_diff(&c_output, &rust_output))
        };

        Ok(ComparisonResult {
            matches,
            c_output,
            rust_output,
            diff,
        })
    }

    pub fn compare_outputs(c_output: &str, rust_output: &str) -> ComparisonResult {
        let matches = c_output == rust_output;
        let diff = if matches {
            None
        } else {
            Some(generate_diff(c_output, rust_output))
        };

        ComparisonResult {
            matches,
            c_output: c_output.to_string(),
            rust_output: rust_output.to_string(),
            diff,
        }
    }
}

fn run_binary(binary: &Path, stdin_input: Option<&str>) -> Result<String> {
    let mut cmd = Command::new(binary);

    if let Some(input) = stdin_input {
        use std::io::Write;
        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(input.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let output = cmd.output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

fn generate_diff(expected: &str, actual: &str) -> String {
    let mut diff = String::new();
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    let max_lines = expected_lines.len().max(actual_lines.len());

    for i in 0..max_lines {
        let exp = expected_lines.get(i).copied().unwrap_or("<missing>");
        let act = actual_lines.get(i).copied().unwrap_or("<missing>");

        if exp != act {
            diff.push_str(&format!("Line {}: \n", i + 1));
            diff.push_str(&format!("  C/C++: {}\n", exp));
            diff.push_str(&format!("  Rust:  {}\n", act));
        }
    }

    if expected_lines.len() != actual_lines.len() {
        diff.push_str(&format!(
            "\nLine count: C/C++={}, Rust={}\n",
            expected_lines.len(),
            actual_lines.len()
        ));
    }

    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_matching_outputs() {
        let result = OutputComparator::compare_outputs("Hello, World!\n", "Hello, World!\n");
        assert!(result.matches);
        assert!(result.diff.is_none());
    }

    #[test]
    fn test_compare_different_outputs() {
        let result = OutputComparator::compare_outputs("Hello\nWorld\n", "Hello\nRust\n");
        assert!(!result.matches);
        assert!(result.diff.is_some());
        let diff = result.diff.unwrap();
        assert!(diff.contains("Line 2"));
    }

    #[test]
    fn test_generate_diff() {
        let diff = generate_diff("line1\nline2\n", "line1\nchanged\n");
        assert!(diff.contains("Line 2"));
        assert!(diff.contains("line2"));
        assert!(diff.contains("changed"));
    }
}
