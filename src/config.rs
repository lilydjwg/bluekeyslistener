use std::collections::HashMap;
use std::borrow::Cow;
use std::str::FromStr;

use serde::Deserialize;
use input_linux::Key;
use eyre::format_err;

pub type Commands = HashMap<Key, String>;

#[derive(Deserialize)]
struct ConfigStr<'a> {
  devname: String,
  commands: HashMap<Cow<'a, str>, Cow<'a, str>>,
}

pub struct Config {
  pub devname: String,
  pub commands: Commands,
}

impl Config {
  pub fn from_str(s: &str) -> Result<Self, eyre::Report> {
    let ss: ConfigStr = toml::from_str(s)?;
    let commands: Result<Commands, _> = ss.commands.into_iter()
      .map(|(k, v)| match Key::from_str(&k) {
        Ok(k) => Ok((k, v.into_owned())),
        Err(()) => Err(format_err!("cannot parse '{}' as a Key", k)),
      })
      .collect();

    Ok(Self {
      devname: ss.devname,
      commands: commands?,
    })
  }
}

