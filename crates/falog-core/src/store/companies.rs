use super::Store;
use crate::date::now;
use crate::domain::{Company, CompanyId, PALETTE, Rgb};
use crate::text::fold;
use crate::{Error, Result};
use rusqlite::{OptionalExtension, Row, params};

const SELECT: &str = "SELECT id, name, color FROM companies";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Company> {
    Ok(Company {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
    })
}

fn validate_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::invalid("company name cannot be empty"));
    }
    Ok(name)
}

impl Store {
    /// All companies, alphabetically.
    pub fn companies(&self) -> Result<Vec<Company>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{SELECT} ORDER BY name COLLATE NOCASE"))?;
        let rows = stmt.query_map([], map_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn company(&self, id: CompanyId) -> Result<Option<Company>> {
        Ok(self
            .conn
            .query_row(&format!("{SELECT} WHERE id = ?1"), [id], map_row)
            .optional()?)
    }

    /// Creates a company; without an explicit color the next palette color is used.
    pub fn create_company(&self, name: &str, color: Option<Rgb>) -> Result<Company> {
        let name = validate_name(name)?;
        let existing = self.companies()?;
        if existing.iter().any(|c| fold(&c.name) == fold(name)) {
            return Err(Error::DuplicateCompany(name.to_string()));
        }
        let color = color.unwrap_or(PALETTE[existing.len() % PALETTE.len()]);
        self.conn.execute(
            "INSERT INTO companies (name, color, created_at) VALUES (?1, ?2, ?3)",
            params![name, color, now()],
        )?;
        Ok(Company {
            id: CompanyId(self.conn.last_insert_rowid()),
            name: name.to_string(),
            color,
        })
    }

    pub fn update_company(&self, id: CompanyId, name: &str, color: Rgb) -> Result<Company> {
        let name = validate_name(name)?;
        if self
            .companies()?
            .iter()
            .any(|c| c.id != id && fold(&c.name) == fold(name))
        {
            return Err(Error::DuplicateCompany(name.to_string()));
        }
        self.conn.execute(
            "UPDATE companies SET name = ?1, color = ?2 WHERE id = ?3",
            params![name, color, id],
        )?;
        Ok(Company {
            id,
            name: name.to_string(),
            color,
        })
    }

    /// Deletes a company; its tasks are kept without a company.
    pub fn delete_company(&self, id: CompanyId) -> Result<()> {
        self.conn.execute("DELETE FROM companies WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Resolves a company from a name or a unique fragment of it, ignoring case and accents
    /// (`"acme"` finds `"Acme Café"`).
    pub fn find_company(&self, query: &str) -> Result<Company> {
        let needle = fold(query);
        let companies = self.companies()?;
        if let Some(exact) = companies.iter().find(|c| fold(&c.name) == needle) {
            return Ok(exact.clone());
        }
        let mut matches: Vec<&Company> = companies
            .iter()
            .filter(|c| {
                let name = fold(&c.name);
                !needle.is_empty() && (name.contains(&needle) || needle.contains(&name))
            })
            .collect();
        match matches.len() {
            1 => Ok(matches.remove(0).clone()),
            0 => Err(Error::CompanyNotFound {
                query: query.to_string(),
                known: companies.into_iter().map(|c| c.name).collect(),
            }),
            _ => Err(Error::AmbiguousCompany {
                query: query.to_string(),
                candidates: matches.into_iter().map(|c| c.name.clone()).collect(),
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
        let a = store.create_company("Acme Café", None).unwrap();
        let b = store.create_company("Globex", None).unwrap();
        assert_eq!(a.color, PALETTE[0]);
        assert_eq!(b.color, PALETTE[1]);
        assert!(matches!(
            store.create_company("acme cafe", None),
            Err(Error::DuplicateCompany(_))
        ));
        assert!(store.create_company("  ", None).is_err());
    }

    #[test]
    fn finds_by_fragment_ignoring_accents() {
        let store = store();
        let acme = store.create_company("Acme Café", None).unwrap();
        store.create_company("Globex", None).unwrap();
        assert_eq!(store.find_company("ACME CAFE").unwrap(), acme);
        assert_eq!(store.find_company("acme").unwrap(), acme);
        assert!(matches!(
            store.find_company("initech"),
            Err(Error::CompanyNotFound { .. })
        ));
    }

    #[test]
    fn reports_ambiguous_fragments() {
        let store = store();
        store.create_company("Acme Labs", None).unwrap();
        store.create_company("Acme Retail", None).unwrap();
        assert!(matches!(
            store.find_company("acme"),
            Err(Error::AmbiguousCompany { .. })
        ));
    }

    #[test]
    fn updates_and_deletes() {
        let store = store();
        let company = store.create_company("Acme", None).unwrap();
        let renamed = store.update_company(company.id, "Acme Inc", PALETTE[3]).unwrap();
        assert_eq!(store.company(company.id).unwrap(), Some(renamed));
        store.delete_company(company.id).unwrap();
        assert!(store.companies().unwrap().is_empty());
    }
}
