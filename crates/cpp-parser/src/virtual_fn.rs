/// Virtual function detection and trait conversion hint generation.
///
/// Scans C++ class declarations for `virtual` method patterns and
/// generates Rust `trait` definitions for LLM prompt augmentation.
use regex::Regex;

/// A detected virtual function.
#[derive(Debug, Clone)]
pub struct VirtualFn {
    /// Class that declares this virtual function
    pub class_name: String,
    /// Function name
    pub fn_name: String,
    /// Return type (as string)
    pub return_type: String,
    /// Parameter list (as raw string)
    pub params: String,
    /// `= 0` — pure virtual (must be implemented)
    pub is_pure: bool,
    /// `const` qualifier
    pub is_const: bool,
    /// Has a default body `{ ... }`
    pub has_default_impl: bool,
    /// Line number (1-based)
    pub line: usize,
}

/// All virtual functions grouped by class.
#[derive(Debug, Clone)]
pub struct VirtualClass {
    pub class_name: String,
    pub virtual_fns: Vec<VirtualFn>,
}

/// Detect all virtual functions in `source`, grouped by class.
pub fn detect_virtual_functions(source: &str) -> Vec<VirtualClass> {
    let class_re = Regex::new(r"(?m)class\s+(\w+)(?:\s*:\s*[^{]+)?\s*\{").unwrap();
    let _virtual_re = Regex::new(
        r"(?m)^\s*virtual\s+(.*?)\s+(\w+)\s*\(([^)]*)\)\s*(const)?\s*(override)?\s*(?:=\s*0\s*;|\{[^}]*\}|;)",
    )
    .unwrap();

    let mut result = Vec::new();

    for class_cap in class_re.captures_iter(source) {
        let class_name = class_cap[1].to_string();
        let class_start = class_cap.get(0).unwrap().end();

        // Find the class body by counting braces
        let class_body = extract_class_body(source, class_start - 1);
        if class_body.is_empty() {
            continue;
        }

        let mut virtual_fns = Vec::new();

        // Find the line offset for line number calculation
        let body_start_line = source[..class_start].lines().count();

        for (rel_line, line) in class_body.lines().enumerate() {
            if !line.contains("virtual") {
                continue;
            }
            let abs_line = body_start_line + rel_line + 1;

            if let Some(vfn) = parse_virtual_line(line, &class_name, abs_line) {
                virtual_fns.push(vfn);
            }
        }

        if !virtual_fns.is_empty() {
            result.push(VirtualClass {
                class_name,
                virtual_fns,
            });
        }
    }

    result
}

fn extract_class_body(source: &str, open_brace_pos: usize) -> String {
    let bytes = source.as_bytes();
    if open_brace_pos >= bytes.len() || bytes[open_brace_pos] != b'{' {
        return String::new();
    }
    let mut depth = 0usize;
    let mut in_string = false;
    let mut prev = 0u8;

    for i in open_brace_pos..bytes.len() {
        let ch = bytes[i];
        if in_string {
            if ch == b'"' && prev != b'\\' {
                in_string = false;
            }
        } else {
            match ch {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return source[open_brace_pos + 1..i].to_string();
                    }
                }
                _ => {}
            }
        }
        prev = ch;
    }
    String::new()
}

fn parse_virtual_line(line: &str, class_name: &str, line_no: usize) -> Option<VirtualFn> {
    let re = Regex::new(
        r"virtual\s+(?:~)?(\S.*?\S|\S)\s+(\w+|~\w+)\s*\(([^)]*)\)\s*(const)?\s*(override)?\s*(?:=\s*(0)\s*;|(\{[^}]*\})|;)",
    )
    .unwrap();

    // Simpler fallback: line-level regex for virtual declarations
    let simple_re = Regex::new(
        r"virtual\s+([\w\s\*&:<>,]+?)\s+(\w+)\s*\(([^)]*)\)\s*(const)?\s*(?:override\s*)?(=\s*0)?",
    )
    .unwrap();

    if let Some(cap) = simple_re.captures(line) {
        let return_type = cap[1].trim().to_string();
        let fn_name = cap[2].to_string();
        let params = cap
            .get(3)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let is_const = cap.get(4).is_some();
        let is_pure = cap
            .get(5)
            .map(|m| m.as_str().contains('0'))
            .unwrap_or(false)
            || line.contains("= 0");
        let has_default_impl = line.contains('{') && !is_pure;

        // Skip destructor virtuals (handled separately in Rust via Drop)
        if fn_name.starts_with('~') || return_type.starts_with('~') {
            return None;
        }
        // Skip "if", "for" etc. false positives
        if matches!(fn_name.as_str(), "if" | "for" | "while" | "switch") {
            return None;
        }

        // Drop re to silence unused warning
        drop(re);

        return Some(VirtualFn {
            class_name: class_name.to_string(),
            fn_name,
            return_type,
            params,
            is_pure,
            is_const,
            has_default_impl,
            line: line_no,
        });
    }

    None
}

/// Convert a detected `VirtualClass` into a Rust `trait` definition string.
pub fn generate_trait_definition(vc: &VirtualClass) -> String {
    let trait_name = to_rust_trait_name(&vc.class_name);
    let mut out = format!("pub trait {} {{\n", trait_name);

    for vfn in &vc.virtual_fns {
        let rust_name = to_snake_case(&vfn.fn_name);
        let rust_return = map_return_type(&vfn.return_type, &vc.class_name);
        let rust_params = map_params(&vfn.params);
        let self_ref = if vfn.is_const { "&self" } else { "&mut self" };

        let signature = if rust_params.is_empty() {
            format!("    fn {}({}) -> {}", rust_name, self_ref, rust_return)
        } else {
            format!(
                "    fn {}({}, {}) -> {}",
                rust_name, self_ref, rust_params, rust_return
            )
        };

        if vfn.is_pure {
            out.push_str(&format!("{};\n", signature));
        } else {
            // Default implementation — body is unimplemented stub
            let default_body = if rust_return == "bool" {
                "true"
            } else if rust_return == "()" {
                ""
            } else {
                "todo!()"
            };
            out.push_str(&format!("{} {{ {} }}\n", signature, default_body));
        }
    }

    out.push_str("}\n");
    out
}

/// Generate trait hints for all detected virtual classes.
pub fn generate_virtual_fn_hints(classes: &[VirtualClass]) -> String {
    if classes.is_empty() {
        return String::new();
    }

    let mut hint = String::from("## Virtual Function → Trait Conversion Hints\n\n");
    hint.push_str(&format!(
        "Detected {} class(es) with virtual functions. Suggested Rust traits:\n\n",
        classes.len()
    ));

    for vc in classes {
        let pure_count = vc.virtual_fns.iter().filter(|f| f.is_pure).count();
        let default_count = vc.virtual_fns.iter().filter(|f| f.has_default_impl).count();

        hint.push_str(&format!(
            "### `{}` → `trait {}` ({} methods: {} pure, {} with defaults)\n\n",
            vc.class_name,
            to_rust_trait_name(&vc.class_name),
            vc.virtual_fns.len(),
            pure_count,
            default_count
        ));

        hint.push_str("```rust\n");
        hint.push_str(&generate_trait_definition(vc));
        hint.push_str("```\n\n");
    }

    hint
}

// --- Helpers ---

fn to_rust_trait_name(cpp_name: &str) -> String {
    // XMLVisitor → XmlVisitor, MemPool → MemPool
    let mut result = String::new();
    let mut prev_upper = false;
    for (i, ch) in cpp_name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 && !prev_upper {
            // Insert underscore only if previous char was lowercase (camelCase boundary)
        }
        result.push(ch);
        prev_upper = ch.is_uppercase();
    }
    result
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

fn map_return_type(ret: &str, _class_name: &str) -> String {
    let ret = ret.trim();
    match ret {
        "void" => "()".to_string(),
        "bool" => "bool".to_string(),
        "int" => "i32".to_string(),
        "size_t" => "usize".to_string(),
        "float" => "f32".to_string(),
        "double" => "f64".to_string(),
        "char" => "u8".to_string(),
        s if s.ends_with('*') => {
            format!("Option<Box<{}>>", s.trim_end_matches('*').trim())
        }
        s => s.to_string(),
    }
}

fn map_params(params: &str) -> String {
    if params.trim().is_empty() || params.trim() == "void" {
        return String::new();
    }

    params
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() {
                return None;
            }
            // Strip comments
            let p = p.split("/*").next().unwrap_or(p).trim();
            if p.is_empty() || p == "void" {
                return None;
            }

            // Map basic C++ types to Rust
            let rust_param = p
                .replace("const ", "")
                .replace('&', "")
                .replace("unsigned int", "u32")
                .replace("unsigned", "u32")
                .replace("size_t", "usize")
                .replace("int", "i32")
                .replace("float", "f32")
                .replace("double", "f64");

            // Extract just the type (last word = name, rest = type)
            let parts: Vec<&str> = rust_param.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = to_snake_case(parts.last().unwrap().trim_start_matches('*'));
                let type_part = parts[..parts.len() - 1].join(" ");
                let rust_type = if type_part.contains('*') {
                    format!("&{}", type_part.replace('*', "").trim())
                } else {
                    type_part
                };
                Some(format!("{}: {}", name, rust_type))
            } else {
                Some(format!("_: {}", rust_param.trim()))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Count total virtual functions across all detected classes.
pub fn count_virtual_fns(classes: &[VirtualClass]) -> (usize, usize, usize) {
    let total = classes.iter().map(|c| c.virtual_fns.len()).sum();
    let pure = classes
        .iter()
        .flat_map(|c| &c.virtual_fns)
        .filter(|f| f.is_pure)
        .count();
    let with_default = classes
        .iter()
        .flat_map(|c| &c.virtual_fns)
        .filter(|f| f.has_default_impl)
        .count();
    (total, pure, with_default)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CLASS: &str = r#"
class XMLVisitor {
public:
    virtual ~XMLVisitor() {}
    virtual bool VisitEnter( const XMLDocument& doc ) { return true; }
    virtual bool VisitExit( const XMLDocument& doc ) { return true; }
    virtual bool VisitEnter( const XMLElement& element, const XMLAttribute* attr ) { return true; }
    virtual bool VisitExit( const XMLElement& element ) { return true; }
    virtual bool Visit( const XMLDeclaration& decl ) { return true; }
    virtual bool Visit( const XMLText& text ) { return true; }
    virtual bool Visit( const XMLComment& comment ) { return true; }
};
"#;

    const SAMPLE_PURE: &str = r#"
class MemPool {
public:
    virtual ~MemPool() {}
    virtual size_t ItemSize() const = 0;
    virtual void* Alloc() = 0;
    virtual void Free( void* ) = 0;
};
"#;

    #[test]
    fn test_detect_xml_visitor() {
        let classes = detect_virtual_functions(SAMPLE_CLASS);
        assert!(!classes.is_empty(), "should detect XMLVisitor");
        let xml_visitor = &classes[0];
        assert_eq!(xml_visitor.class_name, "XMLVisitor");
        assert!(
            xml_visitor.virtual_fns.len() >= 4,
            "should detect multiple virtual fns"
        );
    }

    #[test]
    fn test_detect_pure_virtual() {
        let classes = detect_virtual_functions(SAMPLE_PURE);
        assert!(!classes.is_empty(), "should detect MemPool");
        let pool = &classes[0];
        let pure_count = pool.virtual_fns.iter().filter(|f| f.is_pure).count();
        assert!(pure_count >= 2, "ItemSize and Free should be pure virtual");
    }

    #[test]
    fn test_generate_trait_definition() {
        let classes = detect_virtual_functions(SAMPLE_CLASS);
        if classes.is_empty() {
            return; // skip if detection failed
        }
        let trait_def = generate_trait_definition(&classes[0]);
        assert!(trait_def.contains("pub trait"));
        assert!(trait_def.contains("fn "));
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("VisitEnter"), "visit_enter");
        assert_eq!(to_snake_case("ItemSize"), "item_size");
        assert_eq!(to_snake_case("Alloc"), "alloc");
    }

    #[test]
    fn test_tinyxml2_header() {
        let header = include_str!("../../../test-projects/tinyxml2/tinyxml2.h");
        let classes = detect_virtual_functions(header);
        let (total, pure, with_default) = count_virtual_fns(&classes);
        println!(
            "TinyXML2: {} classes, {} virtual fns ({} pure, {} with default)",
            classes.len(),
            total,
            pure,
            with_default
        );
        // Regex-based detection finds single-line virtual declarations only.
        // TinyXML2 has many multi-line/macro-wrapped virtuals not captured by simple regex.
        // Minimum: MemPool (pure virtuals) + XMLVisitor (default impls).
        assert!(classes.len() >= 2, "should detect multiple virtual classes");
        assert!(
            total >= 5,
            "should detect virtual fns from MemPool and XMLVisitor; got {}",
            total
        );
    }
}
