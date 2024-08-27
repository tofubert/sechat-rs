# Sechat-rs

## Setup

* Copy the Example.config file and add your general information. You also need to set your default room.
* run "cargo r" or "sechat-rs" and enjoy
* If no config is found a default config will be created, which you can fill in.
* a "-c" Option for console exists, if none is proveded it will default to XDG default paths. 
* Logs will be written to "dev.log". This is so we dont write log output into the ncurses UI.

## Logs
Logs will stored in the related XDG data dir. 
You can suppress both app log output and json dumping of failed http requests through the config.
The chat history goes into the data dir as well.

## Controls

### Screens
There is different screen to move around sechat-rs:
#### Reading/Editing
This is the main screen to view a Chat and write Messages.
To switch to Editing use "e" or "i". To switch back to Reading use "ESC".
Sending Messages is done via "Enter", which also switches back to Reading.

#### Opening
When in Reading Mode Press "o" to enter the Openening screen.
Use the Arrow keys to select a Room. Use "Enter" to open the Room. Once Enter is pressed the Cleint fetches new messages for the Room, hence a short delay might ocure.
Use "Esc" to exit back to the current chat.

#### Exiting
When in Reading Mode Press "q" to enter the Quiting Screen, confirm with "y" or abort with "n".
On Exit all log files are written to the folder choosen in the config file.

#### Help
Use "?" to get to the help screen.

## Bugs and Todos
Please open issues in the issue tracker.
A list of outstanding freatures is also kept there.

