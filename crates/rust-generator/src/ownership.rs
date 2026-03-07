use cpp_parser::memory::{analyze_memory_patterns, summarize_memory_patterns};
use cpp_parser::types::{CppFunction, CppParam, MemoryPatternKind};

pub struct OwnershipHint {
    pub parameter: String,
    pub suggestion: OwnershipSuggestion,
    pub reason: String,
}

pub enum OwnershipSuggestion {
    Owned,
    Borrowed,
    BorrowedMut,
    BoxOwned,
    ArcShared,
}

impl std::fmt::Display for OwnershipSuggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owned => write!(f, "owned (move)"),
            Self::Borrowed => write!(f, "&T (immutable borrow)"),
            Self::BorrowedMut => write!(f, "&mut T (mutable borrow)"),
            Self::BoxOwned => write!(f, "Box<T> (heap allocation)"),
            Self::ArcShared => write!(f, "Arc<T> (shared ownership)"),
        }
    }
}

pub fn infer_ownership(func: &CppFunction) -> Vec<OwnershipHint> {
    let mut hints = Vec::new();

    for param in &func.params {
        let hint = infer_param_ownership(param, &func.body);
        hints.push(hint);
    }

    hints
}

fn infer_param_ownership(param: &CppParam, body: &str) -> OwnershipHint {
    let name = &param.name;

    // const pointer/reference → immutable borrow
    if param.is_const && (param.is_pointer || param.is_reference) {
        return OwnershipHint {
            parameter: name.clone(),
            suggestion: OwnershipSuggestion::Borrowed,
            reason: format!("'{}' is const pointer/reference → immutable borrow", name),
        };
    }

    // Non-const pointer that gets free'd → owned
    if param.is_pointer && body.contains(&format!("free({})", name)) {
        return OwnershipHint {
            parameter: name.clone(),
            suggestion: OwnershipSuggestion::Owned,
            reason: format!("'{}' is freed in the function → takes ownership", name),
        };
    }

    // Non-const pointer/reference → mutable borrow
    if (param.is_pointer || param.is_reference) && !param.is_const {
        return OwnershipHint {
            parameter: name.clone(),
            suggestion: OwnershipSuggestion::BorrowedMut,
            reason: format!("'{}' is non-const pointer/reference → mutable borrow", name),
        };
    }

    // Value type → owned (move)
    OwnershipHint {
        parameter: name.clone(),
        suggestion: OwnershipSuggestion::Owned,
        reason: format!("'{}' is passed by value → owned", name),
    }
}

pub fn generate_ownership_context(func: &CppFunction) -> String {
    let hints = infer_ownership(func);
    let memory_patterns = analyze_memory_patterns(&func.body);

    let mut context = String::new();

    if !hints.is_empty() {
        context.push_str("Ownership hints for parameters:\n");
        for hint in &hints {
            context.push_str(&format!(
                "- {}: {} ({})\n",
                hint.parameter, hint.suggestion, hint.reason
            ));
        }
    }

    if !memory_patterns.is_empty() {
        context.push_str(&format!(
            "\nMemory management:\n{}\n",
            summarize_memory_patterns(&memory_patterns)
        ));
    }

    // Check return type patterns
    if func.return_type.contains('*') && !func.return_type.contains("const") {
        let allocates = memory_patterns
            .iter()
            .any(|p| matches!(p.kind, MemoryPatternKind::Malloc | MemoryPatternKind::New));
        if allocates {
            context.push_str(
                "\nReturn: Function allocates and returns a pointer → return owned type (Box<T>, Vec<T>, or String)\n",
            );
        } else {
            context.push_str(
                "\nReturn: Returns pointer but doesn't allocate → consider returning a reference or Option<&T>\n",
            );
        }
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_param(name: &str, is_const: bool, is_pointer: bool, is_reference: bool) -> CppParam {
        CppParam {
            name: name.to_string(),
            type_name: "int".to_string(),
            default_value: None,
            is_const,
            is_reference,
            is_pointer,
        }
    }

    #[test]
    fn test_const_pointer_borrowed() {
        let param = make_param("data", true, true, false);
        let hint = infer_param_ownership(&param, "");
        assert!(matches!(hint.suggestion, OwnershipSuggestion::Borrowed));
    }

    #[test]
    fn test_freed_pointer_owned() {
        let param = make_param("buf", false, true, false);
        let hint = infer_param_ownership(&param, "free(buf);");
        assert!(matches!(hint.suggestion, OwnershipSuggestion::Owned));
    }

    #[test]
    fn test_mutable_pointer() {
        let param = make_param("out", false, true, false);
        let hint = infer_param_ownership(&param, "out[0] = 42;");
        assert!(matches!(hint.suggestion, OwnershipSuggestion::BorrowedMut));
    }

    #[test]
    fn test_value_param_owned() {
        let param = make_param("x", false, false, false);
        let hint = infer_param_ownership(&param, "return x + 1;");
        assert!(matches!(hint.suggestion, OwnershipSuggestion::Owned));
    }
}
