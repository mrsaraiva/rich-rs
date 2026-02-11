use std::sync::{Arc, Mutex};

use rich_rs::Console;
use rich_tracing::RichTracingLayer;
use tracing_subscriber::prelude::*;

fn main() {
    let console = Arc::new(Mutex::new(Console::new()));

    let layer = RichTracingLayer::new(console).with_target(true);
    let subscriber = tracing_subscriber::registry().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(request_id = 7, route = "/health", "request accepted");
        tracing::warn!(
            service = "cache",
            "cache backend unavailable, using fallback"
        );
        tracing::error!(code = 500, "request failed");
    });
}
