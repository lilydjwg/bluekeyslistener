use std::collections::HashMap;
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;

use serde::Deserialize;
use input_linux::Key;
use eyre::format_err;

pub type Commands = HashMap<Key, String>;

#[derive(Deserialize)]
struct ConfigStr<'a> {
  devnames: Vec<String>,
  commands: HashMap<Cow<'a, str>, Cow<'a, str>>,
}

pub struct Config {
  pub devnames: Vec<String>,
  pub commands: Arc<Commands>,
}

impl Config {
  pub fn from_str(s: &str) -> Result<Self, eyre::Report> {
    let ss: ConfigStr = toml::from_str(s)?;
    let commands: Result<Commands, _> = ss.commands.into_iter()
      .map(|(k, v)| match Key::from_str(&k) {
        Ok(k) => Ok((k, v.into_owned())),
        Err(_) => Err(format_err!("cannot parse '{}' as a Key", k)),
      })
      .collect();

    Ok(Self {
      devnames: ss.devnames,
      commands: Arc::new(commands?),
    })
  }
}

