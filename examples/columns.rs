//! Columns example - Display data in neat columns.
//!
//! This is a port of Python Rich's `examples/columns.py`.
//!
//! Run with: `cargo run --example columns`

use rich_rs::{Columns, Console, Json, OverflowMethod, Panel, Renderable, Text};
use serde_json::Value;

/// Get content for display from a user.
fn get_content(user: &Value) -> Text {
    let first = user
        .get("name")
        .and_then(|name| name.get("first"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
    let last = user
        .get("name")
        .and_then(|name| name.get("last"))
        .and_then(Value::as_str)
        .unwrap_or("User");
    let country = user
        .get("location")
        .and_then(|location| location.get("country"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown");

    let markup = format!("[b]{} {}[/b]\n[yellow]{}", first, last, country);
    Text::from_markup(&markup, false)
        .unwrap_or_else(|_| Text::plain(&format!("{first} {last}\n{country}")))
}

fn fetch_users(results: usize) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let url = format!("https://randomuser.me/api/?results={results}");
    let response = ureq::get(&url).call()?;
    let body = response.into_body().read_to_string()?;
    let root: Value = serde_json::from_str(&body)?;
    let users = root
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(users)
}

fn main() {
    let mut console = Console::new();
    let users = match fetch_users(30) {
        Ok(users) => users,
        Err(err) => {
            eprintln!("Failed to fetch users from randomuser.me: {err}");
            std::process::exit(1);
        }
    };

    // Match Python example flow: print fetched user structures first.
    let users_payload = serde_json::to_string_pretty(&users).unwrap_or_else(|_| "[]".to_string());
    let users_json = Json::new(&users_payload, 2, true, false);
    let _ = console.print(
        &users_json,
        None,
        None,
        Some(OverflowMethod::Ignore),
        true,
        "\n",
    );

    // Create panels for each user.
    // NOTE: Python Rich supports Panel(expand=True) inside Columns while still
    // negotiating per-column widths. rich-rs currently measures expanding panels
    // against terminal width, so use Panel::fit here to preserve a multi-column layout.
    let user_renderables: Vec<Box<dyn Renderable + Send + Sync>> = users
        .iter()
        .map(|user| {
            let content = get_content(user);
            Box::new(Panel::fit(Box::new(content))) as Box<dyn Renderable + Send + Sync>
        })
        .collect();

    // Display in columns
    let columns = Columns::new(user_renderables);

    let _ = console.print(&columns, None, None, None, false, "\n");
}
