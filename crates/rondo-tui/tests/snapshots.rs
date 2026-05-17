use chrono::{TimeZone, Utc};
use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};
use rondo_core::store::sqlite::SqliteStore;
use rondo_tui::{action::Page, app::AppState, clock::FixedClock, components, filter};
use std::sync::Arc;

fn fixture_store() -> Arc<SqliteStore> {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    let seed = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/seed.sql"),
    )
    .unwrap();
    conn.execute_batch(&seed).unwrap();
    drop(conn);
    Arc::new(SqliteStore::open_readonly(tmp.path()).unwrap())
}

/// Pinned clock so date strings (today's date, "in 2d" badges) are
/// deterministic across snapshot runs and across machines.
fn fixed_clock() -> Arc<FixedClock> {
    Arc::new(FixedClock::new(
        Utc.with_ymd_and_hms(2026, 5, 17, 10, 0, 0).unwrap(),
    ))
}

fn snapshot(_name: &str, width: u16, height: u16, mutate: impl FnOnce(&mut AppState)) -> String {
    // Pin HOME so anything that renders `$HOME/...` (plugins overlay)
    // produces the same string length on every machine; otherwise
    // `/Users/<user>` vs `/home/<user>` lengths shift the trailing
    // column padding and churn the snapshot.
    // SAFETY: every snapshot test sets the same value; this never
    // observes a mid-flight rewrite from a different value.
    unsafe {
        std::env::set_var("HOME", "/snapshot-fixture");
    }
    // Pin the legacy `strings.rs` table to Spanish so the snapshots
    // (captured before the new English default) stay stable, and pin the
    // file-based i18n stack to the baked English baseline so any new
    // `i18n::t()` call sites are locale-independent.
    rondo_core::i18n::force_for_tests();
    let mut app = AppState::with_writable_and_clock(fixture_store(), false, fixed_clock()).unwrap();
    app.lang = rondo_tui::strings::Lang::Es;
    mutate(&mut app);
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| components::root::draw(&mut app, f)).unwrap();
    let raw = term.backend().to_string();
    // The clock is pinned, but `pomodoro_started` (from `Instant::now()`)
    // and other monotonic clock readings can still leak HH:MM:SS strings
    // into the buffer. Keep the time-of-day redaction for those paths;
    // the `today()` date string no longer needs redaction.
    let re_hms = regex::Regex::new(r"\d{2}:\d{2}:\d{2}").unwrap();
    let stage1 = re_hms.replace_all(&raw, "HH:MM:SS").to_string();
    let re_hm = regex::Regex::new(r"\b\d{2}:\d{2}\b").unwrap();
    let stage2 = re_hm.replace_all(&stage1, "HH:MM").to_string();
    let re_hm_frag = regex::Regex::new(r"\b\d:\d{2}\b").unwrap();
    let stage3 = re_hm_frag.replace_all(&stage2, "H:MM").to_string();
    // Redact wall-clock dates that leak through detail-pane "CREADA" /
    // "VENCE" rows (`YYYY-MM-DD`) so snapshots don't churn each midnight.
    let re_date = regex::Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap();
    let stage4 = re_date.replace_all(&stage3, "YYYY-MM-DD").to_string();
    // Localised weekday names trail some date strings (e.g. "(Monday)" /
    // "(lunes)"). They also rotate with the clock — strip parentheticals
    // that look like weekday-ish single English/Spanish words.
    let re_weekday =
        regex::Regex::new(r"\((Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday|lunes|martes|miércoles|jueves|viernes|sábado|domingo)\)").unwrap();
    let stage5 = re_weekday.replace_all(&stage4, "(WEEKDAY)").to_string();
    // The plugins overlay surfaces `$HOME/.rondo-rs/plugins` when no
    // external plugin is installed. HOME differs per dev machine and per
    // CI runner (`/Users/runner` macOS, `/home/runner` linux). Collapse
    // both shapes to a placeholder so the snapshot stays portable.
    let re_home = regex::Regex::new(r"(?:/Users|/home)/[^/\s]+").unwrap();
    re_home.replace_all(&stage5, "$$HOME").to_string()
}

#[test]
fn tasks_default() {
    let s = snapshot("tasks_default", 120, 32, |_| {});
    assert_snapshot!(s);
}

#[test]
fn tasks_selected_second() {
    let s = snapshot("tasks_selected_second", 120, 32, |a| {
        a.data.selected_task = 1
    });
    assert_snapshot!(s);
}

#[test]
fn tasks_blocked() {
    let s = snapshot("tasks_blocked", 120, 32, |a| a.data.selected_task = 3);
    assert_snapshot!(s);
}

#[test]
fn journal_view() {
    let s = snapshot("journal_view", 120, 32, |a| a.ui.page = Page::Journal);
    assert_snapshot!(s);
}

#[test]
fn pomodoro_overlay() {
    let s = snapshot("pomodoro_overlay", 120, 32, |a| {
        a.modals.pomodoro_open = true;
        a.modals.pomodoro_started = Some(std::time::Instant::now());
    });
    assert_snapshot!(s);
}

#[test]
fn command_palette() {
    let s = snapshot("command_palette", 120, 32, |a| {
        a.modals.command_palette_open = true;
        a.modals.command_buf = "p".to_string();
    });
    assert_snapshot!(s);
}

#[test]
fn narrow_terminal() {
    let s = snapshot("narrow_terminal", 80, 24, |_| {});
    assert_snapshot!(s);
}

#[test]
fn wide_terminal() {
    let s = snapshot("wide_terminal", 160, 40, |_| {});
    assert_snapshot!(s);
}

#[test]
fn full_dashboard_140x42() {
    let s = snapshot("full_dashboard_140x42", 140, 42, |_| {});
    assert_snapshot!(s);
}

#[test]
fn help_overlay() {
    let s = snapshot("help_overlay", 120, 32, |a| a.modals.help_open = true);
    assert_snapshot!(s);
}

#[test]
fn search_overlay() {
    let s = snapshot("search_overlay", 120, 32, |a| {
        a.apply_filter(filter::Filter::All);
        a.modals.search_open = true;
        a.modals.search_buf = "deploy".to_string();
    });
    assert_snapshot!(s);
}

#[test]
fn empty_tasks() {
    let s = snapshot("empty_tasks", 120, 32, |a| {
        a.data.tasks.clear();
        a.data.task_list_state.select(None);
    });
    assert_snapshot!(s);
}

#[test]
fn visual_mode_multi_select() {
    let s = snapshot("visual_mode_multi_select", 120, 32, |a| {
        a.ui.mode = rondo_tui::focus::Mode::Visual;
        a.ui.selection.insert(1);
        a.ui.selection.insert(2);
        a.data.selected_task = 1;
        a.data.task_list_state.select(Some(1));
    });
    assert_snapshot!(s);
}

#[test]
fn sidebar_focused() {
    let s = snapshot("sidebar_focused", 140, 32, |a| {
        a.ui.focus.pane = rondo_tui::focus::Pane::Sidebar;
        a.ui.focus.sidebar_item = 1; // HOY
    });
    assert_snapshot!(s);
}

#[test]
fn filter_today_applied() {
    let s = snapshot("filter_today_applied", 140, 32, |a| {
        a.data.active_filter = rondo_tui::filter::Filter::Today;
    });
    assert_snapshot!(s);
}

#[test]
fn filter_completed_applied() {
    let s = snapshot("filter_completed_applied", 140, 32, |a| {
        a.data.active_filter = rondo_tui::filter::Filter::Completed;
    });
    assert_snapshot!(s);
}

#[test]
fn quick_actions_overlay() {
    let s = snapshot("quick_actions_overlay", 140, 32, |a| {
        a.modals.quick_actions_open = true;
    });
    assert_snapshot!(s);
}

#[test]
fn lang_picker_overlay() {
    let s = snapshot("lang_picker_overlay", 120, 32, |a| {
        a.modals.open_lang_picker("en");
    });
    assert_snapshot!(s);
}

#[test]
fn quick_add_overlay() {
    let s = snapshot("quick_add_overlay", 120, 32, |a| {
        a.modals.quick_add_open = true;
        a.modals.quick_add_buf = "ship the demo #work !p3 due:tmrw".to_string();
        a.ui.mode = rondo_tui::focus::Mode::Insert;
    });
    assert_snapshot!(s);
}

#[test]
fn detail_focused_subtasks_section() {
    let s = snapshot("detail_focused_subtasks_section", 120, 32, |a| {
        a.ui.focus.pane = rondo_tui::focus::Pane::Detail;
        a.ui.focus.section = rondo_tui::focus::DetailSection::Subtasks;
        a.ui.focus.section_item = 1;
        a.data.selected_task = 2; // Review API spec has 5 subtasks
        a.data.task_list_state.select(Some(2));
    });
    assert_snapshot!(s);
}

#[test]
fn confirm_delete_overlay() {
    let s = snapshot("confirm_delete_overlay", 120, 32, |a| {
        a.modals.confirm_delete_open = true;
    });
    assert_snapshot!(s);
}

#[test]
fn edit_title_overlay() {
    let s = snapshot("edit_title_overlay", 120, 32, |a| {
        a.modals.edit_title_open = true;
        a.modals.edit_title_buf = "ship the demo".to_string();
        a.ui.mode = rondo_tui::focus::Mode::Insert;
    });
    assert_snapshot!(s);
}

#[test]
fn calendar_plugin_page() {
    let s = snapshot("calendar_plugin_page", 140, 36, |a| {
        a.modals.plugin_page = Some("builtin.calendar".to_string());
    });
    assert_snapshot!(s);
}

#[test]
fn dep_graph_plugin_page() {
    let s = snapshot("dep_graph_plugin_page", 140, 36, |a| {
        a.modals.plugin_page = Some("builtin.dep-graph".to_string());
    });
    assert_snapshot!(s);
}

#[test]
fn focus_plugin_page() {
    let s = snapshot("focus_plugin_page", 140, 36, |a| {
        a.modals.plugin_page = Some("builtin.focus-page".to_string());
    });
    assert_snapshot!(s);
}

#[test]
fn analytics_plugin_page() {
    let s = snapshot("analytics_plugin_page", 140, 36, |a| {
        a.modals.plugin_page = Some("builtin.analytics".to_string());
    });
    assert_snapshot!(s);
}

#[test]
fn plugins_overlay() {
    let s = snapshot("plugins_overlay", 140, 36, |a| {
        a.modals.plugins_overlay_open = true;
    });
    assert_snapshot!(s);
}

#[test]
fn sort_overlay_default() {
    let s = snapshot("sort_overlay_default", 120, 32, |a| {
        a.modals.sort_overlay_open = true;
        a.ui.sort_order = rondo_tui::app::ui_state::SortOrder::Default;
    });
    assert_snapshot!(s);
}

#[test]
fn sort_overlay_priority_desc() {
    let s = snapshot("sort_overlay_priority_desc", 120, 32, |a| {
        a.modals.sort_overlay_open = true;
        a.ui.sort_order = rondo_tui::app::ui_state::SortOrder::PriorityDesc;
    });
    assert_snapshot!(s);
}

#[test]
fn sort_overlay_due_asc() {
    let s = snapshot("sort_overlay_due_asc", 120, 32, |a| {
        a.modals.sort_overlay_open = true;
        a.ui.sort_order = rondo_tui::app::ui_state::SortOrder::DueAsc;
    });
    assert_snapshot!(s);
}

#[test]
fn sort_overlay_created_desc() {
    let s = snapshot("sort_overlay_created_desc", 120, 32, |a| {
        a.modals.sort_overlay_open = true;
        a.ui.sort_order = rondo_tui::app::ui_state::SortOrder::CreatedAtDesc;
    });
    assert_snapshot!(s);
}

#[test]
fn sort_overlay_title_asc() {
    let s = snapshot("sort_overlay_title_asc", 120, 32, |a| {
        a.modals.sort_overlay_open = true;
        a.ui.sort_order = rondo_tui::app::ui_state::SortOrder::TitleAsc;
    });
    assert_snapshot!(s);
}

#[test]
fn empty_journal() {
    let s = snapshot("empty_journal", 120, 32, |a| {
        a.ui.page = Page::Journal;
        a.data.journal_notes.clear();
        a.data.journal_entries.clear();
        a.data.journal_list_state.select(None);
    });
    assert_snapshot!(s);
}
