use anyhow::{Context, Result};

mod picker;

fn main() -> Result<()> {
    let data_set = picker::fetch_emoji().context("Failed to fetch emoji data 😞")?;

    picker::run(data_set).context("Failed to run fuzzy finder")?;

    Ok(())
}
