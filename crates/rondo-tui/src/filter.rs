use chrono::Local;
use rondo_core::domain::task::{Priority, Status, Task};

/// What subset of tasks the user is currently looking at.
/// Drives both the sidebar highlight and the task_list filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Inbox,
    Today,
    Upcoming,
    AllProjects,
    AllTags,
    Calendar,
    Analysis,
    Graph,
    Automations,
    Trash,
    // Quick filters
    Urgent,
    HighPriority,
    AssignedToMe,
    NoTag,
    Completed,
    Overdue,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Self::Inbox => "INBOX",
            Self::Today => "HOY",
            Self::Upcoming => "PRÓXIMAS",
            Self::AllProjects => "PROYECTOS",
            Self::AllTags => "ETIQUETAS",
            Self::Calendar => "CALENDARIO",
            Self::Analysis => "ANÁLISIS",
            Self::Graph => "GRAFO",
            Self::Automations => "AUTOMAT.",
            Self::Trash => "PAPELERA",
            Self::Urgent => "URGENTES",
            Self::HighPriority => "ALTA PRIO",
            Self::AssignedToMe => "ASIG. A MÍ",
            Self::NoTag => "SIN ETIQUETA",
            Self::Completed => "COMPLETADAS",
            Self::Overdue => "VENCIDAS",
        }
    }

    pub fn applies_to(self, task: &Task) -> bool {
        let today = Local::now().date_naive();
        match self {
            Self::Inbox => task.status != Status::Done,
            Self::Today => task.due_date == Some(today) && task.status != Status::Done,
            Self::Upcoming => task.status != Status::Done
                && task
                    .due_date
                    .is_some_and(|d| (1..=7).contains(&(d - today).num_days())),
            Self::Urgent => task.priority == Priority::Urgent,
            Self::HighPriority => matches!(task.priority, Priority::High | Priority::Urgent),
            Self::AssignedToMe => true, // single-user app
            Self::NoTag => task.tags.is_empty(),
            Self::Completed => task.status == Status::Done,
            Self::Overdue => task.status != Status::Done
                && task.due_date.is_some_and(|d| d < today),
            // Stubs — no task field yet for projects/calendar/etc.
            Self::AllProjects | Self::AllTags | Self::Calendar | Self::Analysis
            | Self::Graph | Self::Automations | Self::Trash => true,
        }
    }
}

/// Ordered list of all sidebar items (navigation + quick filters).
pub const SIDEBAR_ITEMS: &[Filter] = &[
    Filter::Inbox,
    Filter::Today,
    Filter::Upcoming,
    Filter::AllProjects,
    Filter::AllTags,
    Filter::Calendar,
    Filter::Analysis,
    Filter::Graph,
    Filter::Automations,
    Filter::Trash,
    Filter::Urgent,
    Filter::HighPriority,
    Filter::AssignedToMe,
    Filter::NoTag,
    Filter::Completed,
    Filter::Overdue,
];

/// Boundary between nav block and quick-filter block in SIDEBAR_ITEMS.
pub const NAV_BLOCK_LEN: usize = 10;
