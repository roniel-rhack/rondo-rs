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
            Constraint::Length(1), // brand strip
            Constraint::Min(1),    // body (sidebar + content)
            Constraint::Length(analytics_height), // analytics row (optional)
            Constraint::Length(1), // footer
        ])
        .split(area);

    components::header::draw(app, f, chunks[0]);
    body_with_sidebar(app, f, chunks[1]);
    if show_analytics {
        components::analytics::draw(app, f, chunks[2]);
    }
    components::footer::draw(app, f, chunks[3]);

    if app.pomodoro_open {
        components::pomodoro::draw(app, f, centered(60, 14, area));
    }
    if app.search_open {
        components::search::draw(app, f, search_rect(area));
    }
    if app.quick_add_open {
        components::quick_add::draw(app, f, search_rect(area));
    }
    if app.command_palette_open {
        components::command_palette::draw(app, f, palette_rect(area));
    }
    if app.help_open {
        components::help::draw(app, f, centered(60, 40, area));
    }
    if app.quick_actions_open {
        components::quick_actions::draw(app, f, centered(72, 7, area));
    }
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
    let narrow = area.width < NARROW_BREAKPOINT;
    match app.page {
        Page::Tasks => {
            if narrow {
                if app.focus_left() {
                    components::task_list::draw(app, f, area);
                } else {
                    components::task_detail::draw(app, f, area);
                }
            } else {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(app.split_ratio),
                        Constraint::Percentage(100 - app.split_ratio),
                    ])
                    .split(area);
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
