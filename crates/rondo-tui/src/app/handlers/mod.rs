//! Action handlers extracted from `AppState::update()`.
//!
//! Each module owns the cross-cutting logic for a coherent group of
//! `Action` variants. Handlers take `&mut AppState` and may dispatch
//! follow-up `Action`s via the returned `Option<Action>` — the main
//! `update()` loop preserves the follow-up dispatch tail.
//!
//! The split mirrors the action vocabulary, not the call graph: each
//! module corresponds to a *domain* (journal entries, subtasks,
//! dependencies, …), keeping related state mutations together.

pub mod dep;
pub mod journal;
pub mod pomodoro;
pub mod subtask;
pub mod task;
