#[cfg(feature = "parity-trace")]
use std::fs::{File, OpenOptions};
#[cfg(feature = "parity-trace")]
use std::io::Write;
#[cfg(feature = "parity-trace")]
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "parity-trace")]
static TRACE: OnceLock<Option<Mutex<File>>> = OnceLock::new();

#[cfg(feature = "parity-trace")]
fn output() -> Option<&'static Mutex<File>> {
    TRACE
        .get_or_init(|| {
            let path = std::env::var_os("RICH_FFI_PARITY_TRACE")?;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .ok()
                .map(Mutex::new)
        })
        .as_ref()
}

pub(crate) fn event(name: &str, width: Option<u32>) {
    #[cfg(feature = "parity-trace")]
    if let Some(output) = output() {
        if let Ok(mut output) = output.lock() {
            let thread = format!("{:?}", std::thread::current().id());
            let _ = match width {
                Some(width) => writeln!(
                    output,
                    "{{\"event\":\"{name}\",\"thread\":\"{thread}\",\"width\":{width}}}"
                ),
                None => writeln!(output, "{{\"event\":\"{name}\",\"thread\":\"{thread}\"}}"),
            };
        }
    }

    #[cfg(not(feature = "parity-trace"))]
    let _ = (name, width);
}
