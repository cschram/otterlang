//! Type representations for the Otter compiler.

use otterc_symbol::symbol::Symbol;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRef(Symbol);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntTy {
    U8,
    U16,
    U32,
    I32,
    I64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FloatTy {
    F32,
    F64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionTy {
    pub params: Vec<TypeRef>,
    pub return_ty: Option<TypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumTy {
    pub variants: HashMap<Symbol, Vec<TypeRef>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructMethod {
    pub func_ty: FunctionTy,
    pub trait_impl: Option<Symbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructTy {
    pub fields: HashMap<Symbol, TypeRef>,
    pub traits: Vec<Symbol>,
    pub methods: HashMap<Symbol, StructMethod>,
}

impl StructTy {
    #[inline]
    pub fn get_field(&self, name: &Symbol) -> Option<&TypeRef> {
        self.fields.get(name)
    }

    #[inline]
    pub fn method(&self, name: &Symbol) -> Option<&StructMethod> {
        self.methods.get(name)
    }

    #[inline]
    pub fn implements(&self, trait_name: &Symbol) -> bool {
        self.traits.contains(trait_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Unknown,
    Unit,
    Bool,
    Int(IntTy),
    Float(FloatTy),
    Str,
    Array(TypeRef),
    Function(FunctionTy),
    Enum(EnumTy),
    Struct(StructTy),
    Union(Vec<TypeRef>),
}
