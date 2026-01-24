use crate::checker::TypeChecker;
use crate::error::{TypeCheckResult, TypeError};
use ahash::AHashMap;
use otterc_ast::{self, Function, Node, Program, Stmt};
use otterc_ident::Identifier;
use otterc_symbol::{Symbol, Visibility};
use otterc_ty::{
    self, AliasDef, EnumDef, FunctionDef, GenericArg, GenericKind, StructDef, Ty, TyId, TyKind,
    TyRef,
};

impl TypeChecker {
    pub(crate) fn register_types(&mut self, ast: &mut Program) -> TypeCheckResult<()> {
        for stmt in &mut ast.statements {
            match stmt.as_mut() {
                Stmt::Function { func, sym } => {
                    *sym = Some(self.register_function(func)?);
                }
                Stmt::Struct {
                    name,
                    fields,
                    methods,
                    visibility,
                    generics,
                    sym,
                } => {
                    *sym = Some(if generics.is_empty() {
                        self.register_struct(*name, fields, methods, *visibility)
                    } else {
                        self.register_generic_struct(*name, fields, methods, generics, *visibility)
                    }?);
                }
                Stmt::Enum {
                    name,
                    variants,
                    generics,
                    visibility,
                    sym,
                    ..
                } => {
                    *sym = Some(if generics.is_empty() {
                        self.register_enum(*name, variants, *visibility)
                    } else {
                        self.register_generic_enum(*name, variants, generics, *visibility)
                    }?);
                }
                Stmt::TypeAlias {
                    name,
                    target,
                    generics,
                    visibility,
                    sym,
                } => {
                    *sym = Some(if generics.is_empty() {
                        self.register_alias(*name, target, *visibility)
                    } else {
                        self.register_generic_alias(*name, target, generics, *visibility)
                    }?);
                }
                Stmt::Use { imports } => {
                    todo!("importing modules not yet implemented");
                }
                Stmt::PubUse {
                    module,
                    item,
                    alias,
                } => {
                    todo!("importing modules not yet implemented");
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn register_function(&mut self, func: &Node<Function>) -> TypeCheckResult<Symbol> {
        let f = func.as_ref();
        let ty = TyKind::Function(self.get_resolved_function_def(func, false)?, None);
        self.registry
            .register_type(Ty::new(f.name, ty), f.visibility)
            .map_err(|err| self.errors.push(err.into()))
    }

    fn register_struct(
        &mut self,
        name: Identifier,
        fields: &[(Identifier, Node<TyRef>)],
        methods: &[Node<Function>],
        visibility: Visibility,
    ) -> TypeCheckResult<Symbol> {
        let mut ty_members = AHashMap::new();
        for (field_ident, field_ty) in fields.iter() {
            let ty_id = self
                .resolve_ref(field_ty.as_ref(), field_ty.span())?
                .ty_id();
            ty_members.insert(*field_ident, ty_id);
        }
        let mut ty_methods = AHashMap::new();
        for method in methods.iter() {
            ty_methods.insert(
                method.as_ref().name,
                self.get_resolved_function_def(method, true)?,
            );
        }
        let ty = TyKind::Struct(StructDef::new(ty_members, ty_methods), None);
        self.registry
            .register_type(Ty::new(name, ty), visibility)
            .map_err(|err| self.errors.push(err.into()))
    }

    fn register_generic_struct(
        &mut self,
        name: Identifier,
        fields: &[(Identifier, Node<TyRef>)],
        methods: &[Node<Function>],
        generics: &[GenericArg],
        visibility: Visibility,
    ) -> TypeCheckResult<Symbol> {
        let mut ty_members = AHashMap::new();
        for (field_ident, field_ty) in fields.iter() {
            ty_members.insert(*field_ident, field_ty.as_ref().clone());
        }
        let mut ty_methods = AHashMap::new();
        for method in methods.iter() {
            ty_methods.insert(method.as_ref().name, self.get_function_def(method, true));
        }
        let ty = TyKind::Generic(
            GenericKind::Struct(StructDef::new(ty_members, ty_methods)),
            generics.into(),
        );
        self.registry
            .register_type(Ty::new(name, ty), visibility)
            .map_err(|err| self.errors.push(err.into()))
    }

    fn register_enum(
        &mut self,
        name: Identifier,
        variants: &[Node<otterc_ast::EnumVariant>],
        visibility: Visibility,
    ) -> TypeCheckResult<Symbol> {
        let mut ty_variants = Vec::new();
        for (discriminant, variant) in variants.iter().enumerate() {
            let mut fields = Vec::new();
            for field in &variant.as_ref().fields {
                let ty_id = self.resolve_ref(field.as_ref(), variant.span())?.ty_id();
                fields.push(ty_id);
            }
            ty_variants.push(otterc_ty::EnumVariant::new(
                discriminant as u32,
                variant.as_ref().name,
                fields,
            ));
        }
        let ty = TyKind::Enum(EnumDef::new(ty_variants), None);
        self.registry
            .register_type(Ty::new(name, ty), visibility)
            .map_err(|err| self.errors.push(err.into()))
    }

    fn register_generic_enum(
        &mut self,
        name: Identifier,
        variants: &[Node<otterc_ast::EnumVariant>],
        generics: &[GenericArg],
        visibility: Visibility,
    ) -> TypeCheckResult<Symbol> {
        let mut ty_variants = Vec::new();
        for (discriminant, variant) in variants.iter().enumerate() {
            let fields = variant
                .as_ref()
                .fields
                .iter()
                .map(|field| field.as_ref().clone())
                .collect::<Vec<TyRef>>();
            ty_variants.push(otterc_ty::EnumVariant::new(
                discriminant as u32,
                variant.as_ref().name,
                fields,
            ));
        }
        let ty = TyKind::Generic(
            GenericKind::Enum(EnumDef::new(ty_variants)),
            generics.into(),
        );
        self.registry
            .register_type(Ty::new(name, ty), visibility)
            .map_err(|err| self.errors.push(err.into()))
    }

    fn register_alias(
        &mut self,
        name: Identifier,
        target: &Node<TyRef>,
        visibility: Visibility,
    ) -> TypeCheckResult<Symbol> {
        let target_id = self.resolve_ref(target.as_ref(), target.span())?.ty_id();
        let ty = TyKind::Alias(AliasDef::new(target_id), None);
        self.registry
            .register_type(Ty::new(name, ty), visibility)
            .map_err(|err| self.errors.push(err.into()))
    }

    fn register_generic_alias(
        &mut self,
        name: Identifier,
        target: &Node<TyRef>,
        generics: &[GenericArg],
        visibility: Visibility,
    ) -> TypeCheckResult<Symbol> {
        let ty = TyKind::Generic(
            GenericKind::Alias(AliasDef::new(target.as_ref().clone())),
            generics.into(),
        );
        self.registry
            .register_type(Ty::new(name, ty), visibility)
            .map_err(|err| self.errors.push(err.into()))
    }

    fn get_function_def(&mut self, func: &Node<Function>, method: bool) -> FunctionDef<TyRef> {
        let f = func.as_ref();
        let params = f
            .params
            .iter()
            .map(|param| {
                let p = param.as_ref();
                (*p.name.as_ref(), p.ty.as_ref().clone())
            })
            .collect::<Vec<(Identifier, TyRef)>>();
        if method && (params.is_empty() || !(params[0].0 == Identifier::from("self"))) {
            self.errors.push(
                TypeError::new("first parameter of method must be 'self'".to_string())
                    .with_span(func.span()),
            );
        }
        let return_type = match f.ret_ty.clone() {
            Some(ty_ref) => ty_ref.as_ref().clone(),
            None => TyRef::from(Identifier::from("unit")),
        };
        FunctionDef::new(params, Some(return_type))
    }

    fn get_resolved_function_def(
        &mut self,
        func: &Node<Function>,
        method: bool,
    ) -> TypeCheckResult<FunctionDef<TyId>> {
        let f = func.as_ref();
        let mut params = vec![];
        for param in f.params.iter() {
            let p = param.as_ref();
            let ident = *p.name.as_ref();
            let ty_ref = p.ty.as_ref().clone();
            let ty_id = self.resolve_ref(&ty_ref, p.ty.span())?.ty_id();
            params.push((ident, ty_id));
        }
        if method && (params.is_empty() || !(params[0].0 == Identifier::from("self"))) {
            self.errors.push(
                TypeError::new("first parameter of method must be 'self'".to_string())
                    .with_span(func.span()),
            );
        }
        let return_type = match &f.ret_ty {
            Some(ret_ty) => self.resolve_ref(ret_ty.as_ref(), ret_ty.span())?.ty_id(),
            None => self.unknown_ty.id(),
        };
        Ok(FunctionDef::new(params, Some(return_type)))
    }
}
