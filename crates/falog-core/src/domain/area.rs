use super::{AreaId, Rgb};

/// A part of the user's life a task belongs to: "Work", "Personal", "Health", "Home"...
/// Every task may belong to one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Area {
    pub id: AreaId,
    pub name: String,
    pub color: Rgb,
}
