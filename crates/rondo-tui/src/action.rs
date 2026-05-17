use strum::Display;

#[derive(Debug, Clone, Display)]
pub enum Action {
    Tick,
    Quit,
    Render,
    Resize {
        width: u16,
        height: u16,
    },

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
    JumpDetailSection(u8),
    ToggleSelected,
    ResizeSplit {
        delta: i16,
    },
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
    BulkDelete,
    BulkSetStatus(rondo_core::domain::task::Status),
    BulkSetDueDate(Option<chrono::NaiveDate>),
    BulkAddTag(String),
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
    DepPickerNext,
    DepPickerPrev,
    /// Submit the highlighted picker candidate (Enter in Add mode).
    SubmitDepPickerHighlighted,

    PluginKeyPress(String),
    /// Bracketed-paste payload from the terminal. Routed by the app
    /// dispatcher to whichever input surface is currently open.
    Paste(String),

    RequestEditDescription,
    DescriptionEditorKey(crossterm::event::KeyEvent),
    SubmitEditDescription,
    CancelEditDescription,

    RequestEditFocusedSubtask,
    EditSubtaskInput(String),
    SubmitEditSubtask(String),
    CancelEditSubtask,
    RequestDeleteFocusedSubtask,

    RequestAddNote,
    RequestEditFocusedNote,
    RequestDeleteFocusedNote,
    NoteEditorKey(crossterm::event::KeyEvent),
    SubmitNote,
    CancelNote,

    RequestEditRecurrence,
    SubmitRecurrence(rondo_core::domain::task::RecurFreq, i64),
    CancelEditRecurrence,

    RequestEditDueDate,
    EditDueDateInput(String),
    /// `None` clears the date; `Some(date)` sets it. Empty string in
    /// custom-input mode falls through to a no-op + toast.
    SubmitDueDate(Option<chrono::NaiveDate>),
    CancelEditDueDate,

    Undo,

    OpenLangPicker,
    CloseLangPicker,
    LangPickerMoveUp,
    LangPickerMoveDown,
    /// Apply the highlighted pack: persist `[ui].language`, swap the active
    /// `Translations`, close the picker.
    LangPickerApply,

    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Tasks,
    Journal,
}
