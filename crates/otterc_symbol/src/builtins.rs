/// WIP: This is temporary - builtin registration happens in otterc_runtime/src/stdlib/builtins.rs.
/// This is to simplify getting builtins into the symbol registry while working on this refactor.
use crate::registry::SymbolRegistry;
use crate::symbol::Visibility;
use otterc_ident::Identifier;
use otterc_ty::{FunctionDef, PrimitiveKind, Ty, TyKind};

static PRIMITIVE_TYPES: &[(&str, PrimitiveKind)] = &[
    ("bool", PrimitiveKind::Bool),
    ("u8", PrimitiveKind::U8),
    ("u16", PrimitiveKind::U16),
    ("u32", PrimitiveKind::U32),
    ("u64", PrimitiveKind::U64),
    ("u128", PrimitiveKind::U128),
    ("i8", PrimitiveKind::I8),
    ("i16", PrimitiveKind::I16),
    ("i32", PrimitiveKind::I32),
    ("i64", PrimitiveKind::I64),
    ("i128", PrimitiveKind::I128),
    ("f32", PrimitiveKind::F32),
    ("f64", PrimitiveKind::F64),
    ("str", PrimitiveKind::String),
];

impl SymbolRegistry {
    pub fn register_builtins(&mut self) {
        // Primitives
        for (name, primitive_ty) in PRIMITIVE_TYPES {
            self.register_type(
                Ty::new(Identifier::from(*name), TyKind::Primitive(*primitive_ty)),
                Visibility::Public,
            )
            .expect("Failed to register primitive type");
        }

        // Functions
        // print(str)
        self.register_type(
            Ty::new(
                "print",
                TyKind::Function(
                    FunctionDef::new(
                        vec![(
                            Identifier::from("message"),
                            self.lookup_type("str").unwrap().id(),
                        )],
                        None,
                    ),
                    None,
                ),
            ),
            Visibility::Public,
        )
        .expect("Failed to register builtin function 'print'");
    }
}
