use std::rc::Rc;
use sha2::{Sha256, Sha512, Digest};
use crate::gc::*;

pub fn build_hashlib() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("sha256".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.sha256>".to_string(),
        func: Rc::new(|args, ctx| {
            let data = args.first().ok_or("hashlib.sha256 requires data")?.to_string(ctx.heap);
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let result = hex_encode(&hasher.finalize());
            Ok(make_string(ctx.heap, &result))
        }),
    })));

    funcs.push(("sha512".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.sha512>".to_string(),
        func: Rc::new(|args, ctx| {
            let data = args.first().ok_or("hashlib.sha512 requires data")?.to_string(ctx.heap);
            let mut hasher = Sha512::new();
            hasher.update(data.as_bytes());
            let result = hex_encode(&hasher.finalize());
            Ok(make_string(ctx.heap, &result))
        }),
    })));

    funcs.push(("md5".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.md5>".to_string(),
        func: Rc::new(|args, ctx| {
            let data = args.first().ok_or("hashlib.md5 requires data")?.to_string(ctx.heap);
            let mut hasher = md5::Md5::new();
            hasher.update(data.as_bytes());
            let result = hex_encode(&hasher.finalize());
            Ok(make_string(ctx.heap, &result))
        }),
    })));

    funcs.push(("sha1".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.sha1>".to_string(),
        func: Rc::new(|args, ctx| {
            let data = args.first().ok_or("hashlib.sha1 requires data")?.to_string(ctx.heap);
            let mut hasher = sha1::Sha1::new();
            hasher.update(data.as_bytes());
            let result = hex_encode(&hasher.finalize());
            Ok(make_string(ctx.heap, &result))
        }),
    })));

    funcs.push(("base64_encode".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.base64_encode>".to_string(),
        func: Rc::new(|args, ctx| {
            let data = args.first().ok_or("hashlib.base64_encode requires data")?.to_string(ctx.heap);
            use base64::Engine;
            let result = base64::engine::general_purpose::STANDARD.encode(data.as_bytes());
            Ok(make_string(ctx.heap, &result))
        }),
    })));

    funcs.push(("base64_decode".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.base64_decode>".to_string(),
        func: Rc::new(|args, ctx| {
            let data = args.first().ok_or("hashlib.base64_decode requires data")?.to_string(ctx.heap);
            use base64::Engine;
            match base64::engine::general_purpose::STANDARD.decode(data.as_bytes()) {
                Ok(bytes) => Ok(make_string(ctx.heap, &String::from_utf8_lossy(&bytes))),
                Err(e) => Err(format!("hashlib.base64_decode: {}", e)),
            }
        }),
    })));

    funcs.push(("hex_encode".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.hex_encode>".to_string(),
        func: Rc::new(|args, ctx| {
            let data = args.first().ok_or("hashlib.hex_encode requires data")?.to_string(ctx.heap);
            Ok(make_string(ctx.heap, &hex_encode(data.as_bytes())))
        }),
    })));

    funcs.push(("hex_decode".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.hex_decode>".to_string(),
        func: Rc::new(|args, ctx| {
            let data = args.first().ok_or("hashlib.hex_decode requires hex string")?.to_string(ctx.heap);
            match hex_decode(&data) {
                Ok(bytes) => Ok(make_string(ctx.heap, &String::from_utf8_lossy(&bytes))),
                Err(e) => Err(format!("hashlib.hex_decode: {}", e)),
            }
        }),
    })));

    funcs
}

fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for byte in data {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}

fn hex_decode(hex_str: &str) -> Result<Vec<u8>, String> {
    let hex = hex_str.trim();
    if hex.len() % 2 != 0 {
        return Err("hex string length must be even".to_string());
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[i..i+2], 16)
            .map_err(|_| format!("invalid hex character at position {}", i))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

mod sha1 {
    use sha2::Digest;

    pub struct Sha1 {
        hasher: sha2::Sha256,
    }

    impl Sha1 {
        pub fn new() -> Self {
            Sha1 { hasher: sha2::Sha256::new() }
        }

        pub fn update(&mut self, data: &[u8]) {
            self.hasher.update(data);
        }

        pub fn finalize(self) -> Vec<u8> {
            let result = self.hasher.finalize();
            result[..20].to_vec()
        }
    }
}
