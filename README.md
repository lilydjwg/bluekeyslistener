# What

Take action when you use your bluetooth headphone to indicate pause, play, next or previous. For Linux only.

# Why

You can bind global keys to XF86AudioPause, XF86AudioPlay, XF86AudioNext, XF86AudioPrev, but does it work if you lock your screen? Probably not.

You can listen to ACPI event with `acpid`, but then you'll need to forward events from a system service to your user session.

Or you can read the events in your session (e.g. as a user service) with this program.

# Setup

To give yourself permissions to read from the input device, put this in `/etc/udev/rules.d/66-headphones.rules` (the number in the filename should be lower than 70):

```
# give the user permissions to receive bluetooth headphone keys
SUBSYSTEM=="input", ATTRS{name}=="WH-1000XM2 (AVRCP)", TAG+="uaccess"
```

Where `WH-1000XM2 (AVRCP)` is the device name of your bluetooth headphone. Find out with the `evtest` program.

And then reload the rules:

```sh
sudo udevadm control --reload-rules
sudo udevadm trigger # or reconnect your headphone
```

# Configure

See `config.toml` and update as you like. The values of `commands` are run by the system shell (`/bin/sh`).

# Run

You need to run this program in your session (i.e. no `sudo`, no `su`).

```sh
cargo run --release config.toml
```

Or you can copy the built executable where you like and run that.
