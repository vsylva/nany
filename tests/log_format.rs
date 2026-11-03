use nany::log;

fn is_clock(s: &str,) -> bool {
    let b = s.as_bytes();
    b.len() == 12
        && [0, 1, 3, 4, 6, 7, 9, 10, 11,].iter().all(|i| b[*i].is_ascii_digit(),)
        && b[2] == b':'
        && b[5] == b':'
        && b[8] == b'.'
}

fn is_since(s: &str,) -> bool {
    let Some(rest,) = s.strip_prefix('+',)
    else
    {
        return false;
    };
    let Some(rest,) = rest.strip_suffix('s',)
    else
    {
        return false;
    };
    let mut parts = rest.split('.',);
    let (Some(sec,), Some(ms,), None,) = (parts.next(), parts.next(), parts.next(),)
    else
    {
        return false;
    };
    !sec.is_empty()
        && sec.bytes().all(|b| b.is_ascii_digit(),)
        && ms.len() == 3
        && ms.bytes().all(|b| b.is_ascii_digit(),)
}

fn is_level(s: &str,) -> bool {
    matches!(s, "ERROR" | "WARN " | "INFO " | "DEBUG" | "TRACE")
}

fn is_module(s: &str,) -> bool {
    let name = s.trim_end();
    !name.is_empty()
        && name.len() <= 12
        && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.' || b == b'-',)
}

fn is_location(s: &str,) -> bool {
    let Some((file, line,),) = s.rsplit_once(':',)
    else
    {
        return false;
    };
    !file.is_empty() && file.ends_with(".rs",) && !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit(),)
}

fn validate(line: &str,) -> Result<(), String,> {
    let (clock, rest,) = line.split_once(' ',).ok_or("clock",)?;
    if !is_clock(clock,)
    {
        return Err(format!("bad clock {clock:?}"),);
    }
    let (since, rest,) = rest.split_once(' ',).ok_or("elapsed",)?;
    if !is_since(since,)
    {
        return Err(format!("bad elapsed {since:?}"),);
    }
    if rest.len() < 6
    {
        return Err("short level field".into(),);
    }
    let (level, rest,) = rest.split_at(5,);
    if !is_level(level,)
    {
        return Err(format!("bad level {level:?}"),);
    }
    let rest = rest.strip_prefix(' ',).ok_or("level separator",)?;
    let split = rest.rfind("  ",).ok_or("missing message/location separator",)?;
    let head = &rest[..split];
    let location = rest[split..].trim_start();
    let module = head.split_whitespace().next().unwrap_or("",);
    if !is_module(module,)
    {
        return Err(format!("bad module {module:?}"),);
    }
    if head.trim() == module
    {
        return Err(format!("message missing in {head:?}"),);
    }
    if !is_location(location,)
    {
        return Err(format!("bad location {location:?}"),);
    }
    Ok((),)
}
#[test]
fn levels_have_fixed_five_column_labels() {
    unsafe {
        for (level, want,) in [
            (log::Level::Error, "ERROR",),
            (log::Level::Warn, "WARN ",),
            (log::Level::Info, "INFO ",),
            (log::Level::Debug, "DEBUG",),
            (log::Level::Trace, "TRACE",),
        ]
        {
            assert_eq!(level.label(), want);
            assert_eq!(level.label().len(), 5, "label width must stay stable");
        }
    }
}

#[test]
fn levels_order_from_error_to_trace() {
    assert!(log::Level::Error < log::Level::Warn);
    assert!(log::Level::Warn < log::Level::Info);
    assert!(log::Level::Info < log::Level::Debug);
    assert!(log::Level::Debug < log::Level::Trace);
}

#[test]
fn level_filter_hides_lower_priority_lines() {
    unsafe {
        let previous = log::current_level();
        log::set_level(log::Level::Warn,);
        assert_eq!(log::current_level(), log::Level::Warn);
        log::emit_info("test", "this line must be filtered out",);
        log::emit_warn("test", "this line must stay",);
        let recent = log::recent(64,);
        assert!(recent.iter().all(|l| !l.contains("filtered out")), "info leaked past a warn filter");
        assert!(recent.iter().any(|l| l.contains("this line must stay")));
        log::set_level(previous,);
    }
}

#[test]
fn recent_lines_are_plain_ascii_and_follow_the_format() {
    unsafe {
        log::emit_info("format", "size=1280x720 fmt=44 images=3",);
        log::emit_warn("format", "d3d11.dll not loaded, skipping d3d11 probe",);
        log::emit_error("format", "painter creation failed",);
        let recent = log::recent(8,);
        assert!(!recent.is_empty());
        for line in recent
        {
            assert!(!line.contains('\u{1b}'), "ansi escape leaked into the memory ring: {line:?}");
            assert!(line.is_ascii(), "log line is not ascii: {line:?}");
            if let Err(e,) = validate(&line,)
            {
                panic!("log line breaks the standard format ({e}): {line:?}");
            }
        }
    }
}

#[test]
fn fields_are_rendered_as_key_value_pairs() {
    unsafe {
        let size = 1280u32;
        let fmt = 44u32;
        let text = log::fields(&[("size", &size,), ("fmt", &fmt,), ("backend", &"vk",),],);
        assert_eq!(text, "size=1280 fmt=44 backend=vk");
    }
}

#[test]
fn every_source_log_message_is_ascii() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"),).join("src",);
    let mut checked = 0usize;
    for path in rust_files(&root,)
    {
        let text = std::fs::read_to_string(&path,).expect("read source",);
        for (idx, line,) in text.lines().enumerate()
        {
            if !(line.contains("log_info!(",)
                || line.contains("log_warn!(",)
                || line.contains("log_error!(",)
                || line.contains("log_debug!(",)
                || line.contains("log_trace!(",))
            {
                continue;
            }
            if line.bytes().any(|b| b >= 0x80,)
            {
                panic!("non-ascii log line at {}:{}: {}", path.display(), idx + 1, line.trim());
            }
            checked += 1;
        }
    }
    assert!(checked > 80, "expected many log call sites, saw {checked}");
}

fn rust_files(dir: &std::path::Path,) -> Vec<std::path::PathBuf,> {
    let mut out = Vec::new();
    let Ok(rd,) = std::fs::read_dir(dir,)
    else
    {
        return out;
    };
    for entry in rd.flatten()
    {
        let path = entry.path();
        if path.is_dir()
        {
            out.extend(rust_files(&path,),);
        }
        else if path.extension().and_then(|s| s.to_str(),) == Some("rs",)
        {
            out.push(path,);
        }
    }
    out
}
