use rondo_core::domain::task::UndoSnapshot;
use std::collections::VecDeque;

const CAP: usize = 50;

#[derive(Default)]
pub struct UndoStack {
    entries: VecDeque<UndoSnapshot>,
}

impl UndoStack {
    pub fn push(&mut self, snap: UndoSnapshot) {
        if self.entries.len() == CAP {
            self.entries.pop_front();
        }
        self.entries.push_back(snap);
    }

    pub fn pop(&mut self) -> Option<UndoSnapshot> {
        self.entries.pop_back()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rondo_core::domain::task::{UndoKind, UndoSnapshot};

    fn snap(kind: UndoKind) -> UndoSnapshot {
        UndoSnapshot {
            kind,
            task_before: None,
            created_id: None,
        }
    }

    #[test]
    fn push_pop_lifo() {
        let mut s = UndoStack::default();
        s.push(snap(UndoKind::Create));
        s.push(snap(UndoKind::Delete));
        assert!(matches!(s.pop().unwrap().kind, UndoKind::Delete));
        assert!(matches!(s.pop().unwrap().kind, UndoKind::Create));
        assert!(s.pop().is_none());
    }

    #[test]
    fn cap_drops_oldest() {
        let mut s = UndoStack::default();
        for _ in 0..(CAP + 5) {
            s.push(snap(UndoKind::Create));
        }
        assert_eq!(s.len(), CAP);
    }

    #[test]
    fn is_empty_and_clear() {
        let mut s = UndoStack::default();
        assert!(s.is_empty());
        s.push(snap(UndoKind::Create));
        assert!(!s.is_empty());
        s.clear();
        assert!(s.is_empty());
    }
}
