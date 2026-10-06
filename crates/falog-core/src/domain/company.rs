use super::{CompanyId, Rgb};

/// A client or employer the user works for. Every task may belong to one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Company {
    pub id: CompanyId,
    pub name: String,
    pub color: Rgb,
}
