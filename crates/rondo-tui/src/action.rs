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

    ToggleQuickActions,
    CloseQuickActions,
    ApplySidebarSelection,
    ApplyFilter(crate::filter::Filter),
    LeaderGoto,
    EnterVisual,
    BulkDone,
    BulkPriority,
    OpenQuickAdd,
    QuickAddUpdate(String),
    SubmitQuickAdd(String),

    JournalStartEntry,
    JournalEntryInput(String),
    JournalEditorKey(crossterm::event::KeyEvent),
    JournalSubmitEntry,
    JournalCancelEntry,
    JournalDeleteDay,
    JournalToggleHidden,
    JournalGotoTop,
    JournalGotoBottom,
    JournalDeleteEntry,
    JournalEditFocusedEntry,
    JournalNextEntry,
    JournalPrevEntry,
    JournalNextDay,
    JournalPrevDay,

    OpenSortOverlay,
    CloseSortOverlay,
    SetSortOrder(crate::app::ui_state::SortOrder),

    RequestDeleteTask,
    ConfirmDeleteTask,
    CancelDelete,
    RequestEditTitle,
    EditTitleInput(String),
    SubmitEditTitle(String),
    CancelEditTitle,
    ToggleFocusedSubtask,

    RequestAddSubtask,
    AddSubtaskInput(String),
    SubmitAddSubtask(String),
    CancelAddSubtask,

    RequestAddDependency,
    DepOverlayInput(String),
    SubmitAddDependency(String),
    SubmitRemoveDependency(String),
    ToggleDepOverlayMode,
    CancelDepOverlay,

    PluginKeyPress(String),

    Undo,

    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Tasks,
    Journal,
}
