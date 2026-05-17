use crate::{action::Page, app::AppState, components};
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    Frame,
};

const NARROW_BREAKPOINT: u16 = 100;
const SIDEBAR_WIDTH: u16 = 26;

pub fn draw(app: &mut AppState, f: &mut Frame<'_>) {
    let area = f.area();
    let show_analytics = area.height >= 30 && area.width >= NARROW_BREAKPOINT;
    let analytics_height = if show_analytics { 9 } else { 0 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                // brand strip
            Constraint::Min(1),                   // body (sidebar + content)
            Constraint::Length(analytics_height), // analytics row (optional)
            Constraint::Length(1),                // footer
        ])
        .split(area);

    components::header::draw(app, f, chunks[0]);
    body_with_sidebar(app, f, chunks[1]);
    if show_analytics {
        crate::plugins::builtin::analytics::draw(app, f, chunks[2]);
    }
    components::footer::draw(app, f, chunks[3]);
    app.ui.last_footer_rect = chunks[3];

    if app.modals.pomodoro_open {
        let r = centered(60, 14, area);
        app.ui.last_pomodoro_rect = r;
        components::pomodoro::draw(app, f, r);
    }
    if app.modals.search_open {
        components::search::draw(app, f, search_rect(area));
    }
    if app.modals.quick_add_open {
        let r = search_rect(area);
        app.ui.last_quick_add_rect = r;
        components::quick_add::draw(app, f, r);
    }
    if app.modals.journal_editor_open && app.ui.page == Page::Journal {
        components::journal::draw_editor_overlay(app, f, app.ui.last_body_rect);
    }
    if app.modals.command_palette_open {
        components::command_palette::draw(app, f, palette_rect(area));
    }
    if app.modals.help_open {
        components::help::draw(app, f, centered(60, 40, area));
    }
    if app.modals.quick_actions_open {
        components::quick_actions::draw(app, f, centered(100, 8, area));
    }
    if app.modals.sort_overlay_open {
        let h = crate::app::ui_state::SortOrder::ALL.len() as u16 + 2;
        components::sort_overlay::draw(app, f, centered(48, h, area));
    }
    if app.modals.edit_title_open {
        components::edit_title::draw(app, f, search_rect(area));
    }
    if app.modals.confirm_delete_open {
        components::confirm::draw(app, f, centered(60, 7, area));
    }
    if app.modals.add_subtask_open {
        components::add_subtask::draw(app, f, centered(70, 6, area));
    }
    if app.modals.dep_overlay_open {
        components::dep_overlay::draw(app, f, centered(70, 8, area));
    }
    if app.modals.plugins_overlay_open {
        components::plugins_overlay::draw(app, f, centered(86, 40, area));
    }
    if app.modals.plugin_page.is_some() {
        let w = (area.width.saturating_sub(8)).min(120);
        let h = area.height.saturating_sub(4);
        components::plugin_page::draw(app, f, centered(w, h, area));
    }

    // Run live effects after all widgets have painted; effects mutate cells in
    // place to produce fades/sweeps/dissolves.
    app.fx.tick_and_render(f);
}

fn body_with_sidebar(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let show_sidebar = area.width >= NARROW_BREAKPOINT;
    if show_sidebar {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(1)])
            .split(area);
        components::sidebar::draw(app, f, cols[0]);
        body(app, f, cols[1]);
    } else {
        body(app, f, area);
    }
}

fn body(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    app.ui.last_body_rect = area;
    let narrow = area.width < NARROW_BREAKPOINT;
    match app.ui.page {
        Page::Tasks => {
            if narrow {
                if app.focus_left() {
                    app.ui.last_task_list_rect = area;
                    components::task_list::draw(app, f, area);
                } else {
                    app.ui.last_detail_rect = area;
                    components::task_detail::draw(app, f, area);
                }
            } else {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(app.ui.split_ratio),
                        Constraint::Percentage(100 - app.ui.split_ratio),
                    ])
                    .split(area);
                app.ui.last_task_list_rect = split[0];
                app.ui.last_detail_rect = split[1];
                components::task_list::draw(app, f, split[0]);
                components::task_detail::draw(app, f, split[1]);
            }
        }
        Page::Journal => components::journal::draw(app, f, area),
    }
}

fn centered(w: u16, h: u16, area: Rect) -> Rect {
    let [vertical] = Layout::vertical([Constraint::Length(h.min(area.height))])
        .flex(Flex::Center)
        .areas(area);
    let [centered] = Layout::horizontal([Constraint::Length(w.min(area.width))])
        .flex(Flex::Center)
        .areas(vertical);
    centered
}

fn palette_rect(area: Rect) -> Rect {
    let h = 12u16.min(area.height.saturating_sub(4));
    let [_, anchored] = Layout::vertical([Constraint::Min(0), Constraint::Length(h)])
        .flex(Flex::End)
        .margin(2)
        .areas(area);
    anchored
}

fn search_rect(area: Rect) -> Rect {
    let h = 4u16.min(area.height.saturating_sub(4));
    let [_, anchored] = Layout::vertical([Constraint::Min(0), Constraint::Length(h)])
        .flex(Flex::End)
        .margin(2)
        .areas(area);
    anchored
}
