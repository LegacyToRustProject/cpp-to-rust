use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::memory::analyze_memory_patterns;
use crate::preprocessor::{extract_includes, extract_macros};
use crate::types::*;

pub fn scan_source_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            if matches!(
                ext.as_str(),
                "c" | "h" | "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh"
            ) {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    Ok(files)
}

pub fn detect_language(path: &Path) -> Language {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
    {
        Some("cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh") => Language::Cpp,
        _ => Language::C,
    }
}

pub fn detect_standard(sources: &[(&str, Language)]) -> CppStandard {
    let mut c_score = 0i32;
    let mut cpp_score = 0i32;
    let mut has_cpp = false;

    // Pre-compile all regexes
    let cpp20_res = [
        Regex::new(r"\bconcept\s+\w+").unwrap(),
        Regex::new(r"\brequires\s*\(").unwrap(),
        Regex::new(r"\bco_await\b").unwrap(),
        Regex::new(r"\bco_yield\b").unwrap(),
    ];
    let cpp17_res = [
        Regex::new(r"\bstd::optional<").unwrap(),
        Regex::new(r"\bstd::variant<").unwrap(),
        Regex::new(r"\bstd::string_view\b").unwrap(),
        Regex::new(r"\bif\s+constexpr\b").unwrap(),
        Regex::new(r"\[\[nodiscard\]\]").unwrap(),
        Regex::new(r"\bauto\s+\[").unwrap(),
    ];
    let cpp11_res = [
        Regex::new(r"\bauto\s+\w+\s*=").unwrap(),
        Regex::new(r"\[\s*[&=]?\s*\]\s*\(").unwrap(),
        Regex::new(r"\bstd::move\(").unwrap(),
        Regex::new(r"\bnullptr\b").unwrap(),
        Regex::new(r"\boverride\b").unwrap(),
        Regex::new(r"\bstd::unique_ptr<").unwrap(),
        Regex::new(r"\bstd::shared_ptr<").unwrap(),
        Regex::new(r"->.*=\s*default|=\s*delete").unwrap(),
    ];
    let cpp03_res = [
        Regex::new(r"\bclass\s+\w+").unwrap(),
        Regex::new(r"\btemplate\s*<").unwrap(),
        Regex::new(r"\bnamespace\s+\w+").unwrap(),
        Regex::new(r"\bstd::").unwrap(),
    ];
    let c11_res = [
        Regex::new(r"\b_Atomic\b").unwrap(),
        Regex::new(r"\b_Thread_local\b").unwrap(),
        Regex::new(r"\b_Static_assert\b").unwrap(),
        Regex::new(r"\b_Generic\b").unwrap(),
    ];
    let c99_res = [
        Regex::new(r"\b(inline|restrict|_Bool)\b").unwrap(),
        Regex::new(r"//").unwrap(),
        Regex::new(r"\bfor\s*\(\s*(int|size_t|unsigned)\s+").unwrap(),
    ];

    for (source, lang) in sources {
        if *lang == Language::Cpp {
            has_cpp = true;
        }

        if cpp20_res.iter().any(|re| re.is_match(source)) {
            cpp_score = cpp_score.max(20);
        }
        if cpp17_res.iter().any(|re| re.is_match(source)) {
            cpp_score = cpp_score.max(17);
        }
        if cpp11_res.iter().any(|re| re.is_match(source)) {
            cpp_score = cpp_score.max(11);
        }
        if cpp03_res.iter().any(|re| re.is_match(source)) {
            cpp_score = cpp_score.max(3);
        }
        if c11_res.iter().any(|re| re.is_match(source)) {
            c_score = c_score.max(110);
        }
        if c99_res.iter().any(|re| re.is_match(source)) {
            c_score = c_score.max(99);
        }
    }

    if has_cpp || cpp_score > 0 {
        match cpp_score {
            20.. => CppStandard::Cpp20,
            17..=19 => CppStandard::Cpp17,
            11..=16 => CppStandard::Cpp11,
            3..=10 => CppStandard::Cpp03,
            _ => CppStandard::Cpp11,
        }
    } else {
        match c_score {
            110.. => CppStandard::C11,
            1..=109 => CppStandard::C99,
            _ => CppStandard::C99,
        }
    }
}

pub fn analyze_file(path: &Path, source: &str) -> CppFile {
    let language = detect_language(path);
    let includes = extract_includes(source);
    let macros = extract_macros(source);
    let structs = extract_structs(source);
    let classes = if language == Language::Cpp {
        extract_classes(source)
    } else {
        Vec::new()
    };
    let functions = extract_functions(source);
    let globals = extract_globals(source);

    CppFile {
        path: path.to_path_buf(),
        source: source.to_string(),
        language,
        includes,
        macros,
        structs,
        classes,
        functions,
        globals,
    }
}

pub fn analyze_project(root: &Path) -> Result<CppProject> {
    let file_paths = scan_source_files(root)?;
    let mut files = Vec::new();
    let mut source_lang_pairs = Vec::new();

    for path in &file_paths {
        let source = std::fs::read_to_string(path)?;
        let lang = detect_language(path);
        source_lang_pairs.push((source.clone(), lang.clone()));
        files.push(analyze_file(path, &source));
    }

    let pairs: Vec<(&str, Language)> = source_lang_pairs
        .iter()
        .map(|(s, l)| (s.as_str(), l.clone()))
        .collect();
    let standard = detect_standard(&pairs);

    let language = if files.iter().any(|f| f.language == Language::Cpp) {
        Language::Cpp
    } else {
        Language::C
    };

    Ok(CppProject {
        root: root.to_path_buf(),
        language,
        standard,
        files,
    })
}

pub fn generate_report(project: &CppProject) -> String {
    use std::fmt::Write;
    let mut report = String::new();
    report.push_str("=== C/C++ Project Analysis ===\n");
    let _ = writeln!(report, "Root: {}", project.root.display());
    let _ = writeln!(report, "Language: {}", project.language);
    let _ = writeln!(report, "Detected Standard: {}", project.standard);
    let _ = writeln!(report, "Files: {}\n", project.files.len());

    let mut total_structs = 0;
    let mut total_classes = 0;
    let mut total_functions = 0;
    let mut total_macros = 0;

    for file in &project.files {
        let _ = writeln!(report, "--- {} ---", file.path.display());
        let _ = writeln!(report, "  Language: {}", file.language);
        let _ = writeln!(report, "  Includes: {}", file.includes.len());
        let _ = writeln!(report, "  Macros: {}", file.macros.len());
        let _ = writeln!(report, "  Structs: {}", file.structs.len());
        let _ = writeln!(report, "  Classes: {}", file.classes.len());
        let _ = writeln!(report, "  Functions: {}", file.functions.len());

        let memory_patterns = analyze_memory_patterns(&file.source);
        if !memory_patterns.is_empty() {
            let summary = crate::memory::summarize_memory_patterns(&memory_patterns);
            report.push_str("  Memory patterns:\n");
            for line in summary.lines() {
                let _ = writeln!(report, "    {}", line);
            }
        }

        total_structs += file.structs.len();
        total_classes += file.classes.len();
        total_functions += file.functions.len();
        total_macros += file.macros.len();
    }

    report.push_str("\n=== Summary ===\n");
    let _ = writeln!(report, "Total structs: {}", total_structs);
    let _ = writeln!(report, "Total classes: {}", total_classes);
    let _ = writeln!(report, "Total functions: {}", total_functions);
    let _ = writeln!(report, "Total macros: {}", total_macros);

    report
}

fn extract_structs(source: &str) -> Vec<CppStruct> {
    let mut structs = Vec::new();
    // typedef struct { ... } Name; or struct Name { ... };
    let re = Regex::new(r"(?m)(typedef\s+)?struct\s+(\w+)?\s*\{([^}]*)\}\s*(\w+)?\s*;").unwrap();

    for cap in re.captures_iter(source) {
        let is_typedef = cap.get(1).is_some();
        let name = cap
            .get(2)
            .or(cap.get(4))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "anonymous".to_string());
        let body = &cap[3];
        let fields = extract_struct_fields(body);

        structs.push(CppStruct {
            name,
            fields,
            is_typedef,
        });
    }

    structs
}

fn extract_struct_fields(body: &str) -> Vec<CppField> {
    let mut fields = Vec::new();
    let re = Regex::new(r"(?m)^\s*(const\s+)?(\w[\w\s\*&<>,]*?)\s+(\w+)\s*;").unwrap();

    for cap in re.captures_iter(body) {
        let is_const = cap.get(1).is_some();
        let type_name = cap[2].trim().to_string();
        let name = cap[3].to_string();
        fields.push(CppField {
            name,
            type_name,
            visibility: Visibility::Public,
            is_static: false,
            is_const,
        });
    }

    fields
}

fn extract_classes(source: &str) -> Vec<CppClass> {
    let mut classes = Vec::new();
    let re = Regex::new(r"(?m)class\s+(\w+)(?:\s*:\s*(.*?))?\s*\{").unwrap();

    for cap in re.captures_iter(source) {
        let name = cap[1].to_string();
        let bases = cap
            .get(2)
            .map(|m| parse_bases(m.as_str()))
            .unwrap_or_default();

        let start = cap.get(0).unwrap().end() - 1;
        if let Some(body) = extract_brace_block(source, start) {
            let methods = extract_class_methods(&body);
            let fields = extract_class_fields(&body);

            classes.push(CppClass {
                name,
                bases,
                fields,
                methods,
                visibility_default: Visibility::Private,
            });
        }
    }

    classes
}

fn parse_bases(bases_str: &str) -> Vec<CppBase> {
    bases_str
        .split(',')
        .filter_map(|b| {
            let b = b.trim();
            if b.is_empty() {
                return None;
            }
            let parts: Vec<&str> = b.split_whitespace().collect();
            let (visibility, name, is_virtual) = match parts.len() {
                1 => (Visibility::Private, parts[0].to_string(), false),
                2 if parts[0] == "virtual" => (Visibility::Private, parts[1].to_string(), true),
                2 => (parse_visibility(parts[0]), parts[1].to_string(), false),
                3.. => {
                    let is_virtual = parts.contains(&"virtual");
                    let vis = parts
                        .iter()
                        .find(|&&p| matches!(p, "public" | "protected" | "private"))
                        .map(|&p| parse_visibility(p))
                        .unwrap_or(Visibility::Private);
                    let name = parts
                        .iter()
                        .find(|&&p| !matches!(p, "public" | "protected" | "private" | "virtual"))
                        .map(|&p| p.to_string())
                        .unwrap_or_default();
                    (vis, name, is_virtual)
                }
                _ => return None,
            };
            Some(CppBase {
                name,
                visibility,
                is_virtual,
            })
        })
        .collect()
}

fn parse_visibility(s: &str) -> Visibility {
    match s {
        "public" => Visibility::Public,
        "protected" => Visibility::Protected,
        _ => Visibility::Private,
    }
}

fn extract_functions(source: &str) -> Vec<CppFunction> {
    let mut functions = Vec::new();
    // Match top-level function definitions (not inside classes)
    let re = Regex::new(
        r"(?m)^(template\s*<[^>]*>\s*)?((?:static\s+|virtual\s+|inline\s+|const\s+)*)([\w:*&<>, ]+?)\s+(\w+)\s*\(([^)]*)\)\s*(?:const\s*)?\{",
    )
    .unwrap();

    for cap in re.captures_iter(source) {
        let is_template = cap.get(1).is_some();
        let template_params = cap
            .get(1)
            .map(|m| parse_template_params(m.as_str()))
            .unwrap_or_default();
        let qualifiers = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let return_type = cap[3].trim().to_string();
        let name = cap[4].to_string();
        let params_str = &cap[5];

        // Skip if this looks like a control flow statement
        if matches!(name.as_str(), "if" | "for" | "while" | "switch" | "catch") {
            continue;
        }

        let start = cap.get(0).unwrap().end() - 1;
        let body = extract_brace_block(source, start).unwrap_or_default();

        functions.push(CppFunction {
            name,
            return_type,
            params: parse_params(params_str),
            body,
            is_static: qualifiers.contains("static"),
            is_virtual: qualifiers.contains("virtual"),
            is_const: qualifiers.contains("const"),
            is_template,
            template_params,
            visibility: Visibility::Public,
        });
    }

    functions
}

fn extract_class_methods(class_body: &str) -> Vec<CppFunction> {
    let mut methods = Vec::new();
    let vis_re = Regex::new(r"(?m)^\s*(public|protected|private)\s*:").unwrap();
    let method_re = Regex::new(
        r"(?m)(template\s*<[^>]*>\s*)?((?:static\s+|virtual\s+|inline\s+)*)([\w:*&<>, ]+?)\s+(\w+)\s*\(([^)]*)\)\s*(?:const\s*)?(?:override\s*)?(?:=\s*(?:0|default|delete)\s*;|\{)",
    )
    .unwrap();

    // Find visibility sections
    let mut sections: Vec<(usize, Visibility)> = vis_re
        .captures_iter(class_body)
        .map(|cap| {
            let pos = cap.get(0).unwrap().start();
            let vis = parse_visibility(&cap[1]);
            (pos, vis)
        })
        .collect();
    sections.insert(0, (0, Visibility::Private));

    for cap in method_re.captures_iter(class_body) {
        let pos = cap.get(0).unwrap().start();
        // Determine visibility at this position
        let current_visibility = sections
            .iter()
            .rev()
            .find(|(p, _)| *p <= pos)
            .map(|(_, v)| v.clone())
            .unwrap_or(Visibility::Private);

        let is_template = cap.get(1).is_some();
        let template_params = cap
            .get(1)
            .map(|m| parse_template_params(m.as_str()))
            .unwrap_or_default();
        let qualifiers = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let return_type = cap[3].trim().to_string();
        let name = cap[4].to_string();
        let params_str = &cap[5];

        if matches!(name.as_str(), "if" | "for" | "while" | "switch" | "catch") {
            continue;
        }

        let full_match = cap.get(0).unwrap().as_str();
        let body = if full_match.ends_with('{') {
            let start = cap.get(0).unwrap().end() - 1;
            extract_brace_block(class_body, start).unwrap_or_default()
        } else {
            String::new() // pure virtual or default/delete
        };

        methods.push(CppFunction {
            name,
            return_type,
            params: parse_params(params_str),
            body,
            is_static: qualifiers.contains("static"),
            is_virtual: qualifiers.contains("virtual"),
            is_const: full_match.contains("const"),
            is_template,
            template_params,
            visibility: current_visibility.clone(),
        });
    }

    methods
}

fn extract_class_fields(class_body: &str) -> Vec<CppField> {
    let mut fields = Vec::new();
    let re =
        Regex::new(r"(?m)^\s*(static\s+)?(const\s+)?([\w:*&<>, ]+?)\s+(\w+)\s*(?:=\s*[^;]+)?;")
            .unwrap();

    for cap in re.captures_iter(class_body) {
        let is_static = cap.get(1).is_some();
        let is_const = cap.get(2).is_some();
        let type_name = cap[3].trim().to_string();
        let name = cap[4].to_string();

        // Skip if it looks like a method declaration
        if type_name.contains('(') || name == "return" {
            continue;
        }

        fields.push(CppField {
            name,
            type_name,
            visibility: Visibility::Private,
            is_static,
            is_const,
        });
    }

    fields
}

fn extract_globals(source: &str) -> Vec<CppGlobal> {
    let mut globals = Vec::new();
    let re =
        Regex::new(r"(?m)^(extern\s+)?(const\s+)?([\w:*&<>, ]+?)\s+(\w+)\s*=\s*[^;]+;").unwrap();

    for cap in re.captures_iter(source) {
        let is_extern = cap.get(1).is_some();
        let is_const = cap.get(2).is_some();
        let type_name = cap[3].trim().to_string();
        let name = cap[4].to_string();

        // Skip if inside a function (rough heuristic: not indented)
        if type_name.contains('(') {
            continue;
        }

        globals.push(CppGlobal {
            name,
            type_name,
            is_const,
            is_extern,
        });
    }

    globals
}

fn parse_params(params_str: &str) -> Vec<CppParam> {
    let trimmed = params_str.trim();
    if trimmed.is_empty() || trimmed == "void" {
        return Vec::new();
    }

    trimmed
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() {
                return None;
            }

            let (param, default_value) = if let Some(idx) = p.find('=') {
                (p[..idx].trim(), Some(p[idx + 1..].trim().to_string()))
            } else {
                (p, None)
            };

            let is_const = param.contains("const ");
            let is_reference = param.contains('&');
            let is_pointer = param.contains('*');

            let parts: Vec<&str> = param.split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }

            let (type_name, name) = if parts.len() == 1 {
                (parts[0].to_string(), String::new())
            } else {
                let name = parts
                    .last()
                    .unwrap()
                    .trim_start_matches('*')
                    .trim_start_matches('&')
                    .to_string();
                let type_parts: Vec<&str> = parts[..parts.len() - 1].to_vec();
                (type_parts.join(" "), name)
            };

            Some(CppParam {
                name,
                type_name,
                default_value,
                is_const,
                is_reference,
                is_pointer,
            })
        })
        .collect()
}

fn parse_template_params(template_str: &str) -> Vec<String> {
    let re = Regex::new(r"<(.*)>").unwrap();
    re.captures(template_str)
        .map(|cap| cap[1].split(',').map(|p| p.trim().to_string()).collect())
        .unwrap_or_default()
}

fn extract_brace_block(source: &str, open_brace_pos: usize) -> Option<String> {
    let bytes = source.as_bytes();
    if open_brace_pos >= bytes.len() || bytes[open_brace_pos] != b'{' {
        return None;
    }

    let mut depth = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut prev = 0u8;

    for i in open_brace_pos..bytes.len() {
        let ch = bytes[i];

        if in_line_comment {
            if ch == b'\n' {
                in_line_comment = false;
            }
            prev = ch;
            continue;
        }

        if in_block_comment {
            if prev == b'*' && ch == b'/' {
                in_block_comment = false;
            }
            prev = ch;
            continue;
        }

        if in_string {
            if ch == b'"' && prev != b'\\' {
                in_string = false;
            }
            prev = ch;
            continue;
        }

        if in_char {
            if ch == b'\'' && prev != b'\\' {
                in_char = false;
            }
            prev = ch;
            continue;
        }

        match ch {
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                in_line_comment = true;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                in_block_comment = true;
            }
            b'"' => in_string = true,
            b'\'' => in_char = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(source[open_brace_pos + 1..i].to_string());
                }
            }
            _ => {}
        }

        prev = ch;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("main.c")), Language::C);
        assert_eq!(detect_language(Path::new("main.cpp")), Language::Cpp);
        assert_eq!(detect_language(Path::new("util.h")), Language::C);
        assert_eq!(detect_language(Path::new("util.hpp")), Language::Cpp);
    }

    #[test]
    fn test_detect_standard_c99() {
        let sources = vec![("for (int i = 0; i < 10; i++) {}", Language::C)];
        assert_eq!(detect_standard(&sources), CppStandard::C99);
    }

    #[test]
    fn test_detect_standard_c11() {
        let sources = vec![("_Atomic int counter = 0;", Language::C)];
        assert_eq!(detect_standard(&sources), CppStandard::C11);
    }

    #[test]
    fn test_detect_standard_cpp17() {
        let sources = vec![(
            "std::optional<int> x; if constexpr (true) {}",
            Language::Cpp,
        )];
        assert_eq!(detect_standard(&sources), CppStandard::Cpp17);
    }

    #[test]
    fn test_detect_standard_cpp11() {
        let sources = vec![("auto x = std::move(y); nullptr;", Language::Cpp)];
        assert_eq!(detect_standard(&sources), CppStandard::Cpp11);
    }

    #[test]
    fn test_extract_structs() {
        let source = r#"
struct Point {
    int x;
    int y;
};
"#;
        let structs = extract_structs(source);
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Point");
        assert_eq!(structs[0].fields.len(), 2);
    }

    #[test]
    fn test_extract_typedef_struct() {
        let source = r#"
typedef struct {
    char name[64];
    int age;
} Person;
"#;
        let structs = extract_structs(source);
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Person");
        assert!(structs[0].is_typedef);
    }

    #[test]
    fn test_extract_brace_block() {
        let source = "{ int x = 1; { int y = 2; } }";
        let block = extract_brace_block(source, 0);
        assert_eq!(block, Some(" int x = 1; { int y = 2; } ".to_string()));
    }

    #[test]
    fn test_parse_params() {
        let params = parse_params("int x, const char* name, double val = 3.14");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "x");
        assert_eq!(params[1].name, "name");
        assert!(params[1].is_const);
        assert!(params[1].is_pointer);
        assert_eq!(params[2].default_value.as_deref(), Some("3.14"));
    }

    #[test]
    fn test_analyze_file_c() {
        let source = r#"
#include <stdio.h>
#define MAX_SIZE 100

struct Point {
    int x;
    int y;
};

int add(int a, int b) {
    return a + b;
}
"#;
        let file = analyze_file(Path::new("test.c"), source);
        assert_eq!(file.language, Language::C);
        assert_eq!(file.includes, vec!["stdio.h"]);
        assert!(!file.structs.is_empty());
        assert!(!file.functions.is_empty());
    }

    #[test]
    fn test_analyze_project() {
        let dir = std::env::temp_dir().join("cpp_to_rust_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("main.c"),
            "#include <stdio.h>\nint main() { return 0; }\n",
        )
        .unwrap();

        let project = analyze_project(&dir).unwrap();
        assert_eq!(project.language, Language::C);
        assert_eq!(project.files.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
