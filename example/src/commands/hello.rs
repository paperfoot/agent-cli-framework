use serde::Serialize;

use crate::error::AppError;
use crate::output::{Format, print_success_or};

#[derive(Serialize)]
struct Greeting {
    name: String,
    style: String,
    message: String,
}

pub fn run(format: Format, name: String, style: String) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::InvalidInput("name cannot be empty".into()));
    }

    let message = match style.as_str() {
        "friendly" => format!("Hey {name}, good to see you!"),
        "formal" => format!("Good day, {name}. A pleasure."),
        "pirate" => format!("Ahoy, {name}! Welcome aboard!"),
        other => format!("Hello, {name}! ({other} style)"),
    };

    let greeting = Greeting { name, style, message };

    print_success_or(format, &greeting, |g| {
        use owo_colors::OwoColorize;
        println!("{}", g.message.green());
    });

    Ok(())
}
