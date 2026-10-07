use super::{AreaId, Rgb};

/// A sphere of the user's life a task belongs to: an employer, a client, "Personal", "Health"...
/// Every task may belong to one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Area {
    pub id: AreaId,
    pub name: String,
    pub color: Rgb,
}
