//! Progress: task tracking with live-updating progress bars.
//!
//! Port of Python Rich's `progress.py` (subset).

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Stdout;
use std::io::{self, BufRead, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::console::ConsoleOptions;
use crate::console::OverflowMethod;
use crate::filesize;
use crate::live::{Live, LiveOptions};
use crate::progress_bar::ProgressBar;
use crate::spinner::Spinner;
use crate::style::Style;
use crate::table::{Column, Row, Table};
use crate::text::Text;
use crate::{Console, JustifyMethod, Renderable, Segments};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskID(pub usize);

#[derive(Debug, Clone)]
struct ProgressSample {
    timestamp: f64,
    completed: f64,
}

#[derive(Debug, Clone)]
pub struct ProgressTask {
    pub id: TaskID,
    pub description: String,
    pub total: Option<f64>,
    pub completed: f64,
    pub visible: bool,
    pub fields: HashMap<String, String>,

    pub finished_time: Option<f64>,
    pub finished_speed: Option<f64>,

    start_time: Option<f64>,
    stop_time: Option<f64>,
    progress: VecDeque<ProgressSample>,
}

impl ProgressTask {
    fn started(&self) -> bool {
        self.start_time.is_some()
    }

    fn finished(&self) -> bool {
        self.finished_time.is_some()
    }

    fn remaining(&self) -> Option<f64> {
        self.total.map(|t| t - self.completed)
    }

    fn elapsed(&self, now: f64) -> Option<f64> {
        let start = self.start_time?;
        if let Some(stop) = self.stop_time {
            return Some(stop - start);
        }
        Some(now - start)
    }

    fn percentage(&self) -> f64 {
        let Some(total) = self.total else { return 0.0 };
        if total <= 0.0 {
            return 0.0;
        }
        ((self.completed / total) * 100.0).clamp(0.0, 100.0)
    }

    fn speed(&self) -> Option<f64> {
        if !self.started() {
            return None;
        }
        let first = self.progress.front()?;
        let last = self.progress.back()?;
        let total_time = last.timestamp - first.timestamp;
        if total_time == 0.0 {
            return None;
        }
        // Skip the first sample (which is usually the initial state) like Rich does.
        let total_completed: f64 = self.progress.iter().skip(1).map(|s| s.completed).sum();
        Some(total_completed / total_time)
    }

    fn time_remaining(&self) -> Option<f64> {
        if self.finished() {
            return Some(0.0);
        }
        let speed = self.speed()?;
        if speed <= 0.0 {
            return None;
        }
        let remaining = self.remaining()?;
        if remaining <= 0.0 {
            return Some(0.0);
        }
        Some((remaining / speed).ceil())
    }
}

pub trait ProgressColumn: Send + Sync {
    fn table_column(&self) -> Column;
    fn render(
        &self,
        task: &ProgressTask,
        now: f64,
        options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync>;
    fn max_refresh(&self) -> Option<Duration> {
        None
    }
}

#[derive(Debug)]
pub struct SpinnerColumn {
    spinner: Spinner,
    finished_text: Text,
    start_time: Mutex<Option<f64>>,
    style_name: String,
}

impl SpinnerColumn {
    pub fn new() -> Self {
        Self::with_spinner("dots")
    }

    pub fn with_spinner(name: &str) -> Self {
        let spinner = Spinner::new(name).unwrap_or_else(|_| Spinner::new("dots").unwrap());
        Self {
            spinner,
            finished_text: Text::plain(" "),
            start_time: Mutex::new(None),
            style_name: "progress.spinner".to_string(),
        }
    }

    pub fn with_style_name(mut self, style: &str) -> Self {
        self.style_name = style.to_string();
        self
    }
}

impl ProgressColumn for SpinnerColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn render(
        &self,
        task: &ProgressTask,
        now: f64,
        options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        if task.finished() {
            return Box::new(self.finished_text.clone());
        }
        let mut start = self
            .start_time
            .lock()
            .expect("spinner start mutex poisoned");
        let start_time = *start.get_or_insert(now);
        let style = options.get_style(&self.style_name);
        Box::new(self.spinner.render_at(now, Some(start_time), style))
    }
}

#[derive(Debug, Clone)]
pub struct TextColumn {
    text_format: String,
    style_name: String,
    justify: JustifyMethod,
    markup: bool,
}

impl TextColumn {
    pub fn new(text_format: &str) -> Self {
        Self {
            text_format: text_format.to_string(),
            style_name: "none".to_string(),
            justify: JustifyMethod::Left,
            markup: true,
        }
    }

    pub fn with_style_name(mut self, style: &str) -> Self {
        self.style_name = style.to_string();
        self
    }

    pub fn with_justify(mut self, justify: JustifyMethod) -> Self {
        self.justify = justify;
        self
    }

    pub fn with_markup(mut self, markup: bool) -> Self {
        self.markup = markup;
        self
    }
}

impl ProgressColumn for TextColumn {
    fn table_column(&self) -> Column {
        // Match Rich: TextColumn uses a no-wrap column by default.
        Column::new().no_wrap(true).justify(self.justify)
    }

    fn render(
        &self,
        task: &ProgressTask,
        now: f64,
        options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let formatted = format_task_template(&self.text_format, task, now);
        let mut text = if self.markup {
            Text::from_markup(&formatted, true).unwrap_or_else(|_| Text::plain(&formatted))
        } else {
            Text::plain(&formatted)
        };
        if self.style_name != "none" {
            if let Some(style) = options.get_style(&self.style_name) {
                text.stylize_before(style, 0, None);
            }
        }
        Box::new(text)
    }
}

#[derive(Debug, Clone)]
pub struct BarColumn {
    bar_width: Option<usize>,
    style: String,
    complete_style: String,
    finished_style: String,
    pulse_style: String,
}

impl BarColumn {
    pub fn new() -> Self {
        Self {
            // Match Rich: default bar width is 40 cells.
            bar_width: Some(40),
            style: "bar.back".to_string(),
            complete_style: "bar.complete".to_string(),
            finished_style: "bar.finished".to_string(),
            pulse_style: "bar.pulse".to_string(),
        }
    }

    pub fn with_bar_width(mut self, width: Option<usize>) -> Self {
        self.bar_width = width;
        self
    }
}

impl ProgressColumn for BarColumn {
    fn table_column(&self) -> Column {
        Column::new()
    }

    fn render(
        &self,
        task: &ProgressTask,
        now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let mut bar = ProgressBar::new();
        bar.total = task.total.map(|t| t.max(0.0));
        bar.completed = task.completed.max(0.0);
        bar.width = self.bar_width.map(|w| w.max(1));
        bar.pulse = !task.started();
        bar.animation_time = Some(now);
        bar.style = self.style.clone();
        bar.complete_style = self.complete_style.clone();
        bar.finished_style = self.finished_style.clone();
        bar.pulse_style = self.pulse_style.clone();
        Box::new(bar)
    }
}

#[derive(Debug, Clone)]
pub struct TaskProgressColumn {
    show_speed: bool,
}

impl TaskProgressColumn {
    pub fn new(show_speed: bool) -> Self {
        Self { show_speed }
    }

    fn render_speed(speed: Option<f64>) -> Text {
        let Some(speed) = speed else {
            return Text::plain("");
        };
        let speed = speed.max(0.0);
        let (unit, suffix) = filesize::pick_unit_and_suffix(
            speed as u64,
            &["", "×10³", "×10⁶", "×10⁹", "×10¹²"],
            1000,
        );
        let data_speed = speed / unit as f64;
        Text::from_markup(
            &format!("[progress.percentage]{data_speed:.1}{suffix} it/s"),
            true,
        )
        .unwrap_or_else(|_| Text::plain(format!("{data_speed:.1}{suffix} it/s")))
    }
}

impl ProgressColumn for TaskProgressColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn render(
        &self,
        task: &ProgressTask,
        _now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        // Match Rich: if total is unknown, show an empty cell unless show_speed is enabled.
        if task.total.is_none() {
            if self.show_speed {
                return Box::new(Self::render_speed(
                    task.finished_speed.or_else(|| task.speed()),
                ));
            }
            return Box::new(Text::plain(""));
        }
        let percent = task.percentage();
        Box::new(
            Text::from_markup(&format!("[progress.percentage]{percent:>3.0}%"), true)
                .unwrap_or_else(|_| Text::plain(format!("{percent:>3.0}%"))),
        )
    }
}

#[derive(Debug, Clone)]
pub struct TimeRemainingColumn {
    pub compact: bool,
    pub elapsed_when_finished: bool,
}

impl TimeRemainingColumn {
    pub fn new(elapsed_when_finished: bool) -> Self {
        Self {
            compact: false,
            elapsed_when_finished,
        }
    }

    pub fn with_compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

impl ProgressColumn for TimeRemainingColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn max_refresh(&self) -> Option<Duration> {
        // Match Rich: only refresh twice a second to prevent jitter.
        Some(Duration::from_secs_f64(0.5))
    }

    fn render(
        &self,
        task: &ProgressTask,
        _now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let (task_time, style) = if task.finished() && self.elapsed_when_finished {
            (task.finished_time, "progress.elapsed")
        } else {
            (task.time_remaining(), "progress.remaining")
        };

        if task.total.is_none() {
            return Box::new(Text::plain(""));
        }

        let placeholder = if self.compact { "--:--" } else { "-:--:--" };
        let Some(task_time) = task_time else {
            return Box::new(
                Text::from_markup(&format!("[{style}]{placeholder}"), true)
                    .unwrap_or_else(|_| Text::plain(placeholder)),
            );
        };

        let secs = task_time.max(0.0) as u64;
        let minutes_total = secs / 60;
        let seconds = secs % 60;
        let hours = minutes_total / 60;
        let minutes = minutes_total % 60;

        let formatted = if self.compact && hours == 0 {
            format!("{minutes:02}:{seconds:02}")
        } else {
            format!("{hours}:{minutes:02}:{seconds:02}")
        };

        Box::new(
            Text::from_markup(&format!("[{style}]{formatted}"), true)
                .unwrap_or_else(|_| Text::plain(formatted)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct TimeElapsedColumn;

impl TimeElapsedColumn {
    pub fn new() -> Self {
        Self
    }
}

impl ProgressColumn for TimeElapsedColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn render(
        &self,
        task: &ProgressTask,
        now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let elapsed = if task.finished() {
            task.finished_time
        } else {
            task.elapsed(now)
        };
        let Some(elapsed) = elapsed else {
            return Box::new(
                Text::from_markup("[progress.elapsed]-:--:--", true)
                    .unwrap_or_else(|_| Text::plain("-:--:--")),
            );
        };
        let secs = elapsed.max(0.0) as u64;
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;
        Box::new(
            Text::from_markup(
                &format!("[progress.elapsed]{hours}:{minutes:02}:{seconds:02}"),
                true,
            )
            .unwrap_or_else(|_| Text::plain(format!("{hours}:{minutes:02}:{seconds:02}"))),
        )
    }
}

#[derive(Debug, Clone)]
pub struct FileSizeColumn;

impl FileSizeColumn {
    pub fn new() -> Self {
        Self
    }
}

impl ProgressColumn for FileSizeColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn render(
        &self,
        task: &ProgressTask,
        _now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let data_size = filesize::decimal(task.completed.max(0.0) as u64);
        Box::new(
            Text::from_markup(&format!("[progress.filesize]{data_size}"), true)
                .unwrap_or_else(|_| Text::plain(data_size)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct TotalFileSizeColumn;

impl TotalFileSizeColumn {
    pub fn new() -> Self {
        Self
    }
}

impl ProgressColumn for TotalFileSizeColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn render(
        &self,
        task: &ProgressTask,
        _now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let data_size = task
            .total
            .map(|t| filesize::decimal(t.max(0.0) as u64))
            .unwrap_or_default();
        Box::new(
            Text::from_markup(&format!("[progress.filesize.total]{data_size}"), true)
                .unwrap_or_else(|_| Text::plain(data_size)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct MofNCompleteColumn {
    separator: String,
}

impl MofNCompleteColumn {
    pub fn new() -> Self {
        Self {
            separator: "/".to_string(),
        }
    }

    pub fn with_separator(mut self, separator: &str) -> Self {
        self.separator = separator.to_string();
        self
    }
}

impl ProgressColumn for MofNCompleteColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn render(
        &self,
        task: &ProgressTask,
        _now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let completed = task.completed.max(0.0) as u64;
        let total = task.total.map(|t| t.max(0.0) as u64);
        let total_str = total
            .map(|t| t.to_string())
            .unwrap_or_else(|| "?".to_string());
        let total_width = total_str.len();
        let completed_str = format!("{completed:width$}", width = total_width);
        let text = format!("{completed_str}{}{}", self.separator, total_str);
        Box::new(
            Text::from_markup(&format!("[progress.download]{text}"), true)
                .unwrap_or_else(|_| Text::plain(text)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct DownloadColumn {
    pub binary_units: bool,
}

impl DownloadColumn {
    pub fn new() -> Self {
        Self {
            binary_units: false,
        }
    }

    pub fn with_binary_units(mut self, binary_units: bool) -> Self {
        self.binary_units = binary_units;
        self
    }
}

impl ProgressColumn for DownloadColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn render(
        &self,
        task: &ProgressTask,
        _now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let completed = task.completed.max(0.0) as u64;
        let calc_base = task.total.map(|t| t.max(0.0) as u64).unwrap_or(completed);
        let (unit, suffix) = if self.binary_units {
            filesize::pick_unit_and_suffix(
                calc_base,
                &[
                    "bytes", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB",
                ],
                1024,
            )
        } else {
            filesize::pick_unit_and_suffix(
                calc_base,
                &["bytes", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"],
                1000,
            )
        };
        let precision = if unit == 1 { 0 } else { 1 };
        let completed_ratio = completed as f64 / unit as f64;
        let completed_str = if precision == 0 {
            format!("{completed_ratio:.0}")
        } else {
            format!("{completed_ratio:.1}")
        };

        let total_str = if let Some(total) = task.total {
            let total = total.max(0.0) as u64;
            let total_ratio = total as f64 / unit as f64;
            if precision == 0 {
                format!("{total_ratio:.0}")
            } else {
                format!("{total_ratio:.1}")
            }
        } else {
            "?".to_string()
        };

        let download_status = format!("{completed_str}/{total_str} {suffix}");
        Box::new(
            Text::from_markup(&format!("[progress.download]{download_status}"), true)
                .unwrap_or_else(|_| Text::plain(download_status)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct TransferSpeedColumn;

impl TransferSpeedColumn {
    pub fn new() -> Self {
        Self
    }
}

impl ProgressColumn for TransferSpeedColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(true)
    }

    fn render(
        &self,
        task: &ProgressTask,
        _now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        let speed = task.finished_speed.or_else(|| task.speed());
        let Some(speed) = speed else {
            return Box::new(
                Text::from_markup("[progress.data.speed]?", true)
                    .unwrap_or_else(|_| Text::plain("?")),
            );
        };
        let data_speed = filesize::decimal(speed.max(0.0) as u64);
        Box::new(
            Text::from_markup(&format!("[progress.data.speed]{data_speed}/s"), true)
                .unwrap_or_else(|_| Text::plain(format!("{data_speed}/s"))),
        )
    }
}

/// A column that renders an arbitrary `Renderable` from the task's fields.
///
/// This is the most flexible column type, allowing custom rendering logic
/// via a closure that extracts a renderable from the task state.
pub struct RenderableColumn {
    render_fn: Box<dyn Fn(&ProgressTask) -> Box<dyn Renderable + Send + Sync> + Send + Sync>,
    no_wrap: bool,
    justify: JustifyMethod,
}

impl RenderableColumn {
    /// Create a new RenderableColumn with a render function.
    ///
    /// The function receives a `ProgressTask` reference and should return
    /// a boxed `Renderable` to display in the column.
    pub fn new(
        f: impl Fn(&ProgressTask) -> Box<dyn Renderable + Send + Sync> + Send + Sync + 'static,
    ) -> Self {
        Self {
            render_fn: Box::new(f),
            no_wrap: false,
            justify: JustifyMethod::Left,
        }
    }

    /// Set whether the column should avoid wrapping.
    pub fn with_no_wrap(mut self, no_wrap: bool) -> Self {
        self.no_wrap = no_wrap;
        self
    }

    /// Set the column justification.
    pub fn with_justify(mut self, justify: JustifyMethod) -> Self {
        self.justify = justify;
        self
    }
}

impl std::fmt::Debug for RenderableColumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderableColumn")
            .field("no_wrap", &self.no_wrap)
            .field("justify", &self.justify)
            .finish_non_exhaustive()
    }
}

impl ProgressColumn for RenderableColumn {
    fn table_column(&self) -> Column {
        Column::new().no_wrap(self.no_wrap).justify(self.justify)
    }

    fn render(
        &self,
        task: &ProgressTask,
        _now: f64,
        _options: &ConsoleOptions,
    ) -> Box<dyn Renderable + Send + Sync> {
        (self.render_fn)(task)
    }
}

struct ProgressState {
    start: Instant,
    tasks: HashMap<TaskID, ProgressTask>,
    order: Vec<TaskID>,
    next_id: usize,
    speed_estimate_period: f64,
    expand: bool,
    // Cache for columns with max_refresh, keyed by (task_id, column_index).
    cell_cache: HashMap<(TaskID, usize), (f64, Segments)>,
}

impl ProgressState {
    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

#[derive(Clone)]
struct ProgressRenderable {
    state: Arc<Mutex<ProgressState>>,
    columns: Arc<Vec<Box<dyn ProgressColumn>>>,
}

impl Renderable for ProgressRenderable {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let (tasks, now, expand, cache_snapshot) = {
            let state = self.state.lock().expect("progress state mutex poisoned");
            let now = state.now();
            let tasks: Vec<ProgressTask> = state
                .order
                .iter()
                .filter_map(|id| state.tasks.get(id).cloned())
                .collect();
            (tasks, now, state.expand, state.cell_cache.clone())
        };

        let mut table = Table::grid().with_padding(1, 1).with_expand(expand);
        for col in self.columns.iter() {
            table.add_column(col.table_column());
        }

        let mut new_cache: HashMap<(TaskID, usize), (f64, Segments)> = cache_snapshot;

        for task in tasks.iter().filter(|t| t.visible) {
            let mut row_cells: Vec<Box<dyn Renderable + Send + Sync>> =
                Vec::with_capacity(self.columns.len());
            for (col_index, col) in self.columns.iter().enumerate() {
                let segs = if let Some(max_refresh) = col.max_refresh() {
                    // Match Python Rich behavior in ProgressColumn.__call__:
                    // apply max_refresh caching only while task.completed is zero.
                    if task.completed <= 0.0 {
                        let key = (task.id, col_index);
                        if let Some((last_ts, cached)) = new_cache.get(&key) {
                            if now - *last_ts < max_refresh.as_secs_f64() {
                                cached.clone()
                            } else {
                                let renderable = col.render(task, now, options);
                                let segs = renderable.render(console, options);
                                new_cache.insert(key, (now, segs.clone()));
                                segs
                            }
                        } else {
                            let renderable = col.render(task, now, options);
                            let segs = renderable.render(console, options);
                            new_cache.insert(key, (now, segs.clone()));
                            segs
                        }
                    } else {
                        let renderable = col.render(task, now, options);
                        renderable.render(console, options)
                    }
                } else {
                    let renderable = col.render(task, now, options);
                    renderable.render(console, options)
                };
                row_cells.push(Box::new(SegmentsCell::new(segs)));
            }
            table.add_row(Row::new(row_cells));
        }

        // Persist cache updates.
        {
            let mut state = self.state.lock().expect("progress state mutex poisoned");
            state.cell_cache = new_cache;
        }

        table.render(console, options)
    }
}

#[derive(Clone)]
struct SegmentsCell {
    segments: Segments,
}

impl SegmentsCell {
    fn new(segments: Segments) -> Self {
        Self { segments }
    }
}

impl Renderable for SegmentsCell {
    fn render(&self, _console: &Console, _options: &ConsoleOptions) -> Segments {
        self.segments.clone()
    }

    fn measure(&self, _console: &Console, _options: &ConsoleOptions) -> crate::Measurement {
        // Derive width from the pre-rendered segments.
        let mut max_width: usize = 0;
        let mut current_width: usize = 0;
        for seg in self.segments.iter() {
            if seg.text.as_ref() == "\n" {
                max_width = max_width.max(current_width);
                current_width = 0;
                continue;
            }
            current_width += seg.cell_len();
        }
        max_width = max_width.max(current_width);
        crate::Measurement::new(max_width, max_width)
    }
}

enum DeferredConsoleCall {
    Print {
        segments: Segments,
        style: Option<Style>,
        justify: Option<JustifyMethod>,
        overflow: Option<OverflowMethod>,
        no_wrap: bool,
        end: String,
    },
    Log {
        segments: Segments,
        file: Option<String>,
        line: Option<u32>,
    },
}

pub struct Progress {
    state: Arc<Mutex<ProgressState>>,
    columns: Arc<Vec<Box<dyn ProgressColumn>>>,
    live: Live,
    disable: bool,
    auto_refresh: bool,
    started: Arc<AtomicBool>,
    deferred_console_calls: Arc<Mutex<Vec<DeferredConsoleCall>>>,
}

impl Progress {
    pub fn new(
        columns: Vec<Box<dyn ProgressColumn>>,
        live_options: LiveOptions,
        disable: bool,
        expand: bool,
    ) -> Self {
        Self::with_console(columns, Console::new(), live_options, disable, expand)
    }

    pub fn with_console(
        columns: Vec<Box<dyn ProgressColumn>>,
        console: Console<Stdout>,
        live_options: LiveOptions,
        disable: bool,
        expand: bool,
    ) -> Self {
        let auto_refresh = live_options.auto_refresh;
        let state = Arc::new(Mutex::new(ProgressState {
            start: Instant::now(),
            tasks: HashMap::new(),
            order: Vec::new(),
            next_id: 0,
            speed_estimate_period: 30.0,
            expand,
            cell_cache: HashMap::new(),
        }));
        let columns = Arc::new(columns);
        let renderable = ProgressRenderable {
            state: state.clone(),
            columns: columns.clone(),
        };
        let live = Live::with_console(Box::new(renderable), console, live_options);
        Self {
            state,
            columns,
            live,
            disable,
            auto_refresh,
            started: Arc::new(AtomicBool::new(false)),
            deferred_console_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// A convenience constructor matching Rich's default `track()` columns:
    /// description, bar, percentage, and time remaining.
    pub fn new_default(
        live_options: LiveOptions,
        disable: bool,
        expand: bool,
        show_speed: bool,
    ) -> Self {
        let columns: Vec<Box<dyn ProgressColumn>> = vec![
            Box::new(TextColumn::new("[progress.description]{task.description}")),
            Box::new(BarColumn::new()),
            Box::new(TaskProgressColumn::new(show_speed)),
            Box::new(TimeRemainingColumn::new(false)),
        ];
        Self::new(columns, live_options, disable, expand)
    }

    pub fn start(&mut self) -> io::Result<()> {
        if self.disable {
            return Ok(());
        }
        self.live.start(true)?;
        self.started.store(true, Ordering::SeqCst);
        self.flush_deferred_console_calls()
    }

    pub fn stop(&mut self) -> io::Result<()> {
        if self.disable {
            return Ok(());
        }
        self.started.store(false, Ordering::SeqCst);
        self.live.stop()
    }

    pub fn refresh(&self) -> io::Result<()> {
        if self.disable {
            return Ok(());
        }
        self.live.refresh()
    }

    pub fn add_task(
        &self,
        description: &str,
        start: bool,
        total: Option<f64>,
        completed: f64,
        visible: bool,
    ) -> TaskID {
        let id = {
            let mut state = self.state.lock().expect("progress state mutex poisoned");
            let id = TaskID(state.next_id);
            state.next_id += 1;

            let task = ProgressTask {
                id,
                description: description.to_string(),
                total,
                completed,
                visible,
                fields: HashMap::new(),
                finished_time: None,
                finished_speed: None,
                start_time: None,
                stop_time: None,
                progress: VecDeque::with_capacity(1000),
            };

            state.order.push(id);
            state.tasks.insert(id, task);
            id
        };
        if start {
            self.start_task(id);
        }
        let _ = self.refresh();
        id
    }

    pub fn remove_task(&self, task_id: TaskID) {
        let mut state = self.state.lock().expect("progress state mutex poisoned");
        state.tasks.remove(&task_id);
        state.order.retain(|&id| id != task_id);
        state.cell_cache.retain(|(id, _), _| *id != task_id);
    }

    pub fn start_task(&self, task_id: TaskID) {
        let mut state = self.state.lock().expect("progress state mutex poisoned");
        let now = state.now();
        if let Some(task) = state.tasks.get_mut(&task_id) {
            if task.start_time.is_none() {
                task.start_time = Some(now);
            }
        }
    }

    pub fn stop_task(&self, task_id: TaskID) {
        let mut state = self.state.lock().expect("progress state mutex poisoned");
        let now = state.now();
        if let Some(task) = state.tasks.get_mut(&task_id) {
            if task.start_time.is_none() {
                task.start_time = Some(now);
            }
            task.stop_time = Some(now);
        }
    }

    pub fn advance(&self, task_id: TaskID, advance: f64) {
        let mut state = self.state.lock().expect("progress state mutex poisoned");
        let now = state.now();
        let speed_estimate_period = state.speed_estimate_period;
        let Some(task) = state.tasks.get_mut(&task_id) else {
            return;
        };

        let completed_start = task.completed;
        task.completed += advance;
        let update_completed = task.completed - completed_start;

        let old_sample_time = now - speed_estimate_period;
        while let Some(front) = task.progress.front() {
            if front.timestamp < old_sample_time {
                task.progress.pop_front();
            } else {
                break;
            }
        }
        while task.progress.len() > 1000 {
            task.progress.pop_front();
        }
        task.progress.push_back(ProgressSample {
            timestamp: now,
            completed: update_completed,
        });

        if let Some(total) = task.total {
            if task.completed >= total && task.finished_time.is_none() {
                task.finished_time = task.elapsed(now);
                task.finished_speed = task.speed();
            }
        }
    }

    pub fn update(
        &self,
        task_id: TaskID,
        total: Option<Option<f64>>,
        completed: Option<f64>,
        advance: Option<f64>,
        description: Option<String>,
        visible: Option<bool>,
        refresh: bool,
        fields: Option<HashMap<String, String>>,
    ) {
        let mut state = self.state.lock().expect("progress state mutex poisoned");
        let now = state.now();
        let speed_estimate_period = state.speed_estimate_period;
        let Some(task) = state.tasks.get_mut(&task_id) else {
            return;
        };
        let completed_start = task.completed;

        if let Some(total) = total {
            if task.total != total {
                task.total = total;
                task.progress.clear();
                task.finished_time = None;
                task.finished_speed = None;
            }
        }
        if let Some(advance) = advance {
            task.completed += advance;
        }
        if let Some(completed) = completed {
            task.completed = completed;
        }
        if let Some(description) = description {
            task.description = description;
        }
        if let Some(visible) = visible {
            task.visible = visible;
        }
        if let Some(fields) = fields {
            task.fields.extend(fields);
        }

        let update_completed = task.completed - completed_start;
        let old_sample_time = now - speed_estimate_period;
        while let Some(front) = task.progress.front() {
            if front.timestamp < old_sample_time {
                task.progress.pop_front();
            } else {
                break;
            }
        }
        while task.progress.len() > 1000 {
            task.progress.pop_front();
        }
        if update_completed > 0.0 {
            task.progress.push_back(ProgressSample {
                timestamp: now,
                completed: update_completed,
            });
        }

        if let Some(total) = task.total {
            if task.completed >= total && task.finished_time.is_none() {
                task.finished_time = task.elapsed(now);
            }
        }
        drop(state);
        if refresh {
            let _ = self.refresh();
        }
    }

    pub fn update_task(
        &self,
        task_id: TaskID,
        total: Option<Option<f64>>,
        completed: Option<f64>,
        description: Option<String>,
        visible: Option<bool>,
    ) {
        self.update(
            task_id,
            total,
            completed,
            None,
            description,
            visible,
            false,
            None,
        );
    }

    pub fn reset(
        &self,
        task_id: TaskID,
        start: bool,
        total: Option<Option<f64>>,
        completed: f64,
        visible: Option<bool>,
        description: Option<String>,
        fields: Option<HashMap<String, String>>,
    ) {
        let mut state = self.state.lock().expect("progress state mutex poisoned");
        let now = state.now();
        let Some(task) = state.tasks.get_mut(&task_id) else {
            return;
        };
        task.progress.clear();
        task.finished_time = None;
        task.finished_speed = None;
        task.start_time = if start { Some(now) } else { None };
        if let Some(total) = total {
            task.total = total;
        }
        task.completed = completed;
        if let Some(visible) = visible {
            task.visible = visible;
        }
        if let Some(fields) = fields {
            task.fields = fields;
        }
        if let Some(description) = description {
            task.description = description;
        }
        task.finished_time = None;
        drop(state);
        let _ = self.refresh();
    }

    pub fn track<'a, I>(
        &'a self,
        iter: I,
        task_id: TaskID,
        update_period: Duration,
    ) -> ProgressIterator<'a, I::IntoIter>
    where
        I: IntoIterator,
    {
        ProgressIterator {
            iter: iter.into_iter(),
            progress: self,
            task_id,
            track_thread: TrackThread::new(
                self.auto_refresh && !self.disable,
                self.state.clone(),
                self.live.started_flag(),
                task_id,
                update_period,
            ),
            pending_increment: false,
        }
    }

    /// Create a task and return an iterator that advances it.
    pub fn track_iter<'a, I>(
        &'a self,
        iter: I,
        description: &str,
        total: Option<f64>,
        completed: f64,
        update_period: Duration,
    ) -> ProgressIterator<'a, I::IntoIter>
    where
        I: IntoIterator,
    {
        let task_id = self.add_task(description, true, total, completed, true);
        self.track(iter, task_id, update_period)
    }

    pub fn track_sequence<'a, I>(
        &'a self,
        sequence: I,
        config: TrackConfig,
    ) -> ProgressIterator<'a, I::IntoIter>
    where
        I: IntoIterator,
    {
        let iter = sequence.into_iter();
        let inferred_total = config.total.or_else(|| {
            let (lower, upper) = iter.size_hint();
            let hint = upper.or(Some(lower)).unwrap_or(0);
            if hint == 0 { None } else { Some(hint as f64) }
        });

        let task_id = if let Some(task_id) = config.task_id {
            self.update(
                task_id,
                // Match Rich: explicitly set total (including to None) for existing tasks.
                Some(inferred_total),
                Some(config.completed),
                None,
                None,
                None,
                false,
                None,
            );
            task_id
        } else {
            self.add_task(
                &config.description,
                true,
                inferred_total,
                config.completed,
                true,
            )
        };

        self.track(iter, task_id, config.update_period)
    }

    fn render_to_deferred_segments<R: Renderable + ?Sized>(renderable: &R) -> Segments {
        let options = ConsoleOptions::default();
        let temp_console = Console::<Stdout>::with_options(options.clone());
        renderable.render(&temp_console, &options)
    }

    fn flush_deferred_console_calls(&self) -> io::Result<()> {
        let calls = {
            let mut deferred = self
                .deferred_console_calls
                .lock()
                .expect("deferred console calls mutex poisoned");
            deferred.drain(..).collect::<Vec<_>>()
        };

        for call in calls {
            match call {
                DeferredConsoleCall::Print {
                    segments,
                    style,
                    justify,
                    overflow,
                    no_wrap,
                    end,
                } => {
                    let renderable = SegmentsCell::new(segments);
                    self.live
                        .print(&renderable, style, justify, overflow, no_wrap, &end)?;
                }
                DeferredConsoleCall::Log {
                    segments,
                    file,
                    line,
                } => {
                    let renderable = SegmentsCell::new(segments);
                    self.live.log(&renderable, file.as_deref(), line)?;
                }
            }
        }

        Ok(())
    }

    pub fn print<R: Renderable + ?Sized>(
        &self,
        renderable: &R,
        style: Option<Style>,
        justify: Option<JustifyMethod>,
        overflow: Option<OverflowMethod>,
        no_wrap: bool,
        end: &str,
    ) -> io::Result<()> {
        if self.disable {
            return Ok(());
        }

        if self.started.load(Ordering::SeqCst) {
            return self
                .live
                .print(renderable, style, justify, overflow, no_wrap, end);
        }

        let mut deferred = self
            .deferred_console_calls
            .lock()
            .expect("deferred console calls mutex poisoned");
        deferred.push(DeferredConsoleCall::Print {
            segments: Self::render_to_deferred_segments(renderable),
            style,
            justify,
            overflow,
            no_wrap,
            end: end.to_string(),
        });
        Ok(())
    }

    pub fn log<R: Renderable + ?Sized>(
        &self,
        renderable: &R,
        file: Option<&str>,
        line: Option<u32>,
    ) -> io::Result<()> {
        if self.disable {
            return Ok(());
        }

        if self.started.load(Ordering::SeqCst) {
            return self.live.log(renderable, file, line);
        }

        let mut deferred = self
            .deferred_console_calls
            .lock()
            .expect("deferred console calls mutex poisoned");
        deferred.push(DeferredConsoleCall::Log {
            segments: Self::render_to_deferred_segments(renderable),
            file: file.map(ToString::to_string),
            line,
        });
        Ok(())
    }
}

impl Progress {
    /// Access the task list (snapshot).
    pub fn tasks(&self) -> Vec<ProgressTask> {
        let state = self.state.lock().expect("progress state mutex poisoned");
        state
            .order
            .iter()
            .filter_map(|id| state.tasks.get(id).cloned())
            .collect()
    }

    /// List of task IDs in order.
    pub fn task_ids(&self) -> Vec<TaskID> {
        let state = self.state.lock().expect("progress state mutex poisoned");
        state.order.clone()
    }

    /// Whether all tasks are complete.
    pub fn finished(&self) -> bool {
        let state = self.state.lock().expect("progress state mutex poisoned");
        state.tasks.values().all(|t| t.finished())
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl Renderable for Progress {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        // Mirror Python Rich: Progress itself is renderable and produces the tasks table.
        let renderable = ProgressRenderable {
            state: self.state.clone(),
            columns: self.columns.clone(),
        };
        renderable.render(console, options)
    }
}

// =============================================================================
// ProgressReader: File I/O Progress Tracking
// =============================================================================

/// A reader that tracks progress as bytes are read.
///
/// This wraps any type implementing `Read` and updates a progress task
/// as data is read. Similar to Python Rich's `_Reader` class.
///
/// # Example
///
/// ```ignore
/// use std::fs::File;
/// use std::io::Read;
/// use rich_rs::progress::{Progress, LiveOptions};
/// use rich_rs::live::LiveOptions;
///
/// let mut progress = Progress::new_default(LiveOptions::default(), false, false, false);
/// progress.start().unwrap();
///
/// let mut reader = progress.open("large_file.bin", "Reading...").unwrap();
/// let mut buffer = Vec::new();
/// reader.read_to_end(&mut buffer).unwrap();
///
/// progress.stop().unwrap();
/// ```
pub struct ProgressReader<'a, R: Read> {
    inner: R,
    progress: &'a Progress,
    task_id: TaskID,
    /// Whether the wrapper "owns" the handle (for API compatibility with Python Rich).
    /// In Rust, ownership is handled by the type system, so this is primarily for
    /// documentation and potential future use (e.g., logging when handles are closed).
    #[allow(dead_code)]
    close_handle: bool,
}

impl<'a, R: Read> ProgressReader<'a, R> {
    /// Create a new `ProgressReader` wrapping the given reader.
    ///
    /// The `close_handle` parameter controls whether the inner reader
    /// should be dropped when this wrapper is dropped (for owned handles).
    pub fn new(inner: R, progress: &'a Progress, task_id: TaskID, close_handle: bool) -> Self {
        Self {
            inner,
            progress,
            task_id,
            close_handle,
        }
    }

    /// Returns the task ID associated with this reader.
    pub fn task_id(&self) -> TaskID {
        self.task_id
    }

    /// Returns a reference to the inner reader.
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the inner reader.
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes the wrapper, returning the inner reader.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> Read for ProgressReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.progress.advance(self.task_id, n as f64);
        Ok(n)
    }
}

impl<R: Read + BufRead> BufRead for ProgressReader<'_, R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
        self.progress.advance(self.task_id, amt as f64);
    }
}

impl<R: Read + Seek> Seek for ProgressReader<'_, R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = self.inner.seek(pos)?;
        // Update completed position to match seek position (like Python Rich).
        self.progress.update(
            self.task_id,
            None,
            Some(new_pos as f64),
            None,
            None,
            None,
            false,
            None,
        );
        Ok(new_pos)
    }
}

/// Builder for creating a `ProgressReader` with optional configuration.
///
/// # Example
///
/// ```ignore
/// use std::fs::File;
/// use std::io::Read;
/// use rich_rs::{Progress, WrapFileBuilder};
/// use rich_rs::live::LiveOptions;
///
/// let progress = Progress::new_default(LiveOptions::default(), false, false, false);
/// let file = File::open("data.bin").unwrap();
///
/// let reader = WrapFileBuilder::new(&progress, file)
///     .total(1024)
///     .description("Processing...")
///     .build();
/// ```
pub struct WrapFileBuilder<'a, R: Read> {
    progress: &'a Progress,
    reader: R,
    total: Option<u64>,
    task_id: Option<TaskID>,
    description: String,
    close_handle: bool,
}

impl<'a, R: Read> WrapFileBuilder<'a, R> {
    /// Create a new builder with the given progress and reader.
    pub fn new(progress: &'a Progress, reader: R) -> Self {
        Self {
            progress,
            reader,
            total: None,
            task_id: None,
            description: "Reading...".to_string(),
            close_handle: false,
        }
    }

    /// Set the total number of bytes expected to be read.
    pub fn total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// Set an optional total (None for indeterminate progress).
    pub fn total_opt(mut self, total: Option<u64>) -> Self {
        self.total = total;
        self
    }

    /// Use an existing task instead of creating a new one.
    pub fn task_id(mut self, task_id: TaskID) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Set the description for the task (used when creating a new task).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set whether the inner handle should be considered "owned" and closed.
    pub fn close_handle(mut self, close: bool) -> Self {
        self.close_handle = close;
        self
    }

    /// Build the `ProgressReader`.
    ///
    /// If no `task_id` was provided, a new task is created with the
    /// configured description and total.
    pub fn build(self) -> ProgressReader<'a, R> {
        let task_id = if let Some(id) = self.task_id {
            // Update existing task with total if provided.
            if let Some(total) = self.total {
                self.progress.update(
                    id,
                    Some(Some(total as f64)),
                    None,
                    None,
                    None,
                    None,
                    false,
                    None,
                );
            }
            id
        } else {
            // Create a new task.
            self.progress.add_task(
                &self.description,
                true,
                self.total.map(|t| t as f64),
                0.0,
                true,
            )
        };

        ProgressReader::new(self.reader, self.progress, task_id, self.close_handle)
    }
}

impl Progress {
    /// Wrap a reader to track progress while reading.
    ///
    /// This creates a new task and wraps the reader to automatically update
    /// progress as data is read.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to wrap (any type implementing `Read`).
    /// * `total` - Total number of bytes expected. Pass `None` for indeterminate.
    /// * `description` - Description shown next to the progress bar.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::fs::File;
    /// use std::io::Read;
    /// use rich_rs::Progress;
    /// use rich_rs::live::LiveOptions;
    ///
    /// let progress = Progress::new_default(LiveOptions::default(), false, false, false);
    /// let file = File::open("data.bin").unwrap();
    ///
    /// let mut reader = progress.wrap_file(file, Some(1024), "Reading data");
    /// let mut buf = Vec::new();
    /// reader.read_to_end(&mut buf).unwrap();
    /// ```
    pub fn wrap_file<'a, R: Read>(
        &'a self,
        reader: R,
        total: Option<u64>,
        description: &str,
    ) -> ProgressReader<'a, R> {
        WrapFileBuilder::new(self, reader)
            .total_opt(total)
            .description(description)
            .close_handle(false)
            .build()
    }

    /// Wrap a reader with an existing task.
    ///
    /// Similar to `wrap_file`, but uses an existing task instead of creating
    /// a new one. Optionally updates the task's total.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to wrap.
    /// * `task_id` - The existing task to update.
    /// * `total` - Optional total to set on the task.
    pub fn wrap_file_with_task<'a, R: Read>(
        &'a self,
        reader: R,
        task_id: TaskID,
        total: Option<u64>,
    ) -> ProgressReader<'a, R> {
        WrapFileBuilder::new(self, reader)
            .task_id(task_id)
            .total_opt(total)
            .close_handle(false)
            .build()
    }

    /// Open a file with progress tracking.
    ///
    /// This is a convenience method that opens a file, determines its size,
    /// creates a progress task, and returns a wrapped reader.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to open.
    /// * `description` - Description shown next to the progress bar.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or its metadata cannot
    /// be read.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::io::Read;
    /// use rich_rs::Progress;
    /// use rich_rs::live::LiveOptions;
    ///
    /// let mut progress = Progress::new_default(LiveOptions::default(), false, false, false);
    /// progress.start().unwrap();
    ///
    /// let mut reader = progress.open("large_file.bin", "Processing file").unwrap();
    /// let mut contents = Vec::new();
    /// reader.read_to_end(&mut contents).unwrap();
    ///
    /// progress.stop().unwrap();
    /// ```
    pub fn open<'a, P: AsRef<Path>>(
        &'a self,
        path: P,
        description: &str,
    ) -> io::Result<ProgressReader<'a, File>> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let total = metadata.len();

        Ok(WrapFileBuilder::new(self, file)
            .total(total)
            .description(description)
            .close_handle(true)
            .build())
    }

    /// Open a file with progress tracking using an existing task.
    ///
    /// Similar to `open`, but uses an existing task instead of creating a
    /// new one. The task's total will be updated to the file size.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to open.
    /// * `task_id` - The existing task to update.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or its metadata cannot
    /// be read.
    pub fn open_with_task<'a, P: AsRef<Path>>(
        &'a self,
        path: P,
        task_id: TaskID,
    ) -> io::Result<ProgressReader<'a, File>> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let total = metadata.len();

        Ok(WrapFileBuilder::new(self, file)
            .task_id(task_id)
            .total(total)
            .close_handle(true)
            .build())
    }
}

pub struct ProgressIterator<'a, I> {
    iter: I,
    progress: &'a Progress,
    task_id: TaskID,
    track_thread: TrackThread,
    pending_increment: bool,
}

struct TrackThread {
    enabled: bool,
    completed: Arc<AtomicUsize>,
    done: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl TrackThread {
    fn new(
        enabled: bool,
        state: Arc<Mutex<ProgressState>>,
        live_started: Arc<AtomicBool>,
        task_id: TaskID,
        update_period: Duration,
    ) -> Self {
        if !enabled {
            return Self {
                enabled: false,
                completed: Arc::new(AtomicUsize::new(0)),
                done: Arc::new(AtomicBool::new(false)),
                handle: None,
            };
        }

        let update_period = update_period.max(Duration::from_millis(1));

        let completed = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(AtomicBool::new(false));
        let completed_thread = completed.clone();
        let done_thread = done.clone();

        let handle = thread::spawn(move || {
            let mut last_completed: usize = 0;
            while !done_thread.load(Ordering::SeqCst) && live_started.load(Ordering::SeqCst) {
                thread::sleep(update_period);
                if done_thread.load(Ordering::SeqCst) || !live_started.load(Ordering::SeqCst) {
                    break;
                }
                let current = completed_thread.load(Ordering::SeqCst);
                if current != last_completed {
                    let delta = (current - last_completed) as f64;
                    last_completed = current;
                    advance_state(&state, task_id, delta);
                }
            }
        });

        Self {
            enabled: true,
            completed,
            done,
            handle: Some(handle),
        }
    }

    fn increment(&self) {
        if self.enabled {
            self.completed.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn take_completed(&self) -> usize {
        self.completed.load(Ordering::SeqCst)
    }

    fn stop(&mut self) {
        if !self.enabled {
            return;
        }
        self.done.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for TrackThread {
    fn drop(&mut self) {
        self.stop();
    }
}

impl<'a, I> Iterator for ProgressIterator<'a, I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.progress.disable && self.pending_increment {
            if self.progress.auto_refresh {
                self.track_thread.increment();
            } else {
                self.progress.advance(self.task_id, 1.0);
                let _ = self.progress.refresh();
            }
        }

        let item = self.iter.next();
        self.pending_increment = item.is_some();
        item
    }
}

impl<'a, I> Drop for ProgressIterator<'a, I> {
    fn drop(&mut self) {
        if self.progress.disable {
            return;
        }
        if self.progress.auto_refresh {
            let completed = self.track_thread.take_completed() as f64;
            self.track_thread.stop();
            self.progress.update(
                self.task_id,
                None,
                Some(completed),
                None,
                None,
                None,
                true,
                None,
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackConfig {
    pub total: Option<f64>,
    pub completed: f64,
    pub task_id: Option<TaskID>,
    pub description: String,
    pub update_period: Duration,
}

impl TrackConfig {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            total: None,
            completed: 0.0,
            task_id: None,
            description: description.into(),
            update_period: Duration::from_millis(100),
        }
    }

    pub fn with_total(mut self, total: Option<f64>) -> Self {
        self.total = total;
        self
    }

    pub fn with_completed(mut self, completed: f64) -> Self {
        self.completed = completed;
        self
    }

    pub fn with_task_id(mut self, task_id: TaskID) -> Self {
        self.task_id = Some(task_id);
        self
    }

    pub fn with_update_period(mut self, update_period: Duration) -> Self {
        self.update_period = update_period;
        self
    }
}

fn advance_state(state: &Arc<Mutex<ProgressState>>, task_id: TaskID, advance: f64) {
    let mut state = state.lock().expect("progress state mutex poisoned");
    let now = state.now();
    let speed_estimate_period = state.speed_estimate_period;
    let Some(task) = state.tasks.get_mut(&task_id) else {
        return;
    };

    let completed_start = task.completed;
    task.completed += advance;
    let update_completed = task.completed - completed_start;

    let old_sample_time = now - speed_estimate_period;
    while let Some(front) = task.progress.front() {
        if front.timestamp < old_sample_time {
            task.progress.pop_front();
        } else {
            break;
        }
    }
    while task.progress.len() > 1000 {
        task.progress.pop_front();
    }
    task.progress.push_back(ProgressSample {
        timestamp: now,
        completed: update_completed,
    });

    if let Some(total) = task.total {
        if task.completed >= total && task.finished_time.is_none() {
            task.finished_time = task.elapsed(now);
            task.finished_speed = task.speed();
        }
    }
}

fn format_task_template(template: &str, task: &ProgressTask, now: f64) -> String {
    // Subset of Python Rich's `str.format(task=task)` support.
    // Supports fields in the form:
    //   {task.description}
    //   {task.percentage:>3.0f}
    //   {task.completed}
    //   {task.total}
    //   {task.elapsed}
    //   {task.remaining}
    //   {task.speed}
    //   {task.time_remaining}
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{task.") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 6..];
        let Some(end) = after.find('}') else {
            out.push_str(rest);
            return out;
        };
        let inside = &after[..end];
        let (field, spec) = inside.split_once(':').unwrap_or((inside, ""));
        let formatted = format_task_field(field, spec, task, now);
        out.push_str(&formatted);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

#[derive(Debug, Default, Clone, Copy)]
struct FormatSpec {
    align: Option<char>,
    width: Option<usize>,
    precision: Option<usize>,
    ty: Option<char>,
}

fn parse_format_spec(spec: &str) -> FormatSpec {
    let mut s = spec;
    let mut out = FormatSpec::default();

    if let Some(first) = s.chars().next() {
        if matches!(first, '<' | '>' | '^') {
            out.align = Some(first);
            s = &s[first.len_utf8()..];
        }
    }

    // width digits
    let mut width_end = 0;
    for (i, ch) in s.char_indices() {
        if ch.is_ascii_digit() {
            width_end = i + 1;
        } else {
            break;
        }
    }
    if width_end > 0 {
        if let Ok(w) = s[..width_end].parse::<usize>() {
            out.width = Some(w);
        }
        s = &s[width_end..];
    }

    // precision .digits
    if let Some(rest) = s.strip_prefix('.') {
        let mut prec_end = 0;
        for (i, ch) in rest.char_indices() {
            if ch.is_ascii_digit() {
                prec_end = i + 1;
            } else {
                break;
            }
        }
        if prec_end > 0 {
            if let Ok(p) = rest[..prec_end].parse::<usize>() {
                out.precision = Some(p);
            }
            s = &rest[prec_end..];
        }
    }

    // type (f, d)
    if let Some(last) = s.chars().next() {
        if matches!(last, 'f' | 'd') {
            out.ty = Some(last);
        }
    }

    out
}

fn pad_aligned(text: &str, width: usize, align: char) -> String {
    let current = crate::cells::cell_len(text);
    if current >= width {
        return text.to_string();
    }
    let pad = width - current;
    match align {
        '<' => format!("{text}{}", " ".repeat(pad)),
        '^' => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
        }
        _ => format!("{}{}", " ".repeat(pad), text),
    }
}

fn apply_format_spec_value(text: String, spec: &FormatSpec) -> String {
    let Some(width) = spec.width else {
        return text;
    };
    let align = spec.align.unwrap_or('>');
    pad_aligned(&text, width, align)
}

fn format_task_field(field: &str, spec: &str, task: &ProgressTask, now: f64) -> String {
    let spec = parse_format_spec(spec);

    let value = match field {
        "description" => return apply_format_spec_value(task.description.clone(), &spec),
        "percentage" => Some(task.percentage()),
        "completed" => Some(task.completed),
        "total" => task.total,
        "elapsed" => task.elapsed(now),
        "remaining" => task.remaining(),
        "speed" => task.speed(),
        "time_remaining" => task.time_remaining(),
        _ => None,
    };

    let Some(value) = value else {
        return apply_format_spec_value(String::new(), &spec);
    };

    let rendered = match spec.ty {
        Some('d') => format!("{}", value as i64),
        Some('f') => {
            let precision = spec.precision.unwrap_or(0);
            format!("{value:.precision$}", precision = precision)
        }
        _ => {
            if let Some(precision) = spec.precision {
                format!("{value:.precision$}", precision = precision)
            } else {
                // Default formatting resembles Python's float -> string for simple cases.
                if value.fract() == 0.0 {
                    format!("{:.0}", value)
                } else {
                    format!("{value}")
                }
            }
        }
    };

    apply_format_spec_value(rendered, &spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_task_template_description() {
        let task = ProgressTask {
            id: TaskID(1),
            description: "Hello".to_string(),
            total: Some(10.0),
            completed: 3.0,
            visible: true,
            fields: HashMap::new(),
            finished_time: None,
            finished_speed: None,
            start_time: Some(0.0),
            stop_time: None,
            progress: VecDeque::new(),
        };

        let s = format_task_template("x {task.description} y", &task, 1.0);
        assert_eq!(s, "x Hello y");
    }

    #[test]
    fn test_apply_format_spec_right_align() {
        let spec = parse_format_spec(">3");
        assert_eq!(apply_format_spec_value("7".to_string(), &spec), "  7");
        assert_eq!(apply_format_spec_value("1234".to_string(), &spec), "1234");
    }

    #[test]
    fn test_progress_update_appends_samples_only_on_positive_change() {
        let live_options = LiveOptions {
            auto_refresh: true,
            ..Default::default()
        };
        let progress = Progress::new_default(live_options, true, false, false);

        let task_id = progress.add_task("t", true, Some(10.0), 0.0, true);

        {
            let state = progress
                .state
                .lock()
                .expect("progress state mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.progress.len(), 0);
        }

        progress.update(task_id, None, None, Some(1.0), None, None, false, None);
        {
            let state = progress
                .state
                .lock()
                .expect("progress state mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.completed, 1.0);
            assert_eq!(task.progress.len(), 1);
        }

        // No change -> no new sample.
        progress.update(task_id, None, Some(1.0), None, None, None, false, None);
        {
            let state = progress
                .state
                .lock()
                .expect("progress state mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.progress.len(), 1);
        }
    }

    #[test]
    fn test_progress_update_total_change_resets_speed_samples() {
        let live_options = LiveOptions {
            auto_refresh: true,
            ..Default::default()
        };
        let progress = Progress::new_default(live_options, true, false, false);

        let task_id = progress.add_task("t", true, Some(10.0), 0.0, true);
        progress.update(task_id, None, None, Some(2.0), None, None, false, None);
        {
            let state = progress
                .state
                .lock()
                .expect("progress state mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.progress.len(), 1);
        }

        // Changing total clears progress samples like Rich.
        progress.update(
            task_id,
            Some(Some(20.0)),
            None,
            None,
            None,
            None,
            false,
            None,
        );
        {
            let state = progress
                .state
                .lock()
                .expect("progress state mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.total, Some(20.0));
            assert_eq!(task.progress.len(), 0);
        }
    }

    #[test]
    fn test_text_column_defaults_to_no_wrap() {
        let col = TextColumn::new("{task.description}").table_column();
        assert!(col.no_wrap);
    }

    #[test]
    fn test_task_progress_column_empty_when_total_unknown() {
        let task = ProgressTask {
            id: TaskID(0),
            description: "t".to_string(),
            total: None,
            completed: 0.0,
            visible: true,
            fields: HashMap::new(),
            finished_time: None,
            finished_speed: None,
            start_time: Some(0.0),
            stop_time: None,
            progress: VecDeque::new(),
        };

        let console = Console::new();
        let options = ConsoleOptions::default();
        let col = TaskProgressColumn::new(false);
        let rendered = col.render(&task, 0.0, &options);
        let segs = rendered.render(&console, &options);
        let text: String = segs.iter().map(|s| s.text.as_ref()).collect();
        assert_eq!(text, "");
    }

    #[test]
    fn test_bar_column_defaults_to_width_40() {
        let task = ProgressTask {
            id: TaskID(0),
            description: "t".to_string(),
            total: Some(100.0),
            completed: 50.0,
            visible: true,
            fields: HashMap::new(),
            finished_time: None,
            finished_speed: None,
            start_time: Some(0.0),
            stop_time: None,
            progress: VecDeque::new(),
        };

        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 120,
            ..ConsoleOptions::default()
        };
        let col = BarColumn::new();
        let rendered = col.render(&task, 0.0, &options);
        let measurement = rendered.measure(&console, &options);
        assert_eq!(measurement.minimum, 40);
        assert_eq!(measurement.maximum, 40);
    }

    #[test]
    fn test_track_sequence_updates_total_to_none_for_existing_task() {
        let live_options = LiveOptions {
            auto_refresh: true,
            ..Default::default()
        };
        let progress = Progress::new_default(live_options, true, false, false);
        let task_id = progress.add_task("t", true, Some(10.0), 0.0, true);

        let config = TrackConfig {
            total: None,
            completed: 0.0,
            task_id: Some(task_id),
            description: "t".to_string(),
            update_period: Duration::from_millis(10),
        };

        let _iter = progress.track_sequence(std::iter::empty::<usize>(), config);
        let state = progress
            .state
            .lock()
            .expect("progress state mutex poisoned");
        let task = state.tasks.get(&task_id).unwrap();
        assert_eq!(task.total, None);
    }

    #[test]
    fn test_progress_reader_advances_on_read() {
        use std::io::Cursor;

        let live_options = LiveOptions {
            auto_refresh: true,
            ..Default::default()
        };
        let progress = Progress::new_default(live_options, true, false, false);

        let data = b"Hello, World!";
        let cursor = Cursor::new(data.to_vec());

        let mut reader = progress.wrap_file(cursor, Some(data.len() as u64), "Reading");
        let task_id = reader.task_id();

        // Verify initial state.
        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.completed, 0.0);
            assert_eq!(task.total, Some(data.len() as f64));
        }

        // Read some data.
        let mut buf = [0u8; 5];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 5);

        // Verify progress was advanced.
        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.completed, 5.0);
        }

        // Read the rest.
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();

        // Verify completed.
        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.completed, data.len() as f64);
        }
    }

    #[test]
    fn test_progress_reader_with_existing_task() {
        use std::io::Cursor;

        let live_options = LiveOptions {
            auto_refresh: true,
            ..Default::default()
        };
        let progress = Progress::new_default(live_options, true, false, false);

        // Create a task manually.
        let task_id = progress.add_task("Existing task", true, None, 0.0, true);

        let data = b"Test data";
        let cursor = Cursor::new(data.to_vec());

        // Wrap with existing task.
        let mut reader = progress.wrap_file_with_task(cursor, task_id, Some(data.len() as u64));

        // Verify the task was updated with the total.
        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.total, Some(data.len() as f64));
        }

        // Read all data.
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();

        // Verify completed.
        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.completed, data.len() as f64);
        }
    }

    #[test]
    fn test_progress_reader_seek_updates_completed() {
        use std::io::Cursor;

        let live_options = LiveOptions {
            auto_refresh: true,
            ..Default::default()
        };
        let progress = Progress::new_default(live_options, true, false, false);

        let data = b"0123456789";
        let cursor = Cursor::new(data.to_vec());

        let mut reader = progress.wrap_file(cursor, Some(data.len() as u64), "Seeking");
        let task_id = reader.task_id();

        // Read some data first.
        let mut buf = [0u8; 3];
        reader.read(&mut buf).unwrap();

        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.completed, 3.0);
        }

        // Seek to position 7.
        reader.seek(SeekFrom::Start(7)).unwrap();

        // Completed should now reflect the seek position.
        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.completed, 7.0);
        }
    }

    #[test]
    fn test_wrap_file_builder() {
        use std::io::Cursor;

        let live_options = LiveOptions {
            auto_refresh: true,
            ..Default::default()
        };
        let progress = Progress::new_default(live_options, true, false, false);

        let data = b"Builder test";
        let cursor = Cursor::new(data.to_vec());

        let reader = WrapFileBuilder::new(&progress, cursor)
            .total(data.len() as u64)
            .description("Custom description")
            .build();

        let task_id = reader.task_id();

        let state = progress.state.lock().expect("mutex poisoned");
        let task = state.tasks.get(&task_id).unwrap();
        assert_eq!(task.description, "Custom description");
        assert_eq!(task.total, Some(data.len() as f64));
    }

    #[test]
    fn test_progress_reader_indeterminate() {
        use std::io::Cursor;

        let live_options = LiveOptions {
            auto_refresh: true,
            ..Default::default()
        };
        let progress = Progress::new_default(live_options, true, false, false);

        let data = b"Unknown size stream";
        let cursor = Cursor::new(data.to_vec());

        // No total provided (indeterminate progress).
        let mut reader = progress.wrap_file(cursor, None, "Streaming");
        let task_id = reader.task_id();

        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.total, None);
        }

        // Read all data.
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();

        // Completed should still track bytes read.
        {
            let state = progress.state.lock().expect("mutex poisoned");
            let task = state.tasks.get(&task_id).unwrap();
            assert_eq!(task.completed, data.len() as f64);
        }
    }

    #[test]
    fn test_progress_print_and_log_are_deferred_until_start() {
        let mut console = Console::new();
        console.set_quiet(true);

        let live_options = LiveOptions {
            auto_refresh: false,
            ..Default::default()
        };
        let mut progress = Progress::with_console(
            vec![Box::new(TextColumn::new("{task.description}"))],
            console,
            live_options,
            false,
            false,
        );

        progress
            .print(&Text::plain("queued print"), None, None, None, false, "\n")
            .unwrap();
        progress
            .log(&Text::plain("queued log"), Some("queued.rs"), Some(12))
            .unwrap();

        {
            let deferred = progress
                .deferred_console_calls
                .lock()
                .expect("deferred console calls mutex poisoned");
            assert_eq!(deferred.len(), 2);
        }

        progress.start().unwrap();

        let deferred = progress
            .deferred_console_calls
            .lock()
            .expect("deferred console calls mutex poisoned");
        assert!(deferred.is_empty());
    }

    #[test]
    fn test_progress_print_and_log_do_not_queue_after_start() {
        let mut console = Console::new();
        console.set_quiet(true);

        let live_options = LiveOptions {
            auto_refresh: false,
            ..Default::default()
        };
        let mut progress = Progress::with_console(
            vec![Box::new(TextColumn::new("{task.description}"))],
            console,
            live_options,
            false,
            false,
        );

        progress.start().unwrap();

        progress
            .print(&Text::plain("live print"), None, None, None, false, "\n")
            .unwrap();
        progress
            .log(&Text::plain("live log"), Some("live.rs"), Some(34))
            .unwrap();

        let deferred = progress
            .deferred_console_calls
            .lock()
            .expect("deferred console calls mutex poisoned");
        assert!(deferred.is_empty());
    }
}
