use crate::error::TypeError;
use crate::{checker::TypeChecker, error::TypeCheckResult};
use otterc_ast::{Block, Expr, Function, Node, Stmt};
use otterc_ident::Identifier;
use otterc_span::Span;
use otterc_symbol::{Symbol, SymbolKind, Visibility};
use otterc_ty::{Ty, TyKind, TyRef};

impl TypeChecker {
    pub(crate) fn check_stmt(
        &mut self,
        stmt: &mut Node<Stmt>,
        return_ty: &mut Option<Ty>,
    ) -> TypeCheckResult<()> {
        let span = stmt.span();
        match stmt.as_mut() {
            Stmt::Let {
                name,
                expr,
                ty,
                visibility,
                sym,
            } => self.check_let(*name.as_ref(), expr, ty, *visibility, sym),
            Stmt::Assignment { name, expr } => self.check_assignment(*name.as_ref(), expr, span),
            Stmt::If {
                cond,
                then_block,
                elif_blocks,
                else_block,
            } => self.check_if(cond, then_block, elif_blocks, else_block, return_ty),
            Stmt::For {
                var,
                iterable,
                body,
            } => self.check_for(var, iterable, body, return_ty),
            Stmt::While { cond, body } => self.check_while(cond, body, return_ty),
            Stmt::Return(expr) => self.check_return(expr, return_ty, span),
            Stmt::Function { func, sym } => self.check_function(func, sym, return_ty, span),
            Stmt::Expr(expr) => {
                let _ = self.infer_expr_type(expr);
                Ok(())
            }
            Stmt::Block(body) => {
                self.check_block(body, return_ty)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn check_block(
        &mut self,
        block: &mut Node<Block>,
        return_ty: &mut Option<Ty>,
    ) -> TypeCheckResult<()> {
        self.registry.push_scope().expect("Failed to push scope");
        let stmts_len = block.as_ref().statements.len();
        for (i, stmt) in block.as_mut().statements.iter_mut().enumerate() {
            let mut stmt_return_ty = None;
            let _ = self.check_stmt(stmt, &mut stmt_return_ty);
            match &stmt_return_ty {
                Some(stmt_ret_ty) => match return_ty {
                    Some(ret_ty) => {
                        if !stmt_ret_ty.can_implicit_cast(ret_ty) {
                            self.errors.push(
                                TypeError::new(format!(
                                    "unexpected return type: expected '{}', got '{}'",
                                    ret_ty.name(),
                                    stmt_ret_ty.name()
                                ))
                                .with_span(stmt.span()),
                            );
                        }
                    }
                    None => *return_ty = stmt_return_ty,
                },
                None => {
                    if i == stmts_len - 1
                        && let Some(ret_ty) = return_ty
                        && !ret_ty.is_unknown()
                    {
                        self.errors.push(
                            TypeError::new(format!(
                                "missing return statement: expected return type '{}'",
                                ret_ty.name()
                            ))
                            .with_span(stmt.span()),
                        );
                    }
                }
            }
        }
        self.registry.pop_scope().expect("Failed to pop scope");
        Ok(())
    }

    fn check_let(
        &mut self,
        name: Identifier,
        expr: &mut Node<Expr>,
        ty: &Option<Node<TyRef>>,
        visibility: Visibility,
        sym: &mut Option<Symbol>,
    ) -> TypeCheckResult<()> {
        let var_ty = if let Some(ty_node) = ty {
            let ty_ref = ty_node.as_ref();
            let ty_span = ty_node.span();
            let sym = self.resolve_ref(ty_ref, ty_span)?;
            self.registry.get_type(sym.ty_id()).expect("type not found")
        } else {
            self.infer_expr_type(expr)?
        };
        // Ignore "_" variables
        if name != Identifier::from("_") {
            let new_sym = Symbol::new(name, SymbolKind::Variable, var_ty.id(), visibility);
            if self.registry.register(new_sym.clone()).is_err() {
                self.errors.push(
                    TypeError::new(format!("variable '{}' already defined", name))
                        .with_span(expr.span()),
                );
            }
            *sym = Some(new_sym);
        }
        Ok(())
    }

    fn check_assignment(
        &mut self,
        name: Identifier,
        expr: &mut Node<Expr>,
        span: Span,
    ) -> TypeCheckResult<()> {
        match self.registry.lookup(name) {
            Some(lhs_sym) => {
                let lhs_ty = self
                    .registry
                    .get_type(lhs_sym.ty_id())
                    .expect("Type not found");
                let rhs_ty = self.infer_expr_type(expr)?;
                if rhs_ty.can_implicit_cast(&lhs_ty) {
                    Ok(())
                } else {
                    self.errors.push(
                        TypeError::new(format!(
                            "type mismatch in assignment to '{}': expected '{}', got '{}'",
                            name,
                            lhs_ty.name(),
                            rhs_ty.name()
                        ))
                        .with_span(span),
                    );
                    Err(())
                }
            }
            None => {
                self.errors.push(
                    TypeError::new(format!("undefined variable: '{}'", name)).with_span(span),
                );
                Err(())
            }
        }
    }

    fn check_if(
        &mut self,
        cond: &mut Node<Expr>,
        then_block: &mut Node<Block>,
        elif_blocks: &mut [(Node<Expr>, Node<Block>)],
        else_block: &mut Option<Node<Block>>,
        return_ty: &mut Option<Ty>,
    ) -> TypeCheckResult<()> {
        let cond_ty = self.infer_expr_type(cond)?;
        if !cond_ty.is_bool() {
            self.errors.push(
                TypeError::new(format!(
                    "if condition must be boolean, got '{}'",
                    cond_ty.name()
                ))
                .with_span(cond.span()),
            );
            return Err(());
        }

        let mut block_return_ty = None;
        let _ = self.check_block(then_block, &mut block_return_ty);
        if let Some(block_ty) = block_return_ty
            && let Some(ret_ty) = return_ty
            && !block_ty.can_implicit_cast(ret_ty)
        {
            self.errors.push(
                TypeError::new(format!(
                    "unexpected return type in if block: expected '{}', got '{}'",
                    ret_ty.name(),
                    block_ty.name()
                ))
                .with_span(then_block.span()),
            );
        }

        for (_, elif_block) in elif_blocks {
            let mut block_return_ty = None;
            let _ = self.check_block(elif_block, &mut block_return_ty);
            if let Some(block_ty) = block_return_ty
                && let Some(ret_ty) = return_ty
                && !block_ty.can_implicit_cast(ret_ty)
            {
                self.errors.push(
                    TypeError::new(format!(
                        "unexpected return type in elif block: expected '{}', got '{}'",
                        ret_ty.name(),
                        block_ty.name()
                    ))
                    .with_span(elif_block.span()),
                );
            }
        }

        if let Some(else_block) = else_block {
            let mut block_return_ty = None;
            let _ = self.check_block(else_block, &mut block_return_ty);
            if let Some(block_ty) = block_return_ty
                && let Some(ret_ty) = return_ty
                && !block_ty.can_implicit_cast(ret_ty)
            {
                self.errors.push(
                    TypeError::new(format!(
                        "unexpected return type in else block: expected '{}', got '{}'",
                        ret_ty.name(),
                        block_ty.name()
                    ))
                    .with_span(else_block.span()),
                );
            }
        }
        Ok(())
    }

    fn check_for(
        &mut self,
        var: &mut Node<Identifier>,
        iterable: &mut Node<Expr>,
        body: &mut Node<Block>,
        return_ty: &mut Option<Ty>,
    ) -> TypeCheckResult<()> {
        self.registry
            .push_scope()
            .expect("Failed to push scope for for-loop");
        let iterable_ty = self.infer_expr_type(iterable)?;
        if !iterable_ty.is_iterable() {
            self.errors.push(
                TypeError::new(format!("type '{}' is not iterable", iterable_ty.name()))
                    .with_span(iterable.span()),
            );
            return Err(());
        }
        self.registry
            .register(Symbol::new(
                *var.as_ref(),
                SymbolKind::Variable,
                iterable_ty.id(),
                Visibility::Private,
            ))
            .expect("Failed to register for-loop variable identifier");
        let _ = self.check_block(body, return_ty);
        self.registry
            .pop_scope()
            .expect("Failed to pop scope for for-loop");
        Ok(())
    }

    fn check_while(
        &mut self,
        cond: &mut Node<Expr>,
        body: &mut Node<Block>,
        return_ty: &mut Option<Ty>,
    ) -> TypeCheckResult<()> {
        let cond_ty = self.infer_expr_type(cond)?;
        if !cond_ty.is_bool() {
            self.errors.push(
                TypeError::new(format!(
                    "while condition must be boolean, got '{}'",
                    cond_ty.name()
                ))
                .with_span(cond.span()),
            );
            return Err(());
        }
        self.check_block(body, return_ty)
    }

    fn check_return(
        &mut self,
        expr: &mut Option<Node<Expr>>,
        return_ty: &mut Option<Ty>,
        span: Span,
    ) -> TypeCheckResult<()> {
        let expr_ty = match expr.as_mut() {
            Some(expr) => self.infer_expr_type(expr)?,
            None => self.unknown_ty.clone(),
        };
        match return_ty {
            Some(ret_ty) => {
                if !expr_ty.can_implicit_cast(ret_ty) {
                    self.errors.push(
                        TypeError::new(format!(
                            "return type mismatch: expected {}, got {}",
                            ret_ty.name(),
                            expr_ty.name()
                        ))
                        .with_span(span),
                    );
                    Err(())
                } else {
                    Ok(())
                }
            }
            None => {
                *return_ty = Some(expr_ty);
                Ok(())
            }
        }
    }

    fn check_function(
        &mut self,
        func: &mut Node<Function>,
        sym: &mut Option<Symbol>,
        return_ty: &mut Option<Ty>,
        span: Span,
    ) -> TypeCheckResult<()> {
        let ty = self
            .registry
            .get_type(
                sym.as_ref()
                    .map(|sym| sym.ty_id())
                    .expect("function not registered"),
            )
            .expect("function type not found");
        match ty.kind() {
            TyKind::Function(def, _) => {
                self.registry
                    .push_scope()
                    .expect("Failed to push scope for function");
                let defaults = func
                    .as_ref()
                    .params
                    .iter()
                    .map(|param| {
                        param.as_ref().default.as_ref().and_then(|default_expr| {
                            self.infer_expr_type(&mut default_expr.clone()).ok()
                        })
                    })
                    .collect::<Vec<Option<Ty>>>();
                let mut should_default = false;
                for (i, (param_name, param_ty_id)) in def.params().iter().enumerate() {
                    if let Some(default_ty) = &defaults[i] {
                        should_default = true;
                        let expected_ty = self
                            .registry
                            .get_type(*param_ty_id)
                            .expect("parameter type not found");
                        if !default_ty.can_implicit_cast(&expected_ty) {
                            self.errors.push(
                                TypeError::new(format!(
                                    "parameter '{}' type mismatch in default value: expected '{}', got '{}'",
                                    param_name,
                                    expected_ty.name(),
                                    default_ty.name()
                                ))
                                .with_span(func.as_ref().params[i].span()),
                            );
                        }
                    } else if should_default {
                        self.errors.push(
                            TypeError::new(format!(
                                "required parameter '{}' cannot follow optional parameters",
                                param_name
                            ))
                            .with_span(func.as_ref().params[i].span()),
                        );
                    }
                    self.registry
                        .register(Symbol::new(
                            *param_name,
                            SymbolKind::Variable,
                            *param_ty_id,
                            Visibility::Private,
                        ))
                        .expect("Failed to register function parameter");
                }
                *return_ty = def
                    .return_type()
                    .and_then(|ty_id| self.registry.get_type(ty_id));
                self.check_block(&mut func.as_mut().body, return_ty)?;
                self.registry
                    .pop_scope()
                    .expect("Failed to pop scope for function");
                Ok(())
            }
            _ => {
                self.errors.push(
                    TypeError::new(format!("'{}' is not a function type", ty.name()))
                        .with_span(span),
                );
                Err(())
            }
        }
    }
}
