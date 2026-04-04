use serde::Serialize;

use crate::error::AppError;
use crate::output::{Format, print_success_or};

#[derive(Serialize)]
struct UpdateResult {
    current_version: String,
    latest_version: String,
    status: String,
}

pub fn run(format: Format, check: bool) -> Result<(), AppError> {
    let current = env!("CARGO_PKG_VERSION");

    // Replace owner/repo with your GitHub repo.
    let updater = self_update::backends::github::Update::configure()
        .repo_owner("199-biotechnologies")
        .repo_name("agent-cli-framework")
        .bin_name("greeter")
        .current_version(current)
        .build()
        .map_err(|e| AppError::Update(e.to_string()))?;

    if check {
        let latest = updater
            .get_latest_release()
            .map_err(|e| AppError::Update(e.to_string()))?;
        let v = latest.version.trim_start_matches('v').to_string();
        let up_to_date = v == current;

        let result = UpdateResult {
            current_version: current.into(),
            latest_version: v.clone(),
            status: if up_to_date { "up_to_date".into() } else { "update_available".into() },
        };
        print_success_or(format, &result, |r| {
            if up_to_date {
                println!("Up to date (v{})", r.current_version);
            } else {
                println!("Update available: v{} -> v{}", r.current_version, r.latest_version);
                println!("Run `greeter update` to install");
            }
        });
    } else {
        let release = updater
            .update()
            .map_err(|e| AppError::Update(e.to_string()))?;
        let v = release.version().trim_start_matches('v').to_string();
        let up_to_date = v == current;

        let result = UpdateResult {
            current_version: current.into(),
            latest_version: v.clone(),
            status: if up_to_date { "up_to_date".into() } else { "updated".into() },
        };
        print_success_or(format, &result, |r| {
            if up_to_date {
                println!("Already up to date (v{})", r.current_version);
            } else {
                println!("Updated: v{} -> v{}", r.current_version, r.latest_version);
                println!("Run `greeter skill install` to update agent skills");
            }
        });
    }

    Ok(())
}
