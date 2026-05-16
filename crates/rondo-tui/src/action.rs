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

    FocusNext,
    ResizeSplit { delta: i16 },

    OpenPomodoro,
    ClosePomodoro,
    TogglePomodoro,
    OpenCommandPalette,
    CloseCommandPalette,
    SearchInput(String),
    SubmitCommand(String),

    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Tasks,
    Journal,
}
