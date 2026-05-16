use strum::Display;

#[derive(Debug, Clone, Display)]
pub enum Action {
    Tick,
    Quit,
    Render,
    Resize { width: u16, height: u16 },

    NextItem,
    PrevItem,
    SelectItem(usize),
    NextTab,
    PrevTab,
    TogglePage(Page),
    JumpTop,
    JumpBottom,
    HalfPageDown,
    HalfPageUp,

    FocusNext,
    FocusLeft,
    FocusRight,
    NextSection,
    PrevSection,
    ToggleSelected,
    ResizeSplit { delta: i16 },
    ResetSplit,

    OpenPomodoro,
    ClosePomodoro,
    TogglePomodoro,
    OpenCommandPalette,
    CloseCommandPalette,
    OpenHelp,
    CloseHelp,
    ToggleHelp,
    OpenSearch,
    CloseSearch,
    SearchUpdate(String),
    SearchInput(String),
    SubmitCommand(String),
    EscapeContext,

    EnterVisual,
    BulkDone,
    BulkPriority,
    OpenQuickAdd,
    QuickAddUpdate(String),
    SubmitQuickAdd(String),

    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Tasks,
    Journal,
}
