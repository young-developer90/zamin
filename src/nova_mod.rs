use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::gc::*;
use crate::vm::call_func_closure;

static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

fn make_request_dict(method: &str, path: &str, headers: &[(String, String)], body: &str, heap: &mut GcHeap) -> Value {
    let mut hdrs = Vec::new();
    for (k, v) in headers {
        hdrs.push((make_string(heap, k), make_string(heap, v)));
    }
    let entries = vec![
        (make_string(heap, "method"), make_string(heap, method)),
        (make_string(heap, "path"), make_string(heap, path)),
        (make_string(heap, "headers"), Value::Dict(heap.alloc(GcObj::Dict(hdrs)))),
        (make_string(heap, "body"), make_string(heap, body)),
    ];
    Value::Dict(heap.alloc(GcObj::Dict(entries)))
}

fn parse_http_request(stream: &mut TcpStream) -> Result<(String, String, Vec<(String, String)>, String), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| format!("clone: {}", e))?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).map_err(|e| format!("read request line: {}", e))?;
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Err("invalid request line".to_string());
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| format!("read header: {}", e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(pos) = trimmed.find(':') {
            let name = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            headers.push((name, value));
        }
    }

    let mut body = String::new();
    let content_length: usize = headers.iter()
        .find(|(k, _)| k.to_lowercase() == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf).map_err(|e| format!("read body: {}", e))?;
        body = String::from_utf8_lossy(&buf).to_string();
    }

    Ok((method, path, headers, body))
}

fn send_response(stream: &mut TcpStream, status: u16, content: &str, content_type: &str) -> Result<(), String> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status, status_text, content_type, content.len(), content
    );
    stream.write_all(response.as_bytes()).map_err(|e| format!("write response: {}", e))?;
    stream.flush().map_err(|e| format!("flush: {}", e))?;
    Ok(())
}

fn match_route<'a>(routes: &'a [(String, String, Value)], method: &str, path: &str) -> Option<&'a Value> {
    for (rmethod, rpath, handler) in routes {
        if rmethod == "*" || rmethod == method {
            let norm = format!("/{}", rpath.trim_start_matches('/'));
            if path == norm {
                return Some(handler);
            }
        }
    }
    None
}

fn make_route_func(
    routes_ref: ObjRef,
    app_ref: ObjRef,
    default_method: &str,
    func_name: &str,
) -> Value {
    let method_str = default_method.to_string();
    let name_str = func_name.to_string();
    Value::NativeFunc(NativeFunc {
        name: name_str,
        func: Rc::new(move |args2, ctx2| {
            if args2.len() < 2 {
                return Err(format!("{} requires (path, handler)", method_str));
            }
            let path = args2[0].to_string(ctx2.heap);
            let handler = args2[1].clone();

            let method_key = make_string(ctx2.heap, "method");
            let path_key = make_string(ctx2.heap, "path");
            let handler_key = make_string(ctx2.heap, "handler");

            let method_val = if method_str == "*" {
                make_string(ctx2.heap, "*")
            } else {
                make_string(ctx2.heap, &method_str)
            };

            let route = vec![
                (method_key, method_val),
                (path_key, make_string(ctx2.heap, &path)),
                (handler_key, handler),
            ];
            let route_dict = Value::Dict(ctx2.heap.alloc(GcObj::Dict(route)));

            if let GcObj::List(routes) = ctx2.heap.get_mut(routes_ref) {
                routes.push(route_dict);
            }

            Ok(Value::Dict(app_ref))
        }),
    })
}

pub fn build_nova() -> Vec<(String, Value)> {
    vec![(
        "nova".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<nova>".to_string(),
            func: Rc::new(|_args, ctx| {
                let routes_ref = ctx.heap.alloc(GcObj::List(Vec::new()));
                let app_ref = ctx.heap.alloc(GcObj::Dict(Vec::new()));

                // Pre-allocate all keys
                let routes_key = make_string(ctx.heap, "_routes");
                let port_key = make_string(ctx.heap, "_port");
                let serve_key = make_string(ctx.heap, "serve");
                let get_key = make_string(ctx.heap, "get");
                let post_key = make_string(ctx.heap, "post");
                let run_key = make_string(ctx.heap, "run");

                let serve_func = make_route_func(routes_ref, app_ref, "*", "<app.serve>");
                let get_func = make_route_func(routes_ref, app_ref, "GET", "<app.get>");
                let post_func = make_route_func(routes_ref, app_ref, "POST", "<app.post>");

                let run_func = {
                    let routes_ref = routes_ref;
                    Value::NativeFunc(NativeFunc {
                        name: "<app.run>".to_string(),
                        func: Rc::new(move |args2, ctx2| {
                            let port: u16 = if let Some(arg) = args2.first() {
                                match arg {
                                    Value::Int(n) => *n as u16,
                                    Value::UInt(n) => *n as u16,
                                    _ => return Err("port must be an integer".to_string()),
                                }
                            } else {
                                8080
                            };

                            let addr = format!("0.0.0.0:{}", port);
                            let listener = TcpListener::bind(&addr)
                                .map_err(|e| format!("failed to bind {}: {}", addr, e))?;
                            listener.set_nonblocking(true)
                                .map_err(|e| format!("set nonblocking: {}", e))?;

                            SERVER_RUNNING.store(true, Ordering::SeqCst);

                            println!("Nova server running on http://localhost:{}", port);
                            println!("Press Ctrl+C to stop");

                            let routes_snapshot: Vec<(String, String, Value)> = {
                                if let GcObj::List(routes_list) = ctx2.heap.get(routes_ref) {
                                    let mut snap = Vec::new();
                                    for route_val in routes_list {
                                        if let Value::Dict(r_ref) = route_val {
                                            if let GcObj::Dict(entries) = ctx2.heap.get(*r_ref) {
                                                let mut method = String::new();
                                                let mut path = String::new();
                                                let mut handler = Value::Nil;
                                                for (k, v) in entries {
                                                    let key = k.to_string(ctx2.heap);
                                                    if key == "method" { method = v.to_string(ctx2.heap); }
                                                    else if key == "path" { path = v.to_string(ctx2.heap); }
                                                    else if key == "handler" { handler = v.clone(); }
                                                }
                                                snap.push((method, path, handler));
                                            }
                                        }
                                    }
                                    snap
                                } else {
                                    Vec::new()
                                }
                            };

                            loop {
                                match listener.accept() {
                                    Ok((mut stream, _addr)) => {
                                        match parse_http_request(&mut stream) {
                                            Ok((method, path, headers, body)) => {
                                                let resp = if let Some(handler) = match_route(&routes_snapshot, &method, &path) {
                                                    let req_dict = make_request_dict(&method, &path, &headers, &body, ctx2.heap);
                                                    match call_func_closure(handler, &[req_dict], ctx2) {
                                                        Ok(result_val) => {
                                                            let html = result_val.to_string(ctx2.heap);
                                                            let ct = if html.contains("<html") || html.contains("<HTML") {
                                                                "text/html; charset=utf-8"
                                                            } else {
                                                                "text/plain; charset=utf-8"
                                                            };
                                                            send_response(&mut stream, 200, &html, ct)
                                                        }
                                                        Err(e) => {
                                                            let body = format!("<h1>500 Internal Server Error</h1><p>{}</p>", e);
                                                            send_response(&mut stream, 500, &body, "text/html; charset=utf-8")
                                                        }
                                                    }
                                                } else {
                                                    let body = format!("<h1>404 Not Found</h1><p>{} {}</p>", method, path);
                                                    send_response(&mut stream, 404, &body, "text/html; charset=utf-8")
                                                };
                                                if let Err(e) = resp {
                                                    eprintln!("response error: {}", e);
                                                }
                                            }
                                            Err(_) => {
                                                let _ = send_response(&mut stream, 400, "<h1>400 Bad Request</h1>", "text/html; charset=utf-8");
                                            }
                                        }
                                    }
                                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                        std::thread::sleep(std::time::Duration::from_millis(10));
                                    }
                                    Err(e) => {
                                        eprintln!("accept error: {}", e);
                                    }
                                }

                                if !SERVER_RUNNING.load(Ordering::SeqCst) {
                                    break;
                                }
                            }

                            Ok(Value::Nil)
                        }),
                    })
                };

                if let GcObj::Dict(entries) = ctx.heap.get_mut(app_ref) {
                    entries.push((routes_key, Value::List(routes_ref)));
                    entries.push((port_key, Value::Int(8080)));
                    entries.push((serve_key, serve_func));
                    entries.push((get_key, get_func));
                    entries.push((post_key, post_func));
                    entries.push((run_key, run_func));
                }

                Ok(Value::Dict(app_ref))
            }),
        }),
    )]
}
