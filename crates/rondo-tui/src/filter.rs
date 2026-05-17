use chrono::Local;
use rondo_core::domain::task::{Priority, Status, Task};

/// Active subset of tasks. Only filters that actually apply to rondo's
/// data model are kept — no stub entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Filter {
    Inbox,
    Today,
    Upcoming,
    NoTag,
    Urgent,
    HighPriority,
    Overdue,
    InProgress,
    Completed,
    All,
}

impl Filter {
    /// UPPERCASE label shown in sidebar.
    pub fn label(self) -> &'static str {
        match self {
            Self::Inbox => "INBOX",
            Self::Today => "HOY",
            Self::Upcoming => "PRÓXIMAS",
            Self::NoTag => "SIN ETIQUETA",
            Self::Urgent => "URGENTES",
            Self::HighPriority => "ALTA PRIO",
            Self::Overdue => "VENCIDAS",
            Self::InProgress => "EN PROGRESO",
            Self::Completed => "COMPLETADAS",
            Self::All => "TODAS",
        }
    }

    /// Single-letter shortcut shown as `[x]` prefix and bound to the keyboard.
    pub fn shortcut(self) -> char {
        match self {
            Self::Inbox => 'i',
            Self::Today => 't',
            Self::Upcoming => 'p', // próximas
            Self::NoTag => 'n',
            Self::Urgent => 'u',
            Self::HighPriority => 'H',
            Self::Overdue => 'o',
            Self::InProgress => 'P',
            Self::Completed => 'c',
            Self::All => 'A',
        }
    }

    /// Icon glyph rendered on the left of the row.
    pub fn icon(self) -> &'static str {
        match self {
            Self::Inbox => "◉",
            Self::Today => "◷",
            Self::Upcoming => "⏵",
            Self::NoTag => "#",
            Self::Urgent => "!",
            Self::HighPriority => "↑",
            Self::Overdue => "⌧",
            Self::InProgress => "◐",
            Self::Completed => "✓",
            Self::All => "◇",
        }
    }

    pub fn applies_to(self, task: &Task) -> bool {
        let today = Local::now().date_naive();
        match self {
            Self::Inbox => task.status != Status::Done,
            Self::Today => task.status != Status::Done && task.due_date == Some(today),
            Self::Upcoming => {
                task.status != Status::Done
                    && task
                        .due_date
                        .is_some_and(|d| (1..=7).contains(&(d - today).num_days()))
            }
            Self::NoTag => task.tags.is_empty(),
            Self::Urgent => task.priority == Priority::Urgent && task.status != Status::Done,
            Self::HighPriority => {
                matches!(task.priority, Priority::High | Priority::Urgent)
                    && task.status != Status::Done
            }
            Self::Overdue => {
                task.status != Status::Done && task.due_date.is_some_and(|d| d < today)
            }
            Self::InProgress => task.status == Status::InProgress,
            Self::Completed => task.status == Status::Done,
            Self::All => true,
        }
    }
}

/// Ordered sidebar items. First block = main nav; second block = "quick" filters.
pub const SIDEBAR_ITEMS: &[Filter] = &[
    Filter::Inbox,
    Filter::Today,
    Filter::Upcoming,
    Filter::All,
    Filter::Urgent,
    Filter::HighPriority,
    Filter::Overdue,
    Filter::InProgress,
    Filter::NoTag,
    Filter::Completed,
];

/// Boundary between nav block and quick-filter block.
pub const NAV_BLOCK_LEN: usize = 4;

/// Resolve a key character to a filter, ignoring case where applicable.
pub fn by_shortcut(c: char) -> Option<Filter> {
    SIDEBAR_ITEMS.iter().copied().find(|f| f.shortcut() == c)
}
