# Example Porting Plan: Remaining Python Rich Examples

This document outlines the features needed to port the remaining Python Rich examples to `rich-rs`.

## Overview Table

| Example | Tier | Primary Feature Missing | Status | Complexity |
|---------|------|------------------------|--------|------------|
| `screen.py` | 2 | `Console::screen()` context manager | Partial | ~100 LOC |
| `exception.py` | 3 | `Console::print_exception()` | Todo | ~150 LOC |
| `recursive_error.py` | 3 | Traceback `max_frames` (already done) | **Ready** | - |
| `suppress.py` | 3 | Traceback `suppress` (already done) | **Ready** | - |
| `cp_progress.py` | 4 | `Progress::open()` file wrapper | Todo | ~200 LOC |
| `file_progress.py` | 4 | `wrap_file()` for Read trait | Todo | ~150 LOC |
| `downloader.py` | 5 | HTTP client (external crate) | External | ~50 LOC |
| `attrs.py` | 6 | `Pretty` (already done) | **Ready** | - |
| `repr.py` | 6 | `#[derive(RichRepr)]` macro | Todo | ~300 LOC |
| `group2.py` | 7 | `Group` struct + `group!` macro | Todo | ~80 LOC |

---

## Tier 2: Console Screen Mode

### Example: `screen.py`

**Python Source:**
```python
from rich.console import Console
from rich.panel import Panel
from rich.align import Align

console = Console()

with console.screen(style="bold white on red") as screen:
    text = Align.center("[blink]Don't Panic![/blink]", vertical="middle")
    screen.update(Panel(text))
    sleep(5)
```

### What We Have

- `Console::enter_alt_screen()` / `leave_alt_screen()` - low-level alt screen control
- `Console::set_alt_screen(bool)` - enable/disable alt screen
- `Console::is_alt_screen()` - check if alt screen is active
- `Screen` renderable (fills terminal, crops excess)

### What's Missing

1. **`ScreenContext` struct** - Context manager equivalent
2. **`Console::screen()` method** - Returns `ScreenContext`
3. **`ScreenContext::update()` method** - Update screen content

### Rust API Signatures

```rust
/// Context for alternate screen mode.
pub struct ScreenContext<'a, W: Write = Stdout> {
    console: &'a mut Console<W>,
    hide_cursor: bool,
    screen: Screen,
    changed: bool,
}

impl<'a, W: Write> ScreenContext<'a, W> {
    /// Update the screen with new content.
    pub fn update(
        &mut self,
        renderables: impl IntoIterator<Item = impl Renderable>,
        style: Option<Style>,
    ) -> io::Result<()>;
}

impl<W: Write> Console<W> {
    /// Enter alternate screen mode with a context guard.
    pub fn screen(
        &mut self,
        hide_cursor: bool,
        style: Option<Style>,
    ) -> ScreenContext<'_, W>;
}
```

### Implementation Steps

1. Create `ScreenContext` struct with lifetime tied to Console
2. Implement `ScreenContext::update()` using existing `Screen` renderable
3. Implement `Drop` for `ScreenContext` to auto-leave alt screen
4. Add `Console::screen()` method

### Estimated Complexity

- **Lines of Code:** ~100
- **Dependencies:** None (uses existing `crossterm`)
- **Risk:** Low

---

## Tier 3: Traceback Integration

### Example: `exception.py`

**Python Source:**
```python
console = Console()

try:
    result = divide_by(number, divisor)
except Exception:
    console.print_exception(extra_lines=8, show_locals=True)
```

### What We Have

- Full `Traceback` struct with `TracebackBuilder`
- `Frame`, `Stack`, `Trace` structs
- `SyntaxErrorInfo` for syntax errors
- `Renderable` implementation for `Traceback`
- `install()` panic hook

### What's Missing

1. **`Console::print_exception()` method** - Capture current panic/error and print

### Rust API Signatures

```rust
impl<W: Write> Console<W> {
    /// Print a traceback for the given error.
    ///
    /// This is the Rust equivalent of `console.print_exception()`.
    /// Since Rust doesn't have exceptions, this captures the current
    /// backtrace (if RUST_BACKTRACE is set) and error chain.
    pub fn print_exception<E: std::error::Error>(
        &mut self,
        error: &E,
        extra_lines: usize,
        show_locals: bool,
        max_frames: Option<usize>,
    ) -> io::Result<()>;

    /// Print a traceback from a Trace object.
    pub fn print_traceback(&mut self, traceback: &Traceback) -> io::Result<()>;
}
```

### Implementation Steps

1. Add `Console::print_traceback()` - simple wrapper around existing render
2. Add `Console::print_exception()` - captures backtrace from `std::backtrace::Backtrace`
3. Parse backtrace to create `Frame` objects (requires parsing backtrace output)

### Estimated Complexity

- **Lines of Code:** ~150
- **Dependencies:** `std::backtrace::Backtrace` (nightly or Rust 1.65+)
- **Risk:** Medium (backtrace parsing varies by platform)

### Examples `recursive_error.py` and `suppress.py`

**Status: Ready**

Both examples use features already implemented:
- `max_frames` parameter in `TracebackBuilder::max_frames()`
- `suppress` parameter in `TracebackBuilder::suppress()`

---

## Tier 4: File I/O Progress

### Example: `cp_progress.py`

**Python Source:**
```python
with Progress() as progress:
    desc = os.path.basename(sys.argv[1])
    with progress.open(sys.argv[1], "rb", description=desc) as src:
        with open(sys.argv[2], "wb") as dst:
            shutil.copyfileobj(src, dst)
```

### Example: `file_progress.py`

**Python Source:**
```python
from rich.progress import wrap_file

with wrap_file(response, size) as file:
    for line in file:
        print(line.decode("utf-8"), end="")
```

### What We Have

- Full `Progress` struct with task management
- `ProgressTask`, `TaskID`, all column types
- `Progress::track()` for iterators

### What's Missing

1. **`ProgressReader<R: Read>` struct** - Wrapper that updates progress on read
2. **`Progress::wrap_file()` method** - Wrap a `Read` with progress tracking
3. **`Progress::open()` method** - Open file with progress tracking

### Rust API Signatures

```rust
/// A reader that tracks progress as bytes are read.
pub struct ProgressReader<R: Read> {
    inner: R,
    progress: Arc<Progress>,
    task_id: TaskID,
    close_handle: bool,
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.progress.advance(self.task_id, n as f64);
        Ok(n)
    }
}

impl Progress {
    /// Wrap a reader to track progress while reading.
    pub fn wrap_file<R: Read>(
        &self,
        reader: R,
        total: Option<u64>,
        task_id: Option<TaskID>,
        description: &str,
    ) -> ProgressReader<R>;

    /// Open a file with progress tracking.
    pub fn open<P: AsRef<Path>>(
        &self,
        path: P,
        description: &str,
    ) -> io::Result<ProgressReader<File>>;
}
```

### Standalone Functions

```rust
/// Wrap a reader to track progress (standalone version).
pub fn wrap_file<R: Read>(
    reader: R,
    total: u64,
    description: &str,
    // ... style options
) -> io::Result<impl Read>;
```

### Implementation Steps

1. Create `ProgressReader<R: Read>` struct
2. Implement `Read` trait for `ProgressReader`
3. Add `Progress::wrap_file()` method
4. Add `Progress::open()` method (uses `std::fs::File`)
5. Add standalone `wrap_file()` function

### Estimated Complexity

- **Lines of Code:** ~200
- **Dependencies:** None
- **Risk:** Low

---

## Tier 5: Network Progress

### Example: `downloader.py`

**Python Source:**
```python
from urllib.request import urlopen
from rich.progress import Progress, DownloadColumn, TransferSpeedColumn

progress = Progress(
    TextColumn("[bold blue]{task.fields[filename]}", justify="right"),
    BarColumn(bar_width=None),
    "[progress.percentage]{task.percentage:>3.1f}%",
    DownloadColumn(),
    TransferSpeedColumn(),
    TimeRemainingColumn(),
)

with progress:
    response = urlopen(url)
    progress.update(task_id, total=int(response.info()["Content-length"]))
    for data in iter(partial(response.read, 32768), b""):
        dest_file.write(data)
        progress.update(task_id, advance=len(data))
```

### What We Have

- All progress columns (`DownloadColumn`, `TransferSpeedColumn`, etc.)
- Full `Progress` struct

### What's Missing

This example requires an HTTP client, which is external to Rich.

### Rust API Signatures

No new Rich APIs needed. Example would use:
- `reqwest` or `ureq` for HTTP
- Existing `Progress::update()` with `advance`

### Implementation Steps

1. Create example using `reqwest` or `ureq` crate
2. Demonstrate progress tracking with HTTP downloads

### Estimated Complexity

- **Lines of Code:** ~50 (example only)
- **Dependencies:** `reqwest` or `ureq` (dev-dependency for example)
- **Risk:** Low (external crate handles HTTP)

---

## Tier 6: Pretty Repr

### Example: `attrs.py`

**Python Source:**
```python
from rich.pretty import Pretty

model = Model(name="Alien#1", triangles=[...])
console.print(Pretty(model))
```

**Status: Ready**

The `Pretty` struct is fully implemented. This example works today with Rust's `Debug` trait:

```rust
use rich_rs::pretty::Pretty;

let model = Model { name: "Alien#1", triangles: vec![...] };
console.print(&Pretty::new(&model), ...);
```

### Example: `repr.py`

**Python Source:**
```python
import rich.repr

@rich.repr.auto
class Bird:
    def __init__(self, name, eats=None, fly=True, extinct=False):
        self.name = name
        self.eats = list(eats) if eats else []
        self.fly = fly
        self.extinct = extinct
```

### What We Have

- `Pretty` struct for Debug formatting
- `repr_highlighter()` for syntax highlighting

### What's Missing

1. **`#[derive(RichRepr)]` proc macro** - Generate pretty repr automatically
2. **`rich_repr` attribute macro** - Fine-grained control over repr output

### Rust API Signatures

```rust
// Usage:
#[derive(RichRepr)]
struct Bird {
    name: String,
    #[rich_repr(default)]
    eats: Vec<String>,
    #[rich_repr(skip_if = "true")]
    fly: bool,
    #[rich_repr(skip_if = "false")]
    extinct: bool,
}

// Generated impl:
impl RichRepr for Bird {
    fn rich_repr(&self) -> impl Iterator<Item = RichReprField> {
        // ...
    }
}

// Integration with Pretty:
impl Renderable for RichReprWrapper<T: RichRepr> {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments;
}
```

### Implementation Steps

1. Create `rich_repr` proc-macro crate
2. Implement `#[derive(RichRepr)]` macro
3. Add `#[rich_repr(...)]` field attributes
4. Create `RichReprWrapper` renderable

### Estimated Complexity

- **Lines of Code:** ~300 (proc-macro crate)
- **Dependencies:** `syn`, `quote`, `proc-macro2`
- **Risk:** Medium (proc macros are complex)

---

## Tier 7: Render Groups

### Example: `group2.py`

**Python Source:**
```python
from rich.console import group
from rich.panel import Panel

@group()
def get_panels():
    yield Panel("Hello", style="on blue")
    yield Panel("World", style="on red")

print(Panel(get_panels()))
```

### What We Have

- All panel/renderable infrastructure

### What's Missing

1. **`Group` struct** - Collects multiple renderables
2. **`group!` macro** - Convenience for creating groups from iterators

### Rust API Signatures

```rust
/// A group of renderables rendered sequentially.
pub struct Group {
    renderables: Vec<Box<dyn Renderable + Send + Sync>>,
    fit: bool,
}

impl Group {
    /// Create a new group from renderables.
    pub fn new<I, R>(renderables: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: Renderable + Send + Sync + 'static;

    /// Create a group that fills available width.
    pub fn with_fit(self, fit: bool) -> Self;
}

impl Renderable for Group {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments;
    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement;
}

/// Macro to create a group from yielded renderables.
///
/// ```rust
/// let panels = group! {
///     yield Panel::new("Hello").with_style("on blue");
///     yield Panel::new("World").with_style("on red");
/// };
/// ```
macro_rules! group {
    // ...
}
```

### Implementation Steps

1. Create `Group` struct
2. Implement `Renderable` for `Group`
3. Implement `Measurable` for `Group` (combine measurements)
4. Create `group!` macro (optional, for ergonomics)

### Estimated Complexity

- **Lines of Code:** ~80
- **Dependencies:** None
- **Risk:** Low

---

## Suggested Implementation Order

Based on dependencies and usefulness:

1. **Tier 7: Group** (~80 LOC)
   - Simple, no dependencies
   - Enables cleaner renderable composition
   - Unlocks Screen improvements

2. **Tier 2: ScreenContext** (~100 LOC)
   - Depends on Group
   - Completes TUI foundation
   - Enables `screen.py` example

3. **Tier 4: File I/O Progress** (~200 LOC)
   - No new dependencies
   - Highly useful for CLI apps
   - Enables `cp_progress.py`, `file_progress.py` examples

4. **Tier 3: print_exception** (~150 LOC)
   - May need nightly for `std::backtrace`
   - Enables `exception.py` example

5. **Tier 6: RichRepr Macro** (~300 LOC)
   - Requires new proc-macro crate
   - Nice-to-have, not blocking other features
   - Enables `repr.py` example

6. **Tier 5: Network Example** (~50 LOC)
   - Just an example using external HTTP crate
   - Can be added anytime

---

## Summary

| Priority | Feature | Unlocks Examples | LOC | Risk |
|----------|---------|------------------|-----|------|
| 1 | `Group` struct | `group2.py`, Screen improvements | ~80 | Low |
| 2 | `ScreenContext` | `screen.py` | ~100 | Low |
| 3 | `Progress::open()`/`wrap_file()` | `cp_progress.py`, `file_progress.py` | ~200 | Low |
| 4 | `Console::print_exception()` | `exception.py` | ~150 | Medium |
| 5 | `#[derive(RichRepr)]` | `repr.py` | ~300 | Medium |
| 6 | HTTP example | `downloader.py` | ~50 | Low |

**Total estimated new code:** ~880 lines

**Already working examples:**
- `recursive_error.py` - uses `max_frames` (implemented)
- `suppress.py` - uses `suppress` (implemented)
- `attrs.py` - uses `Pretty` (implemented)
