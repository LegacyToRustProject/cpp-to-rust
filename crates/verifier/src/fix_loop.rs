use anyhow::Result;
use tracing::info;

use rust_generator::llm::LlmProvider;
use rust_generator::prompt::extract_rust_code;

use crate::compiler::{CompileChecker, CompileResult};

pub struct FixLoop {
    llm: Box<dyn LlmProvider>,
    max_iterations: usize,
}

pub struct FixResult {
    pub code: String,
    pub iterations: usize,
    pub success: bool,
    pub final_errors: Vec<String>,
}

impl FixLoop {
    pub fn new(llm: Box<dyn LlmProvider>, max_iterations: usize) -> Self {
        Self {
            llm,
            max_iterations,
        }
    }

    pub async fn fix_compile_errors(
        &self,
        mut rust_code: String,
        project_dir: &std::path::Path,
        rust_file: &std::path::Path,
    ) -> Result<FixResult> {
        let checker = CompileChecker::new(project_dir);

        for iteration in 0..self.max_iterations {
            info!(
                "Fix loop iteration {}/{}",
                iteration + 1,
                self.max_iterations
            );

            // Write current code
            std::fs::write(rust_file, &rust_code)?;

            // Check compilation
            match checker.check()? {
                CompileResult::Success => {
                    info!("Compilation successful after {} iterations", iteration + 1);
                    return Ok(FixResult {
                        code: rust_code,
                        iterations: iteration + 1,
                        success: true,
                        final_errors: Vec::new(),
                    });
                }
                CompileResult::Errors(errors) => {
                    let error_text = CompileChecker::format_errors(&errors);
                    info!(
                        "Compilation failed with {} errors, requesting fix",
                        errors.len()
                    );

                    rust_code = self.request_fix(&rust_code, &error_text).await?;
                }
            }
        }

        // Final check after all iterations
        std::fs::write(rust_file, &rust_code)?;
        match checker.check()? {
            CompileResult::Success => Ok(FixResult {
                code: rust_code,
                iterations: self.max_iterations,
                success: true,
                final_errors: Vec::new(),
            }),
            CompileResult::Errors(errors) => Ok(FixResult {
                code: rust_code,
                iterations: self.max_iterations,
                success: false,
                final_errors: errors.iter().map(|e| e.to_string()).collect(),
            }),
        }
    }

    pub async fn fix_output_mismatch(&self, rust_code: &str, diff: &str) -> Result<String> {
        let prompt = format!(
            r#"The following Rust code produces different output than the original C/C++ code.

## Current Rust Code
```rust
{}
```

## Output Difference
{}

Fix the Rust code so it produces the same output as the original C/C++ code.
Return ONLY the fixed Rust code in a ```rust block."#,
            rust_code, diff
        );

        let system = "You are an expert Rust developer fixing code conversion issues. \
                       Fix the code to produce the exact same output as the original C/C++ program.";

        let response = self.llm.generate(system, &prompt).await?;
        extract_rust_code(&response)
            .ok_or_else(|| anyhow::anyhow!("No Rust code block in fix response"))
    }

    async fn request_fix(&self, rust_code: &str, errors: &str) -> Result<String> {
        let prompt = format!(
            r#"The following Rust code has compilation errors. Fix them.

## Current Code
```rust
{}
```

## Compilation Errors
{}

Fix ALL the errors and return the complete corrected Rust code in a ```rust block.
Do not explain, just return the fixed code."#,
            rust_code, errors
        );

        let system = "You are an expert Rust developer. Fix compilation errors in the code. \
                       Return ONLY the corrected code in a ```rust code block.";

        let response = self.llm.generate(system, &prompt).await?;
        extract_rust_code(&response)
            .ok_or_else(|| anyhow::anyhow!("No Rust code block in fix response"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_result_success() {
        let result = FixResult {
            code: "fn main() {}".to_string(),
            iterations: 1,
            success: true,
            final_errors: Vec::new(),
        };
        assert!(result.success);
        assert_eq!(result.iterations, 1);
    }
}
