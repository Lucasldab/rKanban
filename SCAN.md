# rkanban — Codebase Scan

## (a) Modules and Responsibilities

| Module | File | Responsibility |
|--------|------|----------------|
| **main** | `src/main.rs` | Entry point. Configures crossterm raw-mode terminal, loads config + app state, runs the event loop, dispatches keystrokes to handlers (`handle_normal`, `handle_input`, `handle_column_input`, `handle_delete_confirm`, `handle_text_input`), and calls `app.save()` on clean exit. |
| **app** | `src/app/mod.rs` | Core data model and mutation logic. Defines `Card`, `Column`, `InputMode`, `PopupField`, `App`, and the serialization-only `BoardSnapshot`. Owns `App::load()` / `App::save()`, all board-mutation methods (add/delete/move card, add/rename/delete/reorder column), and UTF-8-safe cursor helpers. |
| **ui** | `src/ui/mod.rs` | Pure rendering layer. All ratatui `draw_*` functions read `App` immutably and paint columns, popups (add/edit card, add/rename/delete column, card detail view), status bar, and help overlay. Computes terminal cursor position for text fields. |
| **config** | `src/config.rs` | Config file I/O. Defines `Key`, `KeyMap`, `ColorScheme`, `Config`. Parses `~/.config/rkanban/config.conf` (INI format) on startup; writes a default config if absent. Provides `Key::matches()` used throughout `main.rs`. |

---

## (b) Data-Loss Risks

### Risk 1 — Non-atomic file write (HIGH)

**Location:** `src/app/mod.rs:149`

```rust
if fs::write(&self.save_path, json).is_err() {
```

`fs::write` truncates the destination file to zero bytes and then writes. If the process is killed (SIGKILL, OOM, power loss) between truncation and completion, the board file is left empty or partially written. The next `App::load()` will silently parse an empty string, fail at:

```rust
// src/app/mod.rs:128
if let Ok(snap) = serde_json::from_str::<BoardSnapshot>(&data) {
```

…and return a fresh empty board, **discarding all user data without warning**. No temp-file-then-rename (write-rename atomicity) is used anywhere.

---

### Risk 2 — Silent discard on parse failure (HIGH)

**Location:** `src/app/mod.rs:126–143`

```rust
let data = fs::read_to_string(path).unwrap_or_default();
if let Ok(snap) = serde_json::from_str::<BoardSnapshot>(&data) {
    // ... restore board
}
// else: silently create a blank board
```

Any parse failure — from a partial write (Risk 1), a hand-edited file, or a future schema change — silently drops all data and presents the user with a blank board. No error message, no backup, no warning is generated.

---

### Risk 3 — No schema version in `BoardSnapshot` (MEDIUM)

**Location:** `src/app/mod.rs:302–306`

```rust
struct BoardSnapshot {
    columns: Vec<Column>,
    selected_column: usize,
}
```

No `version` field. If a future release adds a required field or renames one, old JSON files fail to deserialize (serde returns `Err`), which triggers Risk 2's silent discard. There is no migration path and no way to detect file-format mismatch at load time.

---

### Risk 4 — UI cursor serialized into the board file (LOW)

**Location:** `src/app/mod.rs:13–18`

```rust
pub struct Column {
    pub name: String,
    pub cards: Vec<Card>,
    pub selected: usize,   // UI cursor — no #[serde(skip)]
}
```

`selected` is a runtime cursor, not persistent board data. It is serialized into every save. If an external tool modifies the file and sets `selected` to a value beyond the current card list, `clamp_selected()` silently repairs it on load — but if cards were also lost (e.g., Risk 1 partial write) the cursor clamping masks the data loss.

---

### Risk 5 — Eager save on every mutation (LOW, amplifies Risks 1–2)

`save()` is called inside every mutation method (`add_card`, `delete_card`, `move_card_*`, `add_column`, etc.) and again inline at `src/main.rs:172` (EditingCard confirm). Each call reopens and truncates the file, expanding the crash window. A debounced or session-end-only save would reduce exposure.

---

## (c) Recommended Test: Save/Load Roundtrip with Corrupt-File Regression

This test covers both the atomic-write gap (Risk 1) and the silent-discard regression (Risk 2). It should be added to a new `tests/persistence.rs` integration test file.

```rust
// tests/persistence.rs
//
// Verifies that:
//   1. A full board survives a save → load roundtrip unchanged.
//   2. A corrupt/empty file does NOT silently discard data once
//      atomic-write (write-to-tmp then rename) is implemented.

use std::fs;
use tempfile::NamedTempFile;

// Inline the types we need (or expose them via `pub` + `#[cfg(test)]`).
// Adjust imports once the crate exposes them.
#[test]
fn roundtrip_preserves_all_columns_and_cards() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    // Build a board with two columns and one card each.
    let mut app = rkanban::app::App::load(&path); // loads blank (file is empty)
    app.add_card("Buy milk".into(), "2 litres".into(), vec!["errand".into()]);
    app.add_column("Done".into());
    app.move_card_right(); // move "Buy milk" to Done

    // Reload from the same file.
    let loaded = rkanban::app::App::load(&path);
    assert_eq!(loaded.columns.len(), 2);
    assert_eq!(loaded.columns[1].cards.len(), 1);
    assert_eq!(loaded.columns[1].cards[0].title, "Buy milk");
    assert_eq!(loaded.columns[1].cards[0].tags, vec!["errand"]);
}

#[test]
fn load_from_partial_write_does_not_silently_discard() {
    // Write a valid board, then overwrite the file with truncated JSON
    // (simulating a crash mid-write). After atomic-write is implemented,
    // the original file must remain intact and load must succeed.
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let mut app = rkanban::app::App::load(&path);
    app.add_card("Important task".into(), String::new(), vec![]);

    // Simulate partial write: corrupt the saved file.
    fs::write(&path, b"{\"columns\":[{\"name\":\"Todo\",\"cards\":[{\"tit").unwrap();

    let recovered = rkanban::app::App::load(&path);
    // With current code this assertion FAILS (data is silently discarded).
    // Once atomic-write is implemented, the original file survives and this passes.
    assert_eq!(
        recovered.columns[0].cards[0].title, "Important task",
        "partial write must not destroy the last good save"
    );
}
```

**Why this test:** it documents the current failure mode as a failing test, giving a clear, mechanically verifiable target for the atomic-write fix (`write to <path>.tmp` + `fs::rename` + `sync_data`).
