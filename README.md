# Musicus

At the moment the main branch is buggy. Checkout [f5fc744](https://github.com/Bluemi/musicus_rs/commit/f5fc7445b923963e2d4660aee190f0282b7cf3f3) for a better version.

Musicus is a terminal music player like [cmus](https://github.com/cmus/cmus),
written in the [Rust Programming Language](https://www.rust-lang.org/learn).
This project is still in early development but feel free to try it out.


## Manual

### General Keys
- `q` or `Esc` always quits musicus immediately
- `c` pauses/unpauses
- `L` fast forwards five seconds and `H` rewinds five seconds
- `J` starts the next song
- `s` toggles shuffle mode
- `f` toggles follow mode (cursor follows current song in playlist)
- `+`/`-` increases/decreases volume

### Views
There are three **views** in Musicus.
1. File Browser
2. Playlist
3. Debug

You can switch between these views by pressing one of the buttons `1`, `2`, `3`.

---

#### File Browser View
The File Browser View enables you to search for music files in your Filesystem.
This view is inspired by [ranger](https://github.com/ranger/ranger). You can navigate
by pressing the buttons `h`, `j`, `k`, `l`.

- `h` goes left, upwards in the directory structure
- `j` goes down, jumping to the next directory in the current directory
- `k` goes up, jumping to the previous directory in the current directory
- `l` goes right, enters the selected directory

If you hover a music file you can play it immediately by pressing `Enter`.

With `y` you can add the currently selected file/directory to the playlist currently
shown in the **Playlist View**. If a file is selected, only this file is added to the
playlist (if it is a music file). If a directory is selected, all music files recursively
under this directory are added to the playlist.

With `n` you can create a new playlist. All music files under the current directory are
added to this playlist.

If you have text files that list music file paths (like cmus playlists),
you can import those by pressing `i`.

---

#### Playlist View
The Playlist View manages your playlists. Playlists are shown on the left side. Songs of the selected playlist are shown on the right.

- `h`, `l` switch between playlist selection and song selection
- `j`, `k` next/previous song/playlist
- `Enter` play selected song
- `F` moves the cursor to the currently played song
- `D` remove the selected song
- `O` tries to optimize the song titles by removing parts that occur in every title of a directory
- `y` copy selected song to clipboard
- `p` paste clipboard to playlist

---

#### Debug View
The last view is the Debug View. Here you can see logs of musicus.

You can scroll by pressing `j` or `k`.

## Participate in the project

### Issue Report
If you find a bug, I would appreciate it if you write an issue report.
Since ncurses prevents the normal backtrace output, you can use the following command
in a bash shell to generate a backtrace:
```bash
RUST_BACKTRACE=full cargo run 2> error_log.txt
```

In debug mode it takes some time to start the program and play songs, but the backtrace is more useful.
If it takes too long, add `--release` to the cargo command.

### Discussions
If you have a question or an idea, you are very welcome to express it in the
[discussions](https://github.com/Bluemi/musicus_rs/discussions) page of the repository.
