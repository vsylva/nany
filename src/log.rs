#![allow(dead_code)]

use std::{
    io::Write,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU8, Ordering},
    },
    time::Instant,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug,)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Level {
    pub const unsafe fn label(self,) -> &'static str {
        match self
        {
            Level::Error => "ERROR",
            Level::Warn => "WARN ",
            Level::Info => "INFO ",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    pub const unsafe fn ansi(self,) -> &'static str {
        match self
        {
            Level::Error => "\x1b[1;31m",
            Level::Warn => "\x1b[1;33m",
            Level::Info => "\x1b[1;32m",
            Level::Debug => "\x1b[0;37m",
            Level::Trace => "\x1b[0;90m",
        }
    }
}

const ANSI_IN_FILE: bool = true;
const ANSI_RESET: &str = "\x1b[0m";
const ANSI_TIME: &str = "\x1b[0;90m";
const ANSI_MODULE: &str = "\x1b[0;36m";
const ANSI_LOCATION: &str = "\x1b[0;90m";

const MIN_LEVEL_DEBUG: Level = Level::Debug;
const KEEP: usize = 1024;
const MODULE_WIDTH: usize = 12;

static LINES: Mutex<Vec<String,>,> = Mutex::new(Vec::new(),);
static SINK: OnceLock<Mutex<std::fs::File,>,> = OnceLock::new();
static START: OnceLock<Instant,> = OnceLock::new();
static PATH: OnceLock<String,> = OnceLock::new();
static LEVEL: AtomicU8 = AtomicU8::new(MIN_LEVEL_DEBUG as u8,);

pub const ENABLED: bool = cfg!(debug_assertions);

const unsafe fn logging() -> bool {
    cfg!(debug_assertions)
}

pub unsafe fn enabled() -> bool {
    logging()
}

pub unsafe fn set_level(lv: Level,) {
    LEVEL.store(lv as u8, Ordering::Relaxed,);
}

pub unsafe fn current_level() -> Level {
    match LEVEL.load(Ordering::Relaxed,)
    {
        1 => Level::Error,
        2 => Level::Warn,
        3 => Level::Info,
        4 => Level::Debug,
        _ => Level::Trace,
    }
}

pub unsafe fn init(tag: &str,) {
    if !logging()
    {
        return;
    }
    let own = crate::win::own_module_path();
    let stem = {
        let s = crate::win::file_stem(&own,);
        if s.is_empty() { String::from("nany",) } else { s }
    };
    let name = format!("{stem}.log");
    let mut candidates: Vec<std::path::PathBuf,> = Vec::new();
    if let Some(dir,) = std::path::Path::new(&own,).parent()
    {
        if !own.is_empty()
        {
            candidates.push(dir.join(&name,),);
        }
    }
    if let Some(dir,) = crate::win::exe_dir()
    {
        let p = dir.join(&name,);
        if !candidates.contains(&p,)
        {
            candidates.push(p,);
        }
    }

    let pid = std::process::id();
    let mut chosen = None;
    for path in candidates
    {
        let mut append = false;
        if let Ok(existing,) = std::fs::read(&path,)
        {
            let head = &existing[..existing.len().min(256 * 1024,)];
            append = String::from_utf8_lossy(head,).contains(&format!("pid={pid}"),);
        }
        if !append
        {
            let _ = std::fs::File::create(&path,);
        }
        match std::fs::OpenOptions::new().create(true,).append(true,).open(&path,)
        {
            Ok(mut f,) =>
            {
                if let Ok(g,) = LINES.lock()
                {
                    for line in g.iter()
                    {
                        let _ = writeln!(f, "{line}");
                    }
                }
                let _ = SINK.set(Mutex::new(f,),);
                chosen = Some(path,);
                break;
            }
            Err(_,) => continue,
        }
    }
    if let Some(p,) = &chosen
    {
        let _ = PATH.set(p.to_string_lossy().into_owned(),);
    }
    START.get_or_init(Instant::now,);
    log_at(Level::Info, "log", "log.rs", line!(), &format!("session start tag={tag} pid={pid} file={}", path()),);
}

pub unsafe fn path() -> String {
    PATH.get().cloned().unwrap_or_default()
}

pub unsafe fn is_ready() -> bool {
    PATH.get().is_some()
}

pub unsafe fn elapsed_ms() -> u128 {
    START.get().map(|s| s.elapsed().as_millis(),).unwrap_or(0,)
}

unsafe fn stamp() -> (String, String,) {
    let start = START.get_or_init(Instant::now,);
    let ms = start.elapsed().as_millis();
    let clock =
        format!("{:02}:{:02}:{:02}.{:03}", (ms / 3_600_000) % 24, (ms / 60_000) % 60, (ms / 1000) % 60, ms % 1000);
    let since = format!("+{}.{:03}s", ms / 1000, ms % 1000);
    (clock, since,)
}

unsafe fn short_file(file: &str,) -> &str {
    let bytes = file.as_bytes();
    let mut start = 0usize;
    for (i, b,) in bytes.iter().enumerate()
    {
        if *b == b'/' || *b == b'\\'
        {
            start = i + 1;
        }
    }
    &file[start..]
}

pub unsafe fn log_at(level: Level, module: &str, file: &str, line: u32, msg: &str,) {
    if !logging() || level > current_level()
    {
        return;
    }
    let (clock, since,) = stamp();
    let file = short_file(file,);
    let plain = format!(
        "{clock} {since} {lvl} {module:<width$} {msg}  {file}:{line}",
        clock = clock,
        since = since,
        lvl = level.label(),
        module = module,
        width = MODULE_WIDTH,
        msg = msg,
        file = file,
        line = line
    );
    let colored = format!(
        "{t}{clock}{r} {t}{since}{r} {c}{lvl}{r} {m}{module:<width$}{r} {msg}  {l}{file}:{line}{r}",
        t = ANSI_TIME,
        r = ANSI_RESET,
        c = level.ansi(),
        m = ANSI_MODULE,
        l = ANSI_LOCATION,
        clock = clock,
        since = since,
        lvl = level.label(),
        module = module,
        width = MODULE_WIDTH,
        msg = msg,
        file = file,
        line = line
    );
    {
        let Ok(mut g,) = LINES.lock()
        else
        {
            return;
        };
        g.push(plain.clone(),);
        if g.len() > KEEP
        {
            let drop = g.len() - KEEP;
            g.drain(..drop,);
        }
    }
    if let Some(sink,) = SINK.get()
    {
        if let Ok(mut f,) = sink.lock()
        {
            let line = if ANSI_IN_FILE { &colored } else { &plain };
            let _ = writeln!(f, "{line}");
        }
    }
}

pub unsafe fn emit_error(module: &str, msg: &str,) {
    log_at(Level::Error, module, "log.rs", 0, msg,);
}

pub unsafe fn emit_warn(module: &str, msg: &str,) {
    log_at(Level::Warn, module, "log.rs", 0, msg,);
}

pub unsafe fn emit_info(module: &str, msg: &str,) {
    log_at(Level::Info, module, "log.rs", 0, msg,);
}

pub unsafe fn emit_debug(module: &str, msg: &str,) {
    log_at(Level::Debug, module, "log.rs", 0, msg,);
}

pub unsafe fn emit_trace(module: &str, msg: &str,) {
    log_at(Level::Trace, module, "log.rs", 0, msg,);
}

pub unsafe fn log(msg: &str,) {
    emit_info("core", msg,);
}

pub unsafe fn flush() {
    if !logging()
    {
        return;
    }
    if let Some(sink,) = SINK.get()
    {
        if let Ok(mut f,) = sink.lock()
        {
            let _ = f.flush();
        }
    }
}

pub unsafe fn recent(n: usize,) -> Vec<String,> {
    let Ok(g,) = LINES.lock()
    else
    {
        return Vec::new();
    };
    let start = g.len().saturating_sub(n,);
    g[start..].to_vec()
}

pub unsafe fn strip_ansi(s: &str,) -> String {
    let mut out = String::with_capacity(s.len(),);
    let mut chars = s.chars().peekable();
    while let Some(c,) = chars.next()
    {
        if c == '\u{1b}'
        {
            if chars.peek() == Some(&'[',)
            {
                chars.next();
                while let Some(&n,) = chars.peek()
                {
                    chars.next();
                    if n.is_ascii_alphabetic()
                    {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c,);
    }
    out
}

pub unsafe fn recent_plain(n: usize,) -> Vec<String,> {
    recent(n,).into_iter().map(|l| strip_ansi(&l,),).collect()
}

pub unsafe fn fields(pairs: &[(&str, &dyn std::fmt::Display,)],) -> String {
    let mut out = String::new();
    for (i, (k, v,),) in pairs.iter().enumerate()
    {
        if i > 0
        {
            out.push(' ',);
        }
        out.push_str(k,);
        out.push('=',);
        out.push_str(&v.to_string(),);
    }
    out
}

#[macro_export]
macro_rules! log_error {
    ($m:expr, $fmt:literal $($arg:tt)*) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Error, $m, file!(), line!(), &format!($fmt $($arg)*))
        }
    };
    ($m:expr, $msg:expr) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Error, $m, file!(), line!(), &$msg)
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($m:expr, $fmt:literal $($arg:tt)*) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Warn, $m, file!(), line!(), &format!($fmt $($arg)*))
        }
    };
    ($m:expr, $msg:expr) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Warn, $m, file!(), line!(), &$msg)
        }
    };
}

#[macro_export]
macro_rules! log_info {
    ($m:expr, $fmt:literal $($arg:tt)*) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Info, $m, file!(), line!(), &format!($fmt $($arg)*))
        }
    };
    ($m:expr, $msg:expr) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Info, $m, file!(), line!(), &$msg)
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    ($m:expr, $fmt:literal $($arg:tt)*) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Debug, $m, file!(), line!(), &format!($fmt $($arg)*))
        }
    };
    ($m:expr, $msg:expr) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Debug, $m, file!(), line!(), &$msg)
        }
    };
}

#[macro_export]
macro_rules! log_trace {
    ($m:expr, $fmt:literal $($arg:tt)*) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Trace, $m, file!(), line!(), &format!($fmt $($arg)*))
        }
    };
    ($m:expr, $msg:expr) => {
        if $crate::log::ENABLED {
            $crate::log::log_at($crate::log::Level::Trace, $m, file!(), line!(), &$msg)
        }
    };
}
