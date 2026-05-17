-- Demo seed used by VHS recordings only. Not loaded at runtime; copied into
-- a throwaway DB by `scripts/demo-seed.sh` before each tape runs so the
-- recordings show a rich, deterministic snapshot of the app.

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
CREATE TABLE focus_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER,
  kind INTEGER NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  duration_secs INTEGER NOT NULL,
  phase INTEGER NOT NULL DEFAULT 0,
  cycle_idx INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
);
CREATE INDEX idx_focus_sessions_started_at ON focus_sessions(started_at);
CREATE TABLE plugin_kv (
  plugin_id TEXT NOT NULL,
  key TEXT NOT NULL,
  value BLOB NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (plugin_id, key)
);
CREATE INDEX idx_plugin_kv_plugin ON plugin_kv(plugin_id);

PRAGMA user_version = 4;

-- Status: 0=Todo, 1=InProgress, 2=Done
-- Priority: 0=Low, 1=Med, 2=High, 3=Urgent
INSERT INTO tasks (id, title, description, status, priority, due_date, created_at, recur_freq, recur_interval, metadata) VALUES
  (1,  'Review API spec v2',
       '# Goals

Check **RFC #42** for the new auth flow.

- Validate OAuth scopes
- Cross-check with `internal/auth/middleware.go`
- Confirm error envelope shape',
       1, 2, date('now'), datetime('now','-3 hours'), 0, 0, '{"owner":"me"}'),
  (2,  'Ship release v2.1.0',
       'Cut tag, push to crates.io, announce on Mastodon.',
       2, 1, date('now','-1 day'), datetime('now','-2 days'), 0, 0, '{}'),
  (3,  'Buy groceries',
       'Milk, eggs, bread, olive oil, fresh basil.',
       0, 0, date('now','+1 day'), datetime('now','-1 hour'), 0, 0, '{}'),
  (4,  'Refactor pomodoro module',
       '> Blocked on API spec review.

Split the timer state machine out of `ui::overlay`.',
       0, 3, date('now','+1 day'), datetime('now','-2 hours'), 0, 0, '{}'),
  (5,  'Write blog post: TUIs in Rust',
       '## Outline

1. Why a TUI?
2. ratatui + tachyonfx
3. extism plugins
4. Performance notes',
       1, 1, date('now','+3 days'), datetime('now','-5 hours'), 0, 0, '{}'),
  (6,  'Read "Designing Data-Intensive Applications" ch. 7',
       'Focus on isolation levels and snapshot semantics.',
       0, 1, date('now','+5 days'), datetime('now','-1 day'), 0, 0, '{}'),
  (7,  'Fix flaky snapshot test in journal',
       'Insta diff swings on weekday boundary — timestamp redaction misses the header.',
       0, 2, date('now'), datetime('now','-30 minutes'), 0, 0, '{}'),
  (8,  'Reply to Sarah about gRPC schema',
       NULL,
       0, 2, date('now'), datetime('now','-15 minutes'), 0, 0, '{}'),
  (9,  'Renew domain rondo.sh',
       'Auto-renew is on but card expires in June.',
       0, 1, date('now','+14 days'), datetime('now','-3 days'), 0, 0, '{}'),
  (10, 'Workout — leg day',
       NULL,
       2, 0, date('now','-1 day'), datetime('now','-1 day'), 1, 7, '{}'),
  (11, 'Plan Q3 OKRs',
       '## Themes

- Plugin ecosystem
- Mobile companion (read-only)
- Cloud sync GA',
       0, 2, date('now','+10 days'), datetime('now','-2 days'), 0, 0, '{}'),
  (12, 'Migrate CI to GitHub Actions',
       'Move from CircleCI. Keep matrix on stable + 1.83.',
       1, 1, date('now','+2 days'), datetime('now','-4 days'), 0, 0, '{}'),
  (13, 'Daily standup notes',
       'Roll up yesterday + today + blockers.',
       0, 1, date('now'), datetime('now','-1 day'), 1, 1, '{}'),
  (14, 'Pay rent',
       NULL,
       0, 3, date('now','+3 days'), datetime('now','-1 day'), 1, 30, '{}'),
  (15, 'Review Carlos PR #128',
       'New theme tokens — make sure no hex literals slip in.',
       0, 2, date('now'), datetime('now','-90 minutes'), 0, 0, '{}');

INSERT INTO subtasks (task_id, title, completed, position) VALUES
  (1,  'Read RFC #42 end-to-end',        1, 0),
  (1,  'List affected endpoints',        1, 1),
  (1,  'Diff against current contract',  0, 2),
  (1,  'Draft migration note',           0, 3),
  (1,  'Send to Carlos for review',      0, 4),
  (4,  'Extract timer state machine',    0, 0),
  (4,  'Move overlay rendering',         0, 1),
  (4,  'Wire plugin commands',           0, 2),
  (5,  'Draft outline',                  1, 0),
  (5,  'Write intro section',            1, 1),
  (5,  'Add screenshots',                0, 2),
  (5,  'Edit + publish',                 0, 3),
  (11, 'Collect team input',             1, 0),
  (11, 'Score against H1 results',       0, 1),
  (11, 'Share draft with leads',         0, 2),
  (12, 'Port build matrix',              1, 0),
  (12, 'Cache cargo registry',           1, 1),
  (12, 'Add release workflow',           0, 2);

INSERT INTO tags (task_id, name) VALUES
  (1,  'work'),     (1,  'backend'),  (1,  'review'),
  (2,  'work'),     (2,  'release'),
  (3,  'personal'), (3,  'errands'),
  (4,  'work'),     (4,  'refactor'),
  (5,  'writing'),  (5,  'personal'),
  (6,  'reading'),  (6,  'learning'),
  (7,  'work'),     (7,  'bug'),
  (8,  'work'),     (8,  'comms'),
  (9,  'personal'), (9,  'admin'),
  (10, 'health'),
  (11, 'work'),     (11, 'planning'),
  (12, 'work'),     (12, 'devops'),
  (13, 'work'),
  (14, 'personal'), (14, 'admin'),
  (15, 'work'),     (15, 'review');

INSERT INTO time_logs (task_id, duration, note, logged_at) VALUES
  (1,  2700, 'morning deep read',       datetime('now','-2 hours')),
  (1,  1500, 'pair with Sarah',         datetime('now','-1 hour')),
  (5,  1500, 'draft intro',             datetime('now','-4 hours')),
  (5,  1500, 'tighten outline',         datetime('now','-3 hours')),
  (12, 1500, 'matrix port',             datetime('now','-1 day')),
  (12, 1500, 'cache cargo registry',    datetime('now','-1 day','-2 hours'));

INSERT INTO task_dependencies (task_id, blocked_by) VALUES
  (4,  1),
  (15, 1),
  (11, 5);

INSERT INTO task_notes (task_id, body, created_at) VALUES
  (1, 'Carlos reviewed at 1:30 PM — schema looks good, error envelope still TBD.', datetime('now','-45 minutes')),
  (1, 'Reminder: align with the gRPC proto change before merging.',                datetime('now','-20 minutes')),
  (5, 'Pencil in screenshots once tachyonfx 0.13 lands.',                          datetime('now','-2 hours'));

INSERT INTO journal_notes (id, date, hidden, created_at, updated_at) VALUES
  (1, date('now'),         0, datetime('now'),         datetime('now')),
  (2, date('now','-1 day'),0, datetime('now','-1 day'),datetime('now','-1 day')),
  (3, date('now','-2 day'),0, datetime('now','-2 day'),datetime('now','-2 day')),
  (4, date('now','-4 day'),0, datetime('now','-4 day'),datetime('now','-4 day'));

INSERT INTO journal_entries (note_id, body, created_at) VALUES
  (1, '# Today

Shipped **v2.1.0** with the new auth endpoint. Carlos signed off at 1:30 PM.

- [x] Cut release tag
- [x] Push crate
- [ ] Write announce post
- [ ] Update changelog header

> Felt like a clean ship. Next: pomodoro refactor.',
       datetime('now','-2 hours')),
  (1, '## Afternoon notes

Started on the *pomodoro module split*. State machine wants its own file.

```rust
enum Phase { Work, ShortBreak, LongBreak }
```',
       datetime('now','-30 minutes')),
  (2, '# Yesterday

Pair coded with Sarah on the **gRPC proto**.

- Blocked by infra ticket #1023
- Fixed cache invalidation in `store::tags`
- Wrote 3 new snapshot tests',
       datetime('now','-1 day','-1 hour')),
  (3, '## Two days ago

Slow start, picked up after lunch. Got the CI matrix ported.',
       datetime('now','-2 day')),
  (4, '# Friday

Wrote draft of the *TUIs in Rust* blog post.

> "The thing about terminals is they never went away."',
       datetime('now','-4 day'));

-- Focus heatmap: scatter sessions across last 5 weeks so the 5x7 grid
-- shows variation rather than a single solid block.
INSERT INTO focus_sessions (task_id, kind, started_at, completed_at, duration_secs, phase, cycle_idx) VALUES
  (1,  0, datetime('now','-2 hours'),                  datetime('now','-2 hours','+25 minutes'), 1500, 0, 1),
  (1,  1, datetime('now','-2 hours','+25 minutes'),    datetime('now','-2 hours','+30 minutes'),  300, 1, 1),
  (5,  0, datetime('now','-4 hours'),                  datetime('now','-4 hours','+25 minutes'), 1500, 0, 2),
  (5,  0, datetime('now','-1 day'),                    datetime('now','-1 day','+25 minutes'),   1500, 0, 1),
  (12, 0, datetime('now','-1 day','-2 hours'),         datetime('now','-1 day','-1 hours'),      1500, 0, 2),
  (1,  0, datetime('now','-2 day'),                    datetime('now','-2 day','+25 minutes'),   1500, 0, 1),
  (5,  0, datetime('now','-3 day'),                    datetime('now','-3 day','+25 minutes'),   1500, 0, 1),
  (5,  0, datetime('now','-3 day','+1 hour'),          datetime('now','-3 day','+1 hour','+25 minutes'), 1500, 0, 2),
  (11, 0, datetime('now','-4 day'),                    datetime('now','-4 day','+25 minutes'),   1500, 0, 1),
  (12, 0, datetime('now','-6 day'),                    datetime('now','-6 day','+25 minutes'),   1500, 0, 1),
  (12, 0, datetime('now','-6 day','+30 minutes'),      datetime('now','-6 day','+55 minutes'),   1500, 0, 2),
  (1,  0, datetime('now','-8 day'),                    datetime('now','-8 day','+25 minutes'),   1500, 0, 1),
  (5,  0, datetime('now','-10 day'),                   datetime('now','-10 day','+25 minutes'),  1500, 0, 1),
  (5,  0, datetime('now','-13 day'),                   datetime('now','-13 day','+25 minutes'),  1500, 0, 1),
  (11, 0, datetime('now','-15 day'),                   datetime('now','-15 day','+25 minutes'),  1500, 0, 1),
  (11, 0, datetime('now','-15 day','+1 hour'),         datetime('now','-15 day','+1 hour','+25 minutes'), 1500, 0, 2),
  (1,  0, datetime('now','-18 day'),                   datetime('now','-18 day','+25 minutes'),  1500, 0, 1),
  (12, 0, datetime('now','-22 day'),                   datetime('now','-22 day','+25 minutes'),  1500, 0, 1),
  (5,  0, datetime('now','-25 day'),                   datetime('now','-25 day','+25 minutes'),  1500, 0, 1),
  (1,  0, datetime('now','-28 day'),                   datetime('now','-28 day','+25 minutes'),  1500, 0, 1);
