//! peek - a fuzzy file finder with an inline preview.
//!
//! Non-interactive runs print a ranked list (or the best path with --print),
//! which makes the tool usable inside `$()`; `--interactive` opens a small
//! picker that needs a terminal.

mod fuzzy;
mod preview;
mod walk;

use std::env;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const USAGE: &str = "\
peek - fuzzy file finder with preview

Usage: peek [OPTIONS] [QUERY]

Options:
  --root <DIR>        Directory to search (default: .)
  --limit <N>         Maximum results to keep (default: 20)
  --height <N>        Preview lines (default: 12)
  --depth <N>         Maximum directory depth (default: 8)
  --print             Print only the best match path
  --no-preview        Do not render a preview pane
  -i, --interactive   Pick interactively, Enter prints the path
  -h, --help          Show this help
  -V, --version       Show the version

Exit codes:
  0    match found
  1    no match
  2    usage error
  130  interrupted
";

type Candidate = (String, PathBuf);

struct Opts {
    root: PathBuf,
    query: String,
    limit: usize,
    height: usize,
    depth: usize,
    print_only: bool,
    preview: bool,
    interactive: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("peek: {message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let opts = parse(env::args().skip(1))?;
    if opts.interactive {
        return interactive(&opts);
    }

    let pool = candidates(&opts);
    let hits = rank(&pool, &opts.query, opts.limit);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if opts.print_only {
        match hits.first() {
            Some((rel, _)) => {
                if let Err(err) = writeln!(out, "{rel}") {
                    return Ok(closed_pipe(err));
                }
                Ok(ExitCode::SUCCESS)
            }
            None => Ok(ExitCode::from(1)),
        }
    } else {
        for (rel, _) in &hits {
            if let Err(err) = writeln!(out, "{rel}") {
                return Ok(closed_pipe(err));
            }
        }
        Ok(if hits.is_empty() {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        })
    }
}

/// `peek | head` closes stdout early; that is a normal end, not a failure.
fn closed_pipe(err: io::Error) -> ExitCode {
    if err.kind() == io::ErrorKind::BrokenPipe {
        return ExitCode::SUCCESS;
    }
    eprintln!("peek: {err}");
    ExitCode::from(2)
}

fn parse(args: impl Iterator<Item = String>) -> Result<Opts, String> {
    let mut opts = Opts {
        root: PathBuf::from("."),
        query: String::new(),
        limit: 20,
        height: 12,
        depth: 8,
        print_only: false,
        preview: true,
        interactive: false,
    };
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("peek {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--root" => opts.root = PathBuf::from(next(&mut args, "--root")?),
            "--limit" => opts.limit = number(&mut args, "--limit")?,
            "--height" => opts.height = number(&mut args, "--height")?,
            "--depth" => opts.depth = number(&mut args, "--depth")?,
            "--print" => opts.print_only = true,
            "--no-preview" => opts.preview = false,
            "-i" | "--interactive" => opts.interactive = true,
            flag if flag.starts_with('-') && flag.len() > 1 => {
                return Err(format!("unknown option {flag}\n\n{USAGE}"))
            }
            word => {
                if !opts.query.is_empty() {
                    opts.query.push(' ');
                }
                opts.query.push_str(word);
            }
        }
    }

    if !opts.root.is_dir() {
        return Err(format!("{} is not a directory", opts.root.display()));
    }
    Ok(opts)
}

fn next(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{flag} needs a value"))
}

fn number(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<usize, String> {
    next(args, flag)?
        .parse()
        .map_err(|_| format!("{flag} needs a whole number"))
}

fn candidates(opts: &Opts) -> Vec<Candidate> {
    walk::files(&opts.root, opts.depth)
        .into_iter()
        .map(|path| {
            let rel = path
                .strip_prefix(&opts.root)
                .unwrap_or(&path)
                .display()
                .to_string();
            (rel, path)
        })
        .collect()
}

fn rank<'a>(pool: &'a [Candidate], query: &str, limit: usize) -> Vec<&'a Candidate> {
    let mut hits: Vec<(u32, &Candidate)> = pool
        .iter()
        .filter_map(|cand| fuzzy::score(query, &cand.0).map(|score| (score, cand)))
        .collect();
    hits.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1 .0.len().cmp(&b.1 .0.len()))
            .then_with(|| a.1 .0.cmp(&b.1 .0))
    });
    hits.into_iter().take(limit).map(|(_, cand)| cand).collect()
}

enum Key {
    Up,
    Down,
    Enter,
    Backspace,
    Char(char),
    Quit,
    Cancel,
}

/// Raw mode is delegated to `stty`, so no extra crate is needed.
struct Raw {
    active: bool,
}

impl Raw {
    fn enter() -> io::Result<Self> {
        Command::new("stty").args(["raw", "-echo"]).status()?;
        Ok(Self { active: true })
    }

    fn release(&mut self) {
        if self.active {
            let _ = Command::new("stty").arg("sane").status();
            self.active = false;
        }
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        self.release();
    }
}

fn interactive(opts: &Opts) -> Result<ExitCode, String> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("interactive mode needs a terminal".to_string());
    }
    let pool = candidates(opts);
    let width = env::var("COLUMNS")
        .ok()
        .and_then(|cols| cols.parse::<usize>().ok())
        .unwrap_or(80)
        .clamp(40, 200);

    let mut raw = Raw::enter().map_err(|e| e.to_string())?;
    let mut query = opts.query.clone();
    let mut selected = 0usize;

    loop {
        let hits = rank(&pool, &query, opts.limit);
        selected = selected.min(hits.len().saturating_sub(1));
        draw(&query, &hits, selected, opts, width).map_err(|e| e.to_string())?;

        match read_key().map_err(|e| e.to_string())? {
            Key::Up => selected = selected.saturating_sub(1),
            Key::Down if !hits.is_empty() => selected = (selected + 1).min(hits.len() - 1),
            Key::Down => {}
            Key::Backspace => {
                query.pop();
                selected = 0;
            }
            Key::Char(ch) => {
                query.push(ch);
                selected = 0;
            }
            Key::Enter => {
                raw.release();
                if let Some((rel, _)) = hits.get(selected) {
                    println!("{rel}");
                    return Ok(ExitCode::SUCCESS);
                }
                return Ok(ExitCode::from(1));
            }
            Key::Quit => {
                raw.release();
                return Ok(ExitCode::SUCCESS);
            }
            Key::Cancel => {
                raw.release();
                return Ok(ExitCode::from(130));
            }
        }
    }
}

fn read_key() -> io::Result<Key> {
    let mut byte = [0u8; 1];
    if io::stdin().read(&mut byte)? == 0 {
        return Ok(Key::Quit);
    }
    match byte[0] {
        0x03 => Ok(Key::Cancel),
        b'\r' | b'\n' => Ok(Key::Enter),
        0x7f | 0x08 => Ok(Key::Backspace),
        b'q' => Ok(Key::Quit),
        0x1b => {
            let mut rest = [0u8; 2];
            if io::stdin().read(&mut rest)? < 2 || rest[0] != b'[' {
                return Ok(Key::Quit);
            }
            Ok(match rest[1] {
                b'A' => Key::Up,
                b'B' => Key::Down,
                _ => Key::Quit,
            })
        }
        byte if byte.is_ascii_graphic() || byte == b' ' => Ok(Key::Char(byte as char)),
        _ => Ok(Key::Char(' ')),
    }
}

fn draw(
    query: &str,
    hits: &[&Candidate],
    selected: usize,
    opts: &Opts,
    width: usize,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "\x1b[2J\x1b[H")?;
    writeln!(out, "peek> {query}  [{} match(es)]", hits.len())?;

    if hits.is_empty() {
        writeln!(out, "  (no match)")?;
    }
    for (index, (rel, _)) in hits.iter().enumerate() {
        let line = shrink(rel, width - 2);
        if index == selected {
            writeln!(out, "\x1b[7m> {line}\x1b[0m")?;
        } else {
            writeln!(out, "  {line}")?;
        }
    }

    if opts.preview {
        writeln!(out, "{}", "-".repeat(width))?;
        match hits.get(selected) {
            None => writeln!(out, "(nothing selected)")?,
            Some((_, path)) => match preview::load(path, opts.height, width) {
                preview::Preview::Lines(lines) if lines.is_empty() => writeln!(out, "(empty)")?,
                preview::Preview::Lines(lines) => {
                    for line in lines {
                        writeln!(out, "{line}")?;
                    }
                }
                preview::Preview::Binary => writeln!(out, "(binary file)")?,
                preview::Preview::Unreadable => writeln!(out, "(unreadable)")?,
            },
        }
    }

    write!(out, "up/down move, enter pick, esc cancel, q quit")?;
    out.flush()
}

fn shrink(text: &str, width: usize) -> String {
    let width = width.max(4);
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut kept: String = text.chars().skip(text.chars().count() - (width - 1)).collect();
    kept.insert(0, '~');
    kept
}
