use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub struct SanitizerResult {
    pub passed: bool,
    pub issues: Vec<SanitizerIssue>,
}

#[derive(Debug)]
pub struct SanitizerIssue {
    pub sanitizer: String,
    pub message: String,
}

pub struct SanitizerChecker {
    cc: String,
}

impl SanitizerChecker {
    pub fn new(cc: Option<String>) -> Self {
        Self {
            cc: cc.unwrap_or_else(|| "gcc".to_string()),
        }
    }

    pub fn check_with_asan(&self, source: &Path) -> Result<SanitizerResult> {
        self.run_sanitizer(source, "-fsanitize=address", "AddressSanitizer")
    }

    pub fn check_with_ubsan(&self, source: &Path) -> Result<SanitizerResult> {
        self.run_sanitizer(source, "-fsanitize=undefined", "UndefinedBehaviorSanitizer")
    }

    pub fn check_all(&self, source: &Path) -> Result<SanitizerResult> {
        let mut all_issues = Vec::new();

        match self.check_with_asan(source) {
            Ok(result) => all_issues.extend(result.issues),
            Err(e) => all_issues.push(SanitizerIssue {
                sanitizer: "AddressSanitizer".to_string(),
                message: format!("Failed to run: {}", e),
            }),
        }

        match self.check_with_ubsan(source) {
            Ok(result) => all_issues.extend(result.issues),
            Err(e) => all_issues.push(SanitizerIssue {
                sanitizer: "UndefinedBehaviorSanitizer".to_string(),
                message: format!("Failed to run: {}", e),
            }),
        }

        Ok(SanitizerResult {
            passed: all_issues.is_empty(),
            issues: all_issues,
        })
    }

    fn run_sanitizer(&self, source: &Path, flag: &str, name: &str) -> Result<SanitizerResult> {
        let temp_binary = std::env::temp_dir().join(format!("cpp_to_rust_san_{}", name));

        // Compile with sanitizer
        let compile = Command::new(&self.cc)
            .arg(source)
            .arg(flag)
            .arg("-g")
            .arg("-o")
            .arg(&temp_binary)
            .arg("-lm")
            .output()?;

        if !compile.status.success() {
            let stderr = String::from_utf8_lossy(&compile.stderr);
            bail!("Failed to compile with {}: {}", name, stderr);
        }

        // Run the instrumented binary
        let output = Command::new(&temp_binary).output()?;

        let _ = std::fs::remove_file(&temp_binary);

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let mut issues = Vec::new();

        if !output.status.success() || stderr.contains("ERROR:") || stderr.contains("runtime error")
        {
            for line in stderr.lines() {
                if line.contains("ERROR:")
                    || line.contains("runtime error")
                    || line.contains("heap-buffer-overflow")
                    || line.contains("use-after-free")
                    || line.contains("stack-buffer-overflow")
                    || line.contains("null pointer")
                {
                    issues.push(SanitizerIssue {
                        sanitizer: name.to_string(),
                        message: line.to_string(),
                    });
                }
            }
        }

        Ok(SanitizerResult {
            passed: issues.is_empty(),
            issues,
        })
    }
}

pub fn format_sanitizer_report(result: &SanitizerResult) -> String {
    if result.passed {
        return "All sanitizer checks passed.".to_string();
    }

    let mut report = String::from("Sanitizer issues found:\n\n");
    for issue in &result.issues {
        report.push_str(&format!("[{}] {}\n", issue.sanitizer, issue.message));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_sanitizer_report_passed() {
        let result = SanitizerResult {
            passed: true,
            issues: Vec::new(),
        };
        let report = format_sanitizer_report(&result);
        assert!(report.contains("passed"));
    }

    #[test]
    fn test_format_sanitizer_report_issues() {
        let result = SanitizerResult {
            passed: false,
            issues: vec![SanitizerIssue {
                sanitizer: "AddressSanitizer".to_string(),
                message: "heap-buffer-overflow on address 0x1234".to_string(),
            }],
        };
        let report = format_sanitizer_report(&result);
        assert!(report.contains("AddressSanitizer"));
        assert!(report.contains("heap-buffer-overflow"));
    }
}
