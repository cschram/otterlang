//! Type checking system for OtterLang
//!
//! Provides type inference, validation, and error reporting

pub mod checker;
pub mod diagnostics;
pub mod types;
pub mod workspace;

pub use checker::{ModuleExports, TypeChecker};
pub use diagnostics::from_type_errors as diagnostics_from_type_errors;
pub use types::{EnumLayout, TypeContext, TypeError, TypeInfo};
pub use workspace::{ModuleDependency, ModuleRecord, TypecheckWorkspace};
