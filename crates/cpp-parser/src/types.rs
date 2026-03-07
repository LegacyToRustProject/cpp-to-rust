use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CppStandard {
    C89,
    C99,
    C11,
    C17,
    Cpp03,
    Cpp11,
    Cpp14,
    Cpp17,
    Cpp20,
    Cpp23,
    Unknown,
}

impl std::fmt::Display for CppStandard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::C89 => write!(f, "C89"),
            Self::C99 => write!(f, "C99"),
            Self::C11 => write!(f, "C11"),
            Self::C17 => write!(f, "C17"),
            Self::Cpp03 => write!(f, "C++03"),
            Self::Cpp11 => write!(f, "C++11"),
            Self::Cpp14 => write!(f, "C++14"),
            Self::Cpp17 => write!(f, "C++17"),
            Self::Cpp20 => write!(f, "C++20"),
            Self::Cpp23 => write!(f, "C++23"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    C,
    Cpp,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::C => write!(f, "C"),
            Self::Cpp => write!(f, "C++"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppProject {
    pub root: PathBuf,
    pub language: Language,
    pub standard: CppStandard,
    pub files: Vec<CppFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppFile {
    pub path: PathBuf,
    pub source: String,
    pub language: Language,
    pub includes: Vec<String>,
    pub macros: Vec<CppMacro>,
    pub structs: Vec<CppStruct>,
    pub classes: Vec<CppClass>,
    pub functions: Vec<CppFunction>,
    pub globals: Vec<CppGlobal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppMacro {
    pub name: String,
    pub params: Option<Vec<String>>,
    pub body: String,
    pub is_conditional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppStruct {
    pub name: String,
    pub fields: Vec<CppField>,
    pub is_typedef: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppClass {
    pub name: String,
    pub bases: Vec<CppBase>,
    pub fields: Vec<CppField>,
    pub methods: Vec<CppFunction>,
    pub visibility_default: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppBase {
    pub name: String,
    pub visibility: Visibility,
    pub is_virtual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppField {
    pub name: String,
    pub type_name: String,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_const: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppFunction {
    pub name: String,
    pub return_type: String,
    pub params: Vec<CppParam>,
    pub body: String,
    pub is_static: bool,
    pub is_virtual: bool,
    pub is_const: bool,
    pub is_template: bool,
    pub template_params: Vec<String>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppParam {
    pub name: String,
    pub type_name: String,
    pub default_value: Option<String>,
    pub is_const: bool,
    pub is_reference: bool,
    pub is_pointer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppGlobal {
    pub name: String,
    pub type_name: String,
    pub is_const: bool,
    pub is_extern: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPattern {
    pub kind: MemoryPatternKind,
    pub location: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryPatternKind {
    Malloc,
    Calloc,
    Realloc,
    Free,
    New,
    NewArray,
    Delete,
    DeleteArray,
    UniquePtr,
    SharedPtr,
    WeakPtr,
}

impl std::fmt::Display for MemoryPatternKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malloc => write!(f, "malloc"),
            Self::Calloc => write!(f, "calloc"),
            Self::Realloc => write!(f, "realloc"),
            Self::Free => write!(f, "free"),
            Self::New => write!(f, "new"),
            Self::NewArray => write!(f, "new[]"),
            Self::Delete => write!(f, "delete"),
            Self::DeleteArray => write!(f, "delete[]"),
            Self::UniquePtr => write!(f, "unique_ptr"),
            Self::SharedPtr => write!(f, "shared_ptr"),
            Self::WeakPtr => write!(f, "weak_ptr"),
        }
    }
}
