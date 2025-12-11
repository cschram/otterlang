//! AST visitor implementation.

use crate::nodes::*;

/// A read-only AST visitor.
pub trait Visitor<E> {
    /// Visit a statement node.
    fn visit_statement(&mut self, _stmt: &Node<Statement>) -> Result<(), E> {
        Ok(())
    }

    /// Visit an expression node.
    fn visit_expression(&mut self, _expr: &Node<Expr>) -> Result<(), E> {
        Ok(())
    }

    /// Visit a block node.
    fn visit_block(&mut self, _block: &Node<Block>) -> Result<(), E> {
        Ok(())
    }

    /// Visit a function node.
    fn visit_function(&mut self, _func: &Node<Function>) -> Result<(), E> {
        Ok(())
    }

    /// Visit a pattern node.
    fn visit_pattern(&mut self, _pattern: &Node<Pattern>) -> Result<(), E> {
        Ok(())
    }
}

/// Traverse the AST using a read-only visitor.
pub fn visit<E>(program: &Program, visitor: &mut impl Visitor<E>) -> Result<(), E> {
    for stmt in &program.statements {
        visit_statement(stmt, visitor)?;
    }
    Ok(())
}

fn visit_statement<E>(stmt: &Node<Statement>, visitor: &mut impl Visitor<E>) -> Result<(), E> {
    visitor.visit_statement(stmt)?;
    match stmt.as_ref() {
        Statement::Let { expr, .. }
        | Statement::Assignment { expr, .. }
        | Statement::Expr(expr)
        | Statement::Return(Some(expr)) => {
            visit_expression(expr, visitor)?;
        }
        Statement::If {
            cond,
            then_block,
            elif_blocks,
            else_block,
        } => {
            visitor.visit_expression(cond)?;
            visit_block(then_block, visitor)?;
            for (elif_cond, elif_block) in elif_blocks {
                visit_expression(elif_cond, visitor)?;
                visit_block(elif_block, visitor)?;
            }
            if let Some(else_blk) = else_block {
                visit_block(else_blk, visitor)?;
            }
        }
        Statement::For { iterable, body, .. } => {
            visit_expression(iterable, visitor)?;
            visit_block(body, visitor)?;
        }
        Statement::While { cond, body } => {
            visit_expression(cond, visitor)?;
            visit_block(body, visitor)?;
        }
        Statement::Struct { methods, .. } => {
            for method in methods {
                visit_function(method, visitor)?;
            }
        }
        Statement::Block(block) => {
            visit_block(block, visitor)?;
        }
        _ => {}
    }
    Ok(())
}

fn visit_expression<E>(expr: &Node<Expr>, visitor: &mut impl Visitor<E>) -> Result<(), E> {
    visitor.visit_expression(expr)?;
    match expr.as_ref() {
        Expr::Member { object, .. } => {
            visit_expression(object, visitor)?;
        }
        Expr::Call { func, args } => {
            visit_expression(func, visitor)?;
            for arg in args {
                visit_expression(arg, visitor)?;
            }
        }
        Expr::Binary { left, right, .. } => {
            visit_expression(left, visitor)?;
            visit_expression(right, visitor)?;
        }
        Expr::Unary { expr, .. } | Expr::Await(expr) | Expr::Spawn(expr) => {
            visit_expression(expr, visitor)?;
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit_expression(cond, visitor)?;
            visit_expression(then_branch, visitor)?;
            if let Some(else_expr) = else_branch {
                visit_expression(else_expr, visitor)?;
            }
        }
        Expr::Match { value, arms } => {
            visit_expression(value, visitor)?;
            for arm in arms {
                visit_pattern(&arm.as_ref().pattern, visitor)?;
                if let Some(guard_expr) = &arm.as_ref().guard {
                    visit_expression(guard_expr, visitor)?;
                }
                visit_block(&arm.as_ref().body, visitor)?;
            }
        }
        Expr::Range { start, end } => {
            visit_expression(start, visitor)?;
            visit_expression(end, visitor)?;
        }
        Expr::Array(elements) => {
            for element in elements {
                visit_expression(element, visitor)?;
            }
        }
        Expr::Dict(pairs) => {
            for (key, value) in pairs {
                visit_expression(key, visitor)?;
                visit_expression(value, visitor)?;
            }
        }
        Expr::ListComprehension {
            element,
            iterable,
            condition,
            ..
        } => {
            visit_expression(element, visitor)?;
            visit_expression(iterable, visitor)?;
            if let Some(cond) = condition {
                visit_expression(cond, visitor)?;
            }
        }
        Expr::DictComprehension {
            key,
            value,
            iterable,
            condition,
            ..
        } => {
            visit_expression(key, visitor)?;
            visit_expression(value, visitor)?;
            visit_expression(iterable, visitor)?;
            if let Some(cond) = condition {
                visit_expression(cond, visitor)?;
            }
        }
        Expr::FString { parts } => {
            for part in parts {
                if let FStringPart::Expr(expr) = part.as_ref() {
                    visit_expression(expr, visitor)?;
                }
            }
        }
        Expr::Struct { fields, .. } => {
            for (_, expr) in fields {
                visit_expression(expr, visitor)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn visit_block<E>(block: &Node<Block>, visitor: &mut impl Visitor<E>) -> Result<(), E> {
    visitor.visit_block(block)?;
    for stmt in &block.as_ref().statements {
        visit_statement(stmt, visitor)?;
    }
    Ok(())
}

fn visit_function<E>(func: &Node<Function>, visitor: &mut impl Visitor<E>) -> Result<(), E> {
    visitor.visit_function(func)?;
    for param in &func.as_ref().params {
        if let Some(default_expr) = &param.as_ref().default {
            visit_expression(default_expr, visitor)?;
        }
    }
    visit_block(&func.as_ref().body, visitor)?;
    Ok(())
}

fn visit_pattern<E>(pattern: &Node<Pattern>, visitor: &mut impl Visitor<E>) -> Result<(), E> {
    visitor.visit_pattern(pattern)?;
    match pattern.as_ref() {
        Pattern::EnumVariant { fields, .. } => {
            for field_pattern in fields {
                visit_pattern(field_pattern, visitor)?;
            }
        }
        Pattern::Struct { fields, .. } => {
            for (_, pattern) in fields {
                if let Some(pattern) = pattern {
                    visit_pattern(pattern, visitor)?;
                }
            }
        }
        Pattern::Array { patterns, .. } => {
            for pattern in patterns {
                visit_pattern(pattern, visitor)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// A mutable AST visitor.
pub trait VisitorMut<E> {
    /// Visit a statement node.
    fn visit_statement(&mut self, _stmt: &mut Node<Statement>) -> Result<(), E> {
        Ok(())
    }

    /// Visit an expression node.
    fn visit_expression(&mut self, _expr: &mut Node<Expr>) -> Result<(), E> {
        Ok(())
    }

    /// Visit a block node.
    fn visit_block(&mut self, _block: &mut Node<Block>) -> Result<(), E> {
        Ok(())
    }

    /// Visit a function node.
    fn visit_function(&mut self, _func: &mut Node<Function>) -> Result<(), E> {
        Ok(())
    }

    /// Visit a pattern node.
    fn visit_pattern(&mut self, _pattern: &mut Node<Pattern>) -> Result<(), E> {
        Ok(())
    }
}

/// Mutably traverse the AST using a visitor.
pub fn visit_mut<E>(program: &mut Program, visitor: &mut impl VisitorMut<E>) -> Result<(), E> {
    for stmt in &mut program.statements {
        visit_statement_mut(stmt, visitor)?;
    }
    Ok(())
}

fn visit_statement_mut<E>(
    stmt: &mut Node<Statement>,
    visitor: &mut impl VisitorMut<E>,
) -> Result<(), E> {
    visitor.visit_statement(stmt)?;
    match stmt.as_mut() {
        Statement::Let { expr, .. }
        | Statement::Assignment { expr, .. }
        | Statement::Return(Some(expr))
        | Statement::Expr(expr) => {
            visit_expression_mut(expr, visitor)?;
        }
        Statement::If {
            cond,
            then_block,
            elif_blocks,
            else_block,
        } => {
            visitor.visit_expression(cond)?;
            visit_block_mut(then_block, visitor)?;
            for (elif_cond, elif_block) in elif_blocks {
                visit_expression_mut(elif_cond, visitor)?;
                visit_block_mut(elif_block, visitor)?;
            }
            if let Some(else_blk) = else_block {
                visit_block_mut(else_blk, visitor)?;
            }
        }
        Statement::For { iterable, body, .. } => {
            visit_expression_mut(iterable, visitor)?;
            visit_block_mut(body, visitor)?;
        }
        Statement::While { cond, body } => {
            visit_expression_mut(cond, visitor)?;
            visit_block_mut(body, visitor)?;
        }
        Statement::Struct { methods, .. } => {
            for method in methods {
                visit_function_mut(method, visitor)?;
            }
        }
        Statement::Block(block) => {
            visit_block_mut(block, visitor)?;
        }
        _ => {}
    }
    Ok(())
}

fn visit_expression_mut<E>(
    expr: &mut Node<Expr>,
    visitor: &mut impl VisitorMut<E>,
) -> Result<(), E> {
    visitor.visit_expression(expr)?;
    match expr.as_mut() {
        Expr::Member { object, .. } => {
            visit_expression_mut(object, visitor)?;
        }
        Expr::Call { func, args } => {
            visit_expression_mut(func, visitor)?;
            for arg in args {
                visit_expression_mut(arg, visitor)?;
            }
        }
        Expr::Binary { left, right, .. } => {
            visit_expression_mut(left, visitor)?;
            visit_expression_mut(right, visitor)?;
        }
        Expr::Unary { expr, .. } | Expr::Await(expr) | Expr::Spawn(expr) => {
            visit_expression_mut(expr, visitor)?;
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit_expression_mut(cond, visitor)?;
            visit_expression_mut(then_branch, visitor)?;
            if let Some(else_expr) = else_branch {
                visit_expression_mut(else_expr, visitor)?;
            }
        }
        Expr::Match { value, arms } => {
            visit_expression_mut(value, visitor)?;
            for arm in arms {
                visit_pattern_mut(&mut arm.as_mut().pattern, visitor)?;
                if let Some(guard_expr) = &mut arm.as_mut().guard {
                    visit_expression_mut(guard_expr, visitor)?;
                }
                visit_block_mut(&mut arm.as_mut().body, visitor)?;
            }
        }
        Expr::Range { start, end } => {
            visit_expression_mut(start, visitor)?;
            visit_expression_mut(end, visitor)?;
        }
        Expr::Array(elements) => {
            for element in elements {
                visit_expression_mut(element, visitor)?;
            }
        }
        Expr::Dict(pairs) => {
            for (key, value) in pairs {
                visit_expression_mut(key, visitor)?;
                visit_expression_mut(value, visitor)?;
            }
        }
        Expr::ListComprehension {
            element,
            iterable,
            condition,
            ..
        } => {
            visit_expression_mut(element, visitor)?;
            visit_expression_mut(iterable, visitor)?;
            if let Some(cond) = condition {
                visit_expression_mut(cond, visitor)?;
            }
        }
        Expr::DictComprehension {
            key,
            value,
            iterable,
            condition,
            ..
        } => {
            visit_expression_mut(key, visitor)?;
            visit_expression_mut(value, visitor)?;
            visit_expression_mut(iterable, visitor)?;
            if let Some(cond) = condition {
                visit_expression_mut(cond, visitor)?;
            }
        }
        Expr::FString { parts } => {
            for part in parts {
                if let FStringPart::Expr(expr) = part.as_mut() {
                    visit_expression_mut(expr, visitor)?;
                }
            }
        }
        Expr::Struct { fields, .. } => {
            for (_, expr) in fields {
                visit_expression_mut(expr, visitor)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn visit_block_mut<E>(block: &mut Node<Block>, visitor: &mut impl VisitorMut<E>) -> Result<(), E> {
    visitor.visit_block(block)?;
    for stmt in &mut block.as_mut().statements {
        visit_statement_mut(stmt, visitor)?;
    }
    Ok(())
}

fn visit_function_mut<E>(
    func: &mut Node<Function>,
    visitor: &mut impl VisitorMut<E>,
) -> Result<(), E> {
    visitor.visit_function(func)?;
    for param in &mut func.as_mut().params {
        if let Some(default_expr) = &mut param.as_mut().default {
            visit_expression_mut(default_expr, visitor)?;
        }
    }
    visit_block_mut(&mut func.as_mut().body, visitor)?;
    Ok(())
}

fn visit_pattern_mut<E>(
    pattern: &mut Node<Pattern>,
    visitor: &mut impl VisitorMut<E>,
) -> Result<(), E> {
    visitor.visit_pattern(pattern)?;
    match pattern.as_mut() {
        Pattern::EnumVariant { fields, .. } => {
            for field_pattern in fields {
                visit_pattern_mut(field_pattern, visitor)?;
            }
        }
        Pattern::Struct { fields, .. } => {
            for (_, pattern) in fields {
                if let Some(pattern) = pattern {
                    visit_pattern_mut(pattern, visitor)?;
                }
            }
        }
        Pattern::Array { patterns, .. } => {
            for pattern in patterns {
                visit_pattern_mut(pattern, visitor)?;
            }
        }
        _ => {}
    }
    Ok(())
}
