use crate::error::SymbolError;
use crate::symbol::{SymId, Symbol, SymbolKind, Visibility};
use ahash::AHashMap;
use otterc_ident::{self, Identifier};
use otterc_ty::*;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU32, Ordering};

const MAX_SCOPE_DEPTH: usize = 1024;
static NEXT_SCOPE_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(u32);

impl Default for ScopeId {
    fn default() -> Self {
        let id = NEXT_SCOPE_ID.fetch_add(1, Ordering::SeqCst);
        Self(id)
    }
}

#[derive(Debug)]
struct Scope(Option<ScopeId>, AHashMap<SymId, Symbol>);

impl Scope {
    fn parent(&self) -> Option<ScopeId> {
        self.0
    }

    fn symbol(&self, sym_id: SymId) -> Option<Symbol> {
        self.1.get(&sym_id).cloned()
    }

    fn lookup(&self, ident: Identifier) -> Option<Symbol> {
        self.1.values().find(|sym| sym.name() == ident).cloned()
    }

    fn register(&mut self, symbol: Symbol) -> Result<(), SymbolError> {
        if self.1.contains_key(&symbol.id()) {
            Err(SymbolError::SymbolAlreadyExists(symbol.name()))
        } else {
            self.1.insert(symbol.id(), symbol);
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct SymbolRegistry {
    types: RwLock<AHashMap<TyId, Ty>>,
    scopes: RwLock<AHashMap<ScopeId, Scope>>,
    current_scope: RwLock<ScopeId>,
    root_scope: ScopeId,
    scope_depth: RwLock<usize>,
}

impl Default for SymbolRegistry {
    fn default() -> Self {
        let mut scopes = AHashMap::new();
        let root_scope = ScopeId::default();
        scopes.insert(root_scope, Scope(None, AHashMap::new()));
        Self {
            types: RwLock::new(AHashMap::new()),
            scopes: RwLock::new(scopes),
            current_scope: RwLock::new(root_scope),
            root_scope,
            scope_depth: RwLock::new(1),
        }
    }
}
impl SymbolRegistry {
    /// Push a new scope onto the scope stack
    pub fn push_scope(&mut self) -> Result<ScopeId, SymbolError> {
        let mut scope_depth = self.scope_depth.write();
        if *scope_depth == MAX_SCOPE_DEPTH {
            return Err(SymbolError::MaxScopeDepthExceeded(MAX_SCOPE_DEPTH));
        }
        let new_scope_id = ScopeId::default();
        let mut current_scope = self.current_scope.write();
        self.scopes
            .write()
            .insert(new_scope_id, Scope(Some(*current_scope), AHashMap::new()));
        *current_scope = new_scope_id;
        *scope_depth += 1;
        Ok(new_scope_id)
    }

    /// Pop the current scope from the scope stack
    pub fn pop_scope(&mut self) -> Result<(), SymbolError> {
        let mut scope_depth = self.scope_depth.write();
        let mut current_scope = self.current_scope.write();
        *current_scope = self
            .scopes
            .read()
            .get(&current_scope)
            .and_then(|scope| scope.parent())
            .unwrap();
        *scope_depth -= 1;
        Ok(())
    }

    /// Enter a specific scope.
    pub fn enter_scope(&mut self, scope_id: ScopeId) -> Result<ScopeId, SymbolError> {
        let scopes = self.scopes.read();
        if scopes.contains_key(&scope_id) {
            let mut current_scope = self.current_scope.write();
            let old_scope_id = *current_scope;
            *current_scope = scope_id;
            Ok(old_scope_id)
        } else {
            Err(SymbolError::ScopeNotFound)
        }
    }

    /// Register a new symbol.
    pub fn register(&mut self, symbol: Symbol) -> Result<SymId, SymbolError> {
        let current_scope_id = *self.current_scope.read();
        let mut scopes = self.scopes.write();
        let scope = scopes.get_mut(&current_scope_id).unwrap();
        let id = symbol.id();
        if scope.symbol(symbol.id()).is_some() {
            Err(SymbolError::SymbolAlreadyExists(symbol.name()))
        } else {
            scope.register(symbol)?;
            Ok(id)
        }
    }

    /// Get a symbol by its scoped identifier.
    pub fn get_symbol(&self, sym_id: SymId) -> Option<Symbol> {
        let mut current_scope_id = Some(*self.current_scope.read());
        while let Some(scope_id) = current_scope_id {
            let scopes = self.scopes.read();
            let scope = scopes.get(&scope_id)?;
            if let Some(sym) = scope.symbol(sym_id) {
                return Some(sym);
            } else {
                current_scope_id = scope.parent();
            }
        }
        None
    }

    /// Get the type of a symbol.
    pub fn get_symbol_type(&self, sym_id: SymId) -> Option<Ty> {
        let sym = self.get_symbol(sym_id)?;
        self.get_type(sym.ty_id())
    }

    /// Lookup a symbol by its identifier.
    pub fn lookup(&self, name: impl Into<Identifier>) -> Option<Symbol> {
        let name = name.into();
        let mut current_scope_id = Some(*self.current_scope.read());
        while let Some(scope_id) = current_scope_id {
            let scopes = self.scopes.read();
            let scope = scopes.get(&scope_id)?;
            if let Some(sym) = scope.lookup(name) {
                return Some(sym);
            } else {
                current_scope_id = scope.parent();
            }
        }
        None
    }

    /// Register a new type. Creates a matching symbol for the type.
    pub fn register_type(&mut self, ty: Ty, visibility: Visibility) -> Result<Symbol, SymbolError> {
        let ty_id = ty.id();
        let ty_name = ty.name();
        let mut types = self.types.write();
        if types.contains_key(&ty_id) {
            return Err(SymbolError::TypeAlreadyExists(ty_id, ty_name));
        }
        let mut scopes = self.scopes.write();
        let current_scope_id = *self.current_scope.read();
        let current_scope = scopes.get_mut(&current_scope_id).unwrap();
        if current_scope.lookup(ty_name).is_some() {
            return Err(SymbolError::SymbolAlreadyExists(ty_name));
        }
        types.insert(ty_id, ty);
        let sym = Symbol::new(ty_name, SymbolKind::Type, ty_id, visibility);
        current_scope.register(sym.clone())?;
        Ok(sym)
    }

    /// Register an internal type that cannot be referenced by name.
    pub fn register_internal_type(&mut self, ty: Ty) -> TyId {
        let ty_id = ty.id();
        self.types.write().insert(ty_id, ty);
        ty_id
    }

    /// Get a type by its id.
    pub fn get_type(&self, ty_id: TyId) -> Option<Ty> {
        self.types.read().get(&ty_id).cloned()
    }

    /// Lookup a type by its identifier.
    pub fn lookup_type(&self, name: impl Into<Identifier>) -> Option<Ty> {
        let sym = self.lookup(name.into())?;
        self.get_type(sym.ty_id())
    }

    /// Monomorphize a generic type with given type parameters.
    pub fn monomorphize(&mut self, base: SymId, params: &[SymId]) -> Result<Symbol, SymbolError> {
        // Assumes types are always defined in the root scope, and enters that scope for
        // monomorphization.
        let previous_scope = self.enter_scope(self.root_scope)?;
        let result = self.monomorphize_in_scope(base, params);
        self.enter_scope(previous_scope)?;
        result
    }

    /// Handles the monomorphization logic, wrapped by `monomorphize` to ensure scope is exited,
    /// regardless of whether this returns a success or error.
    fn monomorphize_in_scope(
        &mut self,
        base_id: SymId,
        params: &[SymId],
    ) -> Result<Symbol, SymbolError> {
        let base_sym = self
            .get_symbol(base_id)
            .expect("Missing base symbol for monomorphization");
        let base_ty = self
            .get_type(base_sym.ty_id())
            .expect("Missing base type for monomorphization");
        let visibility = base_sym.visibility();
        match base_ty.kind() {
            TyKind::Generic(kind, args) => {
                if args.len() != params.len() {
                    return Err(SymbolError::TypeParameterMismatch(args.len(), params.len()));
                }
                let param_tys = params
                    .iter()
                    .map(|id| {
                        let symbol = self
                            .get_symbol(*id)
                            .expect("Missing symbol for monomorphization");
                        let ty = self
                            .get_type(symbol.ty_id())
                            .expect("Failed to get type for monomorphization");
                        (symbol, ty)
                    })
                    .collect::<Vec<(Symbol, Ty)>>();
                let ident = Identifier::from(format!(
                    "{}<{}>",
                    base_sym.name(),
                    param_tys
                        .iter()
                        .map(|(sym, _)| sym.name().to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                ));
                // Check for existing monomorphization
                if let Some(sym) = self.lookup(ident) {
                    return Ok(sym);
                }
                // Create new monomorphized type
                let mono_ty = match kind {
                    GenericKind::Function(def) => TyKind::Function(
                        self.monomorphize_function_def(def, args, params)?,
                        Some(base_ty.id()),
                    ),
                    GenericKind::Struct(def) => TyKind::Struct(
                        self.monomorphize_struct_def(def, args, params)?,
                        Some(base_ty.id()),
                    ),
                    GenericKind::Enum(def) => TyKind::Enum(
                        self.monomorphize_enum_def(def, args, params)?,
                        Some(base_ty.id()),
                    ),
                    GenericKind::Alias(def) => {
                        let sym = self
                            .lookup_generic(args, params, def.target())
                            .ok_or(SymbolError::TypeNotFound(def.target().name()))?;
                        TyKind::Alias(AliasDef::new(sym.ty_id()), Some(base_ty.id()))
                    }
                };
                self.register_type(Ty::new(ident, mono_ty), visibility)
            }
            _ => Err(SymbolError::CannotMonomorphizeNonGeneric(
                base_sym.ty_id(),
                base_sym.name(),
            )),
        }
    }

    fn monomorphize_function_def(
        &self,
        def: &FunctionDef<TyRef>,
        generic_names: &[GenericArg],
        generic_tys: &[SymId],
    ) -> Result<FunctionDef<TyId>, SymbolError> {
        let mut mono_params = Vec::new();
        for (param_name, param_ty_ref) in def.params() {
            let sym = self
                .lookup_generic(generic_names, generic_tys, param_ty_ref)
                .ok_or(SymbolError::TypeNotFound(param_ty_ref.name()))?;
            mono_params.push((*param_name, sym.ty_id()));
        }
        let mono_return_type = def
            .return_type()
            .as_ref()
            .and_then(|ty| self.lookup_generic(generic_names, generic_tys, ty))
            .map(|sym| sym.ty_id());
        Ok(FunctionDef::new(mono_params, mono_return_type))
    }

    fn monomorphize_struct_def(
        &self,
        def: &StructDef<TyRef>,
        generic_names: &[GenericArg],
        generic_tys: &[SymId],
    ) -> Result<StructDef<TyId>, SymbolError> {
        let mut mono_members = AHashMap::new();
        for (member_name, member_ty_ref) in def.members() {
            let sym = self
                .lookup_generic(generic_names, generic_tys, member_ty_ref)
                .ok_or(SymbolError::TypeNotFound(member_ty_ref.name()))?;
            mono_members.insert(*member_name, sym.ty_id());
        }
        let mut mono_methods = AHashMap::new();
        for (method_name, method_def) in def.methods() {
            let mono_def =
                self.monomorphize_function_def(method_def, generic_names, generic_tys)?;
            mono_methods.insert(*method_name, mono_def);
        }
        Ok(StructDef::new(mono_members, mono_methods))
    }

    fn monomorphize_enum_def(
        &self,
        def: &EnumDef<TyRef>,
        generic_names: &[GenericArg],
        generic_tys: &[SymId],
    ) -> Result<EnumDef<TyId>, SymbolError> {
        let mut mono_variants = Vec::new();
        for variant in def.variants() {
            let mut mono_fields = Vec::new();
            for field_ty_ref in variant.fields() {
                let sym = self
                    .lookup_generic(generic_names, generic_tys, field_ty_ref)
                    .ok_or(SymbolError::TypeNotFound(field_ty_ref.name()))?;
                mono_fields.push(sym.ty_id());
            }
            mono_variants.push(EnumVariant::new(
                variant.discriminant(),
                variant.name(),
                mono_fields,
            ));
        }
        Ok(EnumDef::new(mono_variants))
    }

    /// Lookup a type reference in the context of generic parameters.
    /// If an identifier matches a generic parameter, the corresponding resolved TyRef is returned. Otherwise
    /// the type is looked up by ident in the registry.
    fn lookup_generic(
        &self,
        generic_names: &[GenericArg],
        generic_tys: &[SymId],
        ty_ref: &TyRef,
    ) -> Option<Symbol> {
        generic_names
            .iter()
            .enumerate()
            .find(|(_, g)| g.name() == ty_ref.name())
            .and_then(|(i, _)| self.get_symbol(generic_tys[i]))
            .or_else(|| {
                // If not a generic, look up the type by ident
                self.lookup(ty_ref.name())
            })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, reason = "Tests should panic on failure")]

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn monomorphize_function() {
        let mut registry = SymbolRegistry::default();
        registry.register_builtins();

        // Register generic function
        let func_sym = registry
            .register_type(
                Ty::new(
                    Identifier::from("my_function"),
                    TyKind::Generic(
                        GenericKind::Function(FunctionDef::new(
                            vec![
                                (Identifier::from("arg1"), TyRef::from("T")),
                                (Identifier::from("arg2"), TyRef::from("U")),
                            ],
                            Some(TyRef::from("T")),
                        )),
                        vec![GenericArg::from("T"), GenericArg::from("U")],
                    ),
                ),
                Visibility::Public,
            )
            .unwrap();

        // Fetch generic types for args
        let u8_sym = registry.lookup(Identifier::from("u8")).unwrap();
        let i32_sym = registry.lookup(Identifier::from("i32")).unwrap();

        // Monomorphize function
        let mono_sym = registry
            .monomorphize(func_sym.id(), &[u8_sym.id(), i32_sym.id()])
            .unwrap();
        let mono_ty = registry.get_type(mono_sym.ty_id()).unwrap();

        assert_eq!(mono_sym.name(), Identifier::from("my_function<u8, i32>"));
        match mono_ty.kind() {
            TyKind::Function(def, base_id) => {
                let params = def.params();
                let return_type = def.return_type();
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].1, u8_sym.ty_id());
                assert_eq!(params[1].1, i32_sym.ty_id());
                assert_eq!(*return_type, Some(u8_sym.ty_id()));
                assert_eq!(*base_id, Some(func_sym.ty_id()));
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn monomorphize_struct() {
        let mut registry = SymbolRegistry::default();
        registry.register_builtins();

        // Register generic struct
        let struct_sym = registry
            .register_type(
                Ty::new(
                    Identifier::from("Pair"),
                    TyKind::Generic(
                        GenericKind::Struct(StructDef::new(
                            AHashMap::from([
                                (Identifier::from("first"), TyRef::from("T")),
                                (Identifier::from("second"), TyRef::from("U")),
                            ]),
                            AHashMap::from([
                                (Identifier::from("swap"), FunctionDef::new(vec![], None)),
                                (
                                    Identifier::from("foo"),
                                    FunctionDef::new(
                                        vec![(Identifier::from("value"), TyRef::from("T"))],
                                        Some(TyRef::from("U")),
                                    ),
                                ),
                            ]),
                        )),
                        vec![GenericArg::from("T"), GenericArg::from("U")],
                    ),
                ),
                Visibility::Public,
            )
            .unwrap();

        // Fetch generic types for members
        let u8_sym = registry.lookup(Identifier::from("u8")).unwrap();
        let i32_sym = registry.lookup(Identifier::from("i32")).unwrap();

        // Monomorphize struct
        let mono_sym = registry
            .monomorphize(struct_sym.id(), &[u8_sym.id(), i32_sym.id()])
            .unwrap();
        let mono_ty = registry.get_type(mono_sym.ty_id()).unwrap();

        assert_eq!(mono_sym.name(), Identifier::from("Pair<u8, i32>"));
        match mono_ty.kind() {
            TyKind::Struct(def, base_id) => {
                let members = def.members();
                let methods = def.methods();
                assert_eq!(members.len(), 2);
                assert_eq!(
                    members.get(&Identifier::from("first")).unwrap(),
                    &u8_sym.ty_id()
                );
                assert_eq!(
                    members.get(&Identifier::from("second")).unwrap(),
                    &i32_sym.ty_id()
                );
                assert_eq!(methods.len(), 2);
                let swap_def = methods.get(&Identifier::from("swap")).unwrap();
                assert_eq!(swap_def.params().len(), 0);
                assert_eq!(*swap_def.return_type(), None);
                let foo_def = methods.get(&Identifier::from("foo")).unwrap();
                assert_eq!(foo_def.params().len(), 1);
                assert_eq!(foo_def.params()[0].1, u8_sym.ty_id());
                assert_eq!(*foo_def.return_type(), Some(i32_sym.ty_id()));
                assert_eq!(*base_id, Some(struct_sym.ty_id()));
            }
            _ => panic!("Expected struct type"),
        }
    }

    #[test]
    fn monomorphize_enum() {
        let mut registry = SymbolRegistry::default();
        registry.register_builtins();

        // Register generic enum
        let enum_sym = registry
            .register_type(
                Ty::new(
                    Identifier::from("Option"),
                    TyKind::Generic(
                        GenericKind::Enum(EnumDef::new(vec![
                            EnumVariant::new(0, Identifier::from("None"), vec![]),
                            EnumVariant::new(1, Identifier::from("Some"), vec![TyRef::from("T")]),
                        ])),
                        vec![GenericArg::from("T")],
                    ),
                ),
                Visibility::Public,
            )
            .unwrap();

        // Fetch generic type for variant
        let u8_sym = registry.lookup(Identifier::from("u8")).unwrap();

        // Monomorphize enum
        let mono_sym = registry
            .monomorphize(enum_sym.id(), &[u8_sym.id()])
            .unwrap();
        let mono_ty = registry.get_type(mono_sym.ty_id()).unwrap();

        assert_eq!(mono_sym.name(), Identifier::from("Option<u8>"));
        match mono_ty.kind() {
            TyKind::Enum(def, base_id) => {
                let variants = def.variants();
                assert_eq!(variants.len(), 2);
                let none_variant = &variants[0];
                assert_eq!(none_variant.name(), Identifier::from("None"));
                assert_eq!(none_variant.fields().len(), 0);
                let some_variant = &variants[1];
                assert_eq!(some_variant.name(), Identifier::from("Some"));
                assert_eq!(some_variant.fields().len(), 1);
                let field_ty_id = some_variant.fields()[0];
                assert_eq!(field_ty_id, u8_sym.ty_id());
                assert_eq!(*base_id, Some(enum_sym.ty_id()));
            }
            _ => panic!("Expected enum type"),
        }
    }

    #[test]
    fn monomorphize_alias() {
        let mut registry = SymbolRegistry::default();
        registry.register_builtins();

        // Register generic alias
        let alias_sym = registry
            .register_type(
                Ty::new(
                    Identifier::from("MyAlias"),
                    TyKind::Generic(
                        GenericKind::Alias(AliasDef::new(TyRef::from("T"))),
                        vec![GenericArg::from("T")],
                    ),
                ),
                Visibility::Public,
            )
            .unwrap();

        // Fetch generic type for alias
        let u8_sym = registry.lookup(Identifier::from("u8")).unwrap();

        // Monomorphize alias
        let mono_sym = registry
            .monomorphize(alias_sym.id(), &[u8_sym.id()])
            .unwrap();
        let mono_ty = registry.get_type(mono_sym.ty_id()).unwrap();

        assert_eq!(mono_sym.name(), Identifier::from("MyAlias<u8>"));
        match mono_ty.kind() {
            TyKind::Alias(def, base_id) => {
                let target_ty_id = def.target();
                assert_eq!(*target_ty_id, u8_sym.ty_id());
                assert_eq!(*base_id, Some(alias_sym.ty_id()));
            }
            _ => panic!("Expected alias type"),
        }
    }
}
