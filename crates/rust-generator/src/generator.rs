use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::info;

use cpp_parser::types::{CppFile, CppProject, Language};

use crate::llm::LlmProvider;
use crate::ownership::generate_ownership_context;
use crate::prompt::{
    build_file_prompt, build_function_prompt, build_system_prompt, extract_rust_code,
    ConversionProfile,
};

pub struct Generator {
    llm: Box<dyn LlmProvider>,
    profile: ConversionProfile,
}

pub struct GeneratedFile {
    pub original_path: PathBuf,
    pub rust_path: PathBuf,
    pub rust_code: String,
}

pub struct GenerationResult {
    pub files: Vec<GeneratedFile>,
    pub errors: Vec<String>,
}

impl Generator {
    pub fn new(llm: Box<dyn LlmProvider>, profile: ConversionProfile) -> Self {
        Self { llm, profile }
    }

    pub async fn convert_project(
        &self,
        project: &CppProject,
        output_dir: &Path,
    ) -> Result<GenerationResult> {
        let mut files = Vec::new();
        let mut errors = Vec::new();

        info!(
            "Converting {} files from {} project",
            project.files.len(),
            project.language
        );

        for file in &project.files {
            match self.convert_file(file, output_dir).await {
                Ok(generated) => files.push(generated),
                Err(e) => {
                    let msg = format!("Failed to convert {}: {}", file.path.display(), e);
                    tracing::error!("{}", msg);
                    errors.push(msg);
                }
            }
        }

        Ok(GenerationResult { files, errors })
    }

    pub async fn convert_file(&self, file: &CppFile, output_dir: &Path) -> Result<GeneratedFile> {
        info!("Converting file: {}", file.path.display());

        let system_prompt = build_system_prompt(&self.profile, &file.language);
        let user_prompt = build_file_prompt(file);

        let response = self.llm.generate(&system_prompt, &user_prompt).await?;
        let rust_code = extract_rust_code(&response).unwrap_or_else(|| response.clone());

        let rust_path = to_rust_path(&file.path, output_dir);
        if let Some(parent) = rust_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&rust_path, &rust_code)?;

        Ok(GeneratedFile {
            original_path: file.path.clone(),
            rust_path,
            rust_code,
        })
    }

    pub async fn convert_function_only(
        &self,
        func: &cpp_parser::types::CppFunction,
        language: &Language,
    ) -> Result<String> {
        let system_prompt = build_system_prompt(&self.profile, language);
        let ownership_context = generate_ownership_context(func);
        let user_prompt = build_function_prompt(func, &ownership_context);

        let response = self.llm.generate(&system_prompt, &user_prompt).await?;
        Ok(extract_rust_code(&response).unwrap_or(response))
    }
}

fn to_rust_path(original: &Path, output_dir: &Path) -> PathBuf {
    let stem = original
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "mod".to_string());

    let relative = original
        .parent()
        .and_then(|p| {
            // Try to make it relative by stripping common prefixes
            p.components()
                .next_back()
                .map(|c| PathBuf::from(c.as_os_str()))
        })
        .unwrap_or_default();

    output_dir.join(relative).join(format!("{}.rs", stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_rust_path() {
        let original = Path::new("/project/src/main.c");
        let output = Path::new("/output");
        let result = to_rust_path(original, output);
        assert_eq!(result, PathBuf::from("/output/src/main.rs"));
    }

    #[test]
    fn test_to_rust_path_cpp() {
        let original = Path::new("/project/lib/utils.cpp");
        let output = Path::new("/output");
        let result = to_rust_path(original, output);
        assert_eq!(result, PathBuf::from("/output/lib/utils.rs"));
    }
}
