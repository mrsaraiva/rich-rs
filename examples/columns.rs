//! Columns example - Display data in neat columns.
//!
//! This is a port of Python Rich's `examples/columns.py`.
//! Instead of fetching from a URL, we use static sample data.
//!
//! Run with: `cargo run --example columns`

use rich_rs::{Columns, Console, Panel, Renderable, Text};

/// Sample user data (simulates data from randomuser.me API).
struct User {
    first_name: &'static str,
    last_name: &'static str,
    country: &'static str,
}

/// Get content for display from a user.
fn get_content(user: &User) -> Text {
    let markup = format!(
        "[b]{} {}[/b]\n[yellow]{}",
        user.first_name, user.last_name, user.country
    );
    Text::from_markup(&markup, false).unwrap_or_else(|_| {
        Text::plain(&format!(
            "{} {}\n{}",
            user.first_name, user.last_name, user.country
        ))
    })
}

fn main() {
    // Static sample users (mimics data from randomuser.me)
    let users = vec![
        User {
            first_name: "Emma",
            last_name: "Watson",
            country: "United Kingdom",
        },
        User {
            first_name: "Liam",
            last_name: "Mueller",
            country: "Germany",
        },
        User {
            first_name: "Sophia",
            last_name: "Dubois",
            country: "France",
        },
        User {
            first_name: "Noah",
            last_name: "Svensson",
            country: "Sweden",
        },
        User {
            first_name: "Olivia",
            last_name: "Rossi",
            country: "Italy",
        },
        User {
            first_name: "James",
            last_name: "Smith",
            country: "Australia",
        },
        User {
            first_name: "Mia",
            last_name: "Tanaka",
            country: "Japan",
        },
        User {
            first_name: "Lucas",
            last_name: "Santos",
            country: "Brazil",
        },
        User {
            first_name: "Amelia",
            last_name: "Garcia",
            country: "Spain",
        },
        User {
            first_name: "Benjamin",
            last_name: "Kim",
            country: "South Korea",
        },
    ];

    let mut console = Console::new();

    // Create panels for each user (like Python: [Panel(get_content(user), expand=True) for user in users])
    // Note: In Python Rich, expand=True means the panel expands to fill its column, not the whole width.
    // The Columns widget handles arranging panels into a grid.
    let user_renderables: Vec<Box<dyn Renderable + Send + Sync>> = users
        .iter()
        .map(|user| {
            let content = get_content(user);
            // Use Panel::fit to not expand to full terminal width - let Columns manage width
            Box::new(Panel::fit(Box::new(content))) as Box<dyn Renderable + Send + Sync>
        })
        .collect();

    // Display in columns
    let columns = Columns::new(user_renderables);

    let _ = console.print(&columns, None, None, None, false, "\n");
}
