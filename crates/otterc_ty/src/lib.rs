#![allow(
    private_bounds,
    reason = "We use a private trait to limit type references"
)]

use ahash::AHashMap;
use otterc_ident::*;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_TYPE_ID: AtomicU32 = AtomicU32::new(0);

/// Unique identifier for a type
#[derive(Debug, Clone, Copy)]
pub struct TyId(u32, Identifier);

impl TyId {
    pub fn new(name: Identifier) -> Self {
        let id = NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst);
        Self(id, name)
    }
}

impl PartialEq for TyId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for TyId {}

impl Hash for TyId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl std::fmt::Display for TyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeId({}, {})", self.0, self.1)
    }
}

/// Unresolved reference to a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyRef {
    name: Identifier,
    params: Vec<TyRef>,
}

impl TyRef {
    pub fn new(name: Identifier, params: Vec<TyRef>) -> Self {
        Self { name, params }
    }

    pub fn name(&self) -> Identifier {
        self.name
    }

    pub fn params(&self) -> &Vec<TyRef> {
        &self.params
    }
}

impl From<Identifier> for TyRef {
    fn from(name: Identifier) -> Self {
        Self {
            name,
            params: Vec::new(),
        }
    }
}

impl From<&str> for TyRef {
    fn from(s: &str) -> Self {
        Self::from(Identifier::from(s))
    }
}

/// Primitive types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    String,
}

/// Generic argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericArg {
    name: Identifier,
}

impl GenericArg {
    pub fn new(name: Identifier) -> Self {
        Self { name }
    }

    pub fn name(&self) -> Identifier {
        self.name
    }
}

impl From<&str> for GenericArg {
    fn from(s: &str) -> Self {
        Self {
            name: Identifier::from(s),
        }
    }
}

impl From<String> for GenericArg {
    fn from(s: String) -> Self {
        Self {
            name: Identifier::from(s),
        }
    }
}

impl From<Identifier> for GenericArg {
    fn from(ident: Identifier) -> Self {
        Self { name: ident }
    }
}

/// Marker trait for type identifiers, so we can limit them to either TyId or TyRef.
trait TyIdent {}

impl TyIdent for TyId {}

impl TyIdent for TyRef {}

/// Function signature.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef<T: TyIdent> {
    params: Vec<(Identifier, T)>,
    return_type: Option<T>,
}

impl<T: TyIdent> FunctionDef<T> {
    pub fn new(params: Vec<(Identifier, T)>, return_type: Option<T>) -> Self {
        Self {
            params,
            return_type,
        }
    }

    pub fn params(&self) -> &Vec<(Identifier, T)> {
        &self.params
    }

    pub fn return_type(&self) -> &Option<T> {
        &self.return_type
    }
}

/// Struct definition.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef<T: TyIdent> {
    members: AHashMap<Identifier, T>,
    methods: AHashMap<Identifier, FunctionDef<T>>,
}

impl<T: TyIdent> StructDef<T> {
    pub fn new(
        members: AHashMap<Identifier, T>,
        methods: AHashMap<Identifier, FunctionDef<T>>,
    ) -> Self {
        Self { members, methods }
    }

    pub fn members(&self) -> &AHashMap<Identifier, T> {
        &self.members
    }

    pub fn member(&self, ident: &Identifier) -> Option<&T> {
        self.members.get(ident)
    }

    pub fn methods(&self) -> &AHashMap<Identifier, FunctionDef<T>> {
        &self.methods
    }

    pub fn method(&self, ident: &Identifier) -> Option<&FunctionDef<T>> {
        self.methods.get(ident)
    }
}

/// Enum definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef<T: TyIdent> {
    variants: Vec<EnumVariant<T>>,
}

impl<T: TyIdent> EnumDef<T> {
    pub fn new(variants: Vec<EnumVariant<T>>) -> Self {
        Self { variants }
    }

    pub fn variants(&self) -> &Vec<EnumVariant<T>> {
        &self.variants
    }

    pub fn variant(&self, index: usize) -> Option<&EnumVariant<T>> {
        self.variants.get(index)
    }
}

/// Enum variant representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant<T: TyIdent> {
    discriminant: u32,
    name: Identifier,
    fields: Vec<T>,
}

impl<T: TyIdent> EnumVariant<T> {
    /// Create a new enum variant
    pub fn new(discriminant: u32, ident: Identifier, fields: Vec<T>) -> Self {
        Self {
            discriminant,
            name: ident,
            fields,
        }
    }

    /// Get the discriminant of the variant
    pub fn discriminant(&self) -> u32 {
        self.discriminant
    }

    /// Get the identifier of the variant
    pub fn name(&self) -> Identifier {
        self.name
    }

    /// Get the fields of the variant
    pub fn fields(&self) -> &Vec<T> {
        &self.fields
    }

    /// Get a field from the variant by index.
    pub fn field(&self, index: usize) -> Option<&T> {
        self.fields.get(index)
    }
}

/// Alias definition.
#[derive(Debug, Clone, PartialEq)]
pub struct AliasDef<T: TyIdent> {
    target: T,
}

impl<T: TyIdent> AliasDef<T> {
    pub fn new(target: T) -> Self {
        Self { target }
    }

    pub fn target(&self) -> &T {
        &self.target
    }
}

/// Generic type kind.
#[derive(Debug, Clone)]
pub enum GenericKind {
    Function(FunctionDef<TyRef>),
    Struct(StructDef<TyRef>),
    Enum(EnumDef<TyRef>),
    Alias(AliasDef<TyRef>),
}

/// Type kind
#[derive(Debug, Clone)]
pub enum TyKind {
    /// Unknown type (used internally in type checking)
    Unknown,
    /// Primitive type
    Primitive(PrimitiveKind),
    /// Range type
    Range,
    /// List type
    List(TyId),
    /// Dictionary type
    Dict(TyId, TyId),
    /// Function type
    /// (Definition, Base generic type)
    Function(FunctionDef<TyId>, Option<TyId>),
    /// Struct type
    /// (Definition, Base generic type)
    Struct(StructDef<TyId>, Option<TyId>),
    /// Enum type
    /// (Definition, Base generic type)
    Enum(EnumDef<TyId>, Option<TyId>),
    /// Alias type
    /// (Definition, Base generic type)
    Alias(AliasDef<TyId>, Option<TyId>),
    /// Generic type.
    Generic(GenericKind, Vec<GenericArg>),
}

#[derive(Debug, Clone)]
pub struct Ty {
    id: TyId,
    name: Identifier,
    kind: TyKind,
}

impl Ty {
    pub fn new(name: impl Into<Identifier>, kind: TyKind) -> Self {
        let name = name.into();
        Self {
            id: TyId::new(name),
            name,
            kind,
        }
    }

    pub fn id(&self) -> TyId {
        self.id
    }

    pub fn name(&self) -> Identifier {
        self.name
    }

    pub fn kind(&self) -> &TyKind {
        &self.kind
    }

    /// Get a member value of the type.
    pub fn member(&self, ident: &Identifier) -> Option<&TyId> {
        match self.kind() {
            TyKind::Struct(def, _) => def.member(ident),
            _ => None,
        }
    }

    /// Check if this type is unknown
    pub fn is_unknown(&self) -> bool {
        matches!(self.kind(), TyKind::Unknown)
    }

    /// Check if this type is boolean
    pub fn is_bool(&self) -> bool {
        matches!(self.kind(), TyKind::Primitive(PrimitiveKind::Bool))
    }

    /// Check if this type is numeric
    pub fn is_numeric(&self) -> bool {
        matches!(
            self.kind(),
            TyKind::Primitive(PrimitiveKind::U8)
                | TyKind::Primitive(PrimitiveKind::U16)
                | TyKind::Primitive(PrimitiveKind::U32)
                | TyKind::Primitive(PrimitiveKind::U64)
                | TyKind::Primitive(PrimitiveKind::U128)
                | TyKind::Primitive(PrimitiveKind::I8)
                | TyKind::Primitive(PrimitiveKind::I16)
                | TyKind::Primitive(PrimitiveKind::I32)
                | TyKind::Primitive(PrimitiveKind::I64)
                | TyKind::Primitive(PrimitiveKind::I128)
                | TyKind::Primitive(PrimitiveKind::F32)
                | TyKind::Primitive(PrimitiveKind::F64)
        )
    }

    /// Check if this type is integer
    pub fn is_integer(&self) -> bool {
        matches!(
            self.kind(),
            TyKind::Primitive(PrimitiveKind::U8)
                | TyKind::Primitive(PrimitiveKind::U16)
                | TyKind::Primitive(PrimitiveKind::U32)
                | TyKind::Primitive(PrimitiveKind::U64)
                | TyKind::Primitive(PrimitiveKind::U128)
                | TyKind::Primitive(PrimitiveKind::I8)
                | TyKind::Primitive(PrimitiveKind::I16)
                | TyKind::Primitive(PrimitiveKind::I32)
                | TyKind::Primitive(PrimitiveKind::I64)
                | TyKind::Primitive(PrimitiveKind::I128)
        )
    }

    /// Check if this type is a string
    pub fn is_string(&self) -> bool {
        matches!(self.kind(), TyKind::Primitive(PrimitiveKind::String))
    }

    /// Check if this type is a range
    pub fn is_range(&self) -> bool {
        matches!(self.kind(), TyKind::Range)
    }

    /// Check if this type is an array
    pub fn is_array(&self) -> bool {
        matches!(self.kind(), TyKind::List(_))
    }

    /// Check if this type is a dictionary
    pub fn is_dict(&self) -> bool {
        matches!(self.kind(), TyKind::Dict(_, _))
    }

    /// Check if this type is a function
    pub fn is_function(&self) -> bool {
        matches!(self.kind(), TyKind::Function(_, _))
    }

    /// Check if this type is a struct
    pub fn is_struct(&self) -> bool {
        matches!(self.kind(), TyKind::Struct(_, _))
    }

    /// Check if this type is an enum
    pub fn is_enum(&self) -> bool {
        matches!(self.kind(), TyKind::Enum(_, _))
    }

    /// Check if this type is an alias
    pub fn is_alias(&self) -> bool {
        matches!(self.kind(), TyKind::Alias(_, _))
    }

    /// Check if this type is generic
    pub fn is_generic(&self) -> bool {
        matches!(self.kind(), TyKind::Generic(_, _))
    }

    /// Check if this is an iterable type
    pub fn is_iterable(&self) -> bool {
        matches!(
            self.kind(),
            TyKind::Range | TyKind::List(_) | TyKind::Dict(_, _)
        )
    }

    /// Check if this type can be implicitly casted to another
    pub fn can_implicit_cast(&self, other: &Self) -> bool {
        if self.id() == other.id() {
            return true;
        }
        match (self.kind(), other.kind()) {
            (TyKind::Unknown, _) | (_, TyKind::Unknown) => true,
            (TyKind::Primitive(a), TyKind::Primitive(b)) => {
                if *a == *b {
                    return true;
                }
                match (a, b) {
                    (PrimitiveKind::U8, other) => matches!(
                        other,
                        PrimitiveKind::U16
                            | PrimitiveKind::U32
                            | PrimitiveKind::U64
                            | PrimitiveKind::U128
                            | PrimitiveKind::I8
                            | PrimitiveKind::I16
                            | PrimitiveKind::I32
                            | PrimitiveKind::I64
                            | PrimitiveKind::I128
                            | PrimitiveKind::F32
                            | PrimitiveKind::F64
                    ),
                    (PrimitiveKind::U16, other) => matches!(
                        other,
                        PrimitiveKind::U32
                            | PrimitiveKind::U64
                            | PrimitiveKind::U128
                            | PrimitiveKind::I16
                            | PrimitiveKind::I32
                            | PrimitiveKind::I64
                            | PrimitiveKind::I128
                            | PrimitiveKind::F32
                            | PrimitiveKind::F64
                    ),
                    (PrimitiveKind::U32, other) => matches!(
                        other,
                        PrimitiveKind::U64
                            | PrimitiveKind::U128
                            | PrimitiveKind::I32
                            | PrimitiveKind::I64
                            | PrimitiveKind::I128
                            | PrimitiveKind::F32
                            | PrimitiveKind::F64
                    ),
                    (PrimitiveKind::U64, other) => matches!(
                        other,
                        PrimitiveKind::U128
                            | PrimitiveKind::I64
                            | PrimitiveKind::I128
                            | PrimitiveKind::F64
                    ),
                    (PrimitiveKind::I8, other) => matches!(
                        other,
                        PrimitiveKind::I16
                            | PrimitiveKind::I32
                            | PrimitiveKind::I64
                            | PrimitiveKind::I128
                            | PrimitiveKind::F32
                            | PrimitiveKind::F64
                    ),
                    (PrimitiveKind::I16, other) => matches!(
                        other,
                        PrimitiveKind::I32
                            | PrimitiveKind::I64
                            | PrimitiveKind::I128
                            | PrimitiveKind::F32
                            | PrimitiveKind::F64
                    ),
                    (PrimitiveKind::I32, other) => matches!(
                        other,
                        PrimitiveKind::I64
                            | PrimitiveKind::I128
                            | PrimitiveKind::F32
                            | PrimitiveKind::F64
                    ),
                    (PrimitiveKind::I64, other) => {
                        matches!(other, PrimitiveKind::I128 | PrimitiveKind::F64)
                    }
                    _ => false,
                }
            }
            (TyKind::List(a_elem), TyKind::List(b_elem)) => a_elem == b_elem,
            (TyKind::Dict(a_key, a_elem), TyKind::Dict(b_key, b_elem)) => {
                a_key == b_key && a_elem == b_elem
            }
            (TyKind::Function(a_def, _), TyKind::Function(b_def, _)) => a_def == b_def,
            (TyKind::Struct(a_def, _), TyKind::Struct(b_def, _)) => a_def == b_def,
            (TyKind::Enum(a_def, _), TyKind::Enum(b_def, _)) => a_def == b_def,
            (TyKind::Alias(a_def, _), TyKind::Alias(b_def, _)) => a_def == b_def,
            _ => false,
        }
    }
}

impl PartialEq for Ty {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
