use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let mut spans = vec![
        kbd(" j/k ", t.accent),
        muted("nav  ", t),
        kbd(" Tab ", t.accent),
        muted("focus  ", t),
        kbd(" 1/2 ", t.accent),
        muted("page  ", t),
        kbd(" p ", t.accent),
        muted("pomodoro  ", t),
        kbd(" : ", t.accent),
        muted("cmd  ", t),
        kbd(" </> ", t.accent),
        muted("resize  ", t),
        kbd(" q ", t.accent),
        muted("quit", t),
    ];
    if let Some(msg) = &app.status_msg {
        spans.push(Span::raw("  │  "));
        spans.push(Span::styled(
            msg.clone(),
            Style::default().fg(t.warn),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn kbd(text: &'static str, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        text,
        Style::default()
            .fg(color)
            .add_modifier(ratatui::style::Modifier::REVERSED),
    )
}

fn muted(text: &'static str, t: &crate::theme::Theme) -> Span<'static> {
    Span::styled(text, Style::default().fg(t.fg_muted))
}
