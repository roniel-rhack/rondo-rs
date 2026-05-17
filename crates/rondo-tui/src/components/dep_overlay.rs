use crate::app::modals_state::DepOverlayMode;
use crate::app::AppState;
use crate::components::task_picker;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use rondo_core::i18n;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let task = app.data.selected_task();
    let task_title = task.map(|x| x.title.clone()).unwrap_or_default();
    let title_key = match app.modals.dep_overlay_mode {
        DepOverlayMode::Add => "dep_overlay.title_add",
        DepOverlayMode::Remove => "dep_overlay.title_remove",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(i18n::t(title_key), t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let context = Line::from(vec![
        Span::styled(i18n::t("dep_overlay.task_label"), t.muted()),
        Span::styled(task_title, Style::default().fg(t.fg_muted)),
    ]);
    f.render_widget(Paragraph::new(context), chunks[0]);

    let existing = task.map(|tk| tk.blocked_by_ids.clone()).unwrap_or_default();
    let existing_line = if existing.is_empty() {
        Line::from(vec![Span::styled(
            i18n::t("dep_overlay.blocked_by_none"),
            t.muted(),
        )])
    } else {
        let mut spans = vec![Span::styled(
            i18n::t("dep_overlay.blocked_by_prefix"),
            t.muted(),
        )];
        for (i, id) in existing.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(", ", t.muted()));
            }
            spans.push(Span::styled(
                format!("#{}", id),
                Style::default().fg(t.danger),
            ));
        }
        Line::from(spans)
    };
    f.render_widget(Paragraph::new(existing_line), chunks[1]);

    let prompt = match app.modals.dep_overlay_mode {
        DepOverlayMode::Add => i18n::t("dep_overlay.filter_prompt"),
        DepOverlayMode::Remove => i18n::t("dep_overlay.remove_prompt"),
    };
    let input = Line::from(vec![
        Span::styled(prompt, t.muted()),
        Span::styled(
            app.modals.dep_overlay_buf.clone(),
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "▏",
            Style::default().fg(t.fg).add_modifier(Modifier::SLOW_BLINK),
        ),
    ]);
    f.render_widget(Paragraph::new(input), chunks[2]);

    if matches!(app.modals.dep_overlay_mode, DepOverlayMode::Add) {
        let self_id = app.data.selected_task_id().unwrap_or(-1);
        let mut exclude = existing.clone();
        exclude.push(self_id);
        let candidates = task_picker::rank(&app.data.tasks, &app.modals.dep_overlay_buf, &exclude);
        let items: Vec<ListItem> = candidates
            .iter()
            .map(|c| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" #{:<4} ", c.id),
                        Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(c.title.clone(), Style::default().fg(t.fg)),
                ]))
            })
            .collect();
        let mut state = ListState::default();
        if !candidates.is_empty() {
            let cursor = app.modals.dep_overlay_cursor.min(candidates.len() - 1);
            state.select(Some(cursor));
        }
        let list = List::new(items).highlight_style(t.accent_style());
        f.render_stateful_widget(list, chunks[3], &mut state);
    }

    let hint = Line::from(vec![
        Span::styled("  ", t.muted()),
        Span::styled("Enter", Style::default().fg(t.accent)),
        Span::styled(i18n::t("dep_overlay.hint_submit"), t.muted()),
        Span::styled("↑↓", Style::default().fg(t.accent)),
        Span::styled(i18n::t("dep_overlay.hint_choose"), t.muted()),
        Span::styled("Tab", Style::default().fg(t.accent)),
        Span::styled(i18n::t("dep_overlay.hint_toggle"), t.muted()),
        Span::styled("Esc", Style::default().fg(t.accent)),
        Span::styled(i18n::t("dep_overlay.hint_cancel"), t.muted()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[4]);
}
