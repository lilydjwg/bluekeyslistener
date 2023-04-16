#![feature(exit_status_error)]

use std::fs;
use std::path::{PathBuf, Path};
use std::os::unix::ffi::OsStrExt;
use std::process::Command;
use std::sync::Arc;

use eyre::{Result, WrapErr, bail};
use tracing::{debug, info, warn, error};
use tracing_subscriber::EnvFilter;
use is_terminal::IsTerminal;

use input_linux::{EvdevHandle, KeyEvent, InputEvent, Key};
use input_linux::sys::{input_event, timeval};
use inotify::{Inotify, WatchMask};

mod config;

use config::Commands;

const INPUT_PATH: &str = "/dev/input";

fn get_device_name<P: AsRef<Path>>(path: P) -> Result<String> {
  let mut buf = [0u8; 1024];
  let file = fs::File::open(path.as_ref()).wrap_err("can't open device file")?;
  let ev = EvdevHandle::new(file);
  let len = ev.device_name_buf(&mut buf)?;
  Ok(std::str::from_utf8(&buf[..len - 1])
    .wrap_err("invalid UTF-8 device name")?.to_string())
}

fn get_dev_by_name(name: &str) -> Result<PathBuf> {
  let entries = fs::read_dir(INPUT_PATH).wrap_err("failed to read /dev/input")?;
  for dir in entries {
    if let Err(ref e) = dir {
      warn!("failed to read dir entry: {:#}", e);
      continue;
    }
    let path = dir.unwrap().path();
    if path.file_name().unwrap().as_bytes().starts_with(b"event") {
      match get_device_name(&path) {
        Ok(n) if n == name => { return Ok(path); },
        Ok(_) => { },
        Err(e) => {
          debug!("failed to get name for {}: {:#}", path.to_string_lossy(), e);
        }
      };
    }
  }
  bail!("{} not found", name);
}

fn listen_input(
  dev: PathBuf,
  commands: Arc<config::Commands>,
) -> Result<()> {
  let file = fs::File::open(&dev)?;
  let ev = EvdevHandle::new(file);
  ev.grab(true)?;

  std::thread::spawn(move || {
    let mut events = [input_event {
      time: timeval { tv_sec: 0, tv_usec: 0 },
      type_: 0, code: 0, value: 0,
    }; 4];
    loop {
      debug!("reading events from {}...", dev.display());
      let n = match ev.read(&mut events[..]) {
        Err(e) if e.raw_os_error() == Some(19) => break, // No such device
        Ok(0) => break,
        Ok(n) => n,
        Err(e) => {
          error!("Error while reading events: {:?}", e);
          break;
        }
      };
      debug!("read {} events: {:?}", n, events);
      for event in events {
        let ke = unsafe { KeyEvent::from_event(InputEvent::from_raw(&event).unwrap()) };
        if ke.value.is_pressed() {
          process_key(ke.key, &commands);
        }
      }
    }
  });

  Ok(())
}

fn process_key(key: Key, cmds: &Arc<Commands>) {
  info!("key {:?}", key);
  if let Some(cmd) = cmds.get(&key) {
    info!("running {}", cmd);
    match Command::new("sh").arg("-c").arg(cmd).status() {
      Ok(st) if st.success() => {},
      Ok(st) => warn!("cmd '{}' exited with {}", cmd, st),
      Err(e) => error!("failed to run cmd '{}': {:#}", cmd, e),
    }
  }
}

fn main() -> Result<()> {
  // default RUST_LOG=warn
  let filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::from("warn"));
  let fmt = tracing_subscriber::fmt::fmt()
    .with_writer(std::io::stderr)
    .with_env_filter(filter);
  if !std::io::stderr().is_terminal() {
    fmt.without_time().init();
  } else {
    fmt.init();
  }

  let arg = if let Some(arg) = std::env::args_os().nth(1) {
    arg
  } else {
    bail!("no config file given.")
  };

  let config_string = fs::read_to_string(arg)?;
  let conf: config::Config = config::Config::from_str(&config_string)?;

  for devname in &conf.devnames {
    if let Ok(dev) = get_dev_by_name(devname) {
      if let Err(e) = listen_input(dev, Arc::clone(&conf.commands)) {
        warn!("reading {} for key events error: {:#}", devname, e);
      }
    }
  }

  let mut inotify = Inotify::init()?;
  inotify.add_watch(INPUT_PATH, WatchMask::CREATE)?;

  let mut buffer = [0; 1024];
  loop {
    let events = inotify.read_events_blocking(&mut buffer)?;

    for event in events {
      match check_event(&event, &conf.devnames) {
        Ok(None) => { },
        Ok(Some((p, devname))) => {
          if let Err(e) = listen_input(p, Arc::clone(&conf.commands)) {
            warn!("reading {} for key events error: {:#}", devname, e);
          }
        },
        Err(e) => debug!("failed to check inotify event: {:#}", e),
      }
    }
  }
}

fn check_event<S: AsRef<Path>>(
  event: &inotify::Event<S>, devnames: &[String],
) -> Result<Option<(PathBuf, String)>> {
  let name = if let Some(n) = &event.name {
    n
  } else {
    return Ok(None)
  };

  let p = Path::new(INPUT_PATH).join(name);
  let devname = get_device_name_slow(&p)?;
  if devnames.contains(&devname) {
    Ok(Some((p, devname)))
  } else {
    Ok(None)
  }
}

fn get_device_name_slow<P: AsRef<Path>>(path: P) -> Result<String> {
  match get_device_name(&path) {
    Err(e) => {
      if let Some(err) = e.downcast_ref::<std::io::Error>() {
        if err.kind() == std::io::ErrorKind::PermissionDenied {
          std::thread::sleep(std::time::Duration::from_millis(1000));
          return get_device_name(&path)
        }
      }
      Err(e)
    }
    o => o,
  }
}
