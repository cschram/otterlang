use std::fmt;

use abi_stable::StableAbi;
use abi_stable::std_types::RVec;

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, Hash, StableAbi)]
pub enum FfiType {
    Unit,
    Bool,
    I32,
    I64,
    F64,
    Str,
    Opaque,
    List,
    Map,
    Struct { fields: RVec<FfiType> },
    Tuple(RVec<FfiType>),
}

impl fmt::Display for FfiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FfiType::Unit => write!(f, "unit"),
            FfiType::Bool => write!(f, "bool"),
            FfiType::I32 => write!(f, "i32"),
            FfiType::I64 => write!(f, "i64"),
            FfiType::F64 => write!(f, "f64"),
            FfiType::Str => write!(f, "str"),
            FfiType::Opaque => write!(f, "opaque"),
            FfiType::List => write!(f, "list"),
            FfiType::Map => write!(f, "map"),
            FfiType::Struct { fields } => {
                write!(f, "struct {{")?;
                for (idx, field) in fields.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field)?;
                }
                write!(f, "}}")
            }
            FfiType::Tuple(fields) => {
                write!(f, "(")?;
                for (idx, field) in fields.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field)?;
                }
                write!(f, ")")
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FfiSignature {
    pub params: Vec<FfiType>,
    pub result: FfiType,
}

impl FfiSignature {
    pub fn new(params: Vec<FfiType>, result: FfiType) -> Self {
        Self { params, result }
    }
}

impl fmt::Display for FfiSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .params
            .iter()
            .map(|ty| ty.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "({params}) -> {}", self.result)
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FfiFunction {
    pub name: String,
    pub symbol: String,
    pub signature: FfiSignature,
}
