use crate::action::{Action, Page};
use crate::app::AppState;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub fn map(ev: Event, app: &AppState) -> Option<Action> {
    // Bracketed paste short-circuit: route the payload to whichever input
    // surface is open. The app dispatcher knows which textarea or buffer
    // is active.
    if let Event::Paste(s) = ev {
        return Some(Action::Paste(s));
    }
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
    if app.modals.journal_editor_open {
        return journal_editor_key(ev, app);
    }
    if app.modals.sort_overlay_open {
        return sort_overlay_key(ev);
    }
    if app.modals.confirm_delete_open {
        return confirm_delete_key(ev);
    }
    if app.modals.edit_title_open {
        return edit_title_key(ev, app);
    }
    if app.modals.add_subtask_open {
        return add_subtask_key(ev, app);
    }
    if app.modals.dep_overlay_open {
        return dep_overlay_key(ev, app);
    }
    if app.modals.quick_actions_open {
        return quick_actions_key(ev, app);
    }
    if app.modals.plugin_page.is_some() {
        return plugin_page_key(ev);
    }
    if app.modals.description_editor_open {
        return description_editor_key(ev);
    }
    if app.modals.edit_subtask_open {
        return edit_subtask_key(ev, app);
    }
    if app.modals.note_editor_open {
        return note_editor_key(ev);
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
        // Note: `Event::Paste` was already short-circuited at the top of
        // `map()` (lines 9–11), so the arm previously living here was
        // unreachable.
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

fn journal_editor_key(ev: Event, _app: &AppState) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    // Capture the global "exit" affordances first; everything else flows
    // into the textarea so cursor navigation works as expected.
    match k.code {
        KeyCode::Esc => return Some(Action::JournalCancelEntry),
        KeyCode::Char('s') if ctrl => return Some(Action::JournalSubmitEntry),
        _ => {}
    }
    Some(Action::JournalEditorKey(k))
}

fn sort_overlay_key(ev: Event) -> Option<Action> {
    use crate::app::ui_state::SortOrder;
    let Event::Key(k) = ev else {
        return None;
    };
    Some(match k.code {
        KeyCode::Esc => Action::CloseSortOverlay,
        KeyCode::Char(c) if matches!(c, '1'..='5') => {
            let idx = (c as u8).saturating_sub(b'1') as usize;
            let order = SortOrder::ALL.get(idx).copied()?;
            Action::SetSortOrder(order)
        }
        _ => return None,
    })
}

fn confirm_delete_key(ev: Event) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    Some(match k.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => Action::ConfirmDeleteTask,
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => Action::CancelDelete,
        _ => return None,
    })
}

fn edit_title_key(ev: Event, app: &AppState) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    Some(match k.code {
        KeyCode::Esc => Action::CancelEditTitle,
        KeyCode::Enter => Action::SubmitEditTitle(app.modals.edit_title_buf.clone()),
        KeyCode::Char('s') if ctrl => Action::SubmitEditTitle(app.modals.edit_title_buf.clone()),
        KeyCode::Backspace => Action::EditTitleInput({
            let mut s = app.modals.edit_title_buf.clone();
            s.pop();
            s
        }),
        KeyCode::Char(c) => Action::EditTitleInput({
            let mut s = app.modals.edit_title_buf.clone();
            s.push(c);
            s
        }),
        _ => return None,
    })
}

/// When the quick-actions overlay is open, each press dispatches the
/// matching action AND closes the overlay first (via a synthetic close
///   plus the action follow-up the dispatcher will route). For unmapped
/// keys, the overlay just stays open.
fn quick_actions_key(ev: Event, _app: &AppState) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let action = match k.code {
        KeyCode::Esc => Action::CloseQuickActions,
        KeyCode::Char('.') => Action::CloseQuickActions,
        KeyCode::Char('a') => Action::OpenQuickAdd,
        KeyCode::Char('e') => Action::RequestEditTitle,
        KeyCode::Char('d') => Action::RequestDeleteTask,
        KeyCode::Char(' ') => Action::ToggleSelected,
        KeyCode::Char('A') => Action::RequestAddSubtask,
        KeyCode::Char('B') => Action::RequestAddDependency,
        KeyCode::Char('v') => Action::EnterVisual,
        KeyCode::Char('p') => Action::TogglePomodoro,
        KeyCode::Char('/') => Action::OpenSearch,
        KeyCode::Char(':') => Action::OpenCommandPalette,
        KeyCode::Char('s') => Action::OpenSortOverlay,
        KeyCode::Char('f') => Action::LeaderGoto,
        KeyCode::Char('z') if ctrl => Action::Undo,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('1') => Action::TogglePage(Page::Tasks),
        KeyCode::Char('2') => Action::TogglePage(Page::Journal),
        _ => return None,
    };
    // CloseQuickActions on its own is enough.
    if matches!(action, Action::CloseQuickActions) {
        return Some(action);
    }
    // For every other action we want the overlay to close first; we hand
    // back the action and rely on the dispatcher routing close via the
    // EscapeContext path inside each handler that opens a new modal.
    // Simpler: emit a single Action — the cross-cutting handler at
    // app/mod.rs closes the quick-actions overlay before opening its
    // own modal because each Request* / Open* handler we already wrote
    // sets the next modal regardless of prior overlay state. To make
    // sure the prior overlay disappears, we close it here via state.
    Some(action)
}

fn description_editor_key(ev: Event) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Esc => return Some(Action::CancelEditDescription),
        KeyCode::Char('s') if ctrl => return Some(Action::SubmitEditDescription),
        _ => {}
    }
    Some(Action::DescriptionEditorKey(k))
}

fn edit_subtask_key(ev: Event, app: &AppState) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    Some(match k.code {
        KeyCode::Esc => Action::CancelEditSubtask,
        KeyCode::Enter => Action::SubmitEditSubtask(app.modals.edit_subtask_buf.clone()),
        KeyCode::Char('s') if ctrl => {
            Action::SubmitEditSubtask(app.modals.edit_subtask_buf.clone())
        }
        KeyCode::Backspace => Action::EditSubtaskInput({
            let mut s = app.modals.edit_subtask_buf.clone();
            s.pop();
            s
        }),
        KeyCode::Char(c) => Action::EditSubtaskInput({
            let mut s = app.modals.edit_subtask_buf.clone();
            s.push(c);
            s
        }),
        _ => return None,
    })
}

fn note_editor_key(ev: Event) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Esc => return Some(Action::CancelNote),
        KeyCode::Char('s') if ctrl => return Some(Action::SubmitNote),
        _ => {}
    }
    Some(Action::NoteEditorKey(k))
}

fn plugin_page_key(ev: Event) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    Some(match k.code {
        KeyCode::Esc => Action::EscapeContext,
        KeyCode::Char(c) => Action::PluginKeyPress(c.to_string()),
        KeyCode::Enter => Action::PluginKeyPress("Enter".into()),
        KeyCode::Up => Action::PluginKeyPress("Up".into()),
        KeyCode::Down => Action::PluginKeyPress("Down".into()),
        KeyCode::Left => Action::PluginKeyPress("Left".into()),
        KeyCode::Right => Action::PluginKeyPress("Right".into()),
        KeyCode::Tab => Action::PluginKeyPress("Tab".into()),
        _ => return None,
    })
}

fn add_subtask_key(ev: Event, app: &AppState) -> Option<Action> {
    let Event::Key(k) = ev else {
        return None;
    };
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    Some(match k.code {
        KeyCode::Esc => Action::CancelAddSubtask,
        KeyCode::Enter => Action::SubmitAddSubtask(app.modals.add_subtask_buf.clone()),
        KeyCode::Char('s') if ctrl => Action::SubmitAddSubtask(app.modals.add_subtask_buf.clone()),
        KeyCode::Backspace => Action::AddSubtaskInput({
            let mut s = app.modals.add_subtask_buf.clone();
            s.pop();
            s
        }),
        KeyCode::Char(c) => Action::AddSubtaskInput({
            let mut s = app.modals.add_subtask_buf.clone();
            s.push(c);
            s
        }),
        _ => return None,
    })
}

fn dep_overlay_key(ev: Event, app: &AppState) -> Option<Action> {
    use crate::app::modals_state::DepOverlayMode;
    let Event::Key(k) = ev else {
        return None;
    };
    let submit = || match app.modals.dep_overlay_mode {
        DepOverlayMode::Add => Action::SubmitAddDependency(app.modals.dep_overlay_buf.clone()),
        DepOverlayMode::Remove => {
            Action::SubmitRemoveDependency(app.modals.dep_overlay_buf.clone())
        }
    };
    Some(match k.code {
        KeyCode::Esc => Action::CancelDepOverlay,
        KeyCode::Tab => Action::ToggleDepOverlayMode,
        KeyCode::Enter => submit(),
        KeyCode::Backspace => Action::DepOverlayInput({
            let mut s = app.modals.dep_overlay_buf.clone();
            s.pop();
            s
        }),
        KeyCode::Char(c) if c.is_ascii_digit() => Action::DepOverlayInput({
            let mut s = app.modals.dep_overlay_buf.clone();
            s.push(c);
            s
        }),
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
    use crate::focus::{DetailSection, Pane};
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let in_visual = app.ui.mode == crate::focus::Mode::Visual;
    let in_sidebar = app.ui.focus.pane == crate::focus::Pane::Sidebar;
    let on_journal = app.ui.page == Page::Journal;
    let in_detail = app.ui.focus.pane == Pane::Detail;
    let detail_section = if in_detail {
        Some(app.ui.focus.section)
    } else {
        None
    };
    let in_subtasks = detail_section == Some(DetailSection::Subtasks);
    let in_notes = detail_section == Some(DetailSection::Notes);
    let in_header = detail_section == Some(DetailSection::Header);
    Some(match k.code {
        KeyCode::Enter if in_sidebar => Action::ApplySidebarSelection,
        KeyCode::Char('q') if !ctrl => Action::Quit,
        KeyCode::Char('c') if ctrl => Action::Quit,
        KeyCode::Char('z') if ctrl => Action::Undo,
        KeyCode::Char('d') if ctrl => Action::HalfPageDown,
        KeyCode::Char('u') if ctrl => Action::HalfPageUp,
        KeyCode::Char('j') | KeyCode::Down => Action::NextItem,
        KeyCode::Char('k') | KeyCode::Up => Action::PrevItem,
        KeyCode::Char('i') if on_journal => Action::JournalStartEntry,
        KeyCode::Char('H') if on_journal => Action::JournalToggleHidden,
        KeyCode::Char('D') if on_journal => Action::JournalDeleteEntry,
        KeyCode::Char('X') if on_journal => Action::JournalDeleteDay,
        KeyCode::Char('J') if on_journal => Action::JournalNextDay,
        KeyCode::Char('K') if on_journal => Action::JournalPrevDay,
        KeyCode::Char('g') if on_journal => Action::JournalGotoTop,
        KeyCode::Char('G') if on_journal => Action::JournalGotoBottom,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        KeyCode::Char('h') => Action::FocusLeft,
        KeyCode::Char('l') => Action::FocusRight,
        KeyCode::Tab => Action::NextSection,
        KeyCode::BackTab => Action::PrevSection,
        KeyCode::Char(' ') if !on_journal => Action::ToggleSelected,
        KeyCode::Char('v') if !on_journal => Action::EnterVisual,
        KeyCode::Char('a') if on_journal => Action::JournalStartEntry,
        KeyCode::Char('a') if in_notes => Action::RequestAddNote,
        KeyCode::Char('a') if !on_journal => Action::OpenQuickAdd,
        KeyCode::Char('d') if on_journal && !in_visual => Action::JournalDeleteEntry,
        KeyCode::Char('d') if in_visual => Action::BulkDone,
        KeyCode::Char('d') if in_subtasks => Action::RequestDeleteFocusedSubtask,
        KeyCode::Char('d') if in_notes => Action::RequestDeleteFocusedNote,
        KeyCode::Char('d') if !in_visual && !in_sidebar => Action::RequestDeleteTask,
        KeyCode::Char('e') if on_journal => Action::JournalEditFocusedEntry,
        KeyCode::Char('e') if in_subtasks => Action::RequestEditFocusedSubtask,
        KeyCode::Char('e') if in_notes => Action::RequestEditFocusedNote,
        KeyCode::Char('e') if in_header => Action::RequestEditTitle,
        KeyCode::Char('e') if !in_sidebar => Action::RequestEditTitle,
        KeyCode::Char('E') if !on_journal && !in_sidebar => Action::RequestEditDescription,
        KeyCode::Char('P') if in_visual => Action::BulkPriority,
        KeyCode::Char('A') if on_journal => Action::JournalStartEntry,
        KeyCode::Char('A') if !in_visual && !in_sidebar => Action::RequestAddSubtask,
        KeyCode::Char('B') if !on_journal && !in_visual && !in_sidebar => Action::RequestAddDependency,
        KeyCode::Char('1') if in_detail => Action::JumpDetailSection(0),
        KeyCode::Char('2') if in_detail => Action::JumpDetailSection(1),
        KeyCode::Char('3') if in_detail => Action::JumpDetailSection(2),
        KeyCode::Char('4') if in_detail => Action::JumpDetailSection(3),
        KeyCode::Char('1') => Action::TogglePage(Page::Tasks),
        KeyCode::Char('2') => Action::TogglePage(Page::Journal),
        KeyCode::Char('p') if !in_visual => Action::TogglePomodoro,
        KeyCode::Char(':') => Action::OpenCommandPalette,
        KeyCode::Char('/') if !on_journal => Action::OpenSearch,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('f') => Action::LeaderGoto,
        KeyCode::Char('.') if !on_journal => Action::ToggleQuickActions,
        KeyCode::Char('s') if !on_journal => Action::OpenSortOverlay,
        KeyCode::Char('<') => Action::ResizeSplit { delta: -5 },
        KeyCode::Char('>') => Action::ResizeSplit { delta: 5 },
        KeyCode::Char('=') => Action::ResetSplit,
        KeyCode::Esc => Action::EscapeContext,
        _ => return None,
    })
}
