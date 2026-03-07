use anyhow::Result;
use regex::Regex;

use crate::types::CppMacro;

pub fn extract_includes(source: &str) -> Vec<String> {
    let re = Regex::new(r#"#\s*include\s*[<"]([^>"]+)[>"]"#).unwrap();
    re.captures_iter(source)
        .map(|cap| cap[1].to_string())
        .collect()
}

pub fn extract_macros(source: &str) -> Vec<CppMacro> {
    let mut macros = Vec::new();

    // Object-like macros: #define NAME value
    let obj_re = Regex::new(r"(?m)^#\s*define\s+(\w+)[^\S\n]+(.+?)(?:\\\n.*)*$").unwrap();
    for cap in obj_re.captures_iter(source) {
        let name = cap[1].to_string();
        let body = cap[2].trim().to_string();
        // Skip include guards
        if name.ends_with("_H") || name.ends_with("_H_") || name.ends_with("_HPP") {
            continue;
        }
        macros.push(CppMacro {
            name,
            params: None,
            body,
            is_conditional: false,
        });
    }

    // Function-like macros: #define NAME(args) body
    let func_re = Regex::new(r"(?m)^#\s*define\s+(\w+)\(([^)]*)\)\s+(.+?)(?:\\\n.*)*$").unwrap();
    for cap in func_re.captures_iter(source) {
        let name = cap[1].to_string();
        let params: Vec<String> = cap[2].split(',').map(|s| s.trim().to_string()).collect();
        let body = cap[3].trim().to_string();
        // Remove duplicate from object-like pass
        macros.retain(|m| m.name != name);
        macros.push(CppMacro {
            name,
            params: Some(params),
            body,
            is_conditional: false,
        });
    }

    // Conditional compilation macros
    let cond_re = Regex::new(r"(?m)^#\s*(ifdef|ifndef|if)\s+(.+)$").unwrap();
    for cap in cond_re.captures_iter(source) {
        let condition = cap[2].trim().to_string();
        macros.push(CppMacro {
            name: condition,
            params: None,
            body: String::new(),
            is_conditional: true,
        });
    }

    macros
}

pub fn resolve_includes(source: &str, include_paths: &[String]) -> Result<String> {
    // For now, return the source as-is. Full include resolution would require
    // reading files from include paths, which is complex and not needed for
    // AI-based conversion (we pass the source directly to the LLM).
    let _ = include_paths;
    Ok(source.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_includes() {
        let source = r#"
#include <stdio.h>
#include <stdlib.h>
#include "myheader.h"
"#;
        let includes = extract_includes(source);
        assert_eq!(includes, vec!["stdio.h", "stdlib.h", "myheader.h"]);
    }

    #[test]
    fn test_extract_macros_object() {
        let source = "#define BUFFER_SIZE 1024\n#define MAX_NAME 256\n";
        let macros = extract_macros(source);
        let non_cond: Vec<_> = macros.iter().filter(|m| !m.is_conditional).collect();
        assert_eq!(non_cond.len(), 2);
        assert_eq!(non_cond[0].name, "BUFFER_SIZE");
        assert_eq!(non_cond[0].body, "1024");
        assert!(non_cond[0].params.is_none());
    }

    #[test]
    fn test_extract_macros_function() {
        let source = "#define MAX(a, b) ((a) > (b) ? (a) : (b))\n";
        let macros = extract_macros(source);
        let func_macros: Vec<_> = macros.iter().filter(|m| m.params.is_some()).collect();
        assert_eq!(func_macros.len(), 1);
        assert_eq!(func_macros[0].name, "MAX");
        assert_eq!(
            func_macros[0].params.as_ref().unwrap(),
            &vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn test_extract_conditional_macros() {
        let source = "#ifdef DEBUG\n#define LOG(msg) printf(msg)\n#endif\n";
        let macros = extract_macros(source);
        let cond: Vec<_> = macros.iter().filter(|m| m.is_conditional).collect();
        assert_eq!(cond.len(), 1);
        assert_eq!(cond[0].name, "DEBUG");
    }

    #[test]
    fn test_skip_include_guards() {
        let source = "#define MY_HEADER_H\n#define BUFFER_SIZE 42\n";
        let macros = extract_macros(source);
        let non_cond: Vec<_> = macros.iter().filter(|m| !m.is_conditional).collect();
        assert_eq!(non_cond.len(), 1);
        assert_eq!(non_cond[0].name, "BUFFER_SIZE");
    }
}
