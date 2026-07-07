use std::rc::Rc;

use crate::cuda::{self, CUfunction};
use crate::gc::*;

struct CachedKernels {
    relu: CUfunction,
    sigmoid: CUfunction,
    tanh: CUfunction,
}

unsafe impl Send for CachedKernels {}
unsafe impl Sync for CachedKernels {}

static KERNELS: std::sync::OnceLock<CachedKernels> = std::sync::OnceLock::new();

fn get_kernels() -> &'static CachedKernels {
    KERNELS.get().expect("CUDA kernels not loaded")
}

pub fn init_kernels() -> Result<(), String> {
    let relu = cuda::get_kernel("relu_activation")?;
    let sigmoid = cuda::get_kernel("sigmoid_activation")?;
    let tanh = cuda::get_kernel("tanh_activation")?;
    KERNELS
        .set(CachedKernels { relu, sigmoid, tanh })
        .map_err(|_| "kernels already loaded".to_string())
}

fn dict_get_entries(d: &Value, heap: &GcHeap) -> Result<Vec<(Value, Value)>, String> {
    match d {
        Value::Dict(r) => match heap.get(*r) {
            GcObj::Dict(e) => Ok(e.clone()),
            _ => Err("not a dict".to_string()),
        },
        _ => Err("not a dict".to_string()),
    }
}

fn dict_get_str<'a>(entries: &'a [(Value, Value)], key: &str, heap: &GcHeap) -> Option<&'a Value> {
    for (k, v) in entries {
        if k.to_string(heap) == key {
            return Some(v);
        }
    }
    None
}

fn dict_set_entry(dict_ref: ObjRef, heap: &mut GcHeap, key: &str, val: Value) {
    let keys: Vec<String> = match heap.get(dict_ref) {
        GcObj::Dict(ref entries) => entries.iter().map(|(k, _)| k.to_string(heap)).collect(),
        _ => return,
    };
    let new_key = make_string(heap, key);
    if let GcObj::Dict(ref mut entries) = heap.get_mut(dict_ref) {
        for (i, s) in keys.iter().enumerate() {
            if s == key {
                entries[i].1 = val;
                return;
            }
        }
        entries.push((new_key, val));
    }
}

fn to_f64(v: &Value) -> f64 {
    match v {
        Value::Int(n) => *n as f64,
        Value::UInt(n) => *n as f64,
        Value::Float(n) => *n,
        _ => 0.0,
    }
}

fn get_type(entries: &[(Value, Value)], heap: &GcHeap) -> Result<String, String> {
    match dict_get_str(entries, "type", heap) {
        Some(Value::String(r)) => match heap.get(*r) {
            GcObj::String(s) => Ok(s.clone()),
            _ => Err("invalid type string".to_string()),
        },
        _ => Err("layer has no type".to_string()),
    }
}

fn mat_data(val: &Value, heap: &GcHeap) -> Result<(usize, usize, Vec<f64>), String> {
    match val {
        Value::Matrix(r) => match heap.get(*r) {
            GcObj::Matrix { rows, cols, data, .. } => Ok((*rows, *cols, data.clone())),
            _ => Err("not a matrix".to_string()),
        },
        _ => Err("expected matrix".to_string()),
    }
}

fn mat_mul(a: &Value, b: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (ar, ac, a_data) = mat_data(a, heap)?;
    let (br, bc, b_data) = mat_data(b, heap)?;
    if ac != br {
        return Err(format!("mat_mul dim mismatch: ({}x{}) x ({}x{})", ar, ac, br, bc));
    }
    let m = ar as i32;
    let n = bc as i32;
    let k = ac as i32;
    if let Ok(r) = gpu_matmul(m, n, k, &a_data, &b_data) {
        return Ok(make_matrix(heap, m as usize, n as usize, r));
    }
    let mut result = vec![0.0; (m * n) as usize];
    for i in 0..m as usize {
        for j in 0..n as usize {
            let mut s = 0.0;
            for t in 0..k as usize {
                s += a_data[i * ac + t] * b_data[t * bc + j];
            }
            result[i * n as usize + j] = s;
        }
    }
    Ok(make_matrix(heap, m as usize, n as usize, result))
}

fn mat_transpose(val: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (r, c, data) = mat_data(val, heap)?;
    if let Ok(result) = gpu_transpose(r as i32, c as i32, &data) {
        return Ok(make_matrix(heap, c, r, result));
    }
    let mut nd = vec![0.0; r * c];
    for i in 0..r {
        for j in 0..c {
            nd[j * r + i] = data[i * c + j];
        }
    }
    Ok(make_matrix(heap, c, r, nd))
}

fn mat_add(a: &Value, b: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (ar, ac, ad) = mat_data(a, heap)?;
    let (br, bc, bd) = mat_data(b, heap)?;
    if ar != br || ac != bc {
        return Err(format!("mat_add dim mismatch: ({}x{}) + ({}x{})", ar, ac, br, bc));
    }
    Ok(make_matrix(heap, ar, ac, ad.iter().zip(bd.iter()).map(|(x, y)| x + y).collect()))
}

fn mat_sub(a: &Value, b: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (ar, ac, ad) = mat_data(a, heap)?;
    let (br, bc, bd) = mat_data(b, heap)?;
    if ar != br || ac != bc {
        return Err(format!("mat_sub dim mismatch: ({}x{}) - ({}x{})", ar, ac, br, bc));
    }
    Ok(make_matrix(heap, ar, ac, ad.iter().zip(bd.iter()).map(|(x, y)| x - y).collect()))
}

fn mat_scale(val: &Value, s: f64, heap: &mut GcHeap) -> Result<Value, String> {
    let (r, c, data) = mat_data(val, heap)?;
    Ok(make_matrix(heap, r, c, data.iter().map(|x| x * s).collect()))
}

fn mat_mul_elem(a: &Value, b: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (ar, ac, ad) = mat_data(a, heap)?;
    let (br, bc, bd) = mat_data(b, heap)?;
    if ar != br || ac != bc {
        return Err(format!("mat_mul_elem dim mismatch: ({}x{}) * ({}x{})", ar, ac, br, bc));
    }
    Ok(make_matrix(heap, ar, ac, ad.iter().zip(bd.iter()).map(|(x, y)| x * y).collect()))
}

fn sum_rows(val: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (r, c, data) = mat_data(val, heap)?;
    let mut result = vec![0.0; c];
    for i in 0..r {
        for j in 0..c {
            result[j] += data[i * c + j];
        }
    }
    Ok(make_matrix(heap, 1, c, result))
}

fn relu_act(val: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (r, c, data) = mat_data(val, heap)?;
    if let Ok(rr) = gpu_relu(&data) {
        return Ok(make_matrix(heap, r, c, rr));
    }
    Ok(make_matrix(heap, r, c, data.iter().map(|x| if *x > 0.0 { *x } else { 0.0 }).collect()))
}

fn sigmoid_act(val: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (r, c, data) = mat_data(val, heap)?;
    if let Ok(rr) = gpu_sigmoid(&data) {
        return Ok(make_matrix(heap, r, c, rr));
    }
    Ok(make_matrix(heap, r, c, data.iter().map(|x| 1.0 / (1.0 + (-x).exp())).collect()))
}

fn tanh_act(val: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (r, c, data) = mat_data(val, heap)?;
    if let Ok(rr) = gpu_tanh(&data) {
        return Ok(make_matrix(heap, r, c, rr));
    }
    Ok(make_matrix(heap, r, c, data.iter().map(|x| x.tanh()).collect()))
}

fn make_linear_layer(heap: &mut GcHeap, input_size: usize, output_size: usize) -> Value {
    let scale = (2.0 / (input_size + output_size) as f64).sqrt();
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
    let mut rng = seed;
    let n = input_size * output_size;
    let mut wd = Vec::with_capacity(n);
    for _ in 0..n {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        wd.push(((rng % 2000000) as f64 / 1000000.0 - 1.0) * scale);
    }
    let w = make_matrix(heap, input_size, output_size, wd);
    let b = make_matrix(heap, 1, output_size, vec![0.0; output_size]);
    let gw = make_matrix(heap, input_size, output_size, vec![0.0; n]);
    let gb = make_matrix(heap, 1, output_size, vec![0.0; output_size]);
    let s_type = make_string(heap, "type");
    let s_linear = make_string(heap, "linear");
    let s_w = make_string(heap, "weights");
    let s_b = make_string(heap, "bias");
    let s_gw = make_string(heap, "grad_weights");
    let s_gb = make_string(heap, "grad_bias");
    let s_is = make_string(heap, "input_size");
    let s_os = make_string(heap, "output_size");
    make_dict(heap, vec![
        (s_type, s_linear),
        (s_w, w),
        (s_b, b),
        (s_gw, gw),
        (s_gb, gb),
        (s_is, Value::Int(input_size as i64)),
        (s_os, Value::Int(output_size as i64)),
    ])
}

fn layer_forward(layer: &Value, input: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let layer_ref = match layer { Value::Dict(r) => *r, _ => return Err("layer must be a dict".to_string()) };
    let entries = dict_get_entries(layer, heap)?;
    let t = get_type(&entries, heap)?;
    match t.as_str() {
        "linear" => {
            let w = dict_get_str(&entries, "weights", heap).ok_or("no weights")?.clone();
            let b = dict_get_str(&entries, "bias", heap).ok_or("no bias")?.clone();
            drop(entries);
            let z = mat_mul(input, &w, heap)?;
            let (zr, zc, zd) = mat_data(&z, heap)?;
            let (_, _, bd) = mat_data(&b, heap)?;
            let out_data: Vec<f64> = zd.iter().enumerate().map(|(i, v)| v + bd[i % zc.min(bd.len())]).collect();
            let out = make_matrix(heap, zr, zc, out_data);
            dict_set_entry(layer_ref, heap, "_in", input.clone());
            Ok(out)
        }
        "relu" => { drop(entries); dict_set_entry(layer_ref, heap, "_in", input.clone()); relu_act(input, heap) }
        "sigmoid" => {
            drop(entries);
            let out = sigmoid_act(input, heap)?;
            dict_set_entry(layer_ref, heap, "_in", input.clone());
            dict_set_entry(layer_ref, heap, "_out", out.clone());
            Ok(out)
        }
        "tanh" => {
            drop(entries);
            let out = tanh_act(input, heap)?;
            dict_set_entry(layer_ref, heap, "_in", input.clone());
            dict_set_entry(layer_ref, heap, "_out", out.clone());
            Ok(out)
        }
        "sequential" => {
            let layers = dict_get_str(&entries, "layers", heap).ok_or("no layers in sequential")?.clone();
            drop(entries);
            let layer_list = match &layers {
                Value::List(r) => match heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("internal error".to_string()) },
                _ => return Err("internal error".to_string()),
            };
            let mut cur = input.clone();
            for l in &layer_list { cur = layer_forward(l, &cur, heap)?; }
            dict_set_entry(layer_ref, heap, "_last_input", input.clone());
            Ok(cur)
        }
        _ => Err(format!("unknown layer type: {}", t)),
    }
}

fn layer_backward(layer: &Value, grad_output: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let layer_ref = match layer { Value::Dict(r) => *r, _ => return Err("layer must be a dict".to_string()) };
    let entries = dict_get_entries(layer, heap)?;
    let t = get_type(&entries, heap)?;
    match t.as_str() {
        "linear" => {
            let w = dict_get_str(&entries, "weights", heap).ok_or("no weights")?.clone();
            let x = dict_get_str(&entries, "_in", heap).ok_or("linear missing cache")?.clone();
            drop(entries);
            let x_t = mat_transpose(&x, heap)?;
            let dw = mat_mul(&x_t, grad_output, heap)?;
            let db = sum_rows(grad_output, heap)?;
            let (dwr, dwc, dw_data) = mat_data(&dw, heap)?;
            if let Some(gw) = (|| { if let GcObj::Dict(ref e) = heap.get(layer_ref) { dict_get_str(e, "grad_weights", heap).cloned() } else { None } })() {
                let (gwr, gwc, mut gwd) = mat_data(&gw, heap)?;
                if gwr == dwr && gwc == dwc {
                    for (g, d) in gwd.iter_mut().zip(dw_data.iter()) { *g += d; }
                    let new_gw = make_matrix(heap, gwr, gwc, gwd);
                    dict_set_entry(layer_ref, heap, "grad_weights", new_gw);
                }
            }
            let db_data = mat_data(&db, heap)?.2;
            if let Some(gb) = (|| { if let GcObj::Dict(ref e) = heap.get(layer_ref) { dict_get_str(e, "grad_bias", heap).cloned() } else { None } })() {
                let (gbr, gbc, mut gbd) = mat_data(&gb, heap)?;
                if gbr == 1 && gbc == db_data.len() {
                    for (g, d) in gbd.iter_mut().zip(db_data.iter()) { *g += d; }
                    let new_gb = make_matrix(heap, gbr, gbc, gbd);
                    dict_set_entry(layer_ref, heap, "grad_bias", new_gb);
                }
            }
            let w_t = mat_transpose(&w, heap)?;
            mat_mul(grad_output, &w_t, heap)
        }
        "relu" => {
            let x = dict_get_str(&entries, "_in", heap).ok_or("relu missing cache")?.clone();
            drop(entries);
            let (r, c, xd) = mat_data(&x, heap)?;
            let (_, _, god) = mat_data(grad_output, heap)?;
            Ok(make_matrix(heap, r, c, xd.iter().zip(god.iter()).map(|(xv, gv)| if *xv > 0.0 { *gv } else { 0.0 }).collect()))
        }
        "sigmoid" => {
            let out = dict_get_str(&entries, "_out", heap).ok_or("sigmoid missing cache")?.clone();
            drop(entries);
            let (r, c, od) = mat_data(&out, heap)?;
            let (_, _, god) = mat_data(grad_output, heap)?;
            Ok(make_matrix(heap, r, c, od.iter().zip(god.iter()).map(|(ov, gv)| gv * ov * (1.0 - ov)).collect()))
        }
        "tanh" => {
            let out = dict_get_str(&entries, "_out", heap).ok_or("tanh missing cache")?.clone();
            drop(entries);
            let (r, c, od) = mat_data(&out, heap)?;
            let (_, _, god) = mat_data(grad_output, heap)?;
            Ok(make_matrix(heap, r, c, od.iter().zip(god.iter()).map(|(ov, gv)| gv * (1.0 - ov * ov)).collect()))
        }
        "sequential" => {
            let layers = dict_get_str(&entries, "layers", heap).ok_or("no layers")?.clone();
            drop(entries);
            let layer_list = match &layers {
                Value::List(r) => match heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("internal error".to_string()) },
                _ => return Err("internal error".to_string()),
            };
            let mut grad = grad_output.clone();
            for l in layer_list.iter().rev() { grad = layer_backward(l, &grad, heap)?; }
            Ok(grad)
        }
        _ => Err(format!("unknown layer type in backward: {}", t)),
    }
}

fn zero_grad_model(model: &Value, heap: &mut GcHeap) -> Result<(), String> {
    let model_ref = match model { Value::Dict(r) => *r, _ => return Err("model must be a dict".to_string()) };
    let entries = dict_get_entries(model, heap)?;
    let t = get_type(&entries, heap)?;
    match t.as_str() {
        "linear" => {
            drop(entries);
            fn zero_dict_entry(heap: &mut GcHeap, r: ObjRef, key: &str) {
                let v = (|| { if let GcObj::Dict(ref e) = heap.get(r) { dict_get_str(e, key, heap).cloned() } else { None } })();
                if let Some(m) = v {
                    if let Ok((rr, cc, _)) = mat_data(&m, heap) {
                        let zm = make_matrix(heap, rr, cc, vec![0.0; rr * cc]);
                        dict_set_entry(r, heap, key, zm);
                    }
                }
            }
            zero_dict_entry(heap, model_ref, "grad_weights");
            zero_dict_entry(heap, model_ref, "grad_bias");
        }
        "sequential" => {
            let layers = dict_get_str(&entries, "layers", heap).ok_or("no layers")?.clone();
            drop(entries);
            let layer_list = match &layers {
                Value::List(r) => match heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("internal error".to_string()) },
                _ => return Err("internal error".to_string()),
            };
            for l in &layer_list { zero_grad_model(l, heap)?; }
        }
        _ => {}
    }
    Ok(())
}

fn sgd_step(model: &Value, lr: f64, heap: &mut GcHeap) -> Result<(), String> {
    let model_ref = match model { Value::Dict(r) => *r, _ => return Err("model must be a dict".to_string()) };
    let entries = dict_get_entries(model, heap)?;
    let t = get_type(&entries, heap)?;
    match t.as_str() {
        "linear" => {
            drop(entries);
            fn update_param(heap: &mut GcHeap, r: ObjRef, pkey: &str, gkey: &str, lr: f64) {
                let (p, g) = (|| {
                    if let GcObj::Dict(ref e) = heap.get(r) {
                        (dict_get_str(e, pkey, heap).cloned(), dict_get_str(e, gkey, heap).cloned())
                    } else { (None, None) }
                })();
                if let (Some(pv), Some(gv)) = (p, g) {
                    if let Ok((pr, pc, pd)) = mat_data(&pv, heap) {
                        if let Ok((_, _, gd)) = mat_data(&gv, heap) {
                            let nd: Vec<f64> = pd.iter().zip(gd.iter()).map(|(x, g)| x - lr * g).collect();
                            let new_p = make_matrix(heap, pr, pc, nd);
                            dict_set_entry(r, heap, pkey, new_p);
                        }
                    }
                }
            }
            update_param(heap, model_ref, "weights", "grad_weights", lr);
            update_param(heap, model_ref, "bias", "grad_bias", lr);
        }
        "sequential" => {
            let layers = dict_get_str(&entries, "layers", heap).ok_or("no layers")?.clone();
            drop(entries);
            let layer_list = match &layers {
                Value::List(r) => match heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("internal error".to_string()) },
                _ => return Err("internal error".to_string()),
            };
            for l in &layer_list { sgd_step(l, lr, heap)?; }
        }
        _ => {}
    }
    Ok(())
}

fn mse_loss(pred: &Value, target: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (pr, pc, pd) = mat_data(pred, heap)?;
    let (tr, tc, td) = mat_data(target, heap)?;
    if pr != tr || pc != tc { return Err(format!("MSELoss dim: ({}x{}) vs ({}x{})", pr, pc, tr, tc)); }
    let n = (pr * pc) as f64;
    let lv: f64 = pd.iter().zip(td.iter()).map(|(p, t)| (p - t) * (p - t)).sum::<f64>() / n;
    let g: Vec<f64> = pd.iter().zip(td.iter()).map(|(p, t)| 2.0 * (p - t) / n).collect();
    let s_type = make_string(heap, "type");
    let s_mse = make_string(heap, "mse_loss");
    let s_val = make_string(heap, "val");
    let s_grad = make_string(heap, "grad");
    let s_pred = make_string(heap, "pred");
    let s_target = make_string(heap, "target");
    let v_mat = make_matrix(heap, 1, 1, vec![lv]);
    let g_mat = make_matrix(heap, pr, pc, g);
    Ok(make_dict(heap, vec![
        (s_type, s_mse),
        (s_val, v_mat),
        (s_grad, g_mat),
        (s_pred, pred.clone()),
        (s_target, target.clone()),
    ]))
}

fn cross_entropy_loss(pred: &Value, target: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    let (pr, pc, pd) = mat_data(pred, heap)?;
    let (tr, tc, td) = mat_data(target, heap)?;
    if pr != tr || pc != tc { return Err(format!("CrossEntropy dim: ({}x{}) vs ({}x{})", pr, pc, tr, tc)); }
    let mut lv = 0.0;
    let mut g = vec![0.0; pr * pc];
    for i in 0..pr {
        let mut maxv = f64::NEG_INFINITY;
        for j in 0..pc { let v = pd[i * pc + j]; if v > maxv { maxv = v; } }
        let mut sum_exp = 0.0;
        for j in 0..pc { sum_exp += (pd[i * pc + j] - maxv).exp(); }
        for j in 0..pc {
            let soft = (pd[i * pc + j] - maxv).exp() / sum_exp;
            if td[i * pc + j] > 0.5 { lv += -soft.ln().max(-100.0); }
            g[i * pc + j] = (soft - td[i * pc + j]) / pr as f64;
        }
    }
    let s_type = make_string(heap, "type");
    let s_ce = make_string(heap, "cross_entropy_loss");
    let s_val = make_string(heap, "val");
    let s_grad = make_string(heap, "grad");
    let s_pred = make_string(heap, "pred");
    let s_target = make_string(heap, "target");
    let v_mat = make_matrix(heap, 1, 1, vec![lv / pr as f64]);
    let g_mat = make_matrix(heap, pr, pc, g);
    Ok(make_dict(heap, vec![
        (s_type, s_ce),
        (s_val, v_mat),
        (s_grad, g_mat),
        (s_pred, pred.clone()),
        (s_target, target.clone()),
    ]))
}

pub fn build_linum() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    fn linear_layer(args: &[Value], ctx: &mut VmContext) -> Result<Value, String> {
        if args.len() < 2 { return Err("Linear requires input_size and output_size".to_string()); }
        let input_size = match &args[0] { Value::Int(n) => *n as usize, _ => return Err("input_size must be an int".to_string()) };
        let output_size = match &args[1] { Value::Int(n) => *n as usize, _ => return Err("output_size must be an int".to_string()) };
        Ok(make_linear_layer(ctx.heap, input_size, output_size))
    }

    funcs.push(("Linear".to_string(), Value::NativeFunc(NativeFunc { name: "<linum.Linear>".to_string(), func: Rc::new(linear_layer) })));
    funcs.push(("ReLU".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.ReLU>".to_string(),
        func: Rc::new(|_, ctx| {
            let s_type = make_string(ctx.heap, "type");
            let s_r = make_string(ctx.heap, "relu");
            Ok(make_dict(ctx.heap, vec![(s_type, s_r)]))
        }),
    })));
    funcs.push(("Sigmoid".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.Sigmoid>".to_string(),
        func: Rc::new(|_, ctx| {
            let s_type = make_string(ctx.heap, "type");
            let s_s = make_string(ctx.heap, "sigmoid");
            Ok(make_dict(ctx.heap, vec![(s_type, s_s)]))
        }),
    })));
    funcs.push(("Tanh".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.Tanh>".to_string(),
        func: Rc::new(|_, ctx| {
            let s_type = make_string(ctx.heap, "type");
            let s_t = make_string(ctx.heap, "tanh");
            Ok(make_dict(ctx.heap, vec![(s_type, s_t)]))
        }),
    })));
    funcs.push(("Sequential".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.Sequential>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("Sequential requires a list of layers".to_string()); }
            let s_type = make_string(ctx.heap, "type");
            let s_seq = make_string(ctx.heap, "sequential");
            let s_layers = make_string(ctx.heap, "layers");
            Ok(make_dict(ctx.heap, vec![
                (s_type, s_seq),
                (s_layers, args[0].clone()),
            ]))
        }),
    })));

    funcs.push(("forward".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.forward>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("forward requires model and input".to_string()); }
            layer_forward(&args[0], &args[1], ctx.heap)
        }),
    })));

    funcs.push(("backward".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.backward>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("backward requires model and loss".to_string()); }
            let entries = dict_get_entries(&args[1], ctx.heap)?;
            let grad = dict_get_str(&entries, "grad", ctx.heap).ok_or("loss has no grad")?.clone();
            drop(entries);
            layer_backward(&args[0], &grad, ctx.heap)
        }),
    })));

    funcs.push(("zero_grad".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.zero_grad>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("zero_grad requires a model".to_string()); }
            zero_grad_model(&args[0], ctx.heap)?;
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("MSELoss".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.MSELoss>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("MSELoss requires pred and target".to_string()); }
            mse_loss(&args[0], &args[1], ctx.heap)
        }),
    })));

    funcs.push(("CrossEntropyLoss".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.CrossEntropyLoss>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("CrossEntropyLoss requires pred and target".to_string()); }
            cross_entropy_loss(&args[0], &args[1], ctx.heap)
        }),
    })));

    // SGD
    funcs.push(("SGD".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.SGD>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("SGD requires model and lr".to_string()); }
            let model = args[0].clone();
            let lr = match &args[1] { Value::Float(f) => *f, Value::Int(i) => *i as f64, _ => return Err("lr must be a number".to_string()) };
            let s_type = make_string(ctx.heap, "type");
            let s_sgd = make_string(ctx.heap, "sgd");
            let s_model = make_string(ctx.heap, "model");
            let s_lr = make_string(ctx.heap, "lr");
            let opt_r = ctx.heap.alloc(GcObj::Dict(vec![
                (s_type, s_sgd),
                (s_model, model),
                (s_lr, Value::Float(lr)),
            ]));
            let oc = opt_r;
            let step_fn = Value::NativeFunc(NativeFunc {
                name: "<opt.step>".to_string(),
                func: Rc::new(move |_, ctx2| {
                    if let GcObj::Dict(ref e) = ctx2.heap.get(oc) {
                        let m = dict_get_str(e, "model", ctx2.heap).ok_or("no model")?.clone();
                        let lr2 = match dict_get_str(e, "lr", ctx2.heap) {
                            Some(Value::Float(f)) => *f, Some(Value::Int(i)) => *i as f64, _ => 0.01,
                        };
                        sgd_step(&m, lr2, ctx2.heap)?;
                    }
                    Ok(Value::Nil)
                }),
            });
            let zf = Value::NativeFunc(NativeFunc {
                name: "<opt.zero_grad>".to_string(),
                func: Rc::new(move |_, ctx2| {
                    if let GcObj::Dict(ref e) = ctx2.heap.get(oc) {
                        let m = dict_get_str(e, "model", ctx2.heap).ok_or("no model")?.clone();
                        zero_grad_model(&m, ctx2.heap)?;
                    }
                    Ok(Value::Nil)
                }),
            });
            let s_step = make_string(ctx.heap, "step");
            let s_zg = make_string(ctx.heap, "zero_grad");
            if let GcObj::Dict(ref mut e) = ctx.heap.get_mut(opt_r) {
                e.push((s_step, step_fn));
                e.push((s_zg, zf));
            }
            Ok(Value::Dict(opt_r))
        }),
    })));

    // Adam
    funcs.push(("Adam".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.Adam>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("Adam requires model and lr".to_string()); }
            let model = args[0].clone();
            let lr = match &args[1] { Value::Float(f) => *f, Value::Int(i) => *i as f64, _ => return Err("lr must be a number".to_string()) };
            let beta1 = if args.len() > 2 { match &args[2] { Value::Float(f) => *f, _ => 0.9 } } else { 0.9 };
            let beta2 = if args.len() > 3 { match &args[3] { Value::Float(f) => *f, _ => 0.999 } } else { 0.999 };
            let s_type = make_string(ctx.heap, "type");
            let s_adam = make_string(ctx.heap, "adam");
            let s_model = make_string(ctx.heap, "model");
            let s_lr = make_string(ctx.heap, "lr");
            let s_b1 = make_string(ctx.heap, "beta1");
            let s_b2 = make_string(ctx.heap, "beta2");
            let s_t = make_string(ctx.heap, "_t");
            let opt_r = ctx.heap.alloc(GcObj::Dict(vec![
                (s_type, s_adam),
                (s_model, model),
                (s_lr, Value::Float(lr)),
                (s_b1, Value::Float(beta1)),
                (s_b2, Value::Float(beta2)),
                (s_t, Value::Int(0)),
            ]));
            let oc = opt_r;
            let step_fn = Value::NativeFunc(NativeFunc {
                name: "<opt.step>".to_string(),
                func: Rc::new(move |_, ctx2| {
                    if let GcObj::Dict(ref e) = ctx2.heap.get(oc) {
                        let m = dict_get_str(e, "model", ctx2.heap).ok_or("no model")?.clone();
                        let lr2 = match dict_get_str(e, "lr", ctx2.heap) { Some(Value::Float(f)) => *f, Some(Value::Int(i)) => *i as f64, _ => 0.001 };
                        let b1 = match dict_get_str(e, "beta1", ctx2.heap) { Some(Value::Float(f)) => *f, _ => 0.9 };
                        let b2 = match dict_get_str(e, "beta2", ctx2.heap) { Some(Value::Float(f)) => *f, _ => 0.999 };
                        let t = match dict_get_str(e, "_t", ctx2.heap) { Some(Value::Int(n)) => *n + 1, _ => 1i64 };
                        dict_set_entry(oc, ctx2.heap, "_t", Value::Int(t));
                        let bt1 = 1.0 - b1.powi(t as i32);
                        let bt2 = 1.0 - b2.powi(t as i32);
                        adam_step_inner(&m, lr2, b1, b2, 1e-8, bt1, bt2, ctx2.heap)?;
                    }
                    Ok(Value::Nil)
                }),
            });
            let zf = Value::NativeFunc(NativeFunc {
                name: "<opt.zero_grad>".to_string(),
                func: Rc::new(move |_, ctx2| {
                    if let GcObj::Dict(ref e) = ctx2.heap.get(oc) {
                        let m = dict_get_str(e, "model", ctx2.heap).ok_or("no model")?.clone();
                        zero_grad_model(&m, ctx2.heap)?;
                    }
                    Ok(Value::Nil)
                }),
            });
            let s_step = make_string(ctx.heap, "step");
            let s_zg = make_string(ctx.heap, "zero_grad");
            if let GcObj::Dict(ref mut e) = ctx.heap.get_mut(opt_r) {
                e.push((s_step, step_fn));
                e.push((s_zg, zf));
            }
            Ok(Value::Dict(opt_r))
        }),
    })));

    // train
    funcs.push(("train".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.train>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 3 { return Err("train requires model, inputs, targets, [options]".to_string()); }
            let model = args[0].clone();
            let inputs = args[1].clone();
            let targets = args[2].clone();
            let mut epochs = 10i64;
            let mut lr = 0.01f64;
            let mut loss_type = "mse".to_string();
            let mut verbose = false;
            if args.len() > 3 {
                if let Value::Dict(r) = &args[3] {
                    if let GcObj::Dict(ref opts) = ctx.heap.get(*r) {
                        if let Some(Value::Int(n)) = dict_get_str(opts, "epochs", ctx.heap) { epochs = *n; }
                        if let Some(v) = dict_get_str(opts, "lr", ctx.heap) { lr = to_f64(v); }
                        if let Some(Value::String(s)) = dict_get_str(opts, "loss", ctx.heap) {
                            if let GcObj::String(st) = ctx.heap.get(*s) { loss_type = st.clone(); }
                        }
                        if let Some(Value::Bool(b)) = dict_get_str(opts, "verbose", ctx.heap) { verbose = *b; }
                    }
                }
            }
            let mut history = Vec::new();
            let report_every = (epochs / 10).max(1);
            for epoch in 0..epochs {
                let out = layer_forward(&model, &inputs, ctx.heap)?;
                let loss = match loss_type.as_str() {
                    "cross_entropy" => cross_entropy_loss(&out, &targets, ctx.heap)?,
                    _ => mse_loss(&out, &targets, ctx.heap)?,
                };
                let loss_val = {
                    let le = dict_get_entries(&loss, ctx.heap)?;
                    let v = dict_get_str(&le, "val", ctx.heap).cloned();
                    drop(le);
                    match v { Some(Value::Matrix(r)) => if let GcObj::Matrix { data, .. } = ctx.heap.get(r) { data[0] } else { 0.0 }, _ => 0.0 }
                };
                history.push(Value::Float(loss_val));
                if verbose && (epoch + 1) % report_every == 0 { println!("[Epoch {}/{}] loss: {:.6}", epoch + 1, epochs, loss_val); }
                let grad = {
                    let le = dict_get_entries(&loss, ctx.heap)?;
                    let g = dict_get_str(&le, "grad", ctx.heap).cloned();
                    drop(le);
                    g.ok_or("loss has no grad")?
                };
                layer_backward(&model, &grad, ctx.heap)?;
                sgd_step(&model, lr, ctx.heap)?;
                zero_grad_model(&model, ctx.heap)?;
            }
            Ok(make_list(ctx.heap, history))
        }),
    })));

    // backward-compatible aliases
    funcs.push(("dense".to_string(), Value::NativeFunc(NativeFunc { name: "<linum.dense>".to_string(), func: Rc::new(linear_layer) })));
    funcs.push(("relu".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.relu>".to_string(),
        func: Rc::new(|_, ctx| {
            let s_type = make_string(ctx.heap, "type");
            let s_r = make_string(ctx.heap, "relu");
            Ok(make_dict(ctx.heap, vec![(s_type, s_r)]))
        }),
    })));
    funcs.push(("sigmoid".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.sigmoid>".to_string(),
        func: Rc::new(|_, ctx| {
            let s_type = make_string(ctx.heap, "type");
            let s_s = make_string(ctx.heap, "sigmoid");
            Ok(make_dict(ctx.heap, vec![(s_type, s_s)]))
        }),
    })));
    funcs.push(("sequential".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.sequential>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("sequential requires a list of layers".to_string()); }
            let s_type = make_string(ctx.heap, "type");
            let s_seq = make_string(ctx.heap, "sequential");
            let s_layers = make_string(ctx.heap, "layers");
            Ok(make_dict(ctx.heap, vec![
                (s_type, s_seq),
                (s_layers, args[0].clone()),
            ]))
        }),
    })));
    funcs.push(("get_device".to_string(), Value::NativeFunc(NativeFunc {
        name: "<linum.get_device>".to_string(),
        func: Rc::new(|_, ctx| {
            let device = match cuda::device_count() {
                Ok(n) if n > 0 => "cuda",
                _ => "cpu",
            };
            Ok(make_string(ctx.heap, device))
        }),
    })));

    funcs
}

fn adam_step_inner(model: &Value, lr: f64, beta1: f64, beta2: f64, eps: f64, bt1: f64, bt2: f64, heap: &mut GcHeap) -> Result<(), String> {
    let model_ref = match model { Value::Dict(r) => *r, _ => return Err("model must be a dict".to_string()) };
    let entries = dict_get_entries(model, heap)?;
    let t = get_type(&entries, heap)?;
    let apply = |entries: &[(Value, Value)], heap: &mut GcHeap, pk: &str, gk: &str, mk: &str, vk: &str| {
        let p = match dict_get_str(entries, pk, heap) { Some(v) => v.clone(), None => return };
        let g = match dict_get_str(entries, gk, heap) { Some(v) => v.clone(), None => return };
        if let (Ok((pr, pc, pd)), Ok((_, _, gd))) = (mat_data(&p, heap), mat_data(&g, heap)) {
            if pd.is_empty() || gd.is_empty() { return; }
            let pm = match dict_get_str(entries, mk, heap) { Some(v) => mat_data(v, heap).unwrap_or((0, 0, vec![])).2, None => vec![0.0; pd.len()] };
            let pv = match dict_get_str(entries, vk, heap) { Some(v) => mat_data(v, heap).unwrap_or((0, 0, vec![])).2, None => vec![0.0; pd.len()] };
            let mut nm = Vec::with_capacity(pd.len());
            let mut nv = Vec::with_capacity(pd.len());
            let mut np = Vec::with_capacity(pd.len());
            for i in 0..pd.len() {
                let mi = beta1 * pm.get(i).copied().unwrap_or(0.0) + (1.0 - beta1) * gd[i];
                let vi = beta2 * pv.get(i).copied().unwrap_or(0.0) + (1.0 - beta2) * gd[i] * gd[i];
                np.push(pd[i] - lr * (mi / bt1) / ((vi / bt2).sqrt() + eps));
                nm.push(mi);
                nv.push(vi);
            }
            let m1 = make_matrix(heap, pr, pc, np);
            let m2 = make_matrix(heap, pr, pc, nm);
            let m3 = make_matrix(heap, pr, pc, nv);
            dict_set_entry(model_ref, heap, pk, m1);
            dict_set_entry(model_ref, heap, mk, m2);
            dict_set_entry(model_ref, heap, vk, m3);
        }
    };
    match t.as_str() {
        "linear" => { apply(&entries, heap, "weights", "grad_weights", "m_w", "v_w"); apply(&entries, heap, "bias", "grad_bias", "m_b", "v_b"); }
        "sequential" => {
            let layers = dict_get_str(&entries, "layers", heap).ok_or("no layers")?.clone();
            drop(entries);
            let layer_list = match &layers { Value::List(r) => match heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("internal error".to_string()) }, _ => return Err("internal error".to_string()) };
            for l in &layer_list { adam_step_inner(l, lr, beta1, beta2, eps, bt1, bt2, heap)?; }
        }
        _ => {}
    }
    Ok(())
}

fn cuda_ok() -> bool { crate::cuda::is_initialized() }

fn gpu_matmul(m: i32, n: i32, k: i32, a: &[f64], b: &[f64]) -> Result<Vec<f64>, String> {
    if !cuda_ok() { return Err("no cuda".to_string()); }
    let a_sz = (m * k) as usize * 8;
    let b_sz = (k * n) as usize * 8;
    let c_sz = (m * n) as usize * 8;
    let a_g = cuda::gpu_alloc(a_sz)?;
    let b_g = cuda::gpu_alloc(b_sz)?;
    let c_g = cuda::gpu_alloc(c_sz)?;
    let r = (|| {
        cuda::copy_to_gpu(a_g, a.as_ptr() as *const std::ffi::c_void, a_sz)?;
        cuda::copy_to_gpu(b_g, b.as_ptr() as *const std::ffi::c_void, b_sz)?;
        cuda::blas_dgemm(false, false, m, n, k, 1.0, a_g as *const f64, m, b_g as *const f64, k, 0.0, c_g as *mut f64, m)?;
        cuda::sync()?;
        let mut r = vec![0.0; (m * n) as usize];
        cuda::copy_to_cpu(r.as_mut_ptr() as *mut std::ffi::c_void, c_g, c_sz)?;
        Ok(r)
    })();
    let _ = cuda::gpu_free(a_g);
    let _ = cuda::gpu_free(b_g);
    let _ = cuda::gpu_free(c_g);
    r
}

fn gpu_transpose(rows: i32, cols: i32, data: &[f64]) -> Result<Vec<f64>, String> {
    if !cuda_ok() { return Err("no cuda".to_string()); }
    let sz = (rows * cols) as usize * 8;
    let sg = cuda::gpu_alloc(sz)?;
    let dg = cuda::gpu_alloc(sz)?;
    let r = (|| {
        cuda::copy_to_gpu(sg, data.as_ptr() as *const std::ffi::c_void, sz)?;
        let func = cuda::get_kernel("transpose")?;
        let func = cuda::get_kernel("transpose")?;
        let args: [*mut std::ffi::c_void; 4] = unsafe { [&mut *sg as *mut _, &mut *dg as *mut _, &rows as *const i32 as *mut _, &cols as *const i32 as *mut _] };
        cuda::launch_2d(func, cols as u32, rows as u32, &args)?;
        cuda::sync()?;
        let mut r = vec![0.0; (rows * cols) as usize];
        cuda::copy_to_cpu(r.as_mut_ptr() as *mut std::ffi::c_void, dg, sz)?;
        Ok(r)
    })();
    let _ = cuda::gpu_free(sg);
    let _ = cuda::gpu_free(dg);
    r
}

fn gpu_relu(data: &[f64]) -> Result<Vec<f64>, String> {
    if !cuda_ok() { return Err("no cuda".to_string()); }
    let n = data.len();
    let sz = n * 8;
    let dg = cuda::gpu_alloc(sz)?;
    let rg = cuda::gpu_alloc(sz)?;
    let r = (|| {
        cuda::copy_to_gpu(dg, data.as_ptr() as *const std::ffi::c_void, sz)?;
        let func = get_kernels().relu;
        let args: [*mut std::ffi::c_void; 3] = unsafe { [&mut *dg as *mut _, &mut *rg as *mut _, &n as *const usize as *mut _] };
        cuda::launch_1d(func, n, &args)?;
        cuda::sync()?;
        let mut r = vec![0.0; n];
        cuda::copy_to_cpu(r.as_mut_ptr() as *mut std::ffi::c_void, rg, sz)?;
        Ok(r)
    })();
    let _ = cuda::gpu_free(dg);
    let _ = cuda::gpu_free(rg);
    r
}

fn gpu_sigmoid(data: &[f64]) -> Result<Vec<f64>, String> {
    if !cuda_ok() { return Err("no cuda".to_string()); }
    let n = data.len();
    let sz = n * 8;
    let dg = cuda::gpu_alloc(sz)?;
    let rg = cuda::gpu_alloc(sz)?;
    let r = (|| {
        cuda::copy_to_gpu(dg, data.as_ptr() as *const std::ffi::c_void, sz)?;
        let func = get_kernels().sigmoid;
        let args: [*mut std::ffi::c_void; 3] = unsafe { [&mut *dg as *mut _, &mut *rg as *mut _, &n as *const usize as *mut _] };
        cuda::launch_1d(func, n, &args)?;
        cuda::sync()?;
        let mut r = vec![0.0; n];
        cuda::copy_to_cpu(r.as_mut_ptr() as *mut std::ffi::c_void, rg, sz)?;
        Ok(r)
    })();
    let _ = cuda::gpu_free(dg);
    let _ = cuda::gpu_free(rg);
    r
}

fn gpu_tanh(data: &[f64]) -> Result<Vec<f64>, String> {
    if !cuda_ok() { return Err("no cuda".to_string()); }
    let n = data.len();
    let sz = n * 8;
    let dg = cuda::gpu_alloc(sz)?;
    let rg = cuda::gpu_alloc(sz)?;
    let r = (|| {
        cuda::copy_to_gpu(dg, data.as_ptr() as *const std::ffi::c_void, sz)?;
        let func = get_kernels().tanh;
        let args: [*mut std::ffi::c_void; 3] = unsafe { [&mut *dg as *mut _, &mut *rg as *mut _, &n as *const usize as *mut _] };
        cuda::launch_1d(func, n, &args)?;
        cuda::sync()?;
        let mut r = vec![0.0; n];
        cuda::copy_to_cpu(r.as_mut_ptr() as *mut std::ffi::c_void, rg, sz)?;
        Ok(r)
    })();
    let _ = cuda::gpu_free(dg);
    let _ = cuda::gpu_free(rg);
    r
}
