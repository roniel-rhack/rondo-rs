# Keybindings reference

Single source of truth for every binding. Footer hints in the TUI mirror this.

## Global

| Key | Action |
|---|---|
| `q` / `Ctrl+C` | quit |
| `?` | toggle help overlay |
| `:` | command palette (`:tasks`, `:journal`, `:pomodoro`, `:plugins`, `:calendar`, `:focus`, `:deps`, `:analytics`, `:help`, `:quit`) |
| `/` | fuzzy search overlay (tasks page only) |
| `Esc` | close top modal (stacked: plugin_page → plugins_overlay → help → confirm → edit_title → add_subtask → dep_overlay → description_editor → edit_subtask → note_editor → sort_overlay → quick_actions → journal_editor → quick_add → command_palette → search → visual → pomodoro → status) |
| `Ctrl+Z` | undo last mutation (cap 50) |
| `1` | switch to Tasks page (outside Detail pane) |
| `2` | switch to Journal page |
| `h` / `l` | focus left / right |
| `Tab` / `Shift+Tab` | next / prev section (in Detail pane) or next/prev pane elsewhere |
| `<` `>` | resize split ±5 % |
| `=` | reset split 50/50 |
| `p` | toggle pomodoro overlay |
| `.` | quick-actions grid overlay |
| `f<letter>` | filter leader — letter applies the filter chip (i/t/p/A/u/H/o/n/c) |

## Tasks page — list pane

| Key | Action |
|---|---|
| `j` / `k` / `↓` / `↑` | next/prev task |
| `g` / `G` | jump top / bottom |
| `Ctrl+D` / `Ctrl+U` | half page down / up |
| `a` | quick-add task (supports `title #tag !p3 due:tmrw` syntax) |
| `v` | enter Visual multi-select |
| `d` (Visual) | bulk done |
| `P` (Visual) | bulk priority |
| `d` (non-Visual) | request delete task (confirm modal) |
| `e` | edit task title |
| `E` | edit description (multiline) |
| `A` | add subtask to focused task |
| `B` | add/remove dependency to focused task |
| `space` | toggle status |
| `s` | sort selector overlay |
| `Ctrl+D` / `Ctrl+U` | half-page |

## Tasks page — detail pane (section-scoped)

`Tab` / `Shift+Tab` cycle sections. `1/2/3/4` jump directly to Header / Subtasks / Deps / Notes.

| Section | Key | Action |
|---|---|---|
| Header | `e` | edit title |
| Header | `E` | edit description (multiline markdown editor) |
| Header | `space` | toggle status |
| Subtasks | `A` | add subtask |
| Subtasks | `e` | rename focused subtask |
| Subtasks | `d` | delete focused subtask |
| Subtasks | `space` | toggle subtask done |
| Subtasks | `j`/`k` | move cursor between subtasks |
| Dependencies | `B` | open dependency overlay (Tab toggles Add↔Remove) |
| Notes | `a` | add note (multiline markdown editor) |
| Notes | `e` | edit focused note |
| Notes | `d` | delete focused note |
| Notes | `j`/`k` | move cursor between notes |

## Journal page

| Pane | Key | Action |
|---|---|---|
| both | `h` / `l` | switch days ↔ entries pane |
| Days | `j` / `k` | next/prev day |
| Entries | `j` / `k` | next/prev entry within day |
| both | `J` / `K` | next/prev day (works from either pane) |
| both | `g` / `G` | first / last day |
| both | `i` / `A` | new entry |
| both | `e` | edit focused entry |
| both | `d` | delete focused entry |
| both | `D` | (alias) delete focused entry |
| both | `H` | toggle hidden filter |
| both | `X` | delete focused DAY (cascade entries) |

## Journal entry editor (modal)

Powered by `tui-textarea`. Cursor navigation works fully.

| Key | Action |
|---|---|
| `Ctrl+S` | save (UPDATE if editing, INSERT otherwise) |
| `Esc` | cancel |
| `←` `↑` `↓` `→` `Home` `End` | navigate |
| `Enter` | newline |
| anything else | text input |

Same widget powers description editor (`E` from task detail), note editor (`a`/`e` in Notes section).

## Multiselect / Visual mode

| Key | Action |
|---|---|
| `v` | enter Visual mode (anchors the cursor task) |
| `j` / `k` | extend selection up/down |
| `d` | bulk done |
| `P` | bulk priority cycle |
| `Esc` | cancel Visual |

## Plugin pages (`:calendar`, `:focus`, `:deps`, `:analytics`)

Every keypress while the overlay is open is forwarded to the plugin's
`handle(KeyPress { key })`.

### Calendar plugin

| Key | Action |
|---|---|
| `h` / `l` | ±1 day |
| `j` / `k` | ±1 week |
| `J` / `K` | ±1 month |
| `t` | jump to today |
| `Esc` | close (host emits Hide) |
