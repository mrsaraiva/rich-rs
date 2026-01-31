# Lost Parity Tests

This document lists parity tests that were lost due to git object corruption on the NFS mount (January 2026). The tests can potentially be recovered from conversation history in `~/.claude/projects/`.

## Overview

The parity test framework compared Python Rich output with Rust rich-rs output to verify behavior matches. Each phase had:
- Python scripts in `tests/parity/phaseN/python/` that used the `rich` library
- Rust binary crate in `tests/parity/phaseN/rust/` that used `rich-rs`
- A runner script `tests/parity/run_parity.sh` to execute both and diff output

## Lost Files by Phase

### Phase 1: Foundation

**Python tests:**
- `tests/parity/phase1/python/test_cells.py` - Cell width calculations
- `tests/parity/phase1/python/test_color.py` - Color parsing, ANSI codes, downgrade
- `tests/parity/phase1/python/test_measure.py` - Measurement operations
- `tests/parity/phase1/python/test_segment.py` - Segment operations (split, divide, simplify)
- `tests/parity/phase1/python/test_style.py` - Style parsing, rendering, combination

**Rust parity crate:**
- `tests/parity/phase1/rust/Cargo.toml`
- `tests/parity/phase1/rust/src/main.rs`
- `tests/parity/phase1/rust/src/cells.rs`
- `tests/parity/phase1/rust/src/color.rs`
- `tests/parity/phase1/rust/src/measure.rs`
- `tests/parity/phase1/rust/src/segment.rs`
- `tests/parity/phase1/rust/src/style.rs`

### Phase 2: Text & Console

**Python tests:**
- `tests/parity/phase2/python/test_console.py` - Console rendering, capture
- `tests/parity/phase2/python/test_markup.py` - BBCode markup parsing
- `tests/parity/phase2/python/test_text.py` - Text with spans, stylize
- `tests/parity/phase2/python/test_text_wrap.py` - Text wrapping with justify
- `tests/parity/phase2/python/test_wrap.py` - Low-level wrap utilities

**Rust parity crate:**
- `tests/parity/phase2/rust/Cargo.toml`
- `tests/parity/phase2/rust/src/main.rs`
- `tests/parity/phase2/rust/src/console.rs`
- `tests/parity/phase2/rust/src/markup.rs`
- `tests/parity/phase2/rust/src/text.rs`
- `tests/parity/phase2/rust/src/text_wrap.rs`
- `tests/parity/phase2/rust/src/wrap.rs`

### Phase 3: Box Drawing & Simple Renderables

**Python tests:**
- `tests/parity/phase3/python/test_align.py` - Align renderable
- `tests/parity/phase3/python/test_box.py` - Box characters, table borders
- `tests/parity/phase3/python/test_padding.py` - Padding renderable
- `tests/parity/phase3/python/test_rule.py` - Horizontal rule

**Rust parity crate:**
- `tests/parity/phase3/rust/Cargo.toml`
- `tests/parity/phase3/rust/src/main.rs`
- `tests/parity/phase3/rust/src/align.rs`
- `tests/parity/phase3/rust/src/box.rs`
- `tests/parity/phase3/rust/src/padding.rs`
- `tests/parity/phase3/rust/src/rule.rs`

### Phase 4: Complex Renderables

**Python tests:**
- `tests/parity/phase4/python/test_columns.py` - Multi-column layout
- `tests/parity/phase4/python/test_panel.py` - Panel with borders
- `tests/parity/phase4/python/test_table.py` - Table rendering
- `tests/parity/phase4/python/test_tree.py` - Tree structure

**Rust parity crate:**
- `tests/parity/phase4/rust/Cargo.toml`
- `tests/parity/phase4/rust/src/main.rs`
- `tests/parity/phase4/rust/src/columns.rs`
- `tests/parity/phase4/rust/src/panel.rs`
- `tests/parity/phase4/rust/src/table.rs`
- `tests/parity/phase4/rust/src/tree.rs`

### Phase 5: Advanced Features

**Python tests:**
- `tests/parity/phase5/python/test_pretty.py` - Pretty printing

**Rust parity crate:**
- `tests/parity/phase5/rust/Cargo.toml`
- `tests/parity/phase5/rust/src/main.rs`
- `tests/parity/phase5/rust/src/pretty.rs`
- `tests/parity/phase5/rust/.gitignore`

### Runner Script

- `tests/parity/run_parity.sh` - Bash script to run Python and Rust, compare output
- `tests/parity/README.md` - Documentation for the parity test framework

## Recovery Sources

Conversation history that may contain test implementations:
- `~/.claude/projects/-mnt-shares-Marcos-dev-mark-Proj-Libs-rich-rs/` - JSONL transcripts

### Complete Git Reflog (captured before reset)

```
e939b3a commit: test: add Phase 5 Pretty parity tests
9032145 commit: feat: implement Phase 5 advanced features (Syntax, Pretty, Markdown)
b16062f commit: docs: add Markdown and Demo phases to roadmap
195841b commit: feat: implement Columns (Phase 4.4 complete)
960787e commit: feat: implement Phase 4 complex renderables (Panel, Tree, Table)
797a3ea commit: fix: resolve Phase 4 blocker - pass console state through ConsoleOptions
f227173 commit: feat: add Rule and Padding modules (Phase 3.2, 3.3 complete)
22d0c01 commit: feat: implement full Box module (Phase 3.1 complete)
904b848 commit: refactor: strengthen Console parity tests per Codex review
adc96ab commit: feat: add Console parity tests (Phase 2.5 complete)
3d26bcc commit: fix: address remaining Codex review issues
918379d commit: feat: implement Console, Theme, and Text wrap modules (Phase 2.3)
3a44bce commit: feat: implement full Text module with parity tests (Phase 2.2)
3bfffe2 commit: feat: implement full Markup Parser module (Phase 2.1)
4236541 commit: feat: implement Emoji and Highlighter modules (Phase 2.0)
abebc07 commit: fix: address Phase 1 Codex review findings (v0.1.0)
ecabe9b commit: feat: add Python/Rust parity testing framework (Phase 1.6)
```

Note: Some intermediate commits (amends, etc.) omitted for clarity. Full history in JSONL transcripts.

### Full Git History Recovery

Since all development was done interactively through Claude Code, the **entire git history** could potentially be reconstructed from conversation transcripts. Each commit message, file content, and change was captured in the JSONL logs.

Available transcripts (27 files, largest ones contain bulk of development):
```
4b4d8640-5e1c-40da-a0d3-6bea4d2e026a.jsonl  252MB
3d93ac42-b152-4e61-9601-585dd3037875.jsonl  111MB
501dd721-30fa-42a0-a3b8-d37d05ecdfd4.jsonl   53MB
```

To recover:
1. Parse JSONL transcripts chronologically
2. Search for `git commit` tool calls and their surrounding context
3. Extract the file writes/edits that preceded each commit
4. Replay commits in order with original messages and timestamps

This would restore not just the parity tests but the full development history with proper commit messages and authorship.

## Reimplementation Priority

Since unit tests in source files provide adequate coverage, parity tests are lower priority. If reimplementing:

1. **High value**: Phase 1 (foundation) - validates core building blocks
2. **Medium value**: Phase 2 (text/console) - validates rendering pipeline
3. **Lower value**: Phases 3-5 - built on validated foundations

## Test Pattern

Each parity test followed this pattern:

**Python (`test_module.py`):**
```python
from rich.module import SomeClass
from rich.console import Console

console = Console(force_terminal=True, width=80)

# Test case
obj = SomeClass(...)
console.print(obj)
```

**Rust (`module.rs`):**
```rust
use rich_rs::module::SomeStruct;
use rich_rs::Console;

fn main() {
    let mut console = Console::new().with_width(80);

    // Test case
    let obj = SomeStruct::new(...);
    console.print(&obj);
}
```

**Runner compared stdout from both and reported differences.**
