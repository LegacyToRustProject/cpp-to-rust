use std::collections::HashMap;

use cpp_parser::types::{CppClass, CppFile, CppFunction, CppStruct, Language};

pub struct ConversionProfile {
    pub name: String,
    pub additional_instructions: String,
    pub api_mappings: HashMap<String, String>,
    pub type_mappings: HashMap<String, String>,
}

impl Default for ConversionProfile {
    fn default() -> Self {
        Self {
            name: "generic".to_string(),
            additional_instructions: String::new(),
            api_mappings: HashMap::new(),
            type_mappings: HashMap::new(),
        }
    }
}

pub fn build_system_prompt(profile: &ConversionProfile, language: &Language) -> String {
    let lang_name = match language {
        Language::C => "C",
        Language::Cpp => "C++",
    };

    let mut prompt = format!(
        r#"You are an expert {lang_name} to Rust conversion engineer.

Your task is to convert {lang_name} code into idiomatic, safe Rust code that preserves the same behavior.

## Core Conversion Rules

1. **Memory Safety First**: Avoid `unsafe` blocks unless absolutely necessary. Convert manual memory management to Rust's ownership system.
2. **Ownership & Borrowing**: Infer correct ownership patterns:
   - `malloc`/`new` allocations → `Vec`, `Box`, `String`, or stack allocation
   - `free`/`delete` → automatic RAII (drop)
   - Raw pointers → references (`&`, `&mut`) or smart pointers (`Box`, `Rc`, `Arc`)
   - Output parameters → return tuples or structs
3. **Error Handling**: Convert error codes and NULL checks to `Result<T, E>` and `Option<T>`
4. **Type Conversions**:
   - `char*` / `const char*` → `String` / `&str`
   - `int`, `long` → `i32`, `i64` (or appropriate sized types)
   - `unsigned` types → `u32`, `u64`, etc.
   - `size_t` → `usize`
   - `bool` / `_Bool` → `bool`
   - `void*` → generics or `Box<dyn Any>` (avoid if possible)
   - C arrays → `Vec<T>` or `[T; N]`
5. **Structs & Enums**: Convert C structs to Rust structs with proper derives (`Debug`, `Clone`, etc.)
6. **Collections**: Use `Vec`, `HashMap`, `HashSet` instead of manual linked lists or hash tables
7. **String Handling**: Use `String` and `&str` instead of `char*` with manual length tracking
8. **Output Format**: Return ONLY the Rust code wrapped in ```rust code blocks. Include TODO comments for anything that cannot be directly converted.
"#
    );

    if *language == Language::Cpp {
        prompt.push_str(
            r#"
## C++ Specific Rules

- **Classes** → Rust structs + `impl` blocks. Use traits for polymorphism.
- **Inheritance** → Trait objects (`dyn Trait`) or enum dispatch. Prefer composition over inheritance.
- **Templates** → Rust generics with trait bounds
- **Virtual methods** → `dyn Trait` or enum dispatch
- **Operator overloading** → Implement `std::ops` traits
- **RAII** → Implement `Drop` trait where needed
- **Exceptions** → `Result<T, E>` with custom error types
- **`std::string`** → `String`
- **`std::vector`** → `Vec<T>`
- **`std::map`** → `HashMap<K, V>` or `BTreeMap<K, V>`
- **`std::unique_ptr`** → `Box<T>`
- **`std::shared_ptr`** → `Arc<T>` or `Rc<T>`
- **`std::optional`** → `Option<T>`
- **Move semantics** → Rust moves by default; explicit `Clone` where needed
- **Namespaces** → Rust modules
"#,
        );
    }

    if !profile.additional_instructions.is_empty() {
        prompt.push_str(&format!(
            "\n## Profile-Specific Instructions ({})\n\n{}\n",
            profile.name, profile.additional_instructions
        ));
    }

    if !profile.type_mappings.is_empty() {
        prompt.push_str("\n## Type Mappings\n\n");
        for (from, to) in &profile.type_mappings {
            prompt.push_str(&format!("- `{}` → `{}`\n", from, to));
        }
    }

    if !profile.api_mappings.is_empty() {
        prompt.push_str("\n## API Mappings\n\n");
        for (from, to) in &profile.api_mappings {
            prompt.push_str(&format!("- `{}` → `{}`\n", from, to));
        }
    }

    prompt
}

pub fn build_file_prompt(file: &CppFile) -> String {
    let mut prompt = format!("Convert the following {} file to Rust:\n\n", file.language);

    if !file.includes.is_empty() {
        prompt.push_str("Dependencies/includes:\n");
        for inc in &file.includes {
            prompt.push_str(&format!("- {}\n", inc));
        }
        prompt.push('\n');
    }

    prompt.push_str(&format!("```{}\n{}\n```\n", file.language, file.source));
    prompt
}

pub fn build_function_prompt(func: &CppFunction, context: &str) -> String {
    let mut prompt = String::new();

    if !context.is_empty() {
        prompt.push_str(&format!("Context:\n{}\n\n", context));
    }

    prompt.push_str("Convert this function to Rust:\n\n```c\n");

    if func.is_template {
        prompt.push_str(&format!("template<{}>\n", func.template_params.join(", ")));
    }
    if func.is_static {
        prompt.push_str("static ");
    }
    if func.is_virtual {
        prompt.push_str("virtual ");
    }

    prompt.push_str(&format!("{} {}(", func.return_type, func.name));
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let mut s = String::new();
            if p.is_const {
                s.push_str("const ");
            }
            s.push_str(&p.type_name);
            if p.is_pointer {
                s.push('*');
            }
            if p.is_reference {
                s.push('&');
            }
            s.push(' ');
            s.push_str(&p.name);
            if let Some(ref default) = p.default_value {
                s.push_str(&format!(" = {}", default));
            }
            s
        })
        .collect();
    prompt.push_str(&params.join(", "));
    prompt.push_str(") {\n");
    prompt.push_str(&func.body);
    prompt.push_str("\n}\n```\n");

    prompt
}

pub fn build_struct_prompt(s: &CppStruct) -> String {
    let mut prompt = String::from("Convert this C struct to Rust:\n\n```c\n");
    if s.is_typedef {
        prompt.push_str("typedef ");
    }
    prompt.push_str(&format!("struct {} {{\n", s.name));
    for field in &s.fields {
        if field.is_const {
            prompt.push_str("    const ");
        } else {
            prompt.push_str("    ");
        }
        prompt.push_str(&format!("{} {};\n", field.type_name, field.name));
    }
    prompt.push_str("};\n```\n");
    prompt
}

pub fn build_class_prompt(class: &CppClass, context: &str) -> String {
    let mut prompt = String::new();

    if !context.is_empty() {
        prompt.push_str(&format!("Context:\n{}\n\n", context));
    }

    prompt.push_str(&format!(
        "Convert this C++ class to Rust (struct + impl + traits):\n\n```cpp\nclass {} ",
        class.name
    ));

    if !class.bases.is_empty() {
        prompt.push_str(": ");
        let bases: Vec<String> = class
            .bases
            .iter()
            .map(|b| {
                let mut s = String::new();
                if b.is_virtual {
                    s.push_str("virtual ");
                }
                match b.visibility {
                    cpp_parser::types::Visibility::Public => s.push_str("public "),
                    cpp_parser::types::Visibility::Protected => s.push_str("protected "),
                    cpp_parser::types::Visibility::Private => s.push_str("private "),
                }
                s.push_str(&b.name);
                s
            })
            .collect();
        prompt.push_str(&bases.join(", "));
        prompt.push(' ');
    }

    prompt.push_str("{\npublic:\n");
    for method in &class.methods {
        prompt.push_str(&format!(
            "    {} {}({});\n",
            method.return_type,
            method.name,
            method
                .params
                .iter()
                .map(|p| format!("{} {}", p.type_name, p.name))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    prompt.push_str("\nprivate:\n");
    for field in &class.fields {
        prompt.push_str(&format!("    {} {};\n", field.type_name, field.name));
    }
    prompt.push_str("};\n```\n");

    prompt
}

pub fn extract_rust_code(response: &str) -> Option<String> {
    let re = regex::Regex::new(r"```rust\n([\s\S]*?)```").unwrap();
    re.captures(response).map(|cap| cap[1].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt_c() {
        let profile = ConversionProfile::default();
        let prompt = build_system_prompt(&profile, &Language::C);
        assert!(prompt.contains("C to Rust"));
        assert!(prompt.contains("Memory Safety First"));
        assert!(!prompt.contains("C++ Specific Rules"));
    }

    #[test]
    fn test_build_system_prompt_cpp() {
        let profile = ConversionProfile::default();
        let prompt = build_system_prompt(&profile, &Language::Cpp);
        assert!(prompt.contains("C++ to Rust"));
        assert!(prompt.contains("C++ Specific Rules"));
        assert!(prompt.contains("Templates"));
    }

    #[test]
    fn test_build_system_prompt_with_profile() {
        let mut profile = ConversionProfile::default();
        profile.name = "embedded".to_string();
        profile.additional_instructions = "Use no_std where possible.".to_string();
        profile
            .type_mappings
            .insert("uint8_t".to_string(), "u8".to_string());

        let prompt = build_system_prompt(&profile, &Language::C);
        assert!(prompt.contains("Use no_std where possible"));
        assert!(prompt.contains("`uint8_t` → `u8`"));
    }

    #[test]
    fn test_extract_rust_code() {
        let response = r#"Here's the converted code:

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

This function takes two integers and returns their sum."#;

        let code = extract_rust_code(response).unwrap();
        assert_eq!(code, "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}");
    }

    #[test]
    fn test_extract_rust_code_none() {
        let response = "No code blocks here.";
        assert!(extract_rust_code(response).is_none());
    }
}
