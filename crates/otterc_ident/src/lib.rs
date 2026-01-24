use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identifier(Ustr);

impl Identifier {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn to_owned(&self) -> String {
        self.0.to_string()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<&str> for Identifier {
    fn from(s: &str) -> Self {
        Self(Ustr::from(s))
    }
}

impl From<String> for Identifier {
    fn from(s: String) -> Self {
        Self(Ustr::from(s.as_str()))
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
