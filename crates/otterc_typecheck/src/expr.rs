use crate::checker::TypeChecker;
use crate::error::{TypeCheckResult, TypeError};
use otterc_ast::{BinaryOp, Expr, FStringPart, Literal, MatchArm, Node, Pattern, UnaryOp};
use otterc_ident::Identifier;
use otterc_span::Span;
use otterc_symbol::{SymId, Symbol, SymbolKind, Visibility};
use otterc_ty::{GenericKind, StructDef, Ty, TyId, TyKind};

impl TypeChecker {
    pub(crate) fn infer_expr_type(&mut self, expr: &mut Node<Expr>) -> TypeCheckResult<Ty> {
        let span = expr.span();
        match expr.as_mut() {
            Expr::Literal(value) => Ok(self.infer_literal_type(value.as_ref())),
            Expr::Identifier(ident) => self.infer_identifier_type(ident, span),
            Expr::Member { object, field } => self.infer_member_type(object, field, span),
            Expr::Call { func, args } => self.infer_call_type(func, args),
            Expr::Binary { left, right, op } => self.infer_binary_op_type(left, right, op, span),
            Expr::Unary { expr, op } => self.infer_unary_op_type(expr, op, span),
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                todo!("if expressions not yet implemented");
            }
            Expr::Match { value, arms } => self.infer_match_type(value, arms),
            Expr::Range { start, end } => self.infer_range_type(start, end, span),
            Expr::Array(expr) => self.infer_array_type(expr),
            Expr::Dict(expr) => self.infer_dict_type(expr),
            Expr::ListComprehension {
                element,
                var,
                iterable,
                condition,
            } => self.infer_list_comprehension_type(element, *var, iterable, condition, span),
            Expr::DictComprehension {
                key,
                value,
                var,
                iterable,
                condition,
            } => self.infer_dict_comprehension_type(key, value, *var, iterable, condition, span),
            Expr::FString { parts } => {
                self.check_fstring(parts);
                Ok(self
                    .registry
                    .lookup_type("str")
                    .expect("Builtin type str not found")
                    .clone())
            }
            Expr::Await(expr) => self.infer_await_type(expr),
            Expr::Spawn(expr) => self.infer_spawn_type(expr),
            Expr::Struct { name, fields } => self.infer_struct_type(*name, fields, span),
            _ => Ok(self.unknown_ty.clone()),
        }
    }

    fn infer_literal_type(&mut self, literal: &Literal) -> Ty {
        match literal {
            Literal::Number(lit) => {
                if lit.is_float_literal {
                    self.registry
                        .lookup_type("f64")
                        .expect("Builtin type f64 not found")
                        .clone()
                } else {
                    self.registry
                        .lookup_type("i64")
                        .expect("Builtin type i64 not found")
                        .clone()
                }
            }
            Literal::Bool(_) => self
                .registry
                .lookup_type("bool")
                .expect("Builtin type bool not found")
                .clone(),
            Literal::String(_) => self
                .registry
                .lookup_type("str")
                .expect("Builtin type str not found")
                .clone(),
            _ => self.unknown_ty.clone(),
        }
    }

    fn infer_identifier_type(&mut self, ident: &mut Identifier, span: Span) -> TypeCheckResult<Ty> {
        if let Some(ty) = self.registry.lookup_type(*ident) {
            Ok(ty.clone())
        } else {
            self.errors
                .push(TypeError::new(format!("undefined variable '{}'", ident)).with_span(span));
            Err(())
        }
    }

    fn infer_member_type(
        &mut self,
        object: &mut Node<Expr>,
        field: &mut Identifier,
        span: otterc_span::Span,
    ) -> TypeCheckResult<Ty> {
        let obj_ty = self.infer_expr_type(object)?;
        if let Some(member_ty_id) = obj_ty.member(field) {
            Ok(self
                .registry
                .get_type(*member_ty_id)
                .expect("member type not found")
                .clone())
        } else {
            self.errors.push(
                TypeError::new(format!("no member '{}' on type '{}'", field, obj_ty.name()))
                    .with_span(span),
            );
            Err(())
        }
    }

    fn infer_call_type(
        &mut self,
        callee: &mut Node<Expr>,
        args: &mut Vec<Node<Expr>>,
    ) -> TypeCheckResult<Ty> {
        if let Expr::Member { object, field } = callee.as_ref()
            && let Expr::Identifier(ident) = object.as_ref().as_ref()
            && let Some(ty) = self.registry.lookup_type(*ident)
            && let TyKind::Enum(def, _) = ty.kind()
        {
            // Check if the field is a variant
            if let Some(variant) = def.variants().iter().find(|v| v.name() == *field) {
                // Check argument count
                if variant.fields().len() != args.len() {
                    self.errors.push(
                        TypeError::new(format!(
                            "enum variant '{}.{}' expects {} argument(s), got {}",
                            ident,
                            field,
                            variant.fields().len(),
                            args.len()
                        ))
                        .with_span(callee.span()),
                    );
                    return Err(());
                }
                // Check argument types
                for (arg, expected_ty_id) in args.iter_mut().zip(variant.fields()) {
                    let arg_ty = self.infer_expr_type(arg)?;
                    let expected_ty = self
                        .registry
                        .get_type(*expected_ty_id)
                        .expect("type not found");
                    if !arg_ty.can_implicit_cast(&expected_ty) {
                        self.errors.push(
                            TypeError::new(format!(
                                "parameter for '{}.{}' expects type '{}', got '{}'",
                                ident,
                                field,
                                expected_ty.name(),
                                arg_ty.name()
                            ))
                            .with_span(arg.span()),
                        );
                    }
                }
                // Return the enum's type as the result type
                return Ok(ty.clone());
            } else {
                self.errors.push(
                    TypeError::new(format!("enum '{}' has no variant '{}'", ident, field))
                        .with_span(callee.span()),
                );
                return Err(());
            }
        }
        let callee_ty = self.infer_expr_type(callee)?;
        if let TyKind::Function(def, _) = callee_ty.kind() {
            // Check argument count
            let params = def.params();
            if params.len() != args.len() {
                self.errors.push(
                    TypeError::new(format!(
                        "function expects {} arguments, got {}",
                        params.len(),
                        args.len()
                    ))
                    .with_span(callee.span()),
                );
            }
            // Check argument types
            for ((param_name, param_ty_id), arg) in params.iter().zip(args.iter_mut()) {
                let arg_ty = self.infer_expr_type(arg)?;
                let param_ty = self
                    .registry
                    .get_type(*param_ty_id)
                    .expect("type not found");
                if arg_ty.id() != param_ty.id() {
                    self.errors.push(
                        TypeError::new(format!(
                            "parameter '{}' type mismatch: expected '{}', got '{}'",
                            param_name,
                            param_ty.name(),
                            arg_ty.name()
                        ))
                        .with_span(arg.span()),
                    );
                }
            }
            return Ok(def
                .return_type()
                .and_then(|ty_id| self.registry.get_type(ty_id))
                .unwrap_or_else(|| self.unknown_ty.clone()));
        }
        self.errors.push(
            TypeError::new(format!(
                "attempted to call a non-function type '{}'",
                callee_ty.name()
            ))
            .with_span(callee.span()),
        );
        Err(())
    }

    fn infer_binary_op_type(
        &mut self,
        left: &mut Node<Expr>,
        right: &mut Node<Expr>,
        op: &BinaryOp,
        span: Span,
    ) -> TypeCheckResult<Ty> {
        let left_ty = self.infer_expr_type(left)?;
        let right_ty = self.infer_expr_type(right)?;
        match op {
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Lt
            | BinaryOp::Gt
            | BinaryOp::LtEq
            | BinaryOp::GtEq => {
                if !(left_ty.is_numeric() || matches!(op, BinaryOp::Add) && left_ty.is_string()) {
                    self.errors.push(
                        TypeError::new(format!(
                            "cannot apply {} to '{}' and '{}'",
                            op,
                            left_ty.name(),
                            right_ty.name()
                        ))
                        .with_span(span),
                    );
                    return Err(());
                }
                if right_ty.can_implicit_cast(&left_ty) {
                    Ok(left_ty)
                } else {
                    self.errors.push(
                        TypeError::new(format!(
                            "binary operation type mismatch: left is '{}', right is '{}'",
                            left_ty.name(),
                            right_ty.name()
                        ))
                        .with_span(span),
                    );
                    Err(())
                }
            }
            BinaryOp::Eq | BinaryOp::Ne => {
                if left_ty.can_implicit_cast(&right_ty) {
                    Ok(self
                        .registry
                        .lookup_type("bool")
                        .expect("Builtin type bool not found")
                        .clone())
                } else {
                    self.errors.push(
                        TypeError::new(format!(
                            "cannot compare '{}' and '{}'",
                            left_ty.name(),
                            right_ty.name()
                        ))
                        .with_span(span),
                    );
                    Err(())
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                if left_ty.is_bool() && right_ty.is_bool() {
                    Ok(self
                        .registry
                        .lookup_type("bool")
                        .expect("Builtin type bool not found")
                        .clone())
                } else {
                    self.errors.push(
                        TypeError::new(format!(
                            "logical operations require boolean operands, got '{}' and '{}'",
                            left_ty.name(),
                            right_ty.name()
                        ))
                        .with_span(span),
                    );
                    Err(())
                }
            }
            BinaryOp::Mod => {
                if left_ty.is_integer() && right_ty.is_integer() {
                    Ok(left_ty)
                } else {
                    self.errors.push(
                        TypeError::new(format!(
                            "modulo requires integer operands, got '{}' and '{}'",
                            left_ty.name(),
                            right_ty.name()
                        ))
                        .with_span(span),
                    );
                    Err(())
                }
            }
            BinaryOp::Is | BinaryOp::IsNot => {
                unimplemented!("type inference for 'is' and 'is not' operators")
            }
        }
    }

    fn infer_unary_op_type(
        &mut self,
        expr: &mut Node<Expr>,
        op: &UnaryOp,
        span: Span,
    ) -> TypeCheckResult<Ty> {
        let expr_ty = self.infer_expr_type(expr)?;
        match op {
            UnaryOp::Neg => {
                if expr_ty.is_numeric() {
                    Ok(expr_ty)
                } else {
                    self.errors.push(
                        TypeError::new(format!(
                            "negation requires numeric operand, got '{}'",
                            expr_ty.name()
                        ))
                        .with_span(span),
                    );
                    Err(())
                }
            }
            UnaryOp::Not => {
                if expr_ty.is_bool() {
                    Ok(expr_ty)
                } else {
                    self.errors.push(
                        TypeError::new(format!(
                            "not operator requires boolean operand, got '{}'",
                            expr_ty.name()
                        ))
                        .with_span(span),
                    );
                    Err(())
                }
            }
        }
    }

    fn infer_match_type(
        &mut self,
        value: &mut Box<Node<Expr>>,
        arms: &mut [Node<MatchArm>],
    ) -> TypeCheckResult<Ty> {
        let value_ty = self.infer_expr_type(value)?;
        let first_arm_ty = if let Some(first_arm) = arms.first_mut() {
            let mut return_ty = None;
            let _ = self.check_block(&mut first_arm.as_mut().body, &mut return_ty);
            return_ty.unwrap_or_else(|| self.unknown_ty.clone())
        } else {
            self.unknown_ty.clone()
        };
        for arm in arms {
            if let Some(pattern_ty) = self.infer_pattern_type(&mut arm.as_mut().pattern)?
                && !pattern_ty.can_implicit_cast(&value_ty)
            {
                self.errors.push(
                    TypeError::new(format!(
                        "match pattern type mismatch: expected '{}', got '{}'",
                        value_ty.name(),
                        pattern_ty.name()
                    ))
                    .with_span(arm.as_ref().pattern.span()),
                );
            }
            let mut return_ty = None;
            let _ = self.check_block(&mut arm.as_mut().body, &mut return_ty);
            let arm_ty = return_ty.unwrap_or_else(|| self.unknown_ty.clone());
            if !arm_ty.can_implicit_cast(&first_arm_ty) {
                self.errors.push(
                    TypeError::new(format!(
                        "mismatched types in match arms: expected '{}', got '{}'",
                        first_arm_ty.name(),
                        arm_ty.name()
                    ))
                    .with_span(arm.as_ref().body.span()),
                );
            }
        }
        Ok(first_arm_ty)
    }

    fn infer_pattern_type(&mut self, pattern: &mut Node<Pattern>) -> TypeCheckResult<Option<Ty>> {
        let span = pattern.span();
        match pattern.as_mut() {
            Pattern::Wildcard => Ok(None),
            Pattern::Literal(lit) => Ok(Some(self.infer_literal_type(lit.as_ref()))),
            Pattern::Identifier(ident) => {
                let sym = self.registry.lookup(*ident).ok_or_else(|| {
                    self.errors.push(
                        TypeError::new(format!("undefined variable '{}'", ident)).with_span(span),
                    );
                })?;
                Ok(Some(
                    self.registry
                        .get_type(sym.ty_id())
                        .expect("type not found")
                        .clone(),
                ))
            }
            _ => todo!("pattern kind not implemented yet"),
        }
    }

    fn infer_range_type(
        &mut self,
        start: &mut Box<Node<Expr>>,
        end: &mut Box<Node<Expr>>,
        span: Span,
    ) -> TypeCheckResult<Ty> {
        let start_ty = self.infer_expr_type(start)?;
        let end_ty = self.infer_expr_type(end)?;
        if !start_ty.is_integer() || !end_ty.is_integer() {
            self.errors.push(
                TypeError::new(format!(
                    "range bounds must be integers, got '{}' and '{}'",
                    start_ty.name(),
                    end_ty.name()
                ))
                .with_span(span),
            );
            Err(())
        } else {
            Ok(self.range_ty.clone())
        }
    }

    fn infer_array_type(&mut self, expr: &mut [Node<Expr>]) -> TypeCheckResult<Ty> {
        let expected_ty = if let Some(first_elem) = expr.first_mut() {
            self.infer_expr_type(first_elem)?
        } else {
            self.unknown_ty.clone()
        };
        for elem in expr.iter_mut().skip(1) {
            let elem_ty = self.infer_expr_type(elem)?;
            if !elem_ty.can_implicit_cast(&expected_ty) {
                self.errors.push(
                    TypeError::new(format!(
                        "array element type mismatch: expected '{}', got '{}'",
                        expected_ty.name(),
                        elem_ty.name()
                    ))
                    .with_span(elem.span()),
                );
            }
        }
        let ty_ident = Identifier::from(format!("list<{}>", expected_ty.name()));
        Ok(self.registry.lookup_type(ty_ident).unwrap_or_else(|| {
            let ty = Ty::new(ty_ident, TyKind::List(expected_ty.id()));
            self.registry
                .register_type(ty.clone(), Visibility::Public)
                .expect("failed to register array type");
            ty
        }))
    }

    fn infer_dict_type(&mut self, expr: &mut [(Node<Expr>, Node<Expr>)]) -> TypeCheckResult<Ty> {
        let expected_key_ty = if let Some((first_key, _)) = expr.first_mut() {
            self.infer_expr_type(first_key)?
        } else {
            self.unknown_ty.clone()
        };
        let expected_value_ty = if let Some((_, first_value)) = expr.first_mut() {
            self.infer_expr_type(first_value)?
        } else {
            self.unknown_ty.clone()
        };
        for (key, value) in expr.iter_mut().skip(1) {
            let key_ty = self.infer_expr_type(key)?;
            if !key_ty.can_implicit_cast(&expected_key_ty) {
                self.errors.push(
                    TypeError::new(format!(
                        "dict key type mismatch: expected '{}', got '{}'",
                        expected_key_ty.name(),
                        key_ty.name()
                    ))
                    .with_span(key.span()),
                );
            }
            let value_ty = self.infer_expr_type(value)?;
            if !value_ty.can_implicit_cast(&expected_value_ty) {
                self.errors.push(
                    TypeError::new(format!(
                        "dict value type mismatch: expected '{}', got '{}'",
                        expected_value_ty.name(),
                        value_ty.name()
                    ))
                    .with_span(value.span()),
                );
            }
        }
        let ty_ident = Identifier::from(format!(
            "dict<{},{}>",
            expected_key_ty.name(),
            expected_value_ty.name()
        ));
        Ok(self.registry.lookup_type(ty_ident).unwrap_or_else(|| {
            let ty = Ty::new(
                ty_ident,
                TyKind::Dict(expected_key_ty.id(), expected_value_ty.id()),
            );
            self.registry
                .register_type(ty.clone(), Visibility::Public)
                .expect("failed to register dict type");
            ty
        }))
    }

    fn infer_list_comprehension_type(
        &mut self,
        element: &mut Box<Node<Expr>>,
        var: Identifier,
        iterable: &mut Box<Node<Expr>>,
        condition: &mut Option<Box<Node<Expr>>>,
        span: Span,
    ) -> TypeCheckResult<Ty> {
        self.registry.push_scope().unwrap();
        let iter_ty = self.infer_expr_type(iterable)?;
        if self
            .registry
            .register(Symbol::new(
                var,
                SymbolKind::Variable,
                iter_ty.id(),
                Visibility::Private,
            ))
            .is_err()
        {
            self.errors.push(
                TypeError::new(format!("variable '{}' is already defined", var)).with_span(span),
            );
        }
        if !iter_ty.is_iterable() {
            self.errors.push(
                TypeError::new(format!("type '{}' is not iterable", iter_ty.name()))
                    .with_span(iterable.span()),
            );
        }
        let elem_ty = self.infer_expr_type(element)?;
        if !elem_ty.can_implicit_cast(&iter_ty) {
            self.errors.push(
                TypeError::new(format!(
                    "element type mismatch: expected '{}', got '{}'",
                    iter_ty.name(),
                    elem_ty.name()
                ))
                .with_span(iterable.span()),
            );
        }
        if let Some(condition) = condition
            && let Ok(cond_ty) = self.infer_expr_type(condition)
            && !cond_ty.is_bool()
        {
            self.errors.push(
                TypeError::new(format!(
                    "if condition must be boolean, got '{}'",
                    cond_ty.name()
                ))
                .with_span(condition.span()),
            );
        }
        let ty_ident = Identifier::from(format!("list<{}>", elem_ty.name()));
        let ty = self.registry.lookup_type(ty_ident).unwrap_or_else(|| {
            let ty = Ty::new(ty_ident, TyKind::List(elem_ty.id()));
            self.registry
                .register_type(ty.clone(), Visibility::Public)
                .expect("failed to register list comprehension type");
            ty
        });
        self.registry.pop_scope().unwrap();
        Ok(ty)
    }

    fn infer_dict_comprehension_type(
        &mut self,
        key: &mut Box<Node<Expr>>,
        value: &mut Box<Node<Expr>>,
        var: Identifier,
        iterable: &mut Box<Node<Expr>>,
        condition: &mut Option<Box<Node<Expr>>>,
        span: Span,
    ) -> TypeCheckResult<Ty> {
        self.registry.push_scope().unwrap();
        let iter_ty = self.infer_expr_type(iterable)?;
        if !iter_ty.is_iterable() {
            self.errors.push(
                TypeError::new(format!("type '{}' is not iterable", iter_ty.name()))
                    .with_span(iterable.span()),
            );
        }
        let var_ty = if iter_ty.is_range() {
            self.registry
                .lookup_type("i64")
                .expect("Builtin type i64 not found")
                .clone()
        } else {
            iter_ty
        };
        if self
            .registry
            .register(Symbol::new(
                var,
                SymbolKind::Variable,
                var_ty.id(),
                Visibility::Private,
            ))
            .is_err()
        {
            self.errors.push(
                TypeError::new(format!("variable '{}' is already defined", var)).with_span(span),
            );
        }
        let key_ty = self.infer_expr_type(key)?;
        let value_ty = self.infer_expr_type(value)?;
        if let Some(condition) = condition
            && let Ok(cond_ty) = self.infer_expr_type(condition)
            && !cond_ty.is_bool()
        {
            self.errors.push(
                TypeError::new(format!(
                    "if condition must be boolean, got '{}'",
                    cond_ty.name()
                ))
                .with_span(condition.span()),
            );
        }
        let ty_ident = Identifier::from(format!("dict<{},{}>", key_ty.name(), value_ty.name()));
        let ty = self.registry.lookup_type(ty_ident).unwrap_or_else(|| {
            let ty = Ty::new(ty_ident, TyKind::Dict(key_ty.id(), value_ty.id()));
            self.registry
                .register_type(ty.clone(), Visibility::Public)
                .expect("failed to register dict comprehension type");
            ty
        });
        self.registry.pop_scope().unwrap();
        Ok(ty)
    }

    fn check_fstring(&mut self, parts: &mut [Node<FStringPart>]) {
        for part in parts {
            if let FStringPart::Expr(expr) = part.as_mut() {
                let _ = self.infer_expr_type(expr);
            }
        }
    }

    fn infer_await_type(&mut self, expr: &mut Box<Node<Expr>>) -> TypeCheckResult<Ty> {
        todo!("await type inference not yet implemented");
    }

    fn infer_spawn_type(&mut self, expr: &mut Box<Node<Expr>>) -> TypeCheckResult<Ty> {
        todo!("spawn type inference not yet implemented");
    }

    fn infer_struct_type(
        &mut self,
        name: Identifier,
        fields: &mut [(Identifier, Node<Expr>)],
        span: Span,
    ) -> TypeCheckResult<Ty> {
        let sym = self.registry.lookup(name).ok_or_else(|| {
            self.errors
                .push(TypeError::new(format!("undefined type '{}'", name)).with_span(span));
        })?;
        let struct_ty = self.registry.get_type(sym.ty_id()).expect("type not found");
        match struct_ty.kind() {
            TyKind::Struct(def, _) => {
                self.check_struct_construction(name, def, fields);
                Ok(struct_ty.clone())
            }
            TyKind::Generic(GenericKind::Struct(def), generic_args) => {
                let members = def.members();
                let mut ty_params: Vec<Option<SymId>> = Vec::new();
                ty_params.resize(members.len(), None);
                for (field_name, field_expr) in fields.iter_mut() {
                    if let Ok(expr_ty) = self.infer_expr_type(field_expr) {
                        match members.get(field_name) {
                            Some(member_ty_ref) => {
                                let generic_arg = generic_args
                                    .iter()
                                    .enumerate()
                                    .find(|(_, arg)| arg.name() == member_ty_ref.name());
                                match generic_arg {
                                    Some((arg_pos, _)) => {
                                        let expr_ty_sym = self
                                            .registry
                                            .lookup(expr_ty.name())
                                            .expect("type not found");
                                        ty_params[arg_pos] = Some(expr_ty_sym.id());
                                    }
                                    None => {
                                        if let Ok(field_sym) =
                                            self.resolve_ref(member_ty_ref, field_expr.span())
                                            && let Some(field_ty) =
                                                self.registry.get_type(field_sym.ty_id())
                                            && !expr_ty.can_implicit_cast(&field_ty)
                                        {
                                            self.errors.push(
                                                TypeError::new(format!(
                                                    "field '{}' type mismatch: expected '{}', got '{}'",
                                                    field_name,
                                                    field_ty.name(),
                                                    expr_ty.name()
                                                ))
                                                .with_span(field_expr.span()),
                                            );
                                        }
                                    }
                                }
                            }
                            None => {
                                self.errors.push(
                                    TypeError::new(format!(
                                        "struct '{}' has no field '{}'",
                                        name, field_name
                                    ))
                                    .with_span(field_expr.span()),
                                );
                            }
                        }
                    }
                }
                let mono_params: Vec<SymId> = ty_params.into_iter().flatten().collect();
                if mono_params.len() == members.len() {
                    let mono_sym = self
                        .registry
                        .monomorphize(sym.id(), &mono_params)
                        .expect("error monomorphizing struct");
                    let mono_ty = self
                        .registry
                        .get_type(mono_sym.ty_id())
                        .expect("type not found");
                    Ok(mono_ty.clone())
                } else {
                    Err(())
                }
            }
            _ => {
                self.errors.push(
                    TypeError::new(format!("type '{}' is not a struct", struct_ty.name()))
                        .with_span(span),
                );
                Err(())
            }
        }
    }

    fn check_struct_construction(
        &mut self,
        struct_name: Identifier,
        def: &StructDef<TyId>,
        fields: &mut [(Identifier, Node<Expr>)],
    ) {
        let members = def.members();
        for (field_name, field_expr) in fields.iter_mut() {
            match members.get(field_name) {
                Some(member_ty_id) => {
                    let member_ty = self
                        .registry
                        .get_type(*member_ty_id)
                        .expect("type not found");
                    if let Ok(expr_ty) = self.infer_expr_type(field_expr)
                        && !expr_ty.can_implicit_cast(&member_ty)
                    {
                        self.errors.push(
                            TypeError::new(format!(
                                "field '{}' type mismatch: expected '{}', got '{}'",
                                field_name,
                                member_ty.name(),
                                expr_ty.name()
                            ))
                            .with_span(field_expr.span()),
                        );
                    }
                }
                None => {
                    self.errors.push(
                        TypeError::new(format!(
                            "struct '{}' has no field '{}'",
                            struct_name, field_name
                        ))
                        .with_span(field_expr.span()),
                    );
                }
            }
        }
    }
}
