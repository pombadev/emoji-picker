use std::{
    borrow::Cow,
    env,
    fs::{create_dir_all, File},
    io::BufReader,
    path::Path,
    sync::Arc,
};

use anyhow::{bail, Result};
use attohttpc;
use clipboard_ext::prelude::ClipboardProvider;
use clipboard_ext::x11_fork::ClipboardContext;
use serde::{Deserialize, Serialize};
use skim::{
    prelude::{unbounded, SkimItemReceiver, SkimItemSender, SkimOptionsBuilder},
    AnsiString, Skim, SkimItem,
};

pub struct CustomSkimItem {
    inner: String,
}

impl SkimItem for CustomSkimItem {
    fn display(&self) -> Cow<AnsiString> {
        Cow::Owned(self.inner.as_str().into())
    }

    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.inner)
    }

    fn output(&self) -> Cow<str> {
        let emoji = self.inner.split('\n').next();
        Cow::Owned(emoji.unwrap().to_string())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmojiContainer {
    emoji: String,
    description: String,
    category: String,
    aliases: Vec<String>,
    tags: Vec<String>,
}

pub fn run(data_set: Vec<EmojiContainer>) -> Result<()> {
    let options = SkimOptionsBuilder::default()
        .height(Some("70%"))
        .reverse(true)
        .multi(true)
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    for item in data_set {
        tx_item.send(Arc::new(CustomSkimItem {
            inner: format!("{}\n{}", item.emoji, item.description),
        }))?;
    }

    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let selected_items = Skim::run_with(&options, Some(rx_item))
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);

    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();

    let selected = selected_items.iter().fold(String::new(), |mut acc, curr| {
        acc.push_str(curr.output().as_ref());

        acc
    });

    match ctx.set_contents(selected) {
        Ok(_) => {}
        Err(err) => eprintln!("{}", err),
    };

    Ok(())
}

pub fn fetch_emoji() -> Result<Vec<EmojiContainer>> {
    let root_dir = Path::new(&env::var("HOME")?).join(".cache/emoji_picker");
    let file_path = root_dir.join("emoji.json");

    if file_path.exists() {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        Ok(serde_json::from_reader(reader)?)
    } else {
        println!("Fetching emoji data...");

        let resp =
            attohttpc::get("https://raw.githubusercontent.com/github/gemoji/master/db/emoji.json")
                .send()?;

        if resp.is_success() {
            let response: Vec<EmojiContainer> = resp.json()?;

            create_dir_all(root_dir)?;

            serde_json::to_writer(&File::create(file_path)?, &response)?;

            Ok(response)
        } else {
            if let Some(err) = resp.error_for_status().err() {
                bail!(err.to_string())
            } else {
                bail!("Unexpected error.")
            }
        }
    }
}
