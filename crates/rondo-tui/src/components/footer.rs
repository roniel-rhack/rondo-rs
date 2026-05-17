use crate::app::AppState;
use crate::focus::{DetailSection, Mode, Pane};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use rondo_core::i18n;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let mode = current_mode(app);

    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::raw(" "));
    let undo_depth = app.undo.len();
    let mode_label = if mode == Mode::Visual && !app.ui.selection.is_empty() {
        format!("[VIS·{}]", app.ui.selection.len())
    } else if undo_depth > 0 {
        format!("[{}·{}]", mode.tag(), undo_depth)
    } else {
        format!("[{}]", mode.tag())
    };
    spans.push(Span::styled(
        mode_label,
        Style::default()
            .fg(t.bg)
            .bg(mode_color(mode, t))
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw("  "));

    for (key, action) in hints(app) {
        spans.push(Span::styled("[", t.muted()));
        spans.push(Span::styled(key.to_string(), t.kbd()));
        spans.push(Span::styled(" → ", t.muted()));
        spans.push(Span::styled(action, Style::default().fg(t.fg)));
        spans.push(Span::styled("] ", t.muted()));
    }

    spans.push(Span::raw(" "));
    if let Some(msg) = &app.status_msg {
        spans.push(Span::styled("· ", t.muted()));
        spans.push(Span::styled(msg.clone(), Style::default().fg(t.warn)));
        spans.push(Span::raw("  "));
    }
    if !app.modals.help_open {
        spans.push(Span::styled("· ", t.muted()));
        spans.push(Span::styled("?", t.kbd()));
        spans.push(Span::styled(
            format!(" {} ", i18n::t("footer.help_hint")),
            t.muted(),
        ));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn current_mode(app: &AppState) -> Mode {
    if app.modals.command_palette_open || app.modals.search_open {
        Mode::Insert
    } else {
        app.ui.mode
    }
}

fn mode_color(mode: Mode, t: &crate::theme::Theme) -> ratatui::style::Color {
    match mode {
        Mode::Normal => t.accent,
        Mode::Insert => t.warn,
        Mode::Visual => t.danger,
    }
}

/// Build a hint row, looking each label up in the active language pack.
/// Keys are looked up lazily so a hot-reload (`:lang`) takes effect on the
/// next frame.
fn hint(key: &'static str, action_key: &'static str) -> (&'static str, String) {
    (key, i18n::t(action_key))
}

/// Context-aware hint dispatcher. Caps at 5 hints; `?` appended separately.
fn hints(app: &AppState) -> Vec<(&'static str, String)> {
    if app.modals.help_open {
        return vec![hint("Esc", "footer.hint.help_close")];
    }
    if app.modals.command_palette_open {
        return vec![
            hint("Enter", "footer.hint.palette_run"),
            hint("Tab", "footer.hint.palette_complete"),
            hint("Esc", "footer.hint.palette_cancel"),
        ];
    }
    if app.modals.search_open {
        return vec![
            hint("Enter", "footer.hint.search_apply"),
            hint("Esc", "footer.hint.search_cancel"),
        ];
    }
    if app.modals.pomodoro_open {
        return vec![
            hint("p", "footer.hint.pomodoro_toggle"),
            hint("Esc", "footer.hint.pomodoro_close"),
            hint(":", "footer.hint.pomodoro_cmd"),
            hint("q", "footer.hint.pomodoro_quit"),
        ];
    }

    if app.ui.mode == Mode::Visual {
        return vec![
            hint("j/k", "footer.hint.visual_extend"),
            hint("d", "footer.hint.visual_bulk_done"),
            hint("P", "footer.hint.visual_bulk_prio"),
            hint("Esc", "footer.hint.visual_cancel"),
            hint(":", "footer.hint.visual_cmd"),
        ];
    }
    if app.modals.journal_editor_open {
        return vec![
            hint("Ctrl+S", "footer.hint.journal_editor_save"),
            hint("Enter", "footer.hint.journal_editor_newline"),
            hint("Esc", "footer.hint.journal_editor_cancel"),
        ];
    }
    if app.ui.page == crate::action::Page::Journal {
        let in_days = matches!(app.ui.journal_pane, crate::app::ui_state::JournalPane::Days);
        if in_days {
            return vec![
                hint("h/l", "footer.hint.journal_days_switch"),
                hint("j/k", "footer.hint.journal_days_change"),
                hint("a/i", "footer.hint.journal_days_new_entry"),
                hint("X", "footer.hint.journal_days_delete_day"),
                hint("H", "footer.hint.journal_days_toggle_hidden"),
            ];
        }
        return vec![
            hint("h/l", "footer.hint.journal_days_switch"),
            hint("j/k", "footer.hint.journal_entries_change"),
            hint("e", "footer.hint.journal_entries_edit"),
            hint("d", "footer.hint.journal_entries_delete"),
            hint("a/i/A", "footer.hint.journal_entries_new"),
        ];
    }
    match app.ui.focus.pane {
        Pane::Sidebar => vec![
            hint("j/k", "footer.hint.sidebar_move"),
            hint("Enter", "footer.hint.sidebar_apply_filter"),
            hint("l", "footer.hint.sidebar_list"),
            hint("Esc", "footer.hint.sidebar_cancel"),
            hint(":", "footer.hint.sidebar_cmd"),
        ],
        Pane::List => vec![
            hint("a", "footer.hint.list_add"),
            hint("space", "footer.hint.list_status"),
            hint("v", "footer.hint.list_select"),
            hint("B", "footer.hint.list_add_dep"),
            hint(":", "footer.hint.list_cmd"),
        ],
        Pane::Detail => match app.ui.focus.section {
            DetailSection::Header => vec![
                hint("e", "footer.hint.detail_header_edit_title"),
                hint("E", "footer.hint.detail_header_edit_description"),
                hint("space", "footer.hint.detail_header_cycle_status"),
                hint("Tab", "footer.hint.detail_header_next_section"),
                hint(":", "footer.hint.common_cmd"),
            ],
            DetailSection::Subtasks => vec![
                hint("A", "footer.hint.detail_subtasks_add"),
                hint("e", "footer.hint.detail_subtasks_rename"),
                hint("d", "footer.hint.detail_subtasks_delete"),
                hint("space", "footer.hint.detail_subtasks_check"),
                hint(":", "footer.hint.common_cmd"),
            ],
            DetailSection::Dependencies => vec![
                hint("B", "footer.hint.detail_deps_add_remove"),
                hint("Tab", "footer.hint.detail_deps_next_section"),
                hint("1/2/3/4", "footer.hint.detail_deps_section_jump"),
                hint("h", "footer.hint.detail_deps_list"),
                hint(":", "footer.hint.common_cmd"),
            ],
            DetailSection::Notes => vec![
                hint("a", "footer.hint.detail_notes_add"),
                hint("e", "footer.hint.detail_notes_edit"),
                hint("d", "footer.hint.detail_notes_delete"),
                hint("Tab", "footer.hint.detail_notes_next_section"),
                hint(":", "footer.hint.common_cmd"),
            ],
        },
    }
}
