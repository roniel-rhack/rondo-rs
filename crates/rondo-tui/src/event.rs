use crate::action::{Action, Page};
use crate::app::AppState;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub fn map(ev: Event, app: &AppState) -> Option<Action> {
    if app.modals.help_open {
        return help_key(ev);
    }
    if app.modals.command_palette_open {
        return palette_key(ev, app);
    }
    if app.modals.search_open {
        return search_key(ev, app);
    }
    if app.modals.quick_add_open {
        return quick_add_key(ev, app);
    }
    if app.modals.sort_overlay_open {
        return sort_overlay_key(ev);
    }
    if app.ui.leader_goto {
        if let Event::Key(k) = ev {
            if let KeyCode::Char(c) = k.code {
                if let Some(f) = crate::filter::by_shortcut(c) {
                    return Some(Action::ApplyFilter(f));
                }
            }
        }
        // Any other key cancels leader.
        return Some(Action::EscapeContext);
    }
    if app.ui.focus.pane == crate::focus::Pane::Sidebar {
        if let Event::Key(k) = ev {
            if let KeyCode::Char(c) = k.code {
                if let Some(f) = crate::filter::by_shortcut(c) {
                    return Some(Action::ApplyFilter(f));
                }
            }
        }
    }
    match ev {
        Event::Key(k) => key_to_action(k, app),
        Event::Resize(w, h) => Some(Action::Resize {
            width: w,
            height: h,
        }),
        _ => None,
    }
}

fn help_key(ev: Event) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    Some(match k.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Action::CloseHelp,
        _ => return None,
    })
}

fn palette_key(ev: Event, app: &AppState) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    Some(match k.code {
        KeyCode::Esc => Action::CloseCommandPalette,
        KeyCode::Enter => Action::SubmitCommand(app.modals.command_buf.clone()),
        KeyCode::Backspace => Action::SearchInput({
            let mut s = app.modals.command_buf.clone();
            s.pop();
            s
        }),
        KeyCode::Char(c) => Action::SearchInput({
            let mut s = app.modals.command_buf.clone();
            s.push(c);
            s
        }),
        _ => return None,
    })
}

fn search_key(ev: Event, app: &AppState) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    Some(match k.code {
        KeyCode::Esc => Action::CloseSearch,
        KeyCode::Enter => Action::CloseSearch,
        KeyCode::Backspace => Action::SearchUpdate({
            let mut s = app.modals.search_buf.clone();
            s.pop();
            s
        }),
        KeyCode::Char(c) => Action::SearchUpdate({
            let mut s = app.modals.search_buf.clone();
            s.push(c);
            s
        }),
        _ => return None,
    })
}

fn sort_overlay_key(ev: Event) -> Option<Action> {
    use crate::app::ui_state::SortOrder;
    let Event::Key(k) = ev else {
        return None;
    };
    Some(match k.code {
        KeyCode::Esc => Action::CloseSortOverlay,
        KeyCode::Char(c) if c.is_ascii_digit() => {
            let idx = (c as u8).saturating_sub(b'1') as usize;
            let order = SortOrder::ALL.get(idx).copied()?;
            Action::SetSortOrder(order)
        }
        _ => return None,
    })
}

fn quick_add_key(ev: Event, app: &AppState) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    Some(match k.code {
        KeyCode::Esc => Action::EscapeContext,
        KeyCode::Enter => Action::SubmitQuickAdd(app.modals.quick_add_buf.clone()),
        KeyCode::Backspace => Action::QuickAddUpdate({
            let mut s = app.modals.quick_add_buf.clone();
            s.pop();
            s
        }),
        KeyCode::Char(c) => Action::QuickAddUpdate({
            let mut s = app.modals.quick_add_buf.clone();
            s.push(c);
            s
        }),
        _ => return None,
    })
}

fn key_to_action(k: KeyEvent, app: &AppState) -> Option<Action> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let in_visual = app.ui.mode == crate::focus::Mode::Visual;
    let in_sidebar = app.ui.focus.pane == crate::focus::Pane::Sidebar;
    Some(match k.code {
        KeyCode::Enter if in_sidebar => Action::ApplySidebarSelection,
        KeyCode::Char('q') if !ctrl => Action::Quit,
        KeyCode::Char('c') if ctrl => Action::Quit,
        KeyCode::Char('d') if ctrl => Action::HalfPageDown,
        KeyCode::Char('u') if ctrl => Action::HalfPageUp,
        KeyCode::Char('j') | KeyCode::Down => Action::NextItem,
        KeyCode::Char('k') | KeyCode::Up => Action::PrevItem,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        KeyCode::Char('h') => Action::FocusLeft,
        KeyCode::Char('l') => Action::FocusRight,
        KeyCode::Tab => Action::NextSection,
        KeyCode::BackTab => Action::PrevSection,
        KeyCode::Char(' ') => Action::ToggleSelected,
        KeyCode::Char('v') => Action::EnterVisual,
        KeyCode::Char('a') => Action::OpenQuickAdd,
        KeyCode::Char('d') if in_visual => Action::BulkDone,
        KeyCode::Char('P') if in_visual => Action::BulkPriority,
        KeyCode::Char('1') => Action::TogglePage(Page::Tasks),
        KeyCode::Char('2') => Action::TogglePage(Page::Journal),
        KeyCode::Char('p') if !in_visual => Action::TogglePomodoro,
        KeyCode::Char(':') => Action::OpenCommandPalette,
        KeyCode::Char('/') => Action::OpenSearch,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('f') => Action::LeaderGoto,
        KeyCode::Char('.') => Action::ToggleQuickActions,
        KeyCode::Char('s') => Action::OpenSortOverlay,
        KeyCode::Char('<') => Action::ResizeSplit { delta: -5 },
        KeyCode::Char('>') => Action::ResizeSplit { delta: 5 },
        KeyCode::Char('=') => Action::ResetSplit,
        KeyCode::Esc => Action::EscapeContext,
        _ => return None,
    })
}
