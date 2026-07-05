use std::fmt::Write;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::gc::*;

pub fn build_datetime(heap: &mut GcHeap) -> Vec<(String, Value)> {
    let key_year = make_string(heap, "year");
    let key_month = make_string(heap, "month");
    let key_day = make_string(heap, "day");
    let key_hour = make_string(heap, "hour");
    let key_minute = make_string(heap, "minute");
    let key_second = make_string(heap, "second");
    let key_unix = make_string(heap, "unix");

    let mut funcs = Vec::new();

    let ky1 = key_year.clone(); let km1 = key_month.clone(); let kd1 = key_day.clone();
    let kh1 = key_hour.clone(); let kmi1 = key_minute.clone(); let ks1 = key_second.clone(); let ku1 = key_unix.clone();
    funcs.push(("now".to_string(), Value::NativeFunc(NativeFunc {
        name: "<datetime.now>".to_string(),
        func: Rc::new(move |_, ctx| {
            let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            Ok(datetime_dict_from_ts(ctx.heap, dur.as_secs() as i64,
                &ky1, &km1, &kd1, &kh1, &kmi1, &ks1, &ku1))
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
            let dt = &args[0];
            let fmt = args[1].to_string(ctx.heap);
            let (year, month, day, hour, minute, second, _) = extract_datetime_fields(dt, ctx.heap)?;
            let mut result = String::with_capacity(fmt.len());
            let mut chars = fmt.chars();
            while let Some(c) = chars.next() {
                if c == '%' {
                    match chars.next() {
                        Some('Y') => { write!(&mut result, "{:04}", year).unwrap(); }
                        Some('y') => { write!(&mut result, "{:02}", year % 100).unwrap(); }
                        Some('m') => { write!(&mut result, "{:02}", month).unwrap(); }
                        Some('d') => { write!(&mut result, "{:02}", day).unwrap(); }
                        Some('H') => { write!(&mut result, "{:02}", hour).unwrap(); }
                        Some('M') => { write!(&mut result, "{:02}", minute).unwrap(); }
                        Some('S') => { write!(&mut result, "{:02}", second).unwrap(); }
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
            let s = args[0].to_string(ctx.heap);
            let fmt = args[1].to_string(ctx.heap);
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
            let (_, _, _, _, _, _, unix) = extract_datetime_fields(&args[0], ctx.heap)?;
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

fn extract_datetime_fields(val: &Value, heap: &GcHeap) -> Result<(i64, i64, i64, i64, i64, i64, i64), String> {
    match val {
        Value::Dict(r) => {
            let entries = match heap.get(*r) { GcObj::Dict(e) => e, _ => return Err("not a dict".to_string()) };
            let mut year = 0i64; let mut month = 0i64; let mut day = 0i64;
            let mut hour = 0i64; let mut minute = 0i64; let mut second = 0i64;
            let mut unix = 0i64;
            let mut found = 0u8;
            for (k, v) in entries {
                let key_str = match heap.get(match k { Value::String(r) => *r, _ => continue }) {
                    GcObj::String(s) => s.as_str(),
                    _ => continue,
                };
                let val = match to_i64(v) { Ok(n) => n, _ => continue };
                match key_str {
                    "year" => { year = val; found |= 1; }
                    "month" => { month = val; found |= 2; }
                    "day" => { day = val; found |= 4; }
                    "hour" => { hour = val; found |= 8; }
                    "minute" => { minute = val; found |= 16; }
                    "second" => { second = val; found |= 32; }
                    "unix" => { unix = val; found |= 64; }
                    _ => {}
                }
            }
            if found & 0x7f != 0x7f {
                return Err("datetime dict missing required fields".to_string());
            }
            Ok((year, month, day, hour, minute, second, unix))
        }
        _ => Err("expected datetime dict".to_string()),
    }
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
