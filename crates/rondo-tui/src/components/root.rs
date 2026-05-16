use crate::{action::Page, app::AppState, components};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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
        components::pomodoro::draw(app, f, centered(44, 11, f.area()));
    }
    if app.command_palette_open {
        components::command_palette::draw(app, f, palette_rect(f.area()));
    }
}

fn body(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    match app.page {
        Page::Tasks => {
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
        Page::Journal => components::journal::draw(app, f, area),
    }
}

fn centered(w: u16, h: u16, area: Rect) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

fn palette_rect(area: Rect) -> Rect {
    let h = 12.min(area.height.saturating_sub(2));
    Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(h + 1),
        width: area.width.saturating_sub(4),
        height: h,
    }
}
