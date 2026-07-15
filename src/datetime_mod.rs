use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::gc::*;

fn push_int_2(s: &mut String, v: i64) {
    s.push((b'0' + (v / 10 % 10) as u8) as char);
    s.push((b'0' + (v % 10) as u8) as char);
}

fn push_int_4(s: &mut String, v: i64) {
    s.push((b'0' + (v / 1000) as u8) as char);
    s.push((b'0' + (v / 100 % 10) as u8) as char);
    s.push((b'0' + (v / 10 % 10) as u8) as char);
    s.push((b'0' + (v % 10) as u8) as char);
}

pub fn build_datetime(heap: &mut GcHeap) -> Vec<(String, Value)> {
    let key_year = make_string(heap, "year");
    let key_month = make_string(heap, "month");
    let key_day = make_string(heap, "day");
    let key_hour = make_string(heap, "hour");
    let key_minute = make_string(heap, "minute");
    let key_second = make_string(heap, "second");
    let key_unix = make_string(heap, "unix");
    heap.permanent_roots.push(key_year.clone());
    heap.permanent_roots.push(key_month.clone());
    heap.permanent_roots.push(key_day.clone());
    heap.permanent_roots.push(key_hour.clone());
    heap.permanent_roots.push(key_minute.clone());
    heap.permanent_roots.push(key_second.clone());
    heap.permanent_roots.push(key_unix.clone());

    let mut funcs = Vec::new();

    // pre-allocated dict pool so `now` can mutate in-place
    let pool_size = 64usize;
    let pool: Vec<Value> = (0..pool_size).map(|_| {
        datetime_dict_from_ts(heap, 0, &key_year, &key_month, &key_day,
            &key_hour, &key_minute, &key_second, &key_unix)
    }).collect();
    for v in &pool {
        heap.permanent_roots.push(v.clone());
    }
    let pool_idx = std::cell::Cell::new(0usize);

    let _ky1 = key_year.clone(); let _km1 = key_month.clone(); let _kd1 = key_day.clone();
    let _kh1 = key_hour.clone(); let _kmi1 = key_minute.clone(); let _ks1 = key_second.clone(); let _ku1 = key_unix.clone();
    funcs.push(("now".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.now>".to_string(),
        func: Rc::new(move |_, ctx| {
            let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let unix_ts = dur.as_secs() as i64;
            let (year, month, day, hour, minute, second) = unix_to_date(unix_ts);
            let idx = pool_idx.get();
            pool_idx.set((idx + 1) % pool_size);
            let val = &pool[idx];
            if let Value::Dict(r) = val {
                if let GcObj::Dict(ref mut entries) = ctx.heap.get_mut(*r) {
                    entries[0].1 = Value::Int(year);
                    entries[1].1 = Value::Int(month);
                    entries[2].1 = Value::Int(day);
                    entries[3].1 = Value::Int(hour);
                    entries[4].1 = Value::Int(minute);
                    entries[5].1 = Value::Int(second);
                    entries[6].1 = Value::Int(unix_ts);
                }
            }
            Ok(val.clone())
        }),
    })));

    let ky2 = key_year.clone(); let km2 = key_month.clone(); let kd2 = key_day.clone();
    let kh2 = key_hour.clone(); let kmi2 = key_minute.clone(); let ks2 = key_second.clone(); let ku2 = key_unix.clone();
    funcs.push(("from_unix".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.from_unix>".to_string(),
        func: Rc::new(move |args, ctx| {
            let ts = to_i64(&args[0])?;
            Ok(datetime_dict_from_ts(ctx.heap, ts,
                &ky2, &km2, &kd2, &kh2, &kmi2, &ks2, &ku2))
        }),
    })));

    funcs.push(("format".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.format>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("datetime.format requires a datetime dict and format string".to_string()); }
            let fmt = match &args[1] {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.as_str(),
                    _ => return Err("invalid format string".to_string()),
                },
                _ => return Err("expected string for format".to_string()),
            };
            let (year, month, day, hour, minute, second) = match &args[0] {
                Value::Dict(r) => {
                    let entries = match ctx.heap.get(*r) { GcObj::Dict(e) => e, _ => return Err("not a dict".to_string()) };
                    if entries.len() < 7 { return Err("datetime dict too short".to_string()); }
                    (
                        to_i64(&entries[0].1).unwrap_or(0),
                        to_i64(&entries[1].1).unwrap_or(0),
                        to_i64(&entries[2].1).unwrap_or(0),
                        to_i64(&entries[3].1).unwrap_or(0),
                        to_i64(&entries[4].1).unwrap_or(0),
                        to_i64(&entries[5].1).unwrap_or(0),
                    )
                }
                _ => return Err("expected datetime dict".to_string()),
            };
            let mut result = String::with_capacity(fmt.len());
            let mut chars = fmt.chars();
            while let Some(c) = chars.next() {
                if c == '%' {
                    match chars.next() {
                        Some('Y') => { push_int_4(&mut result, year); }
                        Some('y') => { push_int_2(&mut result, year % 100); }
                        Some('m') => { push_int_2(&mut result, month); }
                        Some('d') => { push_int_2(&mut result, day); }
                        Some('H') => { push_int_2(&mut result, hour); }
                        Some('M') => { push_int_2(&mut result, minute); }
                        Some('S') => { push_int_2(&mut result, second); }
                        Some(other) => { result.push('%'); result.push(other); }
                        None => { result.push('%'); }
                    }
                } else {
                    result.push(c);
                }
            }
            Ok(make_string_owned(ctx.heap, result))
        }),
    })));

    let ky3 = key_year.clone(); let km3 = key_month.clone(); let kd3 = key_day.clone();
    let kh3 = key_hour.clone(); let kmi3 = key_minute.clone(); let ks3 = key_second.clone(); let ku3 = key_unix.clone();
    funcs.push(("parse".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.parse>".to_string(),
        func: Rc::new(move |args, ctx| {
            if args.len() < 2 { return Err("datetime.parse requires a date string and format string".to_string()); }
            let s = get_str_owned(&args[0], ctx.heap)?;
            let fmt = get_str_owned(&args[1], ctx.heap)?;
            let mut year = 0i64; let mut month = 1i64; let mut day = 1i64;
            let mut hour = 0i64; let mut minute = 0i64; let mut second = 0i64;
            let mut peek_chars = s.chars().peekable();
            let mut fmt_iter = fmt.chars();
            loop {
                let fmt_next = fmt_iter.next();
                match fmt_next {
                    Some('%') => {
                        match fmt_iter.next() {
                            Some('Y') => { year = parse_num_peek(&mut peek_chars, 4); }
                            Some('y') => { year = 2000 + parse_num_peek(&mut peek_chars, 2); }
                            Some('m') => { month = parse_num_peek(&mut peek_chars, 2); }
                            Some('d') => { day = parse_num_peek(&mut peek_chars, 2); }
                            Some('H') => { hour = parse_num_peek(&mut peek_chars, 2); }
                            Some('M') => { minute = parse_num_peek(&mut peek_chars, 2); }
                            Some('S') => { second = parse_num_peek(&mut peek_chars, 2); }
                            _ => { peek_chars.next(); }
                        }
                    }
                    Some(fc) => {
                        match peek_chars.next() {
                            Some(sc) if fc == sc => continue,
                            _ => break,
                        }
                    }
                    None => break,
                }
            }
            let unix = date_to_unix(year, month, day, hour, minute, second);
            Ok(datetime_dict_from_ts(ctx.heap, unix,
                &ky3, &km3, &kd3, &kh3, &kmi3, &ks3, &ku3))
        }),
    })));

    funcs.push(("unix".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.unix>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("datetime.unix requires a datetime dict".to_string()); }
            let unix = match &args[0] {
                Value::Dict(r) => {
                    let entries = match ctx.heap.get(*r) { GcObj::Dict(e) => e, _ => return Err("not a dict".to_string()) };
                    if entries.len() < 7 { return Err("datetime dict too short".to_string()); }
                    to_i64(&entries[6].1).unwrap_or(0)
                }
                _ => return Err("expected datetime dict".to_string()),
            };
            Ok(Value::Int(unix))
        }),
    })));

    funcs
}

fn datetime_dict_from_ts(
    heap: &mut GcHeap, unix_ts: i64,
    key_year: &Value, key_month: &Value, key_day: &Value,
    key_hour: &Value, key_minute: &Value, key_second: &Value, key_unix: &Value,
) -> Value {
    let (year, month, day, hour, minute, second) = unix_to_date(unix_ts);
    let mut entries = Vec::with_capacity(7);
    entries.push((key_year.clone(), Value::Int(year)));
    entries.push((key_month.clone(), Value::Int(month)));
    entries.push((key_day.clone(), Value::Int(day)));
    entries.push((key_hour.clone(), Value::Int(hour)));
    entries.push((key_minute.clone(), Value::Int(minute)));
    entries.push((key_second.clone(), Value::Int(second)));
    entries.push((key_unix.clone(), Value::Int(unix_ts)));
    make_dict(heap, entries)
}

fn unix_to_date(ts: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = ts / 86400;
    let time_secs = ts % 86400;
    let hour = time_secs / 3600;
    let minute = (time_secs % 3600) / 60;
    let second = time_secs % 60;

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as i64, m as i64, d as i64, hour, minute, second)
}

fn date_to_unix(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> i64 {
    let (y, m) = if month <= 2 { (year - 1, month + 9) } else { (year, month - 3) };
    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    days * 86400 + hour * 3600 + minute * 60 + second
}

fn parse_num_peek(chars: &mut std::iter::Peekable<std::str::Chars>, count: usize) -> i64 {
    let mut n = 0i64;
    for _ in 0..count {
        match chars.peek() {
            Some(c) if c.is_ascii_digit() => {
                n = n * 10 + (*c as i64 - '0' as i64);
                chars.next();
            }
            _ => break,
        }
    }
    n
}
