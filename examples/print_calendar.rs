//! Print Calendar example: displays a full year calendar.
//!
//! This is a port of Python Rich's examples/print_calendar.py
//!
//! Run with:
//!   cargo run --example print_calendar 2026
//!
//! Or for current year:
//!   cargo run --example print_calendar $(date +%Y)

use std::env;
use std::io::Stdout;
use std::time::{SystemTime, UNIX_EPOCH};

use rich_rs::r#box::SIMPLE_HEAVY;
use rich_rs::{
    Align, Column, Console, ConsoleOptions, JustifyMethod, Renderable, Rule, Segments, Style,
    Table, Text,
};

// ============================================================================
// Calendar utilities (since we don't have chrono)
// ============================================================================

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const DAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Check if a year is a leap year.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Get the number of days in a month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Get the day of the week for a given date (0=Monday, 6=Sunday).
/// Uses Zeller's congruence adapted for Monday=0.
fn day_of_week(year: i32, month: u32, day: u32) -> u32 {
    let mut y = year;
    let mut m = month as i32;

    if m < 3 {
        m += 12;
        y -= 1;
    }

    let q = day as i32;
    let k = y % 100;
    let j = y / 100;

    // Zeller's congruence for Gregorian calendar
    let h = (q + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 - 2 * j) % 7;

    // Convert from Zeller (0=Sat, 1=Sun, 2=Mon, ..., 6=Fri) to (0=Mon, ..., 6=Sun)
    ((h + 5) % 7) as u32
}

/// Generate a calendar matrix for a month (list of weeks, each week is 7 days).
/// Days are 0 for empty cells, otherwise the day number.
fn month_calendar(year: i32, month: u32) -> Vec<[u32; 7]> {
    let num_days = days_in_month(year, month);
    let first_weekday = day_of_week(year, month, 1);

    let mut weeks: Vec<[u32; 7]> = Vec::new();
    let mut current_week = [0u32; 7];
    let mut day = 1u32;

    // Fill in the first week
    for i in first_weekday..7 {
        if day <= num_days {
            current_week[i as usize] = day;
            day += 1;
        }
    }
    weeks.push(current_week);

    // Fill in remaining weeks
    while day <= num_days {
        current_week = [0u32; 7];
        for slot in &mut current_week {
            if day <= num_days {
                *slot = day;
                day += 1;
            }
        }
        weeks.push(current_week);
    }

    weeks
}

/// Get today's date as (day, month, year).
fn today() -> (u32, u32, i32) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let total_days = (now.as_secs() / 86400) as i64;

    // Calculate date from days since epoch (1970-01-01)
    // This is a simplified calculation
    let mut year = 1970;
    let mut days_remaining = total_days;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days_remaining < days_in_year {
            break;
        }
        days_remaining -= days_in_year;
        year += 1;
    }

    let mut month = 1u32;
    loop {
        let days_in_m = days_in_month(year, month) as i64;
        if days_remaining < days_in_m {
            break;
        }
        days_remaining -= days_in_m;
        month += 1;
    }

    let day = (days_remaining + 1) as u32;
    (day, month, year)
}

// ============================================================================
// Calendar rendering
// ============================================================================

/// A renderable for a single month's calendar table.
struct MonthCalendar {
    year: i32,
    month: u32,
    today: (u32, u32, i32),
}

impl MonthCalendar {
    fn new(year: i32, month: u32, today: (u32, u32, i32)) -> Self {
        Self { year, month, today }
    }
}

impl Renderable for MonthCalendar {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let title = format!("{} {}", MONTH_NAMES[(self.month - 1) as usize], self.year);

        let mut table = Table::new()
            .with_title(&title)
            .with_style(Style::parse("green").unwrap_or_default())
            .with_box(Some(SIMPLE_HEAVY))
            .with_padding(0, 0);

        // Add columns for each day of the week
        for day_name in &DAY_NAMES {
            let header = Text::plain(&day_name[..3]);
            table.add_column(
                Column::with_header(Box::new(header))
                    .justify(JustifyMethod::Right)
                    .min_width(3),
            );
        }

        // Generate the calendar
        let weeks = month_calendar(self.year, self.month);
        let today_tuple = self.today;

        for week in weeks {
            let mut cells: Vec<Box<dyn Renderable + Send + Sync>> = Vec::new();

            for (index, day) in week.iter().enumerate() {
                let day_str = if *day == 0 {
                    String::new()
                } else {
                    day.to_string()
                };

                let mut day_text =
                    Text::styled(&day_str, Style::parse("magenta").unwrap_or_default());

                // Weekend styling (Sat=5, Sun=6)
                if index >= 5 {
                    day_text = Text::styled(&day_str, Style::parse("blue").unwrap_or_default());
                }

                // Today styling
                if *day > 0
                    && *day == today_tuple.0
                    && self.month == today_tuple.1
                    && self.year == today_tuple.2
                {
                    day_text = Text::styled(
                        &day_str,
                        Style::parse("white on dark_red").unwrap_or_default(),
                    );
                }

                cells.push(Box::new(day_text));
            }

            table.add_row(rich_rs::table::Row::new(cells));
        }

        table.render(console, options)
    }
}

fn print_calendar(year: i32) {
    let today_date = today();

    // Create a table for each month
    let mut tables: Vec<Box<dyn Renderable + Send + Sync>> = Vec::new();

    for month in 1..=12 {
        let month_cal = MonthCalendar::new(year, month, today_date);
        let aligned = Align::center(Box::new(month_cal));
        tables.push(Box::new(aligned));
    }

    // Create console and columns layout
    let mut console = Console::new();

    // Create columns with the calendar tables
    let columns = rich_rs::Columns::new(tables)
        .with_padding(1)
        .with_expand(true);

    // Print year rule at top
    let top_rule = Rule::new()
        .with_title(year.to_string())
        .with_style(Style::parse("bold").unwrap_or_default());
    let _ = console.print(&top_rule, None, None, None, false, "");

    // Print empty line
    let _ = console.line(1);

    // Print the columns of calendars
    let _ = console.print(&columns, None, None, None, false, "\n");

    // Print year rule at bottom
    let bottom_rule = Rule::new()
        .with_title(year.to_string())
        .with_style(Style::parse("bold").unwrap_or_default());
    let _ = console.print(&bottom_rule, None, None, None, false, "");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let year: i32 = if args.len() > 1 {
        args[1].parse().unwrap_or_else(|_| {
            eprintln!("Invalid year: {}. Using current year.", args[1]);
            today().2
        })
    } else {
        // Default to current year
        today().2
    };

    print_calendar(year);
}
