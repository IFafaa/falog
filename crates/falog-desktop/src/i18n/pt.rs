//! Brazilian Portuguese for every interface text, keyed by its English original. Write as a person
//! from Brazil would read it in an app: short, natural, "você" (implied, rarely written).

/// The Portuguese for `english`, or `None` when there is no entry yet.
pub fn text(english: &str) -> Option<&'static str> {
    Some(match english {
        // ---- language ----
        "Delete" => "Excluir",
        "Language" => "Idioma",
        "The language of Falog's menus and messages." => "O idioma dos menus e mensagens do Falog.",
        "Archived {n} tasks" => "{n} tarefas arquivadas",
        "Archived #{id}; find it in the Archive (Ctrl+5)" => "#{id} arquivada; ela fica no Arquivo (Ctrl+5)",
        "Archived {n} tasks; find them in the Archive (Ctrl+5)" => {
            "{n} tarefas arquivadas; elas ficam no Arquivo (Ctrl+5)"
        }
        _ => return None,
    })
}
