use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::Button as GtkButton;
use gtk4::Entry as GtkEntry;
use gtk4::Label as GtkLabel;
use gtk4::Window as GtkWindow;
use gtk4::Box as GtkBox;
use gtk4::Frame as GtkFrame;
use gtk4::Align;
use gtk4::Orientation;


use crate::bytecode::Chunk;
use crate::gc::*;
use crate::vm::call_func_closure;

static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

thread_local! {
    static WIDGETS: RefCell<HashMap<usize, gtk4::Widget>> = RefCell::new(HashMap::new());
    static WIDGET_TYPES: RefCell<HashMap<usize, &'static str>> = RefCell::new(HashMap::new());
    static CONTENT_BOX: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());
    static CALLBACKS: RefCell<HashMap<usize, Value>> = RefCell::new(HashMap::new());
    static CALLBACK_CTX: RefCell<Option<(*mut GcHeap, *mut HashMap<String, Value>, *const Vec<Chunk>)>> = RefCell::new(None);
}

fn alloc_id() -> usize {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

fn ensure_init() -> Result<(), String> {
    static INIT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    INIT.get_or_init(|| gtk4::init().is_ok());
    Ok(())
}

fn get_ptr(val: &Value, heap: &GcHeap) -> Result<usize, String> {
    match val {
        Value::Dict(r) => {
            let entries = match heap.get(*r) {
                GcObj::Dict(e) => e,
                _ => return Err("not a widget dict".to_string()),
            };
            for (k, v) in entries {
                if let Value::String(sr) = k {
                    if let GcObj::String(s) = heap.get(*sr) {
                        if s == "ptr" {
                            return match v {
                                Value::Int(n) => Ok(*n as usize),
                                _ => Err("invalid ptr in widget".to_string()),
                            };
                        }
                    }
                }
            }
            Err("widget dict missing 'ptr' key".to_string())
        }
        _ => Err("expected a widget dict".to_string()),
    }
}

fn make_widget_dict(heap: &mut GcHeap, id: usize, widget_type: &str) -> Value {
    let entries = vec![
        (make_string(heap, "ptr"), Value::Int(id as i64)),
        (make_string(heap, "type"), make_string(heap, widget_type)),
    ];
    Value::Dict(heap.alloc(GcObj::Dict(entries)))
}

fn get_widget(id: usize) -> Result<gtk4::Widget, String> {
    WIDGETS.with(|w| w.borrow().get(&id).cloned())
        .ok_or_else(|| "widget not found".to_string())
}

pub fn build_panther() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("Leo".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.Leo>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_init()?;
            let title = args.first().map(|a| a.to_string(ctx.heap)).unwrap_or_else(|| "Panther".to_string());
            let width = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(640) as i32;
            let height = args.get(2).and_then(|a| to_i64(a).ok()).unwrap_or(480) as i32;

            let win = GtkWindow::builder()
                .title(&title)
                .default_width(width)
                .default_height(height)
                .build();

            let content = GtkBox::new(Orientation::Vertical, 4);
            content.set_margin_start(10);
            content.set_margin_end(10);
            content.set_margin_top(10);
            content.set_margin_bottom(10);
            win.set_child(Some(&content));

            let win_id = alloc_id();
            let content_id = alloc_id();
            WIDGETS.with(|w| {
                w.borrow_mut().insert(win_id, win.upcast::<gtk4::Widget>());
                w.borrow_mut().insert(content_id, content.upcast::<gtk4::Widget>());
            });
            WIDGET_TYPES.with(|t| {
                t.borrow_mut().insert(win_id, "window");
            });
            CONTENT_BOX.with(|c| {
                c.borrow_mut().insert(win_id, content_id);
            });

            Ok(make_widget_dict(ctx.heap, win_id, "tk"))
        }),
    })));

    funcs.push(("Frame".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.Frame>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_init()?;
            let parent_ptr = get_ptr(&args[0], ctx.heap)?;
            let parent_box = get_parent_box(parent_ptr)?;

            let frame = GtkFrame::new(None);
            frame.set_hexpand(true);
            frame.set_vexpand(true);

            let frame_box = GtkBox::new(Orientation::Vertical, 4);
            frame.set_child(Some(&frame_box));

            parent_box.dynamic_cast_ref::<GtkBox>()
                .ok_or("parent is not a box container")?
                .append(&frame);

            let frame_id = alloc_id();
            let frame_box_id = alloc_id();
            WIDGETS.with(|w| {
                w.borrow_mut().insert(frame_id, frame.upcast::<gtk4::Widget>());
                w.borrow_mut().insert(frame_box_id, frame_box.upcast::<gtk4::Widget>());
            });
            WIDGET_TYPES.with(|t| {
                t.borrow_mut().insert(frame_id, "frame");
            });
            CONTENT_BOX.with(|c| {
                c.borrow_mut().insert(frame_id, frame_box_id);
            });

            Ok(make_widget_dict(ctx.heap, frame_id, "frame"))
        }),
    })));

    funcs.push(("Label".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.Label>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_init()?;
            let parent_ptr = get_ptr(&args[0], ctx.heap)?;
            let parent_box = get_parent_box(parent_ptr)?;
            let text = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_default();

            let label = GtkLabel::new(Some(&text));
            label.set_halign(Align::Start);

            parent_box.dynamic_cast_ref::<GtkBox>()
                .ok_or("parent is not a box container")?
                .append(&label);

            let id = alloc_id();
            WIDGETS.with(|w| w.borrow_mut().insert(id, label.upcast::<gtk4::Widget>()));
            Ok(make_widget_dict(ctx.heap, id, "label"))
        }),
    })));

    funcs.push(("Button".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.Button>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_init()?;
            let parent_ptr = get_ptr(&args[0], ctx.heap)?;
            let parent_box = get_parent_box(parent_ptr)?;
            let text = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_default();
            let command = args.get(2).cloned();

            let btn = GtkButton::with_label(&text);
            let btn_id = alloc_id();

            if let Some(cmd) = command {
                CALLBACKS.with(|cbs| cbs.borrow_mut().insert(btn_id, cmd));
                let cb_id = btn_id;
                btn.connect_clicked(move |_| {
                    let cmd_val = CALLBACKS.with(|cbs| cbs.borrow().get(&cb_id).cloned());
                    if let Some(cmd_val) = cmd_val {
                        let ctx_opt = CALLBACK_CTX.with(|c| c.borrow().map(|(a, b, c_)| (a, b, c_)));
                        if let Some((heap_ptr, globals_ptr, chunks_ptr)) = ctx_opt {
                            let heap = unsafe { &mut *heap_ptr };
                            let globals = unsafe { &mut *globals_ptr };
                            let chunks = unsafe { &*chunks_ptr };
                            let mut modules = Vec::new();
                            let mut try_frames = Vec::new();
                            let mut ctx = VmContext {
                                heap,
                                globals,
                                modules: &mut modules,
                                chunks,
                                try_frames: &mut try_frames,
                            };
                            let _ = call_func_closure(&cmd_val, &[], &mut ctx);
                        }
                    }
                });
            }

            parent_box.dynamic_cast_ref::<GtkBox>()
                .ok_or("parent is not a box container")?
                .append(&btn);

            WIDGETS.with(|w| w.borrow_mut().insert(btn_id, btn.upcast::<gtk4::Widget>()));
            Ok(make_widget_dict(ctx.heap, btn_id, "button"))
        }),
    })));

    funcs.push(("Entry".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.Entry>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_init()?;
            let parent_ptr = get_ptr(&args[0], ctx.heap)?;
            let parent_box = get_parent_box(parent_ptr)?;

            let entry = GtkEntry::builder()
                .hexpand(true)
                .build();

            parent_box.dynamic_cast_ref::<GtkBox>()
                .ok_or("parent is not a box container")?
                .append(&entry);

            let id = alloc_id();
            WIDGETS.with(|w| w.borrow_mut().insert(id, entry.upcast::<gtk4::Widget>()));
            Ok(make_widget_dict(ctx.heap, id, "entry"))
        }),
    })));

    funcs.push(("pack".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.pack>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let widget = get_widget(id)?;
            widget.set_visible(true);
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("place".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.place>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let widget = get_widget(id)?;
            widget.set_visible(true);
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("config".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.config>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let widget = get_widget(id)?;

            if let Some(prop_val) = args.get(1) {
                let prop_name = prop_val.to_string(ctx.heap);
                if let Some(val) = args.get(2) {
                    match prop_name.as_str() {
                        "text" => {
                            let text = val.to_string(ctx.heap);
                            if let Ok(label) = widget.clone().downcast::<GtkLabel>() {
                                label.set_label(&text);
                            } else if let Ok(btn) = widget.clone().downcast::<GtkButton>() {
                                btn.set_label(&text);
                            } else if let Ok(entry) = widget.clone().downcast::<GtkEntry>() {
                                entry.set_text(&text);
                            }
                        }
                        "command" => {
                            CALLBACKS.with(|cbs| {
                                cbs.borrow_mut().insert(id, val.clone());
                            });
                        }
                        _ => {}
                    }
                }
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("get".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.get>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let widget = get_widget(id)?;
            if let Ok(entry) = widget.downcast::<GtkEntry>() {
                Ok(make_string_owned(ctx.heap, entry.text().to_string()))
            } else {
                Err("get requires an Entry widget".to_string())
            }
        }),
    })));

    funcs.push(("insert".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.insert>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let pos = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(0) as usize;
            let text = args.get(2).map(|a| a.to_string(ctx.heap)).unwrap_or_default();

            let widget = get_widget(id)?;
            if let Ok(entry) = widget.downcast::<GtkEntry>() {
                let current = entry.text();
                let mut new_text = current.to_string();
                let pos = pos.min(new_text.len());
                new_text.insert_str(pos, &text);
                entry.set_text(&new_text);
                Ok(Value::Nil)
            } else {
                Err("insert requires an Entry widget".to_string())
            }
        }),
    })));

    funcs.push(("delete".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.delete>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let start = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(0) as usize;
            let end = args.get(2).and_then(|a| to_i64(a).ok()).map(|n| n as usize);

            let widget = get_widget(id)?;
            if let Ok(entry) = widget.downcast::<GtkEntry>() {
                let current = entry.text();
                let end = end.unwrap_or(current.len()).min(current.len());
                let start = start.min(end);
                let new_text: String = current.chars().take(start)
                    .chain(current.chars().skip(end)).collect();
                entry.set_text(&new_text);
                Ok(Value::Nil)
            } else {
                Err("delete requires an Entry widget".to_string())
            }
        }),
    })));

    funcs.push(("title".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.title>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let text = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_default();

            let widget = get_widget(id)?;
            if let Ok(win) = widget.downcast::<GtkWindow>() {
                win.set_title(Some(&text));
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("geometry".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.geometry>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let w = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(640) as i32;
            let h = args.get(2).and_then(|a| to_i64(a).ok()).unwrap_or(480) as i32;

            let widget = get_widget(id)?;
            if let Ok(win) = widget.downcast::<GtkWindow>() {
                win.set_default_size(w, h);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("click".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.click>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let widget = get_widget(id)?;
            if let Ok(btn) = widget.downcast::<GtkButton>() {
                CALLBACK_CTX.with(|c| {
                    *c.borrow_mut() = Some((ctx.heap as *mut GcHeap, ctx.globals as *mut HashMap<String, Value>, ctx.chunks as *const Vec<Chunk>));
                });
                btn.emit_clicked();
                CALLBACK_CTX.with(|c| {
                    *c.borrow_mut() = None;
                });
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("destroy".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.destroy>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            if let Some(widget) = WIDGETS.with(|w| w.borrow_mut().remove(&id)) {
                widget.unparent();
            }
            CALLBACKS.with(|cbs| cbs.borrow_mut().remove(&id));
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("mainloop".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.mainloop>".to_string(),
        func: Rc::new(|args, ctx| {
            let id = get_ptr(&args[0], ctx.heap)?;
            let widget = get_widget(id)?;
            if let Ok(win) = widget.downcast::<GtkWindow>() {
                win.present();
                win.connect_close_request(move |_| {
                    gtk4::glib::MainLoop::new(None, false).quit();
                    gtk4::glib::Propagation::Proceed
                });
            }
            CALLBACK_CTX.with(|c| {
                *c.borrow_mut() = Some((ctx.heap as *mut GcHeap, ctx.globals as *mut HashMap<String, Value>, ctx.chunks as *const Vec<Chunk>));
            });
            gtk4::glib::MainLoop::new(None, false).run();
            CALLBACK_CTX.with(|c| {
                *c.borrow_mut() = None;
            });
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("messagebox".to_string(), Value::NativeFunc(NativeFunc {
        name: "<panther.messagebox>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_init()?;
            let text = args.get(0).map(|a| a.to_string(ctx.heap)).unwrap_or_default();
            let _title = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_else(|| "Message".to_string());

            let dialog = gtk4::MessageDialog::builder()
                .text(&text)
                .message_type(gtk4::MessageType::Info)
                .buttons(gtk4::ButtonsType::Ok)
                .build();
            dialog.present();
            Ok(Value::Nil)
        }),
    })));

    funcs
}

fn get_parent_box(parent_id: usize) -> Result<gtk4::Widget, String> {
    let box_id = CONTENT_BOX.with(|c| c.borrow().get(&parent_id).copied());
    match box_id {
        Some(box_id) => get_widget(box_id),
        None => get_widget(parent_id),
    }
}
