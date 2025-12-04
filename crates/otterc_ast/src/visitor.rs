use crate::nodes::*;

/// A read-only AST visitor.
pub trait Visitor {
    fn visit_statement(&mut self, _stmt: &Node<Statement>) {}
    fn visit_expression(&mut self, _expr: &Node<Expr>) {}
    fn visit_block(&mut self, _block: &Node<Block>) {}
    fn visit_function(&mut self, _func: &Node<Function>) {}
    fn visit_pattern(&mut self, _pattern: &Node<Pattern>) {}
}

/// Traverse the AST using a read-only visitor.
pub fn visit(program: &Program, visitor: &mut impl Visitor) {
    for stmt in &program.statements {
        visit_statement(stmt, visitor);
    }
}

fn visit_statement(stmt: &Node<Statement>, visitor: &mut impl Visitor) {
    visitor.visit_statement(stmt);
    match stmt.as_ref() {
        Statement::Let { expr, .. } => {
            visit_expression(expr, visitor);
        }
        Statement::Assignment { expr, .. } => {
            visit_expression(expr, visitor);
        }
        Statement::If {
            cond,
            then_block,
            elif_blocks,
            else_block,
        } => {
            visitor.visit_expression(cond);
            visit_block(then_block, visitor);
            for (elif_cond, elif_block) in elif_blocks {
                visit_expression(elif_cond, visitor);
                visit_block(elif_block, visitor);
            }
            if let Some(else_blk) = else_block {
                visit_block(else_blk, visitor);
            }
        }
        Statement::For { iterable, body, .. } => {
            visit_expression(iterable, visitor);
            visit_block(body, visitor);
        }
        Statement::While { cond, body } => {
            visit_expression(cond, visitor);
            visit_block(body, visitor);
        }
        Statement::Return(Some(expr)) => {
            visit_expression(expr, visitor);
        }
        Statement::Struct { methods, .. } => {
            for method in methods {
                visit_function(method, visitor);
            }
        }
        Statement::Expr(expr) => {
            visit_expression(expr, visitor);
        }
        Statement::Block(block) => {
            visit_block(block, visitor);
        }
        _ => {}
    }
}

fn visit_expression(expr: &Node<Expr>, visitor: &mut impl Visitor) {
    visitor.visit_expression(expr);
    match expr.as_ref() {
        Expr::Member { object, .. } => {
            visit_expression(object, visitor);
        }
        Expr::Call { func, args } => {
            visit_expression(func, visitor);
            for arg in args {
                visit_expression(arg, visitor);
            }
        }
        Expr::Binary { left, right, .. } => {
            visit_expression(left, visitor);
            visit_expression(right, visitor);
        }
        Expr::Unary { expr, .. } => {
            visit_expression(expr, visitor);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit_expression(cond, visitor);
            visit_expression(then_branch, visitor);
            if let Some(else_expr) = else_branch {
                visit_expression(else_expr, visitor);
            }
        }
        Expr::Match { value, arms } => {
            visit_expression(value, visitor);
            for arm in arms {
                visit_pattern(&arm.as_ref().pattern, visitor);
                if let Some(guard_expr) = &arm.as_ref().guard {
                    visit_expression(guard_expr, visitor);
                }
                visit_block(&arm.as_ref().body, visitor);
            }
        }
        Expr::Range { start, end } => {
            visit_expression(start, visitor);
            visit_expression(end, visitor);
        }
        Expr::Array(elements) => {
            for element in elements {
                visit_expression(element, visitor);
            }
        }
        Expr::Dict(pairs) => {
            for (key, value) in pairs {
                visit_expression(key, visitor);
                visit_expression(value, visitor);
            }
        }
        Expr::ListComprehension {
            element,
            iterable,
            condition,
            ..
        } => {
            visit_expression(element, visitor);
            visit_expression(iterable, visitor);
            if let Some(cond) = condition {
                visit_expression(cond, visitor);
            }
        }
        Expr::DictComprehension {
            key,
            value,
            iterable,
            condition,
            ..
        } => {
            visit_expression(key, visitor);
            visit_expression(value, visitor);
            visit_expression(iterable, visitor);
            if let Some(cond) = condition {
                visit_expression(cond, visitor);
            }
        }
        Expr::FString { parts } => {
            for part in parts {
                if let FStringPart::Expr(expr) = part.as_ref() {
                    visit_expression(expr, visitor);
                }
            }
        }
        Expr::Await(expr) => {
            visit_expression(expr, visitor);
        }
        Expr::Spawn(expr) => {
            visit_expression(expr, visitor);
        }
        Expr::Struct { fields, .. } => {
            for (_, expr) in fields {
                visit_expression(expr, visitor);
            }
        }
        _ => {}
    }
}

fn visit_block(block: &Node<Block>, visitor: &mut impl Visitor) {
    visitor.visit_block(block);
    for stmt in &block.as_ref().statements {
        visit_statement(stmt, visitor);
    }
}

fn visit_function(func: &Node<Function>, visitor: &mut impl Visitor) {
    visitor.visit_function(func);
    for param in &func.as_ref().params {
        if let Some(default_expr) = &param.as_ref().default {
            visit_expression(default_expr, visitor);
        }
    }
    visit_block(&func.as_ref().body, visitor);
}

fn visit_pattern(pattern: &Node<Pattern>, visitor: &mut impl Visitor) {
    visitor.visit_pattern(pattern);
    match pattern.as_ref() {
        Pattern::EnumVariant { fields, .. } => {
            for field_pattern in fields {
                visit_pattern(field_pattern, visitor);
            }
        }
        Pattern::Struct { fields, .. } => {
            for (_, pattern) in fields {
                if let Some(pattern) = pattern {
                    visit_pattern(pattern, visitor);
                }
            }
        }
        Pattern::Array { patterns, .. } => {
            for pattern in patterns {
                visit_pattern(pattern, visitor);
            }
        }
        _ => {}
    }
}
