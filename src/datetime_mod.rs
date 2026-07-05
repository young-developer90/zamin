use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::gc::*;

pub fn build_datetime() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("now".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.now>".to_string(),
        func: Rc::new(|_, ctx| {
            let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            Ok(make_datetime_dict(ctx.heap, dur.as_secs() as i64))
        }),
    })));

    funcs.push(("from_unix".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.from_unix>".to_string(),
        func: Rc::new(|args, ctx| {
            let ts = to_i64(&args[0])?;
            Ok(make_datetime_dict(ctx.heap, ts))
        }),
    })));

    funcs.push(("format".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.format>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("datetime.format requires a datetime dict and format string".to_string()); }
            let dt = &args[0];
            let fmt = args[1].to_string(ctx.heap);
            let year = get_dict_int(dt, ctx.heap, "year")?;
            let month = get_dict_int(dt, ctx.heap, "month")?;
            let day = get_dict_int(dt, ctx.heap, "day")?;
            let hour = get_dict_int(dt, ctx.heap, "hour")?;
            let minute = get_dict_int(dt, ctx.heap, "minute")?;
            let second = get_dict_int(dt, ctx.heap, "second")?;
            let mut result = fmt;
            result = result.replace("%Y", &format!("{:04}", year));
            result = result.replace("%y", &format!("{:02}", year % 100));
            result = result.replace("%m", &format!("{:02}", month));
            result = result.replace("%d", &format!("{:02}", day));
            result = result.replace("%H", &format!("{:02}", hour));
            result = result.replace("%M", &format!("{:02}", minute));
            result = result.replace("%S", &format!("{:02}", second));
            Ok(make_string(ctx.heap, &result))
        }),
    })));

    funcs.push(("parse".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.parse>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("datetime.parse requires a date string and format string".to_string()); }
            let s = args[0].to_string(ctx.heap);
            let fmt = args[1].to_string(ctx.heap);
            let mut year = 0i64; let mut month = 1i64; let mut day = 1i64;
            let mut hour = 0i64; let mut minute = 0i64; let mut second = 0i64;
            let mut chars = s.chars();
            let mut fmt_chars = fmt.chars();
            loop {
                match (fmt_chars.next(), chars.next()) {
                    (Some('%'), Some(_)) => {
                        match fmt_chars.next() {
                            Some('Y') => { year = parse_num(&mut chars, 4); }
                            Some('y') => { year = 2000 + parse_num(&mut chars, 2); }
                            Some('m') => { month = parse_num(&mut chars, 2); }
                            Some('d') => { day = parse_num(&mut chars, 2); }
                            Some('H') => { hour = parse_num(&mut chars, 2); }
                            Some('M') => { minute = parse_num(&mut chars, 2); }
                            Some('S') => { second = parse_num(&mut chars, 2); }
                            _ => {}
                        }
                    }
                    (Some(fc), Some(sc)) => {
                        if fc != sc { break; }
                    }
                    (None, _) | (_, None) => break,
                }
            }
            let unix = date_to_unix(year, month, day, hour, minute, second);
            let mut entries = Vec::new();
            entries.push((make_string(ctx.heap, "year"), Value::Int(year)));
            entries.push((make_string(ctx.heap, "month"), Value::Int(month)));
            entries.push((make_string(ctx.heap, "day"), Value::Int(day)));
            entries.push((make_string(ctx.heap, "hour"), Value::Int(hour)));
            entries.push((make_string(ctx.heap, "minute"), Value::Int(minute)));
            entries.push((make_string(ctx.heap, "second"), Value::Int(second)));
            entries.push((make_string(ctx.heap, "unix"), Value::Int(unix)));
            Ok(make_dict(ctx.heap, entries))
        }),
    })));

    funcs.push(("unix".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.unix>".to_string(),
        func: Rc::new(|args, ctx| {
            let dt = &args[0];
            if let Value::Dict(r) = dt {
                let entries = match ctx.heap.get(*r) { GcObj::Dict(e) => e, _ => return Err("not a dict".to_string()) };
                for (k, v) in entries {
                    if let GcObj::String(s) = ctx.heap.get(match k { Value::String(r) => *r, _ => continue }) {
                        if s == "unix" { return Ok(v.clone()); }
                    }
                }
            }
            Err("datetime.unix: invalid datetime dict".to_string())
        }),
    })));

    funcs
}

fn make_datetime_dict(heap: &mut GcHeap, unix_ts: i64) -> Value {
    let (year, month, day, hour, minute, second) = unix_to_date(unix_ts);
    let mut entries = Vec::new();
    entries.push((make_string(heap, "year"), Value::Int(year)));
    entries.push((make_string(heap, "month"), Value::Int(month)));
    entries.push((make_string(heap, "day"), Value::Int(day)));
    entries.push((make_string(heap, "hour"), Value::Int(hour)));
    entries.push((make_string(heap, "minute"), Value::Int(minute)));
    entries.push((make_string(heap, "second"), Value::Int(second)));
    entries.push((make_string(heap, "unix"), Value::Int(unix_ts)));
    make_dict(heap, entries)
}

fn unix_to_date(ts: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = ts / 86400;
    let time_secs = ts % 86400;
    let hour = time_secs / 3600;
    let minute = (time_secs % 3600) / 60;
    let second = time_secs % 60;

    let mut y = 1970i64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year { break; }
        remaining -= days_in_year;
        y += 1;
    }

    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0i64;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md { m = i as i64 + 1; break; }
        remaining -= md;
    }
    let day = remaining + 1;

    (y, m, day, hour, minute, second)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn date_to_unix(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> i64 {
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut total_days = 0i64;
    for y in 1970..year {
        total_days += if is_leap(y) { 366 } else { 365 };
    }
    for m in 0..(month - 1) as usize {
        total_days += month_days[m];
        if m == 1 && is_leap(year) { total_days += 1; }
    }
    total_days += day - 1;
    total_days * 86400 + hour * 3600 + minute * 60 + second
}

fn parse_num(chars: &mut std::str::Chars, count: usize) -> i64 {
    let mut n = 0i64;
    for _ in 0..count {
        match chars.next() {
            Some(c) if c.is_ascii_digit() => n = n * 10 + (c as i64 - '0' as i64),
            _ => break,
        }
    }
    n
}

fn get_dict_int(val: &Value, heap: &GcHeap, key: &str) -> Result<i64, String> {
    match val {
        Value::Dict(r) => {
            let entries = match heap.get(*r) { GcObj::Dict(e) => e, _ => return Err("not a dict".to_string()) };
            for (k, v) in entries {
                if let GcObj::String(s) = heap.get(match k { Value::String(r) => *r, _ => continue }) {
                    if s == key {
                        return to_i64(v);
                    }
                }
            }
            Err(format!("datetime dict has no key '{}'", key))
        }
        _ => Err("expected datetime dict".to_string()),
    }
}
