/// STL container pattern detection and Rust type mapping.
///
/// Detects C++ STL container / utility type usage in source code and generates
/// Rust type replacement hints for use in LLM conversion prompts.
///
/// Covers 29 patterns across six categories:
///   Sequence · Ordered · Unordered · Adapter · SmartPointer · String · Utility
use regex::Regex;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Semantic category of an STL type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StlCategory {
    /// std::vector, std::array, std::deque, std::list, std::forward_list
    Sequence,
    /// std::map, std::multimap, std::set, std::multiset
    Ordered,
    /// std::unordered_map, std::unordered_multimap, std::unordered_set, std::unordered_multiset
    Unordered,
    /// std::stack, std::queue, std::priority_queue
    Adapter,
    /// std::unique_ptr, std::shared_ptr, std::weak_ptr
    SmartPointer,
    /// std::string, std::string_view, std::wstring
    String,
    /// std::optional, std::variant, std::tuple, std::pair, std::bitset,
    /// std::span, std::function
    Utility,
}

impl std::fmt::Display for StlCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequence => write!(f, "Sequence"),
            Self::Ordered => write!(f, "Ordered"),
            Self::Unordered => write!(f, "Unordered"),
            Self::Adapter => write!(f, "Adapter"),
            Self::SmartPointer => write!(f, "SmartPointer"),
            Self::String => write!(f, "String"),
            Self::Utility => write!(f, "Utility"),
        }
    }
}

/// Static descriptor for one C++ STL type → Rust mapping.
#[derive(Debug, Clone)]
pub struct StlMapping {
    /// The bare C++ name after `std::` (e.g. `"vector"`)
    pub cpp_name: &'static str,
    /// Full qualified C++ spelling used in regex (e.g. `"std::vector"`)
    pub cpp_qualified: &'static str,
    /// Rust replacement type template (type params shown as `<T>`, `<K, V>`, etc.)
    pub rust_type: &'static str,
    pub category: StlCategory,
    /// Optional migration note displayed in hints
    pub note: Option<&'static str>,
}

/// A single detected occurrence of a C++ STL type in source code.
#[derive(Debug, Clone)]
pub struct StlOccurrence {
    /// Source line (trimmed)
    pub context: String,
    /// 1-based line number
    pub line: usize,
    /// The matched C++ qualified name (e.g. `"std::vector"`)
    pub cpp_type: String,
    /// Suggested Rust replacement (template-level, e.g. `"Vec<T>"`)
    pub rust_type: String,
    pub category: StlCategory,
    /// Optional migration note
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// Mapping table — 29 patterns
// ---------------------------------------------------------------------------

/// Return the canonical STL → Rust mapping table (29 entries).
pub fn all_mappings() -> Vec<StlMapping> {
    use StlCategory::*;
    vec![
        // ── Sequence containers ──────────────────────────────────────────
        StlMapping {
            cpp_name: "vector",
            cpp_qualified: "std::vector",
            rust_type: "Vec<T>",
            category: Sequence,
            note: None,
        },
        StlMapping {
            cpp_name: "array",
            cpp_qualified: "std::array",
            rust_type: "[T; N]",
            category: Sequence,
            note: Some("N must be a compile-time constant"),
        },
        StlMapping {
            cpp_name: "deque",
            cpp_qualified: "std::deque",
            rust_type: "VecDeque<T>",
            category: Sequence,
            note: Some("use std::collections::VecDeque"),
        },
        StlMapping {
            cpp_name: "list",
            cpp_qualified: "std::list",
            rust_type: "LinkedList<T>",
            category: Sequence,
            note: Some("prefer Vec<T> unless O(1) splice is needed; use std::collections::LinkedList"),
        },
        StlMapping {
            cpp_name: "forward_list",
            cpp_qualified: "std::forward_list",
            rust_type: "LinkedList<T>",
            category: Sequence,
            note: Some("singly-linked; prefer Vec<T> in most cases"),
        },
        // ── Ordered associative containers ────────────────────────────────
        StlMapping {
            cpp_name: "map",
            cpp_qualified: "std::map",
            rust_type: "BTreeMap<K, V>",
            category: Ordered,
            note: Some("use std::collections::BTreeMap; iteration order preserved"),
        },
        StlMapping {
            cpp_name: "multimap",
            cpp_qualified: "std::multimap",
            rust_type: "BTreeMap<K, Vec<V>>",
            category: Ordered,
            note: Some("group duplicate keys into Vec<V>"),
        },
        StlMapping {
            cpp_name: "set",
            cpp_qualified: "std::set",
            rust_type: "BTreeSet<T>",
            category: Ordered,
            note: Some("use std::collections::BTreeSet"),
        },
        StlMapping {
            cpp_name: "multiset",
            cpp_qualified: "std::multiset",
            rust_type: "BTreeMap<T, usize>",
            category: Ordered,
            note: Some("track counts: BTreeMap<T, usize>; T must impl Ord"),
        },
        // ── Unordered associative containers ─────────────────────────────
        StlMapping {
            cpp_name: "unordered_map",
            cpp_qualified: "std::unordered_map",
            rust_type: "HashMap<K, V>",
            category: Unordered,
            note: Some("use std::collections::HashMap; K must impl Hash + Eq"),
        },
        StlMapping {
            cpp_name: "unordered_multimap",
            cpp_qualified: "std::unordered_multimap",
            rust_type: "HashMap<K, Vec<V>>",
            category: Unordered,
            note: Some("group duplicate keys into Vec<V>"),
        },
        StlMapping {
            cpp_name: "unordered_set",
            cpp_qualified: "std::unordered_set",
            rust_type: "HashSet<T>",
            category: Unordered,
            note: Some("use std::collections::HashSet; T must impl Hash + Eq"),
        },
        StlMapping {
            cpp_name: "unordered_multiset",
            cpp_qualified: "std::unordered_multiset",
            rust_type: "HashMap<T, usize>",
            category: Unordered,
            note: Some("track counts: HashMap<T, usize>"),
        },
        // ── Container adapters ────────────────────────────────────────────
        StlMapping {
            cpp_name: "stack",
            cpp_qualified: "std::stack",
            rust_type: "Vec<T>",
            category: Adapter,
            note: Some("use Vec::push / Vec::pop; no separate stack type needed"),
        },
        StlMapping {
            cpp_name: "queue",
            cpp_qualified: "std::queue",
            rust_type: "VecDeque<T>",
            category: Adapter,
            note: Some("use VecDeque::push_back / pop_front"),
        },
        StlMapping {
            cpp_name: "priority_queue",
            cpp_qualified: "std::priority_queue",
            rust_type: "BinaryHeap<T>",
            category: Adapter,
            note: Some("use std::collections::BinaryHeap; T must impl Ord"),
        },
        // ── Smart pointers ─────────────────────────────────────────────────
        StlMapping {
            cpp_name: "unique_ptr",
            cpp_qualified: "std::unique_ptr",
            rust_type: "Box<T>",
            category: SmartPointer,
            note: None,
        },
        StlMapping {
            cpp_name: "shared_ptr",
            cpp_qualified: "std::shared_ptr",
            rust_type: "Arc<T>",
            category: SmartPointer,
            note: Some("use Arc<T> for thread-safe; Rc<T> for single-threaded"),
        },
        StlMapping {
            cpp_name: "weak_ptr",
            cpp_qualified: "std::weak_ptr",
            rust_type: "Weak<T>",
            category: SmartPointer,
            note: Some("Arc::downgrade() produces Weak<T>"),
        },
        // ── String types ───────────────────────────────────────────────────
        StlMapping {
            cpp_name: "string",
            cpp_qualified: "std::string",
            rust_type: "String",
            category: StlCategory::String,
            note: None,
        },
        StlMapping {
            cpp_name: "string_view",
            cpp_qualified: "std::string_view",
            rust_type: "&str",
            category: StlCategory::String,
            note: Some("for owned storage use String; &str is a borrowed slice"),
        },
        StlMapping {
            cpp_name: "wstring",
            cpp_qualified: "std::wstring",
            rust_type: "String",
            category: StlCategory::String,
            note: Some("convert UTF-16 wchar_t data to UTF-8 String"),
        },
        // ── Utility types ──────────────────────────────────────────────────
        StlMapping {
            cpp_name: "optional",
            cpp_qualified: "std::optional",
            rust_type: "Option<T>",
            category: Utility,
            note: None,
        },
        StlMapping {
            cpp_name: "variant",
            cpp_qualified: "std::variant",
            rust_type: "enum / Box<dyn Any>",
            category: Utility,
            note: Some("prefer a custom enum; use Box<dyn Any> only as last resort"),
        },
        StlMapping {
            cpp_name: "tuple",
            cpp_qualified: "std::tuple",
            rust_type: "(T, U, ...)",
            category: Utility,
            note: Some("Rust tuples are structural: (T, U, V)"),
        },
        StlMapping {
            cpp_name: "pair",
            cpp_qualified: "std::pair",
            rust_type: "(T, U)",
            category: Utility,
            note: Some(".first → .0, .second → .1"),
        },
        StlMapping {
            cpp_name: "bitset",
            cpp_qualified: "std::bitset",
            rust_type: "[bool; N]",
            category: Utility,
            note: Some("or use the `bitflags` crate for named flags"),
        },
        StlMapping {
            cpp_name: "span",
            cpp_qualified: "std::span",
            rust_type: "&[T]",
            category: Utility,
            note: Some("mutable span → &mut [T]"),
        },
        StlMapping {
            cpp_name: "function",
            cpp_qualified: "std::function",
            rust_type: "Box<dyn Fn(...)>",
            category: Utility,
            note: Some("use impl Fn(...) for generic params; Box<dyn Fn(...)> for stored callables"),
        },
    ]
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Detect all STL container/utility usages in `source`.
///
/// Returns one [`StlOccurrence`] per match; the same line may produce multiple
/// occurrences if it uses more than one STL type.
pub fn detect_stl_patterns(source: &str) -> Vec<StlOccurrence> {
    let mappings = all_mappings();

    // Build (regex, mapping_index) pairs once.
    // Use `\b` word-boundaries so that:
    //   - `\bstd::map\b` does NOT match inside `std::multimap`
    //     (different prefix, no substring overlap)
    //   - `\bstd::string\b` does NOT match inside `std::string_view`
    //     (underscore is \w, so no word boundary between `string` and `_view`)
    // The regex crate does not support lookaround, so \b is the correct choice.
    let compiled: Vec<(Regex, usize)> = mappings
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let pat = format!(r"\bstd::{}\b", regex::escape(m.cpp_name));
            (Regex::new(&pat).unwrap(), i)
        })
        .collect();

    let mut occurrences = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        // Skip pure comment lines
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('*') {
            continue;
        }

        // Track which positions have already been claimed to avoid duplicates
        // when a longer pattern (e.g. unordered_multimap) overlaps a shorter
        // one (unordered_map).
        let mut claimed: Vec<(usize, usize)> = Vec::new();

        // Iterate mappings in table order; longer / more specific names come
        // first within each prefix group (see table above).
        for (re, idx) in &compiled {
            let m = &mappings[*idx];
            for mat in re.find_iter(line) {
                let start = mat.start();
                let end = mat.end();
                // Skip if this range is already covered by a longer match.
                if claimed.iter().any(|&(s, e)| start >= s && end <= e) {
                    continue;
                }
                claimed.push((start, end));
                occurrences.push(StlOccurrence {
                    context: trimmed.to_string(),
                    line: line_idx + 1,
                    cpp_type: m.cpp_qualified.to_string(),
                    rust_type: m.rust_type.to_string(),
                    category: m.category.clone(),
                    note: m.note.map(|s| s.to_string()),
                });
            }
        }
    }

    occurrences
}

// ---------------------------------------------------------------------------
// Hint generation
// ---------------------------------------------------------------------------

/// Generate a Markdown hint block for injection into LLM conversion prompts.
pub fn generate_stl_hints(occurrences: &[StlOccurrence]) -> String {
    if occurrences.is_empty() {
        return String::new();
    }

    // Count unique cpp_type occurrences per mapping.
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for occ in occurrences {
        *counts.entry(occ.cpp_type.clone()).or_insert(0) += 1;
    }

    // Gather one representative occurrence per type (first seen).
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut representatives: Vec<&StlOccurrence> = Vec::new();
    for occ in occurrences {
        if seen.insert(occ.cpp_type.clone()) {
            representatives.push(occ);
        }
    }

    let mut hint =
        String::from("## STL Container / Type Mapping Hints\n\n");
    hint.push_str(&format!(
        "Detected {} STL type usage(s) across {} distinct type(s). \
         Apply these Rust equivalents:\\n\\n",
        occurrences.len(),
        representatives.len()
    ));

    // Group by category for readability.
    let category_order = [
        StlCategory::Sequence,
        StlCategory::Ordered,
        StlCategory::Unordered,
        StlCategory::Adapter,
        StlCategory::SmartPointer,
        StlCategory::String,
        StlCategory::Utility,
    ];
    let category_titles = [
        "Sequence Containers",
        "Ordered Associative Containers",
        "Unordered Associative Containers",
        "Container Adapters",
        "Smart Pointers",
        "String Types",
        "Utility Types",
    ];

    for (cat, title) in category_order.iter().zip(category_titles.iter()) {
        let group: Vec<&&StlOccurrence> = representatives
            .iter()
            .filter(|o| &o.category == cat)
            .collect();
        if group.is_empty() {
            continue;
        }

        hint.push_str(&format!("### {}\n\n", title));
        hint.push_str("| C++ Type | Rust Type | Occurrences | Note |\n");
        hint.push_str("|---|---|---|---|\n");

        for occ in group {
            let n = counts.get(&occ.cpp_type).copied().unwrap_or(1);
            let note = occ.note.as_deref().unwrap_or("—");
            hint.push_str(&format!(
                "| `{}` | `{}` | {} | {} |\n",
                occ.cpp_type, occ.rust_type, n, note
            ));
        }
        hint.push('\n');
    }

    // Emit required `use` statements.
    let mut uses: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
    for occ in occurrences {
        match occ.cpp_type.as_str() {
            "std::deque" | "std::queue" => {
                uses.insert("use std::collections::VecDeque;");
            }
            "std::list" | "std::forward_list" => {
                uses.insert("use std::collections::LinkedList;");
            }
            "std::map" | "std::multimap" => {
                uses.insert("use std::collections::BTreeMap;");
            }
            "std::set" | "std::multiset" => {
                uses.insert("use std::collections::BTreeSet;");
            }
            "std::unordered_map" | "std::unordered_multimap" | "std::unordered_multiset" => {
                uses.insert("use std::collections::HashMap;");
            }
            "std::unordered_set" => {
                uses.insert("use std::collections::HashSet;");
            }
            "std::priority_queue" => {
                uses.insert("use std::collections::BinaryHeap;");
            }
            "std::shared_ptr" | "std::weak_ptr" => {
                uses.insert("use std::sync::{Arc, Weak};");
            }
            _ => {}
        }
    }

    if !uses.is_empty() {
        hint.push_str("### Required `use` statements\n\n```rust\n");
        for u in &uses {
            hint.push_str(u);
            hint.push('\n');
        }
        hint.push_str("```\n\n");
    }

    hint
}

/// Count occurrences by category.
///
/// Returns `(sequence, ordered, unordered, adapter, smart_ptr, string, utility)`.
pub fn count_by_category(occurrences: &[StlOccurrence]) -> (usize, usize, usize, usize, usize, usize, usize) {
    let seq   = occurrences.iter().filter(|o| o.category == StlCategory::Sequence).count();
    let ord   = occurrences.iter().filter(|o| o.category == StlCategory::Ordered).count();
    let unord = occurrences.iter().filter(|o| o.category == StlCategory::Unordered).count();
    let adp   = occurrences.iter().filter(|o| o.category == StlCategory::Adapter).count();
    let sptr  = occurrences.iter().filter(|o| o.category == StlCategory::SmartPointer).count();
    let s     = occurrences.iter().filter(|o| o.category == StlCategory::String).count();
    let util  = occurrences.iter().filter(|o| o.category == StlCategory::Utility).count();
    (seq, ord, unord, adp, sptr, s, util)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- mapping table coverage -------------------------------------------

    #[test]
    fn test_mapping_count_at_least_20() {
        let mappings = all_mappings();
        assert!(
            mappings.len() >= 20,
            "need at least 20 STL mappings; got {}",
            mappings.len()
        );
    }

    #[test]
    fn test_all_categories_covered() {
        let mappings = all_mappings();
        let has = |cat: StlCategory| mappings.iter().any(|m| m.category == cat);
        assert!(has(StlCategory::Sequence));
        assert!(has(StlCategory::Ordered));
        assert!(has(StlCategory::Unordered));
        assert!(has(StlCategory::Adapter));
        assert!(has(StlCategory::SmartPointer));
        assert!(has(StlCategory::String));
        assert!(has(StlCategory::Utility));
    }

    // ----- basic detection --------------------------------------------------

    #[test]
    fn test_detect_vector() {
        let src = "std::vector<int> numbers;";
        let occ = detect_stl_patterns(src);
        assert!(
            occ.iter().any(|o| o.cpp_type == "std::vector"),
            "should detect std::vector"
        );
        assert!(occ.iter().any(|o| o.rust_type == "Vec<T>"));
    }

    #[test]
    fn test_detect_map_and_unordered_map() {
        let src = "std::map<std::string, int> ordered;\nstd::unordered_map<std::string, int> fast;";
        let occ = detect_stl_patterns(src);
        assert!(occ.iter().any(|o| o.cpp_type == "std::map"), "map not detected");
        assert!(
            occ.iter().any(|o| o.cpp_type == "std::unordered_map"),
            "unordered_map not detected"
        );
        // std::string should also be picked up
        assert!(occ.iter().any(|o| o.cpp_type == "std::string"), "string not detected");
    }

    #[test]
    fn test_detect_smart_pointers() {
        let src = "std::unique_ptr<Foo> p1;\nstd::shared_ptr<Bar> p2;\nstd::weak_ptr<Bar> w;";
        let occ = detect_stl_patterns(src);
        assert!(occ.iter().any(|o| o.cpp_type == "std::unique_ptr"));
        assert!(occ.iter().any(|o| o.cpp_type == "std::shared_ptr"));
        assert!(occ.iter().any(|o| o.cpp_type == "std::weak_ptr"));
    }

    #[test]
    fn test_detect_adapters() {
        let src = "std::stack<int> s;\nstd::queue<Task> q;\nstd::priority_queue<int> pq;";
        let occ = detect_stl_patterns(src);
        assert!(occ.iter().any(|o| o.cpp_type == "std::stack"));
        assert!(occ.iter().any(|o| o.cpp_type == "std::queue"));
        assert!(occ.iter().any(|o| o.cpp_type == "std::priority_queue"));
    }

    #[test]
    fn test_detect_utility_types() {
        let src = "std::optional<int> x;\nstd::variant<int, float> v;\nstd::function<void(int)> cb;\nstd::pair<int, int> p;\nstd::tuple<int, int, int> t;";
        let occ = detect_stl_patterns(src);
        assert!(occ.iter().any(|o| o.cpp_type == "std::optional"));
        assert!(occ.iter().any(|o| o.cpp_type == "std::variant"));
        assert!(occ.iter().any(|o| o.cpp_type == "std::function"));
        assert!(occ.iter().any(|o| o.cpp_type == "std::pair"));
        assert!(occ.iter().any(|o| o.cpp_type == "std::tuple"));
    }

    #[test]
    fn test_no_double_count_unordered_map_vs_multimap() {
        // "std::unordered_multimap" must NOT be counted as both
        // unordered_multimap AND unordered_map.
        let src = "std::unordered_multimap<std::string, int> mm;";
        let occ = detect_stl_patterns(src);
        let um_count = occ.iter().filter(|o| o.cpp_type == "std::unordered_map").count();
        let umm_count = occ
            .iter()
            .filter(|o| o.cpp_type == "std::unordered_multimap")
            .count();
        assert_eq!(um_count, 0, "unordered_map should not match inside unordered_multimap");
        assert_eq!(umm_count, 1, "unordered_multimap should match exactly once");
    }

    #[test]
    fn test_no_double_count_map_vs_multimap() {
        let src = "std::multimap<int, int> mm;";
        let occ = detect_stl_patterns(src);
        let map_count = occ.iter().filter(|o| o.cpp_type == "std::map").count();
        let mmap_count = occ.iter().filter(|o| o.cpp_type == "std::multimap").count();
        assert_eq!(map_count, 0, "map should not match inside multimap");
        assert_eq!(mmap_count, 1);
    }

    #[test]
    fn test_comments_skipped() {
        let src = "// std::vector<int> not a real usage\nstd::vector<float> real;";
        let occ = detect_stl_patterns(src);
        // Only line 2 should produce a match; comment line is skipped.
        // Line numbers are 1-based.
        let on_line1 = occ.iter().filter(|o| o.line == 1).count();
        assert_eq!(on_line1, 0, "comment line should be skipped");
        assert!(occ.iter().any(|o| o.line == 2));
    }

    #[test]
    fn test_hint_generation_nonempty() {
        let src = "std::vector<int> v;\nstd::unordered_map<std::string, int> m;";
        let occ = detect_stl_patterns(src);
        let hint = generate_stl_hints(&occ);
        assert!(!hint.is_empty());
        assert!(hint.contains("STL Container"));
        assert!(hint.contains("Vec<T>"));
        assert!(hint.contains("HashMap<K, V>"));
    }

    #[test]
    fn test_hint_includes_use_statements() {
        let src = "std::unordered_map<int,int> a;\nstd::shared_ptr<Foo> b;";
        let occ = detect_stl_patterns(src);
        let hint = generate_stl_hints(&occ);
        assert!(hint.contains("use std::collections::HashMap;"));
        assert!(hint.contains("use std::sync::{Arc, Weak};"));
    }

    #[test]
    fn test_count_by_category() {
        let src = "std::vector<int> v;\nstd::map<int,int> m;\nstd::unique_ptr<Foo> p;";
        let occ = detect_stl_patterns(src);
        let (seq, ord, _unord, _adp, sptr, _s, _util) = count_by_category(&occ);
        assert_eq!(seq, 1);
        assert_eq!(ord, 1);
        assert_eq!(sptr, 1);
    }

    // ----- tinyxml2 integration test ----------------------------------------

    #[test]
    fn test_tinyxml2_header_detection() {
        let header = include_str!("../../../test-projects/tinyxml2/tinyxml2.h");
        let occ = detect_stl_patterns(header);
        println!(
            "TinyXML2 STL patterns: {} occurrences",
            occ.len()
        );
        // TinyXML2 deliberately avoids the STL (no `std::` qualifications).
        // Zero detections is the correct result. Verify the function does not panic
        // and that hint generation handles an empty occurrence list gracefully.
        let hint = generate_stl_hints(&occ);
        assert!(
            hint.is_empty(),
            "hint for zero occurrences should be empty"
        );
    }
}
