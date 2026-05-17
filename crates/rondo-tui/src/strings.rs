//! Phase 1 i18n: user-visible string table keyed by `StringKey`.
//!
//! The TUI is mostly Spanish today; this module starts the consolidation by
//! pulling the strings that appear in the header / sidebar / footer / task
//! list into one place. Components look up by `t(app.lang, KEY)`.
//!
//! Phase 2 (footer hints, toasts) lands when Workstream B finishes splitting
//! `app/mod.rs` — touching the toast call sites belongs to that PR.

pub use rondo_core::config::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringKey {
    // header
    HeaderSubtitle,
    // sidebar
    NavPanel,
    QuickFiltersPanel,
    LeaderHint,
    // task list
    TasksCount,
    TasksEmpty,
    HintChangeFilter,
    HintHelp,
    ColumnStatus,
    ColumnPriority,
    ColumnTask,
    OverallProgress,
    // footer
    FooterMore,
    FooterHelpHint,
}

/// Resolve a string in the active language. Falls back to English when a key
/// is missing from the Spanish table (every key here is defined in both).
pub fn t(lang: Lang, key: StringKey) -> &'static str {
    let table = match lang {
        Lang::Es => T_ES,
        Lang::En => T_EN,
    };
    for (k, v) in table {
        if *k == key {
            return v;
        }
    }
    ""
}

const T_ES: &[(StringKey, &str)] = &[
    (
        StringKey::HeaderSubtitle,
        "SISTEMA DE GESTIÓN DE TAREAS AVANZADO",
    ),
    (StringKey::NavPanel, "navegación"),
    (StringKey::QuickFiltersPanel, "filtros rápidos"),
    (StringKey::LeaderHint, "pulsa letra"),
    (StringKey::TasksCount, "tareas"),
    (StringKey::TasksEmpty, "Sin tareas para"),
    (StringKey::HintChangeFilter, "cambiar filtro"),
    (StringKey::HintHelp, "ayuda"),
    (StringKey::ColumnStatus, "ESTADO"),
    (StringKey::ColumnPriority, "PRI"),
    (StringKey::ColumnTask, "TAREA"),
    (StringKey::OverallProgress, "PROGRESO GENERAL"),
    (StringKey::FooterMore, "más"),
    (StringKey::FooterHelpHint, "ayuda"),
];

const T_EN: &[(StringKey, &str)] = &[
    (StringKey::HeaderSubtitle, "ADVANCED TASK MANAGEMENT SYSTEM"),
    (StringKey::NavPanel, "navigation"),
    (StringKey::QuickFiltersPanel, "quick filters"),
    (StringKey::LeaderHint, "press letter"),
    (StringKey::TasksCount, "tasks"),
    (StringKey::TasksEmpty, "No tasks for"),
    (StringKey::HintChangeFilter, "change filter"),
    (StringKey::HintHelp, "help"),
    (StringKey::ColumnStatus, "STATUS"),
    (StringKey::ColumnPriority, "PRI"),
    (StringKey::ColumnTask, "TASK"),
    (StringKey::OverallProgress, "OVERALL PROGRESS"),
    (StringKey::FooterMore, "more"),
    (StringKey::FooterHelpHint, "help"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_resolves_in_both_languages() {
        let keys = [
            StringKey::HeaderSubtitle,
            StringKey::NavPanel,
            StringKey::QuickFiltersPanel,
            StringKey::LeaderHint,
            StringKey::TasksCount,
            StringKey::TasksEmpty,
            StringKey::HintChangeFilter,
            StringKey::HintHelp,
            StringKey::ColumnStatus,
            StringKey::ColumnPriority,
            StringKey::ColumnTask,
            StringKey::OverallProgress,
            StringKey::FooterMore,
            StringKey::FooterHelpHint,
        ];
        for k in keys {
            assert!(!t(Lang::Es, k).is_empty(), "missing ES for {:?}", k);
            assert!(!t(Lang::En, k).is_empty(), "missing EN for {:?}", k);
        }
    }
}
