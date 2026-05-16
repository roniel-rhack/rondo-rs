use strum::{Display, EnumIter, IntoEnumIterator};

/// Top-level pane that has user focus inside the current page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum Pane {
    Sidebar,
    List,
    Detail,
}

/// Sections cycled by Tab/Shift+Tab within the Detail pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumIter)]
pub enum DetailSection {
    Header,
    Subtasks,
    Dependencies,
    Notes,
}

impl DetailSection {
    pub fn next(self) -> Self {
        let all: Vec<Self> = Self::iter().collect();
        let idx = all.iter().position(|s| *s == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
    pub fn prev(self) -> Self {
        let all: Vec<Self> = Self::iter().collect();
        let idx = all.iter().position(|s| *s == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Subtasks => "subtasks",
            Self::Dependencies => "deps",
            Self::Notes => "notes",
        }
    }
}

/// Concrete focus stack: Page is in `AppState.page`, this captures the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusState {
    pub pane: Pane,
    pub section: DetailSection,
    /// Cursor within the active section's collection (subtask idx, note idx, dep idx).
    pub section_item: usize,
    /// Cursor within the sidebar item list (0..NAV_ITEMS.len() + filters).
    pub sidebar_item: usize,
}

impl Default for FocusState {
    fn default() -> Self {
        Self {
            pane: Pane::List,
            section: DetailSection::Header,
            section_item: 0,
            sidebar_item: 0,
        }
    }
}

/// Editing mode (vim-style). Search/palette/quick-add all map to Insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
}

impl Mode {
    pub fn tag(self) -> &'static str {
        match self {
            Mode::Normal => "NOR",
            Mode::Insert => "INS",
            Mode::Visual => "VIS",
        }
    }
}
