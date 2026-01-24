use otterc_ident::Identifier;
use otterc_ty::TyId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SymbolError {
    #[error("Scope depth exceeded maximum of {0}")]
    MaxScopeDepthExceeded(usize),
    #[error("Scope not found")]
    ScopeNotFound,
    #[error("Type {0}({1}) already exists in scope")]
    TypeAlreadyExists(TyId, Identifier),
    #[error("Symbol {0} already exists in scope")]
    SymbolAlreadyExists(Identifier),
    #[error("Cannot monomorphize non-generic type {0}({1})")]
    CannotMonomorphizeNonGeneric(TyId, Identifier),
    #[error("Expected {0} type parameters, but got {1}")]
    TypeParameterMismatch(usize, usize),
    #[error("Type {0} not found")]
    TypeNotFound(Identifier),
    #[error("Base type {0} not found")]
    BaseTypeNotFound(TyId),
}
