use otterc_ident::Identifier;
use otterc_ty::TyId;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_SYM_ID: AtomicU32 = AtomicU32::new(0);

/// Unique identifier for a type
#[derive(Debug, Clone, Copy)]
pub struct SymId(u32, Identifier);

impl SymId {
    pub fn new(name: Identifier) -> Self {
        let id = NEXT_SYM_ID.fetch_add(1, Ordering::SeqCst);
        Self(id, name)
    }
}

impl PartialEq for SymId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SymId {}

impl Hash for SymId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl std::fmt::Display for SymId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SymId({}, {})", self.0, self.1)
    }
}

/// A symbol kind (type, function, or variable).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Type,
}

/// Type visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Public
    Public,
    /// Private
    Private,
}

/// A symbol in the symbol registry.
#[derive(Debug, Clone)]
pub struct Symbol {
    id: SymId,
    name: Identifier,
    kind: SymbolKind,
    ty_id: TyId,
    visibility: Visibility,
}

impl Symbol {
    /// Create a new symbol.
    pub fn new(name: Identifier, kind: SymbolKind, ty_id: TyId, visibility: Visibility) -> Self {
        Self {
            id: SymId::new(name),
            name,
            kind,
            ty_id,
            visibility,
        }
    }

    /// Get the unique identifier of the symbol.
    pub fn id(&self) -> SymId {
        self.id
    }

    /// Get the name of the symbol.
    pub fn name(&self) -> Identifier {
        self.name
    }

    /// Get the kind of symbol.
    pub fn kind(&self) -> SymbolKind {
        self.kind
    }

    /// Get the type id of the symbol,
    pub fn ty_id(&self) -> TyId {
        self.ty_id
    }

    /// Get the visibility of the symbol.
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Symbol({:?}, {})", self.kind, self.name)
    }
}
