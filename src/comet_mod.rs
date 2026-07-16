use std::rc::Rc;

use crate::gc::*;

fn render_element(val: &Value, heap: &mut GcHeap) -> String {
    match val {
        Value::Dict(ref_obj) => {
            let entries = match heap.get(*ref_obj) {
                GcObj::Dict(e) => e.clone(),
                _ => return val.to_string(heap),
            };
            let mut tag = String::new();
            let mut inner = String::new();
            let mut attrs = Vec::new();

            for (k, v) in &entries {
                let key = k.to_string(heap);
                if key == "_tag" {
                    tag = v.to_string(heap);
                } else if key == "_inner" {
                    inner = render_element(v, heap);
                } else if key == "_attrs" {
                    if let Value::Dict(a_ref) = v {
                        if let GcObj::Dict(a_entries) = heap.get(*a_ref) {
                            for (ak, av) in a_entries {
                                attrs.push((ak.to_string(heap), av.to_string(heap)));
                            }
                        }
                    }
                }
            }

            if tag.is_empty() {
                return inner;
            }

            let attr_str = if attrs.is_empty() {
                String::new()
            } else {
                let mut s = String::new();
                for (name, value) in &attrs {
                    let escaped_val = value
                        .replace('&', "&amp;")
                        .replace('"', "&quot;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    s.push_str(&format!(" {}=\"{}\"", name, escaped_val));
                }
                s
            };

            format!("<{}{}>{}</{}>", tag, attr_str, inner, tag)
        }
        _ => val.to_string(heap),
    }
}

fn make_element(tag: &str, content: Value, heap: &mut GcHeap) -> Value {
    let attrs_ref = heap.alloc(GcObj::Dict(Vec::new()));
    let elem_ref = heap.alloc(GcObj::Dict(Vec::new()));

    let tag_key = make_string(heap, "_tag");
    let tag_val = make_string(heap, tag);
    let inner_key = make_string(heap, "_inner");
    let attrs_key = make_string(heap, "_attrs");
    let attrs_val = Value::Dict(attrs_ref);

    if let GcObj::Dict(entries) = heap.get_mut(elem_ref) {
        entries.push((tag_key, tag_val));
        entries.push((inner_key, content));
        entries.push((attrs_key, attrs_val));
    }

    let style_key = make_string(heap, "style");
    let style_func = {
        let elem_ref = elem_ref;
        let attrs_ref = attrs_ref;
        Value::NativeFunc(NativeFunc {
            name: format!("<{}.style>", tag),
            func: Rc::new(move |args2, ctx2| {
                if args2.is_empty() {
                    return Err("style requires a CSS string argument".to_string());
                }
                let css = args2[0].to_string(ctx2.heap);
                let new_key = make_string(ctx2.heap, "style");
                let new_val = make_string(ctx2.heap, &css);

                // Find position of existing "style" key without borrowing heap
                let pos = if let GcObj::Dict(a_entries) = ctx2.heap.get(attrs_ref) {
                    a_entries.iter().position(|(k, _)| {
                        if let Value::String(r) = k {
                            if let GcObj::String(s) = ctx2.heap.get(*r) {
                                return s == "style";
                            }
                        }
                        false
                    })
                } else {
                    None
                };

                if let Some(idx) = pos {
                    if let GcObj::Dict(a_entries) = ctx2.heap.get_mut(attrs_ref) {
                        a_entries.remove(idx);
                        a_entries.push((new_key, new_val));
                    }
                } else if let GcObj::Dict(a_entries) = ctx2.heap.get_mut(attrs_ref) {
                    a_entries.push((new_key, new_val));
                }
                Ok(Value::Dict(elem_ref))
            }),
        })
    };

    if let GcObj::Dict(entries) = heap.get_mut(elem_ref) {
        entries.push((style_key, style_func));
    }

    Value::Dict(elem_ref)
}

fn push_tag(entries: &mut Vec<(Value, Value)>, heap: &mut GcHeap, tag: &str) {
    let t = tag.to_string();
    let key = make_string(heap, tag);
    let func = Value::NativeFunc(NativeFunc {
        name: format!("<comet.{}>", t),
        func: Rc::new(move |args2, ctx2| {
            let content = args2.first().cloned().unwrap_or(Value::Nil);
            Ok(make_element(&t, content, ctx2.heap))
        }),
    });
    entries.push((key, func));
}

pub fn build_comet() -> Vec<(String, Value)> {
    vec![(
        "comet".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<comet>".to_string(),
            func: Rc::new(|_args, ctx| {
                let comet_ref = ctx.heap.alloc(GcObj::Dict(Vec::new()));
                let mut builder_entries = Vec::new();

                let block_tags = ["body", "center", "div", "span", "section", "article",
                                  "header", "footer", "nav", "main", "aside", "form"];
                for &tag in &block_tags {
                    push_tag(&mut builder_entries, ctx.heap, tag);
                }

                for level in 1..=6 {
                    let tag = format!("h{}", level);
                    push_tag(&mut builder_entries, ctx.heap, &tag);
                }

                let inline_tags = ["p", "a", "strong", "em", "i", "b", "u", "code",
                                   "pre", "br", "hr", "img", "input", "button", "label",
                                   "ul", "ol", "li", "table", "tr", "td", "th"];
                for &tag in &inline_tags {
                    push_tag(&mut builder_entries, ctx.heap, tag);
                }

                // Pre-allocate keys for the final functions
                let raw_key = make_string(ctx.heap, "raw");
                let html_key = make_string(ctx.heap, "html");
                let render_key = make_string(ctx.heap, "render");

                let raw_func = Value::NativeFunc(NativeFunc {
                    name: "<comet.raw>".to_string(),
                    func: Rc::new(|args2, ctx2| {
                        let val = args2.first().cloned().unwrap_or(Value::Nil);
                        let html = render_element(&val, ctx2.heap);
                        Ok(make_string(ctx2.heap, &html))
                    }),
                });

                let html_func = Value::NativeFunc(NativeFunc {
                    name: "<comet.html>".to_string(),
                    func: Rc::new(|args2, ctx2| {
                        let val = args2.first().cloned().unwrap_or(Value::Nil);
                        let html = render_element(&val, ctx2.heap);
                        Ok(make_string(ctx2.heap, &html))
                    }),
                });

                let render_func = Value::NativeFunc(NativeFunc {
                    name: "<comet.render>".to_string(),
                    func: Rc::new(|args2, ctx2| {
                        let val = args2.first().cloned().unwrap_or(Value::Nil);
                        let html = render_element(&val, ctx2.heap);
                        Ok(make_string(ctx2.heap, &html))
                    }),
                });

                if let GcObj::Dict(entries) = ctx.heap.get_mut(comet_ref) {
                    entries.extend(builder_entries);
                    entries.push((raw_key, raw_func));
                    entries.push((html_key, html_func));
                    entries.push((render_key, render_func));
                }

                Ok(Value::Dict(comet_ref))
            }),
        }),
    )]
}
