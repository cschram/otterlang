use otterc_ast::nodes::{
    BinaryOp, Block, Expr, FStringPart, Function, Literal, Node, Pattern, Program, Stmt, UnaryOp,
};
use otterc_ty::TyRef;

/// Formats OtterLang code
pub struct Formatter {
    indent_size: usize,
}

impl Formatter {
    pub fn new() -> Self {
        Self { indent_size: 4 }
    }

    pub fn with_indent_size(indent_size: usize) -> Self {
        Self { indent_size }
    }

    /// Format a program
    pub fn format_program(&self, program: &Program) -> String {
        let mut output = String::new();
        for (i, stmt) in program.statements.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            output.push_str(&self.format_statement(stmt, 0));
        }
        output
    }

    fn format_statement(&self, stmt: &Node<Stmt>, indent: usize) -> String {
        match stmt.as_ref() {
            Stmt::Let {
                name,
                ty,
                expr,
                public,
                ..
            } => {
                let pub_str = if *public { "pub " } else { "" };
                let ty_str = ty
                    .as_ref()
                    .map(|ty| format!(": {}", self.format_type(ty.as_ref())))
                    .unwrap_or_default();
                format!(
                    "{}{}let {}{} = {}\n",
                    self.indent(indent),
                    pub_str,
                    name,
                    ty_str,
                    self.format_expr(expr, indent)
                )
            }
            Stmt::Assignment { name, expr, .. } => {
                format!(
                    "{}{} = {}\n",
                    self.indent(indent),
                    name,
                    self.format_expr(expr, indent)
                )
            }
            Stmt::Function { func, .. } => self.format_function(func, indent),
            Stmt::If {
                cond,
                then_block,
                elif_blocks,
                else_block,
            } => self.format_if(cond, then_block, elif_blocks, else_block, indent),
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                format!(
                    "{}for {} in {}:\n{}",
                    self.indent(indent),
                    var,
                    self.format_expr(iterable, indent),
                    self.format_block(body, indent + 1)
                )
            }
            Stmt::While { cond, body } => {
                format!(
                    "{}while {}:\n{}",
                    self.indent(indent),
                    self.format_expr(cond, indent),
                    self.format_block(body, indent + 1)
                )
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    format!(
                        "{}return {}\n",
                        self.indent(indent),
                        self.format_expr(expr, indent)
                    )
                } else {
                    format!("{}return\n", self.indent(indent))
                }
            }
            Stmt::Break => format!("{}break\n", self.indent(indent)),
            Stmt::Continue => format!("{}continue\n", self.indent(indent)),
            Stmt::Pass => format!("{}pass\n", self.indent(indent)),
            Stmt::Expr(expr) => {
                format!(
                    "{}{}\n",
                    self.indent(indent),
                    self.format_expr(expr, indent)
                )
            }
            Stmt::Struct {
                name,
                fields,
                methods,
                public,
                generics,
                ..
            } => {
                let pub_str = if *public { "pub " } else { "" };
                let gen_str = if generics.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<{}>",
                        generics
                            .iter()
                            .map(|g| g.as_str())
                            .collect::<Vec<&str>>()
                            .join(", ")
                    )
                };
                let mut result = format!(
                    "{}{}struct {}{}:\n",
                    self.indent(indent),
                    pub_str,
                    name,
                    gen_str
                );
                for (field_name, field_type) in fields {
                    result.push_str(&format!(
                        "{}    {}: {}\n",
                        self.indent(indent),
                        field_name,
                        self.format_type(field_type.as_ref())
                    ));
                }
                for method in methods {
                    result.push_str(&self.format_function(method, indent + 1));
                }
                result
            }
            Stmt::Enum {
                name,
                variants,
                public,
                generics,
                ..
            } => {
                let pub_str = if *public { "pub " } else { "" };
                let gen_str = if generics.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<{}>",
                        generics
                            .iter()
                            .map(|g| g.as_str())
                            .collect::<Vec<&str>>()
                            .join(", ")
                    )
                };
                let mut result = format!(
                    "{}{}enum {}{}:\n",
                    self.indent(indent),
                    pub_str,
                    name,
                    gen_str
                );
                for variant in variants {
                    if variant.as_ref().fields.is_empty() {
                        result.push_str(&format!(
                            "{}    {}\n",
                            self.indent(indent),
                            variant.as_ref().name
                        ));
                    } else {
                        let fields = variant
                            .as_ref()
                            .fields
                            .iter()
                            .map(|ty| self.format_type(ty.as_ref()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        result.push_str(&format!(
                            "{}    {}: ({})\n",
                            self.indent(indent),
                            variant.as_ref().name,
                            fields
                        ));
                    }
                }
                result
            }
            Stmt::TypeAlias {
                name,
                target,
                public,
                generics,
                ..
            } => {
                let pub_str = if *public { "pub " } else { "" };
                let gen_str = if generics.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<{}>",
                        generics
                            .iter()
                            .map(|g| g.as_str())
                            .collect::<Vec<&str>>()
                            .join(", ")
                    )
                };
                format!(
                    "{}{}type {}{} = {}\n",
                    self.indent(indent),
                    pub_str,
                    name,
                    gen_str,
                    self.format_type(target.as_ref())
                )
            }
            Stmt::Use { imports } => {
                let modules: Vec<String> = imports
                    .iter()
                    .map(|import| {
                        if let Some(alias) = &import.as_ref().alias {
                            format!("{} as {}", import.as_ref().module, alias)
                        } else {
                            import.as_ref().module.to_string()
                        }
                    })
                    .collect();
                format!("{}use {}\n", self.indent(indent), modules.join(", "))
            }
            Stmt::PubUse {
                module,
                item,
                alias,
            } => {
                let mut re_export = format!("pub use {}", module);
                if let Some(item_name) = item {
                    re_export.push_str(&format!(".{}", item_name));
                }
                if let Some(alias_name) = alias {
                    re_export.push_str(&format!(" as {}", alias_name));
                }
                format!("{}{}\n", self.indent(indent), re_export)
            }
            Stmt::Block(block) => self.format_block(block, indent),
        }
    }

    fn format_function(&self, f: &Node<Function>, indent: usize) -> String {
        let pub_str = if f.as_ref().public { "pub " } else { "" };
        let params_str = f
            .as_ref()
            .params
            .iter()
            .map(|node| {
                let param = node.as_ref();
                let base = format!("{}: {}", param.name, self.format_type(param.ty.as_ref()));
                if let Some(default) = &param.default {
                    format!("{} = {}", base, self.format_expr(default, indent))
                } else {
                    base
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let ret_str = if let Some(ref ret_ty) = f.as_ref().ret_ty {
            format!(" -> {}", self.format_type(ret_ty.as_ref()))
        } else {
            String::new()
        };
        format!(
            "{}{}fn {}({}){}:\n{}",
            self.indent(indent),
            pub_str,
            f.as_ref().name,
            params_str,
            ret_str,
            self.format_block(&f.as_ref().body, indent + 1)
        )
    }

    fn format_block(&self, block: &Node<Block>, indent: usize) -> String {
        let mut result = String::new();
        for stmt in &block.as_ref().statements {
            result.push_str(&self.format_statement(stmt, indent));
        }
        result
    }

    fn format_if(
        &self,
        cond: &Node<Expr>,
        then_block: &Node<Block>,
        elif_blocks: &[(Node<Expr>, Node<Block>)],
        else_block: &Option<Node<Block>>,
        indent: usize,
    ) -> String {
        let mut result = format!(
            "{}if {}:\n{}",
            self.indent(indent),
            self.format_expr(cond, indent),
            self.format_block(then_block, indent + 1)
        );
        for (elif_cond, elif_block) in elif_blocks {
            result.push_str(&format!(
                "{}elif {}:\n{}",
                self.indent(indent),
                self.format_expr(elif_cond, indent),
                self.format_block(elif_block, indent + 1)
            ));
        }
        if let Some(else_block) = else_block {
            result.push_str(&format!(
                "{}else:\n{}",
                self.indent(indent),
                self.format_block(else_block, indent + 1)
            ));
        }
        result
    }

    fn format_expr(&self, expr: &Node<Expr>, indent: usize) -> String {
        match expr.as_ref() {
            Expr::Literal(lit) => self.format_literal(lit),
            Expr::Identifier(name) => name.to_string(),
            Expr::Binary { op, left, right } => {
                format!(
                    "{} {} {}",
                    self.format_expr(left, indent),
                    self.format_binary_op(op),
                    self.format_expr(right, indent)
                )
            }
            Expr::Unary { op, expr } => {
                format!(
                    "{}{}",
                    self.format_unary_op(op),
                    self.format_expr(expr, indent)
                )
            }
            Expr::Call { func, args } => {
                let args_str = args
                    .iter()
                    .map(|arg| self.format_expr(arg, indent))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", self.format_expr(func, indent), args_str)
            }
            Expr::Member { object, field } => {
                format!("{}.{}", self.format_expr(object, indent), field)
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                format!(
                    "{} if {} else {}",
                    self.format_expr(then_branch, indent),
                    self.format_expr(cond, indent),
                    self.format_expr(else_branch, indent),
                )
            }
            Expr::Range { start, end } => {
                format!(
                    "{}..{}",
                    self.format_expr(start, indent),
                    self.format_expr(end, indent)
                )
            }
            Expr::Array(elements) => {
                let elements_str = elements
                    .iter()
                    .map(|e| self.format_expr(e, indent))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", elements_str)
            }
            Expr::Dict(pairs) => {
                let pairs_str = pairs
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}: {}",
                            self.format_expr(k, indent),
                            self.format_expr(v, indent)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", pairs_str)
            }
            Expr::ListComprehension {
                element,
                var,
                iterable,
                condition,
            } => {
                let cond_str = condition
                    .as_ref()
                    .map(|cond| format!(" if {}", self.format_expr(cond, indent)))
                    .unwrap_or_default();
                format!(
                    "[{} for {} in {}{}]",
                    self.format_expr(element, indent),
                    var,
                    self.format_expr(iterable, indent),
                    cond_str
                )
            }
            Expr::DictComprehension {
                key,
                value,
                var,
                iterable,
                condition,
            } => {
                let cond_str = condition
                    .as_ref()
                    .map(|cond| format!(" if {}", self.format_expr(cond, indent)))
                    .unwrap_or_default();
                format!(
                    "{{{}: {} for {} in {}{}}}",
                    self.format_expr(key, indent),
                    self.format_expr(value, indent),
                    var,
                    self.format_expr(iterable, indent),
                    cond_str
                )
            }
            Expr::Match { value, arms } => {
                let mut result = format!("match {}:\n", self.format_expr(value, indent));
                for arm in arms {
                    result.push_str(&format!(
                        "{}    case {} => {}\n",
                        self.indent(indent),
                        self.format_pattern(&arm.as_ref().pattern),
                        self.format_block(&arm.as_ref().body, indent + 1)
                    ));
                }
                result
            }
            Expr::Struct { name, fields } => {
                // Pythonic style: Point(x=1.0, y=2.0)
                let fields_str = fields
                    .iter()
                    .map(|(fname, val)| format!("{}={}", fname, self.format_expr(val, indent)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", name, fields_str)
            }
            // Lambda expressions removed - use anonymous fn syntax instead
            Expr::Await(expr) => format!("await {}", self.format_expr(expr, indent)),
            Expr::Spawn(expr) => format!("spawn {}", self.format_expr(expr, indent)),
            Expr::FString { parts } => {
                let parts_str = parts
                    .iter()
                    .map(|part| match part.as_ref() {
                        FStringPart::Text(s) => s.clone(),
                        FStringPart::Expr(e) => {
                            format!("{{{}}}", self.format_expr(e, indent))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");
                format!("f\"{}\"", parts_str)
            }
        }
    }

    fn format_pattern(&self, pattern: &Node<Pattern>) -> String {
        match pattern.as_ref() {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Literal(lit) => self.format_literal(lit),
            Pattern::Identifier(name) => name.to_string(),
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
            } => {
                if fields.is_empty() {
                    format!("{}.{}", enum_name, variant)
                } else {
                    let inner = fields
                        .iter()
                        .map(|p| self.format_pattern(p))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}.{}({})", enum_name, variant, inner)
                }
            }
            Pattern::Struct { name, fields } => {
                let fields_str = fields
                    .iter()
                    .map(|(f, p_opt)| {
                        if let Some(p) = p_opt {
                            format!("{}: {}", f, self.format_pattern(p))
                        } else {
                            f.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", name, fields_str)
            }
            Pattern::Array { patterns, rest } => {
                let patterns_str = patterns
                    .iter()
                    .map(|p| self.format_pattern(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                let rest_str = if let Some(rest_var) = rest {
                    format!(", ..{}", rest_var)
                } else {
                    String::new()
                };
                format!("[{}{}]", patterns_str, rest_str)
            }
        }
    }

    fn format_literal(&self, lit: &Node<Literal>) -> String {
        match lit.as_ref() {
            Literal::Number(n) => {
                if !n.is_float_literal && n.value.fract() == 0.0 {
                    format!("{}", n.value as i64)
                } else {
                    n.value.to_string()
                }
            }
            Literal::Bool(b) => b.to_string(),
            Literal::String(s) => format!("\"{}\"", s),
            Literal::None => "None".to_string(),
            Literal::Unit => "()".to_string(),
        }
    }

    fn format_type(&self, ty: &TyRef) -> String {
        match ty {
            TyRef::Unresolved { ident, params } => {
                if params.is_empty() {
                    ident.to_string()
                } else {
                    let params_str = params
                        .iter()
                        .map(|ty_ref| self.format_type(ty_ref))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}<{}>", ident, params_str)
                }
            }
            TyRef::Resolved(ty_id) => ty_id.to_string(),
        }
    }

    fn format_binary_op(&self, op: &BinaryOp) -> &str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Mul => "*",
            BinaryOp::Sub => "-",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::Is => "is",
            BinaryOp::IsNot => "is not",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
        }
    }

    fn format_unary_op(&self, op: &UnaryOp) -> &str {
        match op {
            UnaryOp::Not => "not ",
            UnaryOp::Neg => "-",
        }
    }

    fn indent(&self, level: usize) -> String {
        " ".repeat(level * self.indent_size)
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}
