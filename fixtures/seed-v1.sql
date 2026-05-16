-- Pre-migration schema: tasks table WITHOUT `metadata` column.
-- Used by migrations_smoke tests to verify v0 -> v1 ALTER TABLE.
-- Intentionally does NOT set PRAGMA user_version (defaults to 0).
CREATE TABLE tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  description TEXT,
  status INTEGER NOT NULL DEFAULT 0,
  priority INTEGER NOT NULL DEFAULT 0,
  due_date TEXT,
  created_at TEXT NOT NULL,
  recur_freq INTEGER NOT NULL DEFAULT 0,
  recur_interval INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE subtasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  title TEXT NOT NULL,
  completed INTEGER NOT NULL DEFAULT 0,
  position INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE tags (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE time_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  duration INTEGER NOT NULL,
  note TEXT,
  logged_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE task_dependencies (
  task_id INTEGER NOT NULL,
  blocked_by INTEGER NOT NULL,
  PRIMARY KEY (task_id, blocked_by)
);
CREATE TABLE task_notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE TABLE journal_notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  date TEXT NOT NULL UNIQUE,
  hidden INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE journal_entries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  note_id INTEGER NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (note_id) REFERENCES journal_notes(id) ON DELETE CASCADE
);

INSERT INTO tasks (id, title, description, status, priority, due_date, created_at) VALUES
  (1, 'Review API spec', 'Check RFC.', 1, 2, date('now'), datetime('now')),
  (2, 'Deploy v2.1', 'Shipped.', 2, 1, date('now','-1 day'), datetime('now','-3 days'));

INSERT INTO subtasks (task_id, title, completed, position) VALUES
  (1, 'Research tools', 1, 0),
  (1, 'Setup repo', 0, 1);

INSERT INTO tags (task_id, name) VALUES
  (1, 'work');

INSERT INTO journal_notes (date, hidden, created_at, updated_at) VALUES
  (date('now'), 0, datetime('now'), datetime('now'));
