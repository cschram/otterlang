use crate::error::{TypeCheckResult, TypeError};
use otterc_ast::Program;
use otterc_ident::Identifier;
use otterc_span::Span;
use otterc_symbol::{Symbol, SymbolRegistry};
use otterc_ty::{Ty, TyKind, TyRef};

#[derive(Debug)]
pub struct TypeChecker {
    pub(crate) registry: SymbolRegistry,
    pub(crate) errors: Vec<TypeError>,
    pub(crate) unknown_ty: Ty,
    pub(crate) range_ty: Ty,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut registry = SymbolRegistry::default();
        registry.register_builtins();
        // These types have no registered symbols, because they are internal types used during type
        // checking.
        let unknown_ty = Ty::new(Identifier::from("unknown"), TyKind::Unknown);
        let range_ty = Ty::new(Identifier::from("range"), TyKind::Range);
        Self {
            registry,
            errors: Vec::new(),
            unknown_ty,
            range_ty,
        }
    }

    pub fn registry(&self) -> &SymbolRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut SymbolRegistry {
        &mut self.registry
    }

    pub fn errors(&self) -> &Vec<TypeError> {
        &self.errors
    }

    pub fn check(&mut self, ast: &mut Program) {
        // First pass: Register types (structs, enums, etc.)
        if self.register_types(ast).is_err() {
            // Return early if we failed to register types.
            return;
        }
        // Second pass: Resolve types, register identifiers, monomorphize generics
        self.check_program(ast);
    }

    fn check_program(&mut self, ast: &mut Program) {
        for stmt in &mut ast.statements {
            let _ = self.check_stmt(stmt, &mut None);
        }
    }

    /// Resolve the type reference, monomorphizing generics as needed.
    pub(crate) fn resolve_ref(&mut self, ty_ref: &TyRef, span: Span) -> TypeCheckResult<Symbol> {
        match self.registry.lookup(ty_ref.name()) {
            Some(sym) => {
                let ty = self.registry.get_type(sym.ty_id()).expect("type not found");
                match ty.kind() {
                    TyKind::Generic(_, _) => {
                        let mut params = vec![];
                        for param in ty_ref.params().iter() {
                            let sym = self.resolve_ref(param, span)?;
                            params.push(sym.id());
                        }
                        self.registry
                            .monomorphize(sym.id(), &params)
                            .map_err(|err| self.errors.push(TypeError::from(err).with_span(span)))
                    }
                    _ => Ok(sym),
                }
            }
            None => {
                self.errors.push(
                    TypeError::new(format!("unable to resolve type '{}'", ty_ref.name()))
                        .with_span(span),
                );
                Err(())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use otterc_lexer::tokenize;
    use otterc_parser::parse;
    use pretty_assertions::assert_eq;

    fn get_errors(source: &str) -> Vec<String> {
        let tokens = tokenize(source).expect("Lexing failed");
        let mut ast = parse(&tokens).expect("Parsing failed");
        let mut checker = TypeChecker::new();
        checker.check(&mut ast);
        let errors = checker.errors();
        assert!(!errors.is_empty(), "Expected type errors");
        errors.iter().map(|e| e.to_string()).collect()
    }

    #[test]
    fn enum_errors() {
        let errs = get_errors(include_str!("../tests/enum.ot"));
        assert_eq!(
            errs,
            &[
                "enum 'Foo' has no variant 'Baz'",
                "enum variant 'Foo.Bar' expects 1 argument(s), got 2",
                "parameter for 'Foo.Bar' expects type 'i64', got 'str'",
            ]
        );
    }

    #[test]
    fn expr_errors() {
        let errs = get_errors(include_str!("../tests/expr.ot"));
        assert_eq!(
            errs,
            &[
                "undefined variable 'foo'",
                "cannot apply + to 'list<i64>' and 'list<i64>'",
                "cannot compare 'str' and 'i64'",
                "logical operations require boolean operands, got 'bool' and 'i64'",
                "modulo requires integer operands, got 'f64' and 'f64'",
                "not operator requires boolean operand, got 'str'",
                "negation requires numeric operand, got 'str'",
                "attempted to call a non-function type 'str'",
                "function expects 1 arguments, got 2",
                "function expects 1 arguments, got 0",
                "parameter 'person' type mismatch: expected 'str', got 'i64'",
                "range bounds must be integers, got 'i64' and 'str'",
                "if condition must be boolean, got 'i64'",
                "no member 'baz' on type 'i64'",
                "array element type mismatch: expected 'i64', got 'f64'",
                "array element type mismatch: expected 'i64', got 'str'",
                "dict value type mismatch: expected 'i64', got 'str'",
                "type 'str' is not iterable",
                "if condition must be boolean, got 'i64'",
                "type 'str' is not iterable",
                "if condition must be boolean, got 'i64'",
                "field 'bar' type mismatch: expected 'i64', got 'str'",
                "struct 'Foo' has no field 'baz'",
            ]
        );
    }

    #[test]
    fn stmt_errors() {
        let errs = get_errors(include_str!("../tests/stmt.ot"));
        assert_eq!(
            errs,
            &[
                "if condition must be boolean, got 'i64'",
                "type 'i64' is not iterable",
                "while condition must be boolean, got 'i64'",
                "unexpected return type: expected 'i64', got 'str'",
                "missing return statement: expected return type 'i64'",
            ]
        );
    }

    #[test]
    fn func_errors() {
        let errs = get_errors(include_str!("../tests/func.ot"));
        assert_eq!(
            errs,
            &[
                "parameter 'foo' type mismatch in default value: expected 'i64', got 'str'",
                "required parameter 'bar' cannot follow optional parameters",
            ]
        );
    }

    // #[test]
    // fn pattern_match() {
    //     let errs = get_errors(include_str!("../tests/match.ot"));
    //     assert_eq!(
    //         errs,
    //         &[
    //             "literal pattern type 'str' does not match expected type 'i64'",
    //             "enum pattern 'Baz' does not match value type 'Foo'",
    //             "enum variant 'Foo.Bar' has 1 field(s), but pattern destructures 2",
    //             "enum 'Foo' has no variant 'Baz'",
    //             "cannot match enum pattern 'Baz' against non-enum type 'i64'",
    //             "struct 'Person' has no field 'age'",
    //             "cannot match struct pattern 'Person { name, age }' against non-struct type 'i64'",
    //             "cannot match array pattern against non-list type 'i64'",
    //         ]
    //     );
    // }
}
