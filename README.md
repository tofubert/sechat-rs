# Sechat-rs


[![Status](https://github.com/tofubert/sechat-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/tofubert/sechat-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/sechat-rs.svg)](https://crates.io/crates/sechat-rs)
[![Documentation](https://docs.rs/sechat-rs/badge.svg)](https://docs.rs/sechat-rs/)
[![Codecov](https://codecov.io/github/tofubert/sechat-rs/branch/main/graph/badge.svg?token=CUBVLZ27KS)](https://codecov.io/github/tofubert/sechat-rs)
[![Dependency status](https://deps.rs/repo/github/tofubert/sechat-rs/status.svg)](https://deps.rs/repo/github/tofubert/sechat-rs)



> [!WARNING]
> This Software is not a fully stable client. You should not rely on this client for important chats!

## Setup

* run "cargo r" or "sechat-rs" and enjoy
* If no config is found a default config will be created, which you can fill in.
* a "-c" Option for console exists, if none is proveded it will default to XDG default paths.
* Logs will be written to "dev.log". This is so we dont write log output into the terminal UI.

## Logs
Logs will stored in the related XDG data dir.
You can suppress both app log output and json dumping of failed http requests through the config.
The chat history goes into the data dir as well.
Your full chat history is stored unencrypted on disk!

## Controls

### Screens
There is different screen to move around sechat-rs:
#### Reading/Editing
This is the main screen to view a Chat and write Messages.
To switch to Editing use "e" or "i". To switch back to Reading use "ESC".
Sending Messages is done via "Enter", which also switches back to Reading.

#### Opening
When in Reading Mode Press "o" to enter the Opening screen.
Use the Arrow keys to select a Room. Use "Enter" to open the Room. Once Enter is pressed the Client fetches new messages for the Room, hence a short delay might ocure.
Use "Esc" to exit back to the current chat.

#### Exiting
When in Reading Mode Press "q" to enter the Quitting Screen, confirm with "y" or abort with "n".
On Exit all log files are written to the folder chosen in the config file.

#### Help
Use "?" to get to the help screen.

## Bugs and Todos
Please open issues in the issue tracker.
A list of planned and requested freatures is also kept there.

## The Name
Originally intended to be called seshat, after the egyptian goddess of writing, a typo became sechat.
Thank to Sebastian for sugesting the name in the first place.

## Sponsors
Thanks to [emlix gmbh](https://github.com/emlix) for allowing [@tofu](https://github.com/tofubert) and other so spend some of their work time to tinker with this.
