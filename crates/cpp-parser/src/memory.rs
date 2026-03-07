use regex::Regex;

use crate::types::{MemoryPattern, MemoryPatternKind};

pub fn analyze_memory_patterns(source: &str) -> Vec<MemoryPattern> {
    let mut patterns = Vec::new();

    let checks: &[(&str, MemoryPatternKind)] = &[
        (r"\bmalloc\s*\(", MemoryPatternKind::Malloc),
        (r"\bcalloc\s*\(", MemoryPatternKind::Calloc),
        (r"\brealloc\s*\(", MemoryPatternKind::Realloc),
        (r"\bfree\s*\(", MemoryPatternKind::Free),
        (r"\bnew\s+\w+\s*\[", MemoryPatternKind::NewArray),
        (r"\bnew\s+\w+", MemoryPatternKind::New),
        (r"\bdelete\s*\[\s*\]", MemoryPatternKind::DeleteArray),
        (r"\bdelete\s+", MemoryPatternKind::Delete),
        (r"\bunique_ptr\s*<", MemoryPatternKind::UniquePtr),
        (r"\bshared_ptr\s*<", MemoryPatternKind::SharedPtr),
        (r"\bweak_ptr\s*<", MemoryPatternKind::WeakPtr),
    ];

    for (pattern, kind) in checks {
        let re = Regex::new(pattern).unwrap();
        for mat in re.find_iter(source) {
            let line = source[..mat.start()].matches('\n').count() + 1;
            let line_text = source
                .lines()
                .nth(line - 1)
                .unwrap_or("")
                .trim()
                .to_string();
            patterns.push(MemoryPattern {
                kind: kind.clone(),
                location: line_text,
                line,
            });
        }
    }

    patterns.sort_by_key(|p| p.line);
    patterns
}

pub fn summarize_memory_patterns(patterns: &[MemoryPattern]) -> String {
    if patterns.is_empty() {
        return "No manual memory management detected.".to_string();
    }

    let malloc_count = patterns
        .iter()
        .filter(|p| {
            matches!(
                p.kind,
                MemoryPatternKind::Malloc | MemoryPatternKind::Calloc
            )
        })
        .count();
    let free_count = patterns
        .iter()
        .filter(|p| p.kind == MemoryPatternKind::Free)
        .count();
    let new_count = patterns
        .iter()
        .filter(|p| matches!(p.kind, MemoryPatternKind::New | MemoryPatternKind::NewArray))
        .count();
    let delete_count = patterns
        .iter()
        .filter(|p| {
            matches!(
                p.kind,
                MemoryPatternKind::Delete | MemoryPatternKind::DeleteArray
            )
        })
        .count();
    let smart_count = patterns
        .iter()
        .filter(|p| {
            matches!(
                p.kind,
                MemoryPatternKind::UniquePtr
                    | MemoryPatternKind::SharedPtr
                    | MemoryPatternKind::WeakPtr
            )
        })
        .count();

    let mut parts = Vec::new();
    if malloc_count > 0 || free_count > 0 {
        parts.push(format!(
            "C-style: {} malloc/calloc, {} free",
            malloc_count, free_count
        ));
        if malloc_count != free_count {
            parts.push("  WARNING: malloc/free count mismatch (potential leak)".to_string());
        }
    }
    if new_count > 0 || delete_count > 0 {
        parts.push(format!(
            "C++-style: {} new, {} delete",
            new_count, delete_count
        ));
        if new_count != delete_count {
            parts.push("  WARNING: new/delete count mismatch (potential leak)".to_string());
        }
    }
    if smart_count > 0 {
        parts.push(format!("Smart pointers: {} usages", smart_count));
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_malloc_free() {
        let source = r#"
void process() {
    int* data = (int*)malloc(100 * sizeof(int));
    if (!data) return;
    free(data);
}
"#;
        let patterns = analyze_memory_patterns(source);
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].kind, MemoryPatternKind::Malloc);
        assert_eq!(patterns[1].kind, MemoryPatternKind::Free);
    }

    #[test]
    fn test_detect_new_delete() {
        let source = r#"
void process() {
    int* arr = new int[100];
    MyClass* obj = new MyClass();
    delete obj;
    delete[] arr;
}
"#;
        let patterns = analyze_memory_patterns(source);
        let kinds: Vec<_> = patterns.iter().map(|p| &p.kind).collect();
        assert!(kinds.contains(&&MemoryPatternKind::NewArray));
        assert!(kinds.contains(&&MemoryPatternKind::New));
        assert!(kinds.contains(&&MemoryPatternKind::Delete));
        assert!(kinds.contains(&&MemoryPatternKind::DeleteArray));
    }

    #[test]
    fn test_detect_smart_pointers() {
        let source = r#"
auto ptr = std::unique_ptr<MyClass>(new MyClass());
auto shared = std::shared_ptr<int>(new int(42));
"#;
        let patterns = analyze_memory_patterns(source);
        let kinds: Vec<_> = patterns.iter().map(|p| &p.kind).collect();
        assert!(kinds.contains(&&MemoryPatternKind::UniquePtr));
        assert!(kinds.contains(&&MemoryPatternKind::SharedPtr));
    }

    #[test]
    fn test_summary_balanced() {
        let source = "int* p = (int*)malloc(10);\nfree(p);\n";
        let patterns = analyze_memory_patterns(source);
        let summary = summarize_memory_patterns(&patterns);
        assert!(summary.contains("1 malloc/calloc, 1 free"));
        assert!(!summary.contains("WARNING"));
    }

    #[test]
    fn test_summary_leak_warning() {
        let source = "int* p = (int*)malloc(10);\nint* q = (int*)malloc(20);\nfree(p);\n";
        let patterns = analyze_memory_patterns(source);
        let summary = summarize_memory_patterns(&patterns);
        assert!(summary.contains("WARNING"));
    }
}
