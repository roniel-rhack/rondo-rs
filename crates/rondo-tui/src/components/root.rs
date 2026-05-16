use crate::{action::Page, app::AppState, components};
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    Frame,
};

pub fn draw(app: &mut AppState, f: &mut Frame<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(f.area());
    components::header::draw(app, f, chunks[0]);
    body(app, f, chunks[1]);
    components::footer::draw(app, f, chunks[2]);
    if app.pomodoro_open {
        components::pomodoro::draw(app, f, centered(60, 14, f.area()));
    }
    if app.search_open {
        components::search::draw(app, f, search_rect(f.area()));
    }
    if app.command_palette_open {
        components::command_palette::draw(app, f, palette_rect(f.area()));
    }
    if app.help_open {
        components::help::draw(app, f, centered(56, 28, f.area()));
    }
}

fn search_rect(area: Rect) -> Rect {
    let h = 3u16.min(area.height.saturating_sub(4));
    let [_, anchored] = Layout::vertical([Constraint::Min(0), Constraint::Length(h)])
        .flex(Flex::End)
        .margin(2)
        .areas(area);
    anchored
}

const NARROW_BREAKPOINT: u16 = 100;

fn body(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let narrow = area.width < NARROW_BREAKPOINT;
    match app.page {
        Page::Tasks => {
            if narrow {
                // Single-pane: show whichever side has focus.
                if app.focus_left {
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
