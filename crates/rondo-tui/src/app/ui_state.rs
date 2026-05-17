use crate::action::{Action, Page};
use crate::focus::{FocusState, Mode, Pane};
use ratatui::layout::Rect;
use std::collections::HashSet;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashTarget {
    Task(i64),
    Subtask(i64),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Default: by status asc, priority desc, due asc, id desc (matches existing SQL ORDER BY).
    #[default]
    Default,
    PriorityDesc,
    DueAsc,
    CreatedAtDesc,
    TitleAsc,
}

impl SortOrder {
    pub const ALL: &'static [SortOrder] = &[
        SortOrder::Default,
        SortOrder::PriorityDesc,
        SortOrder::DueAsc,
        SortOrder::CreatedAtDesc,
        SortOrder::TitleAsc,
    ];
    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Default => "default (status, priority, due)",
            SortOrder::PriorityDesc => "priority (high to low)",
            SortOrder::DueAsc => "due date (soonest first)",
            SortOrder::CreatedAtDesc => "newest first",
            SortOrder::TitleAsc => "title (A-Z)",
        }
    }
}

pub const FLASH_DURATION_MS: u128 = 220;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum JournalPane {
    #[default]
    Days,
    Entries,
}

/// View-level state: page, focus, mode, splits, flashes, cached rects.
pub struct UiState {
    pub page: Page,
    pub focus: FocusState,
    pub mode: Mode,
    pub split_ratio: u16,
    pub selection: HashSet<i64>,
    pub flash: Option<(FlashTarget, Instant)>,
    pub leader_goto: bool,
    pub last_footer_rect: Rect,
    pub last_task_list_rect: Rect,
    pub last_detail_rect: Rect,
    pub last_body_rect: Rect,
    pub last_pomodoro_rect: Rect,
    pub last_quick_add_rect: Rect,
    pub last_journal_entries_rect: Rect,
    pub sort_order: SortOrder,
    pub journal_pane: JournalPane,
    /// First visible row in the task list. Updated by `move_selection`
    /// to keep the cursor inside the viewport, and clamped on resize.
    pub task_list_scroll: usize,
    /// Selected task as a stable id. Survives `refresh_tasks` reordering
    /// and search-induced re-sorts that would otherwise leave the index
    /// pointing at a different row.
    pub selected_task_id: Option<i64>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            page: Page::Tasks,
            focus: FocusState::default(),
            mode: Mode::Normal,
            split_ratio: 50,
            selection: HashSet::new(),
            flash: None,
            leader_goto: false,
            last_footer_rect: Rect::default(),
            last_task_list_rect: Rect::default(),
            last_detail_rect: Rect::default(),
            last_body_rect: Rect::default(),
            last_pomodoro_rect: Rect::default(),
            last_quick_add_rect: Rect::default(),
            last_journal_entries_rect: Rect::default(),
            sort_order: SortOrder::default(),
            journal_pane: JournalPane::default(),
            task_list_scroll: 0,
            selected_task_id: None,
        }
    }
}

impl UiState {
    /// Is `target` currently flashing? Does not auto-clear (see `expire_flash`).
    pub fn is_flashing(&self, target: FlashTarget) -> bool {
        match self.flash {
            Some((t, when)) if t == target => when.elapsed().as_millis() < FLASH_DURATION_MS,
            _ => false,
        }
    }

    /// Periodic housekeeping (called on tick).
    pub fn expire_flash(&mut self) {
        if let Some((_, when)) = self.flash {
            if when.elapsed().as_millis() >= FLASH_DURATION_MS {
                self.flash = None;
            }
        }
    }

    /// Backward-compat shim — still used by some pane-render code paths to color borders.
    pub fn focus_left(&self) -> bool {
        matches!(self.focus.pane, Pane::List | Pane::Sidebar)
    }

    /// Pure UI mutations that don't need cross-substate access.
    /// Returns optional follow-up for the dispatcher.
    pub fn update(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::FocusLeft => {
                if self.page == Page::Journal {
                    self.journal_pane = JournalPane::Days;
                } else {
                    self.focus.pane = match self.focus.pane {
                        Pane::Detail => Pane::List,
                        Pane::List | Pane::Sidebar => Pane::Sidebar,
                    };
                    self.focus.section_item = 0;
                }
                None
            }
            Action::FocusRight => {
                if self.page == Page::Journal {
                    self.journal_pane = JournalPane::Entries;
                } else {
                    self.focus.pane = match self.focus.pane {
                        Pane::Sidebar => Pane::List,
                        Pane::List | Pane::Detail => Pane::Detail,
                    };
                    self.focus.section_item = 0;
                }
                None
            }
            Action::FocusNext => {
                self.focus.pane = match self.focus.pane {
                    Pane::Sidebar => Pane::List,
                    Pane::List => Pane::Detail,
                    Pane::Detail => Pane::Sidebar,
                };
                self.focus.section_item = 0;
                None
            }
            Action::NextSection => {
                if self.focus.pane == Pane::Detail {
                    self.focus.section = self.focus.section.next();
                    self.focus.section_item = 0;
                }
                None
            }
            Action::PrevSection => {
                if self.focus.pane == Pane::Detail {
                    self.focus.section = self.focus.section.prev();
                    self.focus.section_item = 0;
                }
                None
            }
            Action::JumpDetailSection(idx) => {
                use crate::focus::DetailSection;
                let target = match idx {
                    0 => DetailSection::Header,
                    1 => DetailSection::Subtasks,
                    2 => DetailSection::Dependencies,
                    3 => DetailSection::Notes,
                    _ => return None,
                };
                self.focus.pane = Pane::Detail;
                self.focus.section = target;
                self.focus.section_item = 0;
                None
            }
            Action::ResetSplit => {
                self.split_ratio = 50;
                None
            }
            Action::ResizeSplit { delta } => {
                let new = self.split_ratio as i32 + delta as i32;
                self.split_ratio = new.clamp(20, 80) as u16;
                None
            }
            Action::LeaderGoto => {
                self.leader_goto = true;
                None
            }
            _ => None,
        }
    }
}
