# peek

Fuzzy file finder for the terminal with an inline preview pane. Type a few
characters of a path, look at the file before you commit to opening it, and
press Enter to get the path back.

```
peek --root ~/src --interactive
```

![demo](docs/demo.gif)

*(Demo GIF placeholder; the recording is regenerated with `vhs docs/demo.tape`.)*

## Why

`find | grep` answers "does this path exist", not "which of these did I mean".
`peek` ranks by the shape of the match — consecutive characters, path
boundaries, upper case — and renders the top hit next to the list, so a wrong
pick costs one keystroke instead of an editor round trip.

Non-interactive output prints paths one per line, so it composes:

```sh
nvim "$(peek --print --root src --limit 1 handler)"
```

## Install

Rust 1.75 or newer:

```sh
cargo install --path .
```

Or build once and drop the binary on `$PATH`:

```sh
cargo build --release
install -m755 target/release/peek ~/.local/bin/peek
```

## Usage

```
peek [OPTIONS] [QUERY]
```

| Option | Meaning |
| --- | --- |
| `--root <DIR>` | directory to search, default `.` |
| `--limit <N>` | keep at most N results, default 20 |
| `--height <N>` | preview lines, default 12 |
| `--depth <N>` | maximum directory depth, default 8 |
| `--print` | print only the best match |
| `--no-preview` | skip the preview pane |
| `-i`, `--interactive` | pick with arrow keys, Enter prints |

In the picker: arrows move, `Enter` accepts, `Esc` cancels, `q` quits,
backspace edits the query. Matching is case insensitive and every query
character must appear in order.

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | at least one match |
| 1 | no match |
| 2 | usage error, or the root is not a directory |
| 130 | interrupted |

The exit codes are what make `peek --print` safe inside scripts: code 1 means
"nothing matched", not "an error to swallow".

### Skip rules

`.git`, `target`, `node_modules`, `dist`, `build`, `vendor`, `__pycache__` and
`.venv` are not descended into, and dotfiles are skipped. Preview reads at most
8 KiB, reports binary files instead of dumping them, and cuts long lines at the
terminal width.

### Completions

The generator lives in `contrib/completions/`; see the section in that
directory for the `peek.bash` and `_peek` snippets.

## Layout

```
src/main.rs     argument parsing, ranking glue, picker loop, exit codes
src/walk.rs     directory traversal and skip rules
src/fuzzy.rs    subsequence scoring
src/preview.rs  bounded preview rendering, binary detection
```

## License

MIT, see `LICENSE`.
