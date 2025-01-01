# menu-rs
I originally wrote this menu system in QBASIC, see [here](https://ebruce613.prose.sh/my-dos-menu-system) for a write-up about that.
Now I have ported it to Rust. It retains all the original functionality, with the addition of sub-menus without having to call the executable in a subprocess. This is done by having the executable column be empty in the config file.
I have included two example config files.
## Keybindings
Up and down arrows to select, enter to run, escape to exit.
## Running
Just `cargo run` or run the binary in a working directory with a `menu.csv`. That's it.
