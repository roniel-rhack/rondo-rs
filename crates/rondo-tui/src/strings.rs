//! Legacy in-memory string table keyed by `StringKey`.
//!
//! This module predates the file-based language-pack system in
//! `rondo_core::i18n` and is kept as a compatibility shim while the existing
//! header / sidebar / footer / task-list call sites are migrated to
//! `rondo_core::i18n::t()` in follow-up PRs. New strings must NOT be added
//! here — extend `crates/rondo-core/src/i18n/en.toml` and reference the key
//! from `i18n::t()` / `i18n::tf()` instead.

/// Legacy two-language switch. New code should ignore this and call
/// `rondo_core::i18n::t()` directly; this enum survives only so the existing
/// `tr(app.lang, key)` call sites compile until they are migrated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    Es,
    #[default]
    En,
}

impl Lang {
    /// Map a free-form language code (as stored in `[ui].language`) to the
    /// two legacy variants. Unknown codes fall back to English so the legacy
    /// table stays usable for non-Spanish packs.
    pub fn from_code(code: &str) -> Self {
        match code {
            "es" => Lang::Es,
            _ => Lang::En,
        }
    }
}

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
