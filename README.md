# Musicus

Musicus is a terminal music player like [cmus](https://github.com/cmus/cmus), written in the Rust Programming Language.
This project is still in early development but feel free to try it out.

## Issue Report
If you find a bug, I would be happy if you write a problem report. Since ncurses prevents the normal backtrace output, you can use the following command to generate a backtrace:
```bash
RUST_BACKTRACE=full cargo run --quiet --release 2> error_log.txt
```
