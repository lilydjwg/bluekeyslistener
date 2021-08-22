#![feature(exit_status_error)]

use std::fs;
use std::path::{PathBuf, Path};
use std::os::unix::ffi::OsStrExt;
use std::process::Command;

use eyre::{Result, WrapErr, bail};
use tracing::{debug, info, warn, error};
use tracing_subscriber::EnvFilter;

use input_linux::{EvdevHandle, KeyEvent, InputEvent, Key};
use input_linux::sys::{input_event, timeval};

mod config;

use config::Commands;

fn get_device_name<P: AsRef<Path>>(path: P) -> Result<String> {
  let mut buf = [0u8; 1024];
  let file = fs::File::open(path.as_ref()).wrap_err("can't open device file")?;
  let ev = EvdevHandle::new(file);
  let len = ev.device_name_buf(&mut buf)?;
  Ok(std::str::from_utf8(&buf[..len as usize - 1])
    .wrap_err("invalid UTF-8 device name")?.to_string())
}

fn get_dev_by_name(name: &str) -> Result<PathBuf> {
  let entries = fs::read_dir("/dev/input").wrap_err("failed to read /dev/input")?;
  for dir in entries {
    if let Err(ref e) = dir {
      warn!("failed to read dir entry: {:?}", e);
      continue;
    }
    let path = dir.unwrap().path();
    if path.file_name().unwrap().as_bytes().starts_with(b"event") {
      match get_device_name(&path) {
        Ok(n) if n == name => { return Ok(path); },
        Ok(_) => { },
        Err(e) => {
          warn!("failed to get name for {}: {:?}", path.to_string_lossy(), e);
        }
      };
    }
  }
  bail!("{} not found", name);
}

fn listen_input(dev: &Path, conf: config::Config) -> Result<()> {
  let file = fs::File::open(dev)?;
  let ev = EvdevHandle::new(file);
  Command::new("xinput").args(&["disable", &conf.devname]).status()?.exit_ok()?;
  let mut events = [input_event {
    time: timeval { tv_sec: 0, tv_usec: 0 },
    type_: 0, code: 0, value: 0,
  }; 4];
  loop {
    debug!("reading events...");
    let n = ev.read(&mut events[..])?;
    if n == 0 {
      break;
    }
    debug!("read {} events: {:?}", n, events);
    for event in events {
      let ke = unsafe { KeyEvent::from_event(InputEvent::from_raw(&event)?) };
      if ke.value.is_pressed() {
        process_key(ke.key, &conf.commands);
      }
    }
  }

  Ok(())
}

fn process_key(key: Key, cmds: &Commands) {
  info!("key {:?}", key);
  if let Some(cmd) = cmds.get(&key) {
    info!("running {}", cmd);
    match Command::new("sh").arg("-c").arg(cmd).status() {
      Ok(st) if st.success() => {},
      Ok(st) => warn!("cmd '{}' exited with {}", cmd, st),
      Err(e) => error!("failed to run cmd '{}': {:?}", cmd, e),
    }
  }
}

fn main() -> Result<()> {
  color_eyre::install()?;
  if std::env::var("RUST_LOG").is_err() {
    std::env::set_var("RUST_LOG", "warn")
  }
  tracing_subscriber::fmt::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

  let arg = if let Some(arg) = std::env::args_os().nth(1) {
    arg
  } else {
    bail!("no config file given.")
  };

  let config_string = fs::read_to_string(arg)?;
  let conf: config::Config = config::Config::from_str(&config_string)?;

  let dev = get_dev_by_name(&conf.devname)?;
  listen_input(&dev, conf)?;

  Ok(())
}
