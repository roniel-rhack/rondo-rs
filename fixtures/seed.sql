CREATE TABLE tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  description TEXT,
  status INTEGER NOT NULL DEFAULT 0,
  priority INTEGER NOT NULL DEFAULT 0,
  due_date TEXT,
  created_at TEXT NOT NULL,
  recur_freq INTEGER NOT NULL DEFAULT 0,
  recur_interval INTEGER NOT NULL DEFAULT 0,
  metadata TEXT
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

INSERT INTO tasks (id, title, description, status, priority, due_date, created_at, metadata) VALUES
  (1, 'Review API spec', '# Goals

Check **RFC #42** for the new auth flow.

- Validate OAuth scopes
- Cross-check with `internal/auth/middleware.go`', 1, 2, date('now'), datetime('now'), '{"owner":"me"}'),
  (2, 'Deploy v2.1', 'Shipped with new endpoint.', 2, 1, date('now','-1 day'), datetime('now','-3 days'), '{}'),
  (3, 'Buy groceries', 'Milk, eggs, bread', 0, 0, date('now','+2 days'), datetime('now','-1 hour'), '{}'),
  (4, 'Refactor pomodoro module', 'Pending until Review API spec done.', 0, 3, date('now','+1 day'), datetime('now','-2 hour'), '{}');

INSERT INTO subtasks (task_id, title, completed, position) VALUES
  (1, 'Research tools', 1, 0),
  (1, 'Setup repo', 0, 1),
  (1, 'Config testing', 0, 2),
  (1, 'Deploy staging', 0, 3),
  (1, 'Team review', 0, 4);

INSERT INTO tags (task_id, name) VALUES
  (1, 'work'), (1, 'backend'),
  (3, 'personal'),
  (4, 'work'), (4, 'refactor');

INSERT INTO time_logs (task_id, duration, note, logged_at) VALUES
  (1, 2700, 'morning session', datetime('now','-30 minutes')),
  (1, 1500, 'pair with Sarah', datetime('now','-2 hours'));

INSERT INTO task_dependencies (task_id, blocked_by) VALUES (4, 1);

INSERT INTO task_notes (task_id, body, created_at) VALUES
  (1, 'Carlos reviewed at 1:30 PM, said schema looks good.', datetime('now','-45 minutes'));

INSERT INTO journal_notes (date, hidden, created_at, updated_at) VALUES
  (date('now'), 0, datetime('now'), datetime('now')),
  (date('now','-1 day'), 0, datetime('now','-1 day'), datetime('now','-1 day'));

INSERT INTO journal_entries (note_id, body, created_at) VALUES
  (1, '# Thursday standup

Shipped **v2.1** with new API endpoint. Carlos reviewed at 1:30 PM.

- Fixed cache invalidation
- Blocked by infra ticket #1023
- Pair coded with Sarah on gRPC proto', datetime('now','-3 hours')),
  (1, '## Reflections

Started strong but hit complexity in *auth refactor*. Good recovery.', datetime('now','-1 hour')),
  (2, '# Wednesday

Worked on the pomodoro module rewrite. Slow progress.', datetime('now','-1 day'));
