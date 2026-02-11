//! Live: auto-updating display for terminal renderables.
//!
//! This is a Rust port of Python Rich's `Live` and `LiveRender`:
//! - `rich/live.py`
//! - `rich/live_render.py`
//!
//! The primary use case is to power the Progress system (Phase 5.1).

use std::io;
use std::io::Stdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossterm::terminal;

use crate::Control;
use crate::text::Text;
use crate::{Console, Renderable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalOverflowMethod {
    Crop,
    Ellipsis,
    Visible,
}

impl Default for VerticalOverflowMethod {
    fn default() -> Self {
        Self::Ellipsis
    }
}

#[derive(Debug, Clone)]
pub struct LiveOptions {
    pub screen: bool,
    pub auto_refresh: bool,
    pub refresh_per_second: f64,
    pub transient: bool,
    pub vertical_overflow: VerticalOverflowMethod,
    /// When true, capture writes to stdout and route them through the Console output.
    pub redirect_stdout: bool,
    /// When true, capture writes to stderr and route them through the Console output.
    pub redirect_stderr: bool,
}

impl Default for LiveOptions {
    fn default() -> Self {
        Self {
            screen: false,
            auto_refresh: true,
            refresh_per_second: 4.0,
            transient: false,
            vertical_overflow: VerticalOverflowMethod::Ellipsis,
            redirect_stdout: false,
            redirect_stderr: false,
        }
    }
}

struct LiveState {
    options: LiveOptions,
    started: bool,
    live_id: Option<usize>,
    is_root: bool,
    alt_screen: bool,
    pending_renderable: Option<Box<dyn Renderable + Send + Sync>>,
}

/// A live-updating view of a renderable.
///
/// This owns a Console and drives updates by moving the cursor to re-render
/// in-place. When `auto_refresh` is enabled, a background thread calls `refresh()`
/// at `refresh_per_second`.
pub struct Live {
    console: Arc<Mutex<Console<Stdout>>>,
    state: Arc<Mutex<LiveState>>,
    stop_flag: Arc<AtomicBool>,
    started_flag: Arc<AtomicBool>,
    refresh_thread: Option<thread::JoinHandle<()>>,
    /// Optional callback to get the current renderable on each refresh tick.
    /// When set, this is called instead of requiring manual `update()` calls.
    get_renderable: Option<Arc<dyn Fn() -> Box<dyn Renderable + Send + Sync> + Send + Sync>>,
}

impl Live {
    pub fn new(renderable: Box<dyn Renderable + Send + Sync>) -> Self {
        Self::with_console(renderable, Console::new(), LiveOptions::default())
    }

    pub fn with_options(
        renderable: Box<dyn Renderable + Send + Sync>,
        options: LiveOptions,
    ) -> Self {
        Self::with_console(renderable, Console::new(), options)
    }

    pub fn with_console(
        renderable: Box<dyn Renderable + Send + Sync>,
        console: Console<Stdout>,
        options: LiveOptions,
    ) -> Self {
        assert!(
            options.refresh_per_second > 0.0,
            "refresh_per_second must be > 0"
        );

        let transient = if options.screen {
            true
        } else {
            options.transient
        };
        let options = LiveOptions {
            transient,
            ..options
        };
        let state = LiveState {
            options,
            started: false,
            live_id: None,
            is_root: false,
            alt_screen: false,
            pending_renderable: Some(renderable),
        };

        Live {
            console: Arc::new(Mutex::new(console)),
            state: Arc::new(Mutex::new(state)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            started_flag: Arc::new(AtomicBool::new(false)),
            refresh_thread: None,
            get_renderable: None,
        }
    }

    pub fn is_started(&self) -> bool {
        self.started_flag.load(Ordering::SeqCst)
    }

    /// Set a callback that provides the renderable on each refresh tick.
    ///
    /// When set, this function is called on each refresh to get the latest
    /// renderable to display, instead of requiring manual `update()` calls.
    pub fn with_get_renderable(
        mut self,
        f: impl Fn() -> Box<dyn Renderable + Send + Sync> + Send + Sync + 'static,
    ) -> Self {
        self.get_renderable = Some(Arc::new(f));
        self
    }

    pub(crate) fn started_flag(&self) -> Arc<AtomicBool> {
        self.started_flag.clone()
    }

    pub fn start(&mut self, refresh: bool) -> io::Result<()> {
        let mut state = self.state.lock().expect("live state mutex poisoned");
        if state.started {
            return Ok(());
        }

        let mut console = self.console.lock().expect("console mutex poisoned");
        sync_terminal_size(&mut console);

        let interactive = console.is_terminal() && !console.is_dumb_terminal();
        if !interactive {
            // Degrade gracefully in non-interactive or dumb terminals: don't attempt
            // cursor control, and render final output once on stop (if non-transient).
            state.started = true;
            state.live_id = None;
            state.is_root = false;
            state.alt_screen = false;
            self.started_flag.store(false, Ordering::SeqCst);
            return Ok(());
        }

        let renderable = state
            .pending_renderable
            .take()
            .unwrap_or_else(|| Box::new(Text::plain("")));

        let (id, is_root) = console.live_start(renderable, state.options.vertical_overflow);
        state.live_id = Some(id);
        state.is_root = is_root;
        state.started = true;
        self.started_flag.store(true, Ordering::SeqCst);

        if is_root {
            if state.options.screen {
                state.alt_screen = console.set_alt_screen(true)?;
            }
            let _ = console.show_cursor(false)?;
        }

        drop(console);
        let auto_refresh = state.options.auto_refresh;
        let is_root = state.is_root;
        drop(state);

        if refresh {
            self.refresh()?;
        }

        if auto_refresh && is_root {
            self.spawn_refresh_thread();
        }

        Ok(())
    }

    pub fn stop(&mut self) -> io::Result<()> {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.refresh_thread.take() {
            let _ = handle.join();
        }
        self.started_flag.store(false, Ordering::SeqCst);

        let mut state = self.state.lock().expect("live state mutex poisoned");
        if !state.started {
            return Ok(());
        }
        state.started = false;

        let id = state.live_id.take();
        let is_root = state.is_root;
        state.is_root = false;
        let alt_screen = state.alt_screen;
        state.alt_screen = false;
        let options = state.options.clone();

        let Some(id) = id else {
            // Non-interactive / dumb terminal path: render final output once if non-transient.
            if !options.transient {
                let renderable = state
                    .pending_renderable
                    .take()
                    .unwrap_or_else(|| Box::new(Text::plain("")));
                drop(state);
                let mut console = self.console.lock().expect("console mutex poisoned");
                let _ = console.print(renderable.as_ref(), None, None, None, false, "\n");
            }
            return Ok(());
        };

        let mut console = self.console.lock().expect("console mutex poisoned");

        // Nested Live stop behavior (Rich): remove from stack, optionally print final renderable.
        if !is_root {
            let renderable = console.live_stop(id);
            if !options.transient {
                if let Some(renderable) = renderable {
                    let _ = console.print(renderable.as_ref(), None, None, None, false, "\n");
                }
            } else if console.is_terminal() && !console.is_dumb_terminal() {
                // Ensure the nested entry disappears immediately.
                let _ = console.print(&Control::new(), None, None, None, false, "");
            }
            return Ok(());
        }

        // Best-effort final refresh (matches Rich's stop behavior for terminal output).
        if is_root && console.is_terminal() && !console.is_dumb_terminal() && !alt_screen {
            console.live_set_vertical_overflow(id, VerticalOverflowMethod::Visible);
            let _ = console.print(&Control::new(), None, None, None, false, "");
        }

        // Capture transient restore controls before clearing live state. Rich applies
        // restore after printing a newline, so the cursor starts below the live region.
        let restore_controls = if is_root
            && console.is_terminal()
            && !console.is_dumb_terminal()
            && options.transient
            && !alt_screen
        {
            console.live_restore_cursor()
        } else {
            crate::Segments::new()
        };

        // Unregister this live instance (root clears the full stack, like Rich).
        console.live_clear();

        // Root cleanup (cursor / screen / final newline).
        if is_root {
            if console.is_terminal() && !alt_screen {
                let _ = console.print(&Text::plain(""), None, None, None, false, "\n");
            }

            let _ = console.show_cursor(true);
            if alt_screen {
                let _ = console.set_alt_screen(false);
            }
        }

        if !restore_controls.is_empty() {
            let _ = console.print_segments(&restore_controls);
        }

        Ok(())
    }

    pub fn update(
        &self,
        renderable: Box<dyn Renderable + Send + Sync>,
        refresh: bool,
    ) -> io::Result<()> {
        let (id, started) = {
            let mut state = self.state.lock().expect("live state mutex poisoned");
            if !state.started {
                state.pending_renderable = Some(renderable);
                return Ok(());
            }
            if state.live_id.is_none() {
                // Non-interactive / dumb terminal path: just keep the latest renderable.
                state.pending_renderable = Some(renderable);
                return Ok(());
            }
            (state.live_id, state.started)
        };

        if started {
            if let Some(id) = id {
                let mut console = self.console.lock().expect("console mutex poisoned");
                console.live_update(id, renderable);
            }
        }
        if refresh {
            self.refresh()?;
        }
        Ok(())
    }

    pub fn refresh(&self) -> io::Result<()> {
        let state = self.state.lock().expect("live state mutex poisoned");
        if !state.started {
            return Ok(());
        }
        if state.live_id.is_none() {
            // Non-interactive / dumb terminal path: don't attempt cursor control.
            return Ok(());
        }
        drop(state);
        let mut console = self.console.lock().expect("console mutex poisoned");
        sync_terminal_size(&mut console);
        console.print(&Control::new(), None, None, None, false, "")
    }

    fn spawn_refresh_thread(&mut self) {
        if self.refresh_thread.is_some() {
            return;
        }

        self.stop_flag.store(false, Ordering::SeqCst);
        let stop_flag = self.stop_flag.clone();
        let console = self.console.clone();
        let state = self.state.clone();
        let get_renderable = self.get_renderable.clone();
        let refresh_per_second = state
            .lock()
            .expect("live state mutex poisoned")
            .options
            .refresh_per_second;

        let handle = thread::spawn(move || {
            let sleep = Duration::from_secs_f64(1.0 / refresh_per_second.max(0.001));
            while !stop_flag.load(Ordering::SeqCst) {
                thread::sleep(sleep);
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                let state_guard = match state.lock() {
                    Ok(g) => g,
                    Err(_) => break,
                };
                if !state_guard.started {
                    continue;
                }
                let live_id = state_guard.live_id;
                drop(state_guard);

                // If get_renderable is set, call it and update the live display.
                if let Some(ref get_renderable) = get_renderable {
                    if let Some(id) = live_id {
                        let renderable = get_renderable();
                        let mut console_guard = match console.lock() {
                            Ok(g) => g,
                            Err(_) => break,
                        };
                        console_guard.live_update(id, renderable);
                        sync_terminal_size(&mut console_guard);
                        let _ =
                            console_guard.print(&Control::new(), None, None, None, false, "");
                        continue;
                    }
                }

                let mut console_guard = match console.lock() {
                    Ok(g) => g,
                    Err(_) => break,
                };
                sync_terminal_size(&mut console_guard);
                let _ = console_guard.print(&Control::new(), None, None, None, false, "");
            }
        });

        self.refresh_thread = Some(handle);
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        // Best effort cleanup; ignore IO errors.
        let _ = self.stop();
    }
}

fn sync_terminal_size(console: &mut Console<Stdout>) {
    if !console.is_terminal() {
        return;
    }
    if let Ok((w, h)) = terminal::size() {
        let w = w as usize;
        let h = h as usize;
        let opts = console.options_mut();
        opts.size = (w, h);
        opts.max_width = w.max(1);
        opts.max_height = h;
    }
}
