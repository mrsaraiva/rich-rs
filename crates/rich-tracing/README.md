# rich-tracing

`rich-tracing` is a lightweight adapter crate that routes Rust logging events into `rich-rs` rendering.

It provides:

- `RichTracingLayer` (`tracing_subscriber::Layer`) for the `tracing` ecosystem
- `RichLogger` (`log::Log`) for the `log` ecosystem

Both adapters write through a shared `Arc<Mutex<rich_rs::Console<W>>>` and use `Console::log()` so output keeps Rich-style timestamped layout.

## Tracing quick start

```ignore
use std::sync::{Arc, Mutex};

use rich_rs::Console;
use rich_tracing::RichTracingLayer;
use tracing_subscriber::prelude::*;

let console = Arc::new(Mutex::new(Console::new()));
let layer = RichTracingLayer::new(console.clone()).with_target(true);
let subscriber = tracing_subscriber::registry().with(layer);

tracing::subscriber::with_default(subscriber, || {
    tracing::info!(request_id = 42, "request handled");
});
```

## Log quick start

```ignore
use std::sync::{Arc, Mutex};

use rich_rs::Console;
use rich_tracing::RichLogger;

let logger = RichLogger::new(Arc::new(Mutex::new(Console::new())))
    .with_target(true)
    .with_location(true);

// For app-wide global setup:
// rich_tracing::init_global_logger(logger, log::LevelFilter::Info)?;
```
