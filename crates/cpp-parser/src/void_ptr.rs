/// Void pointer and function pointer pattern detection.
///
/// Classifies `void*` usage into semantic categories and generates
/// Rust conversion hints for use in LLM prompts.
use regex::Regex;

/// Semantic category of a `void*` occurrence.
#[derive(Debug, Clone, PartialEq)]
pub enum VoidPtrKind {
    /// `void *user_data` / `void *data` — opaque context passed through callbacks
    UserData,
    /// `const void *src` / `const void *s` — read-only input buffer
    InputBuffer,
    /// `void *dest` — write output buffer
    OutputBuffer,
    /// `void *(*malloc_func)(size_t)` / `void *(*realloc_func)(void*, size_t)` — allocator fn ptr
    AllocatorFnPtr,
    /// `void (*free_func)(void*)` — deallocator fn ptr
    DeallocatorFnPtr,
    /// `void (*cb)(void*, size_t, void*)` — callback with user_data
    CallbackFnPtr,
    /// Other / unclassified usage
    Other,
}

/// A detected void* or function-pointer occurrence.
#[derive(Debug, Clone)]
pub struct VoidPtrPattern {
    /// Source context (the function/param declaration)
    pub context: String,
    /// Line number (1-based)
    pub line: usize,
    /// Semantic category
    pub kind: VoidPtrKind,
    /// Suggested Rust type replacement
    pub rust_suggestion: String,
}

/// Detect all void* and function-pointer patterns in `source`.
pub fn detect_void_ptr_patterns(source: &str) -> Vec<VoidPtrPattern> {
    let mut patterns = Vec::new();

    // Regex: `void *(*name)(...)` — allocator-style fn ptr returning void*
    let alloc_re = Regex::new(r"void\s*\*\s*\(\s*\*\s*(\w+)\s*\)\s*\(([^)]*)\)").unwrap();
    // Regex: `void (*name)(...)` — callback or deallocator fn ptr returning void
    let void_fn_re = Regex::new(r"void\s+\(\s*\*\s*(\w+)\s*\)\s*\(([^)]*)\)").unwrap();
    // Regex: plain `void *name` or `const void *name` parameter
    let void_param_re = Regex::new(r"(?:const\s+)?void\s*\*\s*(\w+)").unwrap();

    for (line_idx, line) in source.lines().enumerate() {
        let line_no = line_idx + 1;

        // --- Allocator fn ptrs: void *(*f)(...) ---
        for cap in alloc_re.captures_iter(line) {
            let name = &cap[1];
            let params = &cap[2];
            let (kind, rust_suggestion) = classify_allocator_fn(name, params);
            patterns.push(VoidPtrPattern {
                context: line.trim().to_string(),
                line: line_no,
                kind,
                rust_suggestion,
            });
        }

        // --- Callback / deallocator fn ptrs: void (*f)(...) ---
        for cap in void_fn_re.captures_iter(line) {
            let name = &cap[1];
            let params = &cap[2];
            let (kind, rust_suggestion) = classify_void_fn(name, params);
            patterns.push(VoidPtrPattern {
                context: line.trim().to_string(),
                line: line_no,
                kind,
                rust_suggestion,
            });
        }

        // --- Plain void* parameters ---
        // Strip fn-pointer sub-expressions so we don't double-count their
        // internal `void *` params (e.g. the `void *` inside `void (*cb)(void *, ...)`).
        let stripped = alloc_re.replace_all(line, " __FN_PTR__ ");
        let stripped = void_fn_re.replace_all(&stripped, " __FN_PTR__ ");
        for cap in void_param_re.captures_iter(&stripped) {
            let name = &cap[1];
            // Skip `void` standalone (function returning void) and placeholder
            if name == "void" || name.is_empty() || name == "__FN_PTR__" {
                continue;
            }
            {
                let is_const = line.contains("const void");
                let (kind, rust_suggestion) = classify_void_param(name, is_const, line);
                patterns.push(VoidPtrPattern {
                    context: line.trim().to_string(),
                    line: line_no,
                    kind,
                    rust_suggestion,
                });
            }
        }
    }

    patterns
}

fn classify_allocator_fn(name: &str, params: &str) -> (VoidPtrKind, String) {
    // void *(*realloc_func)(void *, size_t) → unsafe: Box::into_raw(Box::new(...))
    // Better: use custom allocator trait or Vec::with_capacity
    let param_count = params.split(',').count();
    if name.contains("realloc") || (param_count == 2 && params.contains("size_t")) {
        (
            VoidPtrKind::AllocatorFnPtr,
            "// TODO: replace with custom Allocator trait or Vec::resize".to_string(),
        )
    } else {
        (
            VoidPtrKind::AllocatorFnPtr,
            "// TODO: replace with Box<T> or Vec<T>".to_string(),
        )
    }
}

fn classify_void_fn(name: &str, params: &str) -> (VoidPtrKind, String) {
    let param_count = params.split(',').filter(|p| !p.trim().is_empty()).count();

    if name.contains("free") || (param_count == 1 && params.trim() == "void *") {
        // void (*free_func)(void*) → deallocator, not needed in Rust (RAII)
        return (
            VoidPtrKind::DeallocatorFnPtr,
            "// Rust RAII: Drop trait replaces free_func".to_string(),
        );
    }

    // Check if it looks like a callback with user_data (last param is void*)
    let last_param = params.split(',').next_back().unwrap_or("").trim();
    if last_param.starts_with("void") || last_param == "void *" {
        // void (*cb1)(void *, size_t, void *) → field callback
        // Suggest generic parameter approach
        let rust_type = build_callback_rust_type(params);
        return (VoidPtrKind::CallbackFnPtr, rust_type);
    }

    (
        VoidPtrKind::CallbackFnPtr,
        format!(
            "impl FnMut({}) + 'static",
            params
                .split(',')
                .map(|_| "_")
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
}

fn build_callback_rust_type(params: &str) -> String {
    let parts: Vec<&str> = params.split(',').map(|p| p.trim()).collect();

    // Pattern: void*, size_t, void* → field data callback
    if parts.len() == 3 {
        let has_size_t = parts[1].contains("size_t") || parts[1].contains("size");
        let last_is_void = parts[2].starts_with("void");
        if last_is_void && has_size_t {
            return "impl FnMut(&[u8], &mut T) where T: user_data_type".to_string();
        }
    }
    // Pattern: int, void* → row terminator callback
    if parts.len() == 2 {
        let last_is_void = parts[1].starts_with("void");
        if last_is_void {
            return "impl FnMut(u8, &mut T) where T: user_data_type".to_string();
        }
    }

    "Box<dyn FnMut(...)>".to_string()
}

fn classify_void_param(name: &str, is_const: bool, _line: &str) -> (VoidPtrKind, String) {
    let lower_name = name.to_lowercase();

    // user_data / data / context / userdata → generic T
    if matches!(
        lower_name.as_str(),
        "data" | "user_data" | "userdata" | "context" | "ctx" | "arg" | "args" | "handle"
    ) {
        return (
            VoidPtrKind::UserData,
            format!("<T> user_data: &{}T", if is_const { "" } else { "mut " }),
        );
    }

    // const void* src/s/buf/input → &[u8]
    if is_const
        || matches!(
            lower_name.as_str(),
            "s" | "src" | "source" | "buf" | "buffer" | "input" | "in" | "data"
        )
    {
        return (VoidPtrKind::InputBuffer, "&[u8]".to_string());
    }

    // void* dest/dst/out → &mut [u8]
    if matches!(
        lower_name.as_str(),
        "dest" | "dst" | "out" | "output" | "buf" | "buffer"
    ) {
        return (VoidPtrKind::OutputBuffer, "&mut [u8]".to_string());
    }

    (VoidPtrKind::Other, "Box<dyn std::any::Any>".to_string())
}

/// Generate a hint string for inclusion in LLM prompts.
pub fn generate_void_ptr_hints(patterns: &[VoidPtrPattern]) -> String {
    if patterns.is_empty() {
        return String::new();
    }

    let mut hint = String::from("## Void Pointer Conversion Hints\n\n");
    hint.push_str("The following `void*` patterns were detected and should be converted:\n\n");

    let user_data: Vec<_> = patterns
        .iter()
        .filter(|p| p.kind == VoidPtrKind::UserData)
        .collect();
    let callbacks: Vec<_> = patterns
        .iter()
        .filter(|p| p.kind == VoidPtrKind::CallbackFnPtr)
        .collect();
    let allocs: Vec<_> = patterns
        .iter()
        .filter(|p| {
            matches!(
                p.kind,
                VoidPtrKind::AllocatorFnPtr | VoidPtrKind::DeallocatorFnPtr
            )
        })
        .collect();
    let buffers: Vec<_> = patterns
        .iter()
        .filter(|p| matches!(p.kind, VoidPtrKind::InputBuffer | VoidPtrKind::OutputBuffer))
        .collect();

    if !user_data.is_empty() {
        hint.push_str(&format!(
            "### User Data (`void *data`) — {} occurrences\n\
             Use a generic type parameter `<T>` and pass `&mut T` through callbacks.\n\
             This eliminates the void* completely and provides type safety.\n\n",
            user_data.len()
        ));
    }

    if !callbacks.is_empty() {
        hint.push_str(&format!(
            "### Callback Function Pointers — {} occurrences\n\
             Convert `void (*cb)(void*, size_t, void*)` to:\n\
             ```rust\n\
             cb1: impl FnMut(&[u8], &mut T)\n\
             cb2: impl FnMut(u8, &mut T)\n\
             ```\n\
             This makes the API generic over the user data type `T`.\n\n",
            callbacks.len()
        ));
    }

    if !allocs.is_empty() {
        hint.push_str(&format!(
            "### Allocator Function Pointers — {} occurrences\n\
             Replace with `Vec<u8>` for buffer management. Rust's allocator handles resizing.\n\
             `realloc_func` → `Vec::resize` or `Vec::extend`\n\
             `free_func` → automatic via Drop (no explicit call needed)\n\n",
            allocs.len()
        ));
    }

    if !buffers.is_empty() {
        hint.push_str(&format!(
            "### Input/Output Buffers — {} occurrences\n\
             `const void *s` → `&[u8]`\n\
             `void *dest` → `&mut [u8]` or return a `Vec<u8>`\n\n",
            buffers.len()
        ));
    }

    hint
}

/// Count void* patterns by kind.
pub fn count_patterns(patterns: &[VoidPtrPattern]) -> (usize, usize, usize, usize) {
    let user_data = patterns
        .iter()
        .filter(|p| p.kind == VoidPtrKind::UserData)
        .count();
    let callbacks = patterns
        .iter()
        .filter(|p| p.kind == VoidPtrKind::CallbackFnPtr)
        .count();
    let allocs = patterns
        .iter()
        .filter(|p| {
            matches!(
                p.kind,
                VoidPtrKind::AllocatorFnPtr | VoidPtrKind::DeallocatorFnPtr
            )
        })
        .count();
    let buffers = patterns
        .iter()
        .filter(|p| matches!(p.kind, VoidPtrKind::InputBuffer | VoidPtrKind::OutputBuffer))
        .count();
    (user_data, callbacks, allocs, buffers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_user_data() {
        let src = "size_t csv_parse(struct csv_parser *p, const void *s, size_t len,\n    void (*cb1)(void *, size_t, void *), void (*cb2)(int, void *), void *data);";
        let patterns = detect_void_ptr_patterns(src);
        let user_data_count = patterns
            .iter()
            .filter(|p| p.kind == VoidPtrKind::UserData)
            .count();
        assert!(user_data_count >= 1, "should detect void *data as UserData");
    }

    #[test]
    fn test_detect_input_buffer() {
        let src = "size_t csv_parse(struct csv_parser *p, const void *s, size_t len, void (*cb1)(void *, size_t, void *), void (*cb2)(int, void *), void *data);";
        let patterns = detect_void_ptr_patterns(src);
        let buf_count = patterns
            .iter()
            .filter(|p| p.kind == VoidPtrKind::InputBuffer)
            .count();
        assert!(buf_count >= 1, "const void *s should be InputBuffer");
    }

    #[test]
    fn test_detect_callback_fn_ptr() {
        let src = "void (*cb1)(void *, size_t, void *);";
        let patterns = detect_void_ptr_patterns(src);
        assert!(
            patterns
                .iter()
                .any(|p| p.kind == VoidPtrKind::CallbackFnPtr),
            "should detect callback fn ptr"
        );
    }

    #[test]
    fn test_detect_deallocator() {
        let src = "void (*free_func)(void *);";
        let patterns = detect_void_ptr_patterns(src);
        assert!(
            patterns
                .iter()
                .any(|p| p.kind == VoidPtrKind::DeallocatorFnPtr),
            "should detect free_func as DeallocatorFnPtr"
        );
    }

    #[test]
    fn test_detect_allocator() {
        let src = "void *(*realloc_func)(void *, size_t);";
        let patterns = detect_void_ptr_patterns(src);
        assert!(
            patterns
                .iter()
                .any(|p| p.kind == VoidPtrKind::AllocatorFnPtr),
            "should detect realloc_func as AllocatorFnPtr"
        );
    }

    #[test]
    fn test_generate_hints_nonempty() {
        let src = "void csv_parse(const void *s, void *data, void (*cb)(void *, size_t, void *));";
        let patterns = detect_void_ptr_patterns(src);
        let hints = generate_void_ptr_hints(&patterns);
        assert!(!hints.is_empty());
        assert!(hints.contains("Void Pointer"));
    }

    #[test]
    fn test_libcsv_full_header() {
        // Inline fixture extracted from libcsv csv.h — avoids depending on test-projects/ in CI
        let header = r#"
struct csv_parser {
    void *(*malloc_func)(size_t);
    void *(*realloc_func)(void *, size_t);
    void (*free_func)(void *);
    void *blk_cur;
    void *entry_buf;
};
int csv_parse(struct csv_parser *p, const void *s, size_t len,
              void (*cb1)(void *, size_t, void *),
              void (*cb2)(int, void *),
              void *data);
int csv_fini(struct csv_parser *p,
             void (*cb1)(void *, size_t, void *),
             void (*cb2)(int, void *),
             void *data);
void *csv_get_delim(struct csv_parser *p);
void csv_set_realloc_func(struct csv_parser *p, void *(*func)(void *, size_t));
void csv_set_free_func(struct csv_parser *p, void (*func)(void *));
"#;
        let patterns = detect_void_ptr_patterns(header);
        let (user_data, callbacks, allocs, buffers) = count_patterns(&patterns);
        assert!(callbacks >= 2, "should find csv callback fn ptrs");
        assert!(allocs >= 1, "should find realloc/free fn ptrs");
        assert!(user_data >= 1, "should find void *data params");
        let auto_convertible = user_data + callbacks + allocs + buffers;
        println!(
            "libcsv fixture: {}/{} void* patterns auto-classified",
            auto_convertible,
            patterns.len()
        );
        assert!(
            auto_convertible >= 8,
            "need at least 8 of 11 patterns classified; got {}",
            auto_convertible
        );
    }
}
