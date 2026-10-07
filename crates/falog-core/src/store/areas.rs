use super::Store;
use crate::date::now;
use crate::domain::{Area, AreaId, PALETTE, Rgb};
use crate::text::fold;
use crate::{Error, Result};
use rusqlite::{OptionalExtension, Row, params};

const SELECT: &str = "SELECT id, name, color FROM areas";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Area> {
    Ok(Area {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
    })
}

fn validate_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::invalid("area name cannot be empty"));
    }
    Ok(name)
}

impl Store {
    /// All areas, alphabetically.
    pub fn areas(&self) -> Result<Vec<Area>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{SELECT} ORDER BY name COLLATE NOCASE"))?;
        let rows = stmt.query_map([], map_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn area(&self, id: AreaId) -> Result<Option<Area>> {
        Ok(self
            .conn
            .query_row(&format!("{SELECT} WHERE id = ?1"), [id], map_row)
            .optional()?)
    }

    /// Creates an area; without an explicit color the next palette color is used.
    pub fn create_area(&self, name: &str, color: Option<Rgb>) -> Result<Area> {
        let name = validate_name(name)?;
        let existing = self.areas()?;
        if existing.iter().any(|a| fold(&a.name) == fold(name)) {
            return Err(Error::DuplicateArea(name.to_string()));
        }
        let color = color.unwrap_or(PALETTE[existing.len() % PALETTE.len()]);
        self.conn.execute(
            "INSERT INTO areas (name, color, created_at) VALUES (?1, ?2, ?3)",
            params![name, color, now()],
        )?;
        Ok(Area {
            id: AreaId(self.conn.last_insert_rowid()),
            name: name.to_string(),
            color,
        })
    }

    pub fn update_area(&self, id: AreaId, name: &str, color: Rgb) -> Result<Area> {
        let name = validate_name(name)?;
        if self
            .areas()?
            .iter()
            .any(|a| a.id != id && fold(&a.name) == fold(name))
        {
            return Err(Error::DuplicateArea(name.to_string()));
        }
        self.conn.execute(
            "UPDATE areas SET name = ?1, color = ?2 WHERE id = ?3",
            params![name, color, id],
        )?;
        Ok(Area {
            id,
            name: name.to_string(),
            color,
        })
    }

    /// Deletes an area; its tasks are kept without an area.
    pub fn delete_area(&self, id: AreaId) -> Result<()> {
        self.conn.execute("DELETE FROM areas WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Resolves an area from a name or a unique fragment of it, ignoring case and accents
    /// (`"work"` finds `"Work Café"`).
    pub fn find_area(&self, query: &str) -> Result<Area> {
        let needle = fold(query);
        let areas = self.areas()?;
        if let Some(exact) = areas.iter().find(|a| fold(&a.name) == needle) {
            return Ok(exact.clone());
        }
        let mut matches: Vec<&Area> = areas
            .iter()
            .filter(|a| {
                let name = fold(&a.name);
                !needle.is_empty() && (name.contains(&needle) || needle.contains(&name))
            })
            .collect();
        match matches.len() {
            1 => Ok(matches.remove(0).clone()),
            0 => Err(Error::AreaNotFound {
                query: query.to_string(),
                known: areas.into_iter().map(|a| a.name).collect(),
            }),
            _ => Err(Error::AmbiguousArea {
                query: query.to_string(),
                candidates: matches.into_iter().map(|a| a.name.clone()).collect(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open_in_memory().unwrap()
    }

    #[test]
    fn creates_with_palette_colors_and_rejects_duplicates() {
        let store = store();
        let a = store.create_area("Work Café", None).unwrap();
        let b = store.create_area("Home", None).unwrap();
        assert_eq!(a.color, PALETTE[0]);
        assert_eq!(b.color, PALETTE[1]);
        assert!(matches!(
            store.create_area("work cafe", None),
            Err(Error::DuplicateArea(_))
        ));
        assert!(store.create_area("  ", None).is_err());
    }

    #[test]
    fn finds_by_fragment_ignoring_accents() {
        let store = store();
        let work = store.create_area("Work Café", None).unwrap();
        store.create_area("Home", None).unwrap();
        assert_eq!(store.find_area("WORK CAFE").unwrap(), work);
        assert_eq!(store.find_area("work").unwrap(), work);
        assert!(matches!(
            store.find_area("studies"),
            Err(Error::AreaNotFound { .. })
        ));
    }

    #[test]
    fn reports_ambiguous_fragments() {
        let store = store();
        store.create_area("Work Projects", None).unwrap();
        store.create_area("Work Admin", None).unwrap();
        assert!(matches!(
            store.find_area("work"),
            Err(Error::AmbiguousArea { .. })
        ));
    }

    #[test]
    fn updates_and_deletes() {
        let store = store();
        let area = store.create_area("Work", None).unwrap();
        let renamed = store.update_area(area.id, "Office", PALETTE[3]).unwrap();
        assert_eq!(store.area(area.id).unwrap(), Some(renamed));
        store.delete_area(area.id).unwrap();
        assert!(store.areas().unwrap().is_empty());
    }
}
