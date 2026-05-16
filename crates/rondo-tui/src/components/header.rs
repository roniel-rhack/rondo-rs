use crate::{action::Page, app::AppState};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let title = Span::styled(
        " RonDO ",
        Style::default()
            .fg(t.accent)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    );
    let sep = Span::styled(" │ ", Style::default().fg(t.fg_muted));
    let tab = |label: &'static str, active: bool| {
        let s = if active {
            Style::default()
                .fg(t.fg)
                .add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default().fg(t.fg_muted)
        };
        Span::styled(format!(" {} ", label), s)
    };
    let line = Line::from(vec![
        title,
        sep.clone(),
        tab("Tasks", app.page == Page::Tasks),
        Span::raw(" "),
        tab("Journal", app.page == Page::Journal),
        sep.clone(),
        Span::styled(
            format!("{} tasks", app.tasks.len()),
            Style::default().fg(t.fg_muted),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}
