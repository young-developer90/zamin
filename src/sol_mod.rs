#![allow(non_snake_case, dead_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::gc::*;
use crate::vm::call_func_closure;

type HWND = isize;
type HINSTANCE = isize;
type HCURSOR = isize;
type HBRUSH = isize;
type HMENU = isize;
type LPCWSTR = *const u16;
type LPWSTR = *mut u16;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type DWORD = u32;
type LONG = i32;
type BOOL = i32;
type UINT = u32;
type LPVOID = *mut u8;

#[link(name = "user32")]
#[link(name = "gdi32")]
#[link(name = "comctl32")]
extern "system" {}
#[repr(C)]
struct WNDCLASSW {
    style: UINT,
    lpfnWndProc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HINSTANCE,
    hIcon: isize,
    hCursor: HCURSOR,
    hbrBackground: HBRUSH,
    lpszMenuName: LPCWSTR,
    lpszClassName: LPCWSTR,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt: POINT,
}

#[repr(C)]
struct POINT { x: LONG, y: LONG }

#[repr(C)]
struct RECT { left: LONG, top: LONG, right: LONG, bottom: LONG }

#[repr(C)]
struct INITCOMMONCONTROLSEX { dwSize: DWORD, dwICC: DWORD }

const WS_OVERLAPPEDWINDOW: DWORD = 0x00CF0000;
const WS_CHILD: DWORD = 0x40000000;
const WS_VISIBLE: DWORD = 0x10000000;
const WS_BORDER: DWORD = 0x00800000;
const WS_EX_CLIENTEDGE: DWORD = 0x00000200;
const ES_LEFT: DWORD = 0x0000;
const ES_AUTOHSCROLL: DWORD = 0x0080;
const SS_LEFT: DWORD = 0x00000000;
const BS_PUSHBUTTON: DWORD = 0x00000000;
const SW_SHOW: i32 = 5;
const SWP_NOMOVE: UINT = 0x0002;
const SWP_NOSIZE: UINT = 0x0001;
const SWP_NOZORDER: UINT = 0x0004;
const WM_COMMAND: UINT = 0x0111;
const WM_DESTROY: UINT = 0x0002;
const BN_CLICKED: UINT = 0;
const COLOR_WINDOW: u32 = 5;
const COLOR_BTNFACE: u32 = 15;
const ICC_WIN95_CLASSES: DWORD = 0x000000FF;
const EM_SETSEL: UINT = 0x00B1;
const EM_REPLACESEL: UINT = 0x00C2;
const ERROR_CLASS_ALREADY_EXISTS: i32 = 1410;
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;

#[link(name = "user32")]
#[link(name = "gdi32")]
#[link(name = "comctl32")]
extern "system" {
    fn GetModuleHandleW(lpModuleName: LPCWSTR) -> HINSTANCE;
    fn RegisterClassW(wc: *const WNDCLASSW) -> u16;
    fn CreateWindowExW(
        dwExStyle: DWORD, lpClassName: LPCWSTR, lpWindowName: LPCWSTR,
        dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: HMENU, hInstance: HINSTANCE, lpParam: LPVOID,
    ) -> HWND;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> BOOL;
    fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    fn PostQuitMessage(nExitCode: i32);
    fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    fn UpdateWindow(hWnd: HWND) -> BOOL;
    fn DestroyWindow(hWnd: HWND) -> BOOL;
    fn SetWindowTextW(hWnd: HWND, lpString: LPCWSTR) -> BOOL;
    fn GetWindowTextLengthW(hWnd: HWND) -> i32;
    fn GetWindowTextW(hWnd: HWND, lpString: LPWSTR, nMaxCount: i32) -> i32;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: LPCWSTR) -> HCURSOR;
    fn SetPropW(hWnd: HWND, lpString: LPCWSTR, hData: isize) -> BOOL;
    fn GetPropW(hWnd: HWND, lpString: LPCWSTR) -> isize;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, X: i32, Y: i32, cx: i32, cy: i32, uFlags: UINT) -> BOOL;
    fn GetParent(hWnd: HWND) -> HWND;
    fn GetDesktopWindow() -> HWND;
    fn AdjustWindowRect(lpRect: *mut RECT, dwStyle: DWORD, bMenu: BOOL) -> BOOL;
    fn GetSystemMetrics(nIndex: i32) -> i32;
    fn InitCommonControlsEx(lpInitCtrls: *const INITCOMMONCONTROLSEX) -> BOOL;
    fn IsWindow(hWnd: HWND) -> BOOL;
    fn MessageBoxW(hWnd: HWND, lpText: LPCWSTR, lpCaption: LPCWSTR, uType: UINT) -> i32;
}

thread_local! {
    static CALLBACKS: RefCell<HashMap<isize, Value>> = RefCell::new(HashMap::new());
    static PENDING: RefCell<Vec<isize>> = RefCell::new(Vec::new());
}

fn get_hwnd(val: &Value, heap: &GcHeap) -> Result<HWND, String> {
    match val {
        Value::Dict(r) => {
            let entries = match heap.get(*r) {
                GcObj::Dict(e) => e,
                _ => return Err("not a widget dict".to_string()),
            };
            for (k, v) in entries {
                if let Value::String(sr) = k {
                    if let GcObj::String(s) = heap.get(*sr) {
                        if s == "hwnd" {
                            return match v {
                                Value::Int(n) => Ok(*n as isize),
                                _ => Err("invalid hwnd in widget".to_string()),
                            };
                        }
                    }
                }
            }
            Err("widget dict missing 'hwnd' key".to_string())
        }
        _ => Err("expected a widget dict".to_string()),
    }
}

fn make_widget_dict(heap: &mut GcHeap, hwnd: HWND, widget_type: &str) -> Value {
    let entries = vec![
        (make_string(heap, "hwnd"), Value::Int(hwnd as i64)),
        (make_string(heap, "type"), make_string(heap, widget_type)),
    ];
    Value::Dict(heap.alloc(GcObj::Dict(entries)))
}

fn utf8_to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static mut SOL_HINSTANCE: HINSTANCE = 0;
static mut SOL_INITIALIZED: bool = false;

unsafe extern "system" fn sol_wndproc(
    hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let notify_code = (wparam >> 16) as UINT;
            if notify_code == BN_CLICKED {
                let btn_hwnd = lparam;
                let _ = PENDING.try_with(|p| p.borrow_mut().push(btn_hwnd));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn ensure_registered() -> Result<(), String> {
    unsafe {
        if SOL_INITIALIZED {
            return Ok(());
        }

        let icex = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as DWORD,
            dwICC: ICC_WIN95_CLASSES,
        };
        InitCommonControlsEx(&icex);

        SOL_HINSTANCE = GetModuleHandleW(std::ptr::null());
        if SOL_HINSTANCE == 0 {
            return Err("failed to get module handle".to_string());
        }

        let cursor_arrow = LoadCursorW(0, 32512 as LPCWSTR);
        let _cursor_ibeam = LoadCursorW(0, 32513 as LPCWSTR);

        let classes = [
            ("Sol_Tk",  COLOR_WINDOW + 1),
            ("Sol_Frame", COLOR_WINDOW + 1),
        ];

        for (name, bg) in &classes {
            let wname = utf8_to_wide(name);
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(sol_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: SOL_HINSTANCE,
                hIcon: 0,
                hCursor: cursor_arrow,
                hbrBackground: *bg as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: wname.as_ptr(),
            };
            let ret = RegisterClassW(&wc);
            if ret == 0 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error().unwrap_or(0) != ERROR_CLASS_ALREADY_EXISTS {
                    return Err(format!("failed to register window class '{}': {}", name, err));
                }
            }
        }

        SOL_INITIALIZED = true;
        Ok(())
    }
}

pub fn build_sol() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("Leo".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.Leo>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_registered()?;
            let title = args.first().map(|a| a.to_string(ctx.heap)).unwrap_or_else(|| "Sol".to_string());
            let width = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(640) as i32;
            let height = args.get(2).and_then(|a| to_i64(a).ok()).unwrap_or(480) as i32;

            unsafe {
                let mut rect = RECT { left: 0, top: 0, right: width, bottom: height };
                AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, 0);
                let win_w = rect.right - rect.left;
                let win_h = rect.bottom - rect.top;
                let x = (GetSystemMetrics(SM_CXSCREEN) - win_w) / 2;
                let y = (GetSystemMetrics(SM_CYSCREEN) - win_h) / 2;

                let hwnd = CreateWindowExW(
                    0,
                    utf8_to_wide("Sol_Tk").as_ptr(),
                    utf8_to_wide(&title).as_ptr(),
                    WS_OVERLAPPEDWINDOW, x, y, win_w, win_h,
                    0, 0, SOL_HINSTANCE, std::ptr::null_mut(),
                );
                if hwnd == 0 {
                    return Err("Failed to create Tk window".to_string());
                }
                Ok(make_widget_dict(ctx.heap, hwnd, "tk"))
            }
        }),
    })));

    funcs.push(("Frame".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.Frame>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_registered()?;
            let parent_hwnd = {
                let p = args.get(0).ok_or("Frame requires a parent widget")?;
                get_hwnd(p, ctx.heap)?
            };

            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    utf8_to_wide("Sol_Frame").as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | WS_BORDER,
                    0, 0, 100, 100,
                    parent_hwnd, 0, SOL_HINSTANCE, std::ptr::null_mut(),
                );
                if hwnd == 0 {
                    return Err("Failed to create Frame".to_string());
                }
                Ok(make_widget_dict(ctx.heap, hwnd, "frame"))
            }
        }),
    })));

    funcs.push(("Label".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.Label>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_registered()?;
            let parent_hwnd = {
                let p = args.get(0).ok_or("Label requires a parent widget")?;
                get_hwnd(p, ctx.heap)?
            };
            let text = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_default();

            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    utf8_to_wide("STATIC").as_ptr(),
                    utf8_to_wide(&text).as_ptr(),
                    WS_CHILD | WS_VISIBLE | SS_LEFT,
                    0, 0, 100, 20,
                    parent_hwnd, 0, SOL_HINSTANCE, std::ptr::null_mut(),
                );
                if hwnd == 0 {
                    return Err("Failed to create Label".to_string());
                }
                Ok(make_widget_dict(ctx.heap, hwnd, "label"))
            }
        }),
    })));

    funcs.push(("Button".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.Button>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_registered()?;
            let parent_hwnd = {
                let p = args.get(0).ok_or("Button requires a parent widget")?;
                get_hwnd(p, ctx.heap)?
            };
            let text = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_default();
            let command = args.get(2).cloned();

            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    utf8_to_wide("BUTTON").as_ptr(),
                    utf8_to_wide(&text).as_ptr(),
                    WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
                    0, 0, 80, 30,
                    parent_hwnd, 0, SOL_HINSTANCE, std::ptr::null_mut(),
                );
                if hwnd == 0 {
                    return Err("Failed to create Button".to_string());
                }

                if let Some(cmd) = command {
                    let _ = CALLBACKS.try_with(|cbs| cbs.borrow_mut().insert(hwnd, cmd));
                }

                Ok(make_widget_dict(ctx.heap, hwnd, "button"))
            }
        }),
    })));

    funcs.push(("Entry".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.Entry>".to_string(),
        func: Rc::new(|args, ctx| {
            ensure_registered()?;
            let parent_hwnd = {
                let p = args.get(0).ok_or("Entry requires a parent widget")?;
                get_hwnd(p, ctx.heap)?
            };

            unsafe {
                let hwnd = CreateWindowExW(
                    WS_EX_CLIENTEDGE,
                    utf8_to_wide("EDIT").as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | ES_LEFT | ES_AUTOHSCROLL,
                    0, 0, 150, 24,
                    parent_hwnd, 0, SOL_HINSTANCE, std::ptr::null_mut(),
                );
                if hwnd == 0 {
                    return Err("Failed to create Entry".to_string());
                }
                Ok(make_widget_dict(ctx.heap, hwnd, "entry"))
            }
        }),
    })));

    funcs.push(("pack".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.pack>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("pack requires a widget")?;
                get_hwnd(w, ctx.heap)?
            };
            let side = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_else(|| "top".to_string());
            let padx = args.get(2).and_then(|a| to_i64(a).ok()).unwrap_or(0) as i32;
            let pady = args.get(3).and_then(|a| to_i64(a).ok()).unwrap_or(0) as i32;

            unsafe {
                let parent_hwnd = GetParent(hwnd);
                let parent_hwnd = if parent_hwnd != 0 { parent_hwnd } else { GetDesktopWindow() };

                let mut parent_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetClientRect(parent_hwnd, &mut parent_rect);

                let mut widget_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(hwnd, &mut widget_rect);
                let w = widget_rect.right - widget_rect.left;
                let h = widget_rect.bottom - widget_rect.top;

                let prop_name = utf8_to_wide("SolPackOffset");
                let raw = GetPropW(parent_hwnd, prop_name.as_ptr());
                let mut offset = raw as i32;

                let (x, y) = match side.as_str() {
                    "top" => {
                        let x = padx + 10;
                        let y = offset + pady + 10;
                        offset = y + h + pady;
                        (x, y)
                    }
                    "bottom" => {
                        (padx + 10, parent_rect.bottom - h - pady - 10)
                    }
                    "left" => {
                        let x = offset + padx + 10;
                        let y = pady + 10;
                        offset = x + w + padx;
                        (x, y)
                    }
                    "right" => {
                        (parent_rect.right - w - padx - 10, pady + 10)
                    }
                    _ => {
                        let x = padx + 10;
                        let y = offset + pady + 10;
                        offset = y + h + pady;
                        (x, y)
                    }
                };

                SetPropW(parent_hwnd, prop_name.as_ptr(), offset as isize);
                SetWindowPos(hwnd, 0, x, y, w, h, SWP_NOZORDER | SWP_NOSIZE);
            }

            Ok(Value::Nil)
        }),
    })));

    funcs.push(("place".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.place>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("place requires a widget")?;
                get_hwnd(w, ctx.heap)?
            };
            let x = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(0) as i32;
            let y = args.get(2).and_then(|a| to_i64(a).ok()).unwrap_or(0) as i32;
            let w = args.get(3).and_then(|a| to_i64(a).ok());
            let h = args.get(4).and_then(|a| to_i64(a).ok());

            unsafe {
                if let (Some(wv), Some(hv)) = (w, h) {
                    SetWindowPos(hwnd, 0, x, y, wv as i32, hv as i32, SWP_NOZORDER);
                } else {
                    SetWindowPos(hwnd, 0, x, y, 0, 0, SWP_NOZORDER | SWP_NOSIZE);
                }
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("config".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.config>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("config requires a widget")?;
                get_hwnd(w, ctx.heap)?
            };

            if let Some(prop_val) = args.get(1) {
                let prop_name = prop_val.to_string(ctx.heap);
                if let Some(val) = args.get(2) {
                    match prop_name.as_str() {
                        "text" => {
                            let text = val.to_string(ctx.heap);
                            let wtext = utf8_to_wide(&text);
                            unsafe { SetWindowTextW(hwnd, wtext.as_ptr()); }
                        }
                        "command" => {
                            let _ = CALLBACKS.try_with(|cbs| {
                                cbs.borrow_mut().insert(hwnd, val.clone());
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
        name: "<sol.get>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("get requires an Entry widget")?;
                get_hwnd(w, ctx.heap)?
            };

            unsafe {
                let len = GetWindowTextLengthW(hwnd) as usize;
                let mut buf = vec![0u16; len + 1];
                GetWindowTextW(hwnd, buf.as_mut_ptr(), (len + 1) as i32);
                let s = String::from_utf16_lossy(&buf[..len]);
                Ok(make_string_owned(ctx.heap, s))
            }
        }),
    })));

    funcs.push(("insert".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.insert>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("insert requires an Entry widget")?;
                get_hwnd(w, ctx.heap)?
            };
            let pos = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(0) as i32;
            let text = args.get(2).map(|a| a.to_string(ctx.heap)).unwrap_or_default();
            let wtext = utf8_to_wide(&text);

            unsafe {
                SendMessageW(hwnd, EM_SETSEL, pos as WPARAM, pos as LPARAM);
                SendMessageW(hwnd, EM_REPLACESEL, 0, wtext.as_ptr() as LPARAM);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("delete".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.delete>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("delete requires an Entry widget")?;
                get_hwnd(w, ctx.heap)?
            };
            let start = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(0) as i32;
            let end = args.get(2).and_then(|a| to_i64(a).ok());

            unsafe {
                let end_pos = end.unwrap_or(-1);
                SendMessageW(hwnd, EM_SETSEL, start as WPARAM, end_pos as LPARAM);
                let empty: &[u16] = &[0];
                SendMessageW(hwnd, EM_REPLACESEL, 0, empty.as_ptr() as LPARAM);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("title".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.title>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("title requires a Tk window")?;
                get_hwnd(w, ctx.heap)?
            };
            let text = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_default();
            let wtext = utf8_to_wide(&text);
            unsafe { SetWindowTextW(hwnd, wtext.as_ptr()); }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("geometry".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.geometry>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("geometry requires a Tk window")?;
                get_hwnd(w, ctx.heap)?
            };
            let w = args.get(1).and_then(|a| to_i64(a).ok()).unwrap_or(640) as i32;
            let h = args.get(2).and_then(|a| to_i64(a).ok()).unwrap_or(480) as i32;

            unsafe {
                let mut rect = RECT { left: 0, top: 0, right: w, bottom: h };
                AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, 0);
                SetWindowPos(hwnd, 0, 0, 0, rect.right - rect.left, rect.bottom - rect.top,
                             SWP_NOMOVE | SWP_NOZORDER);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("destroy".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.destroy>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("destroy requires a widget")?;
                get_hwnd(w, ctx.heap)?
            };
            unsafe { DestroyWindow(hwnd); }
            let _ = CALLBACKS.try_with(|cbs| cbs.borrow_mut().remove(&hwnd));
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("mainloop".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.mainloop>".to_string(),
        func: Rc::new(|args, ctx| {
            let hwnd = {
                let w = args.get(0).ok_or("mainloop requires a Tk window")?;
                get_hwnd(w, ctx.heap)?
            };

            unsafe { ShowWindow(hwnd, SW_SHOW); UpdateWindow(hwnd); }

            let mut msg = MSG {
                hwnd: 0, message: 0, wParam: 0, lParam: 0, time: 0,
                pt: POINT { x: 0, y: 0 },
            };

            loop {
                let has_msg = unsafe { GetMessageW(&mut msg, 0, 0, 0) };
                if has_msg <= 0 { break; }

                unsafe { TranslateMessage(&msg); DispatchMessageW(&msg); }

                let pending = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
                for btn_hwnd in pending {
                    let cmd = CALLBACKS.with(|cbs| cbs.borrow().get(&btn_hwnd).cloned());
                    if let Some(cmd_val) = cmd {
                        let _ = call_func_closure(&cmd_val, &[], ctx);
                    }
                }
            }

            Ok(Value::Nil)
        }),
    })));

    funcs.push(("messagebox".to_string(), Value::NativeFunc(NativeFunc {
        name: "<sol.messagebox>".to_string(),
        func: Rc::new(|args, ctx| {
            let text = args.get(0).map(|a| a.to_string(ctx.heap)).unwrap_or_default();
            let title = args.get(1).map(|a| a.to_string(ctx.heap)).unwrap_or_else(|| "Message".to_string());
            unsafe {
                MessageBoxW(0, utf8_to_wide(&text).as_ptr(), utf8_to_wide(&title).as_ptr(), 0);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs
}
