use std::rc::Rc;
use sha2::{Sha256, Sha512, Digest};
use md5::Md5;
use base64::Engine;
use crate::gc::*;

static HEX_TABLE: &[u8; 16] = b"0123456789abcdef";

pub fn build_hashlib() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("sha256".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.sha256>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("hashlib.sha256 requires data")?;
            let s = match val {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.as_str(),
                    _ => return Err("invalid string".to_string()),
                },
                _ => return Err("expected string".to_string()),
            };
            Ok(make_string_owned(ctx.heap, hex_encode(&Sha256::digest(s.as_bytes()))))
        }),
    })));

    funcs.push(("sha512".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.sha512>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("hashlib.sha512 requires data")?;
            let s = match val {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.as_str(),
                    _ => return Err("invalid string".to_string()),
                },
                _ => return Err("expected string".to_string()),
            };
            Ok(make_string_owned(ctx.heap, hex_encode(&Sha512::digest(s.as_bytes()))))
        }),
    })));

    funcs.push(("md5".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.md5>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("hashlib.md5 requires data")?;
            let s = match val {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.as_str(),
                    _ => return Err("invalid string".to_string()),
                },
                _ => return Err("expected string".to_string()),
            };
            Ok(make_string_owned(ctx.heap, hex_encode(&Md5::digest(s.as_bytes()))))
        }),
    })));

    funcs.push(("sha1".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.sha1>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("hashlib.sha1 requires data")?;
            let s = match val {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.as_str(),
                    _ => return Err("invalid string".to_string()),
                },
                _ => return Err("expected string".to_string()),
            };
            let hash = sha1::Sha1::digest(s.as_bytes());
            Ok(make_string_owned(ctx.heap, hex_encode(&hash)))
        }),
    })));

    funcs.push(("base64_encode".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.base64_encode>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("hashlib.base64_encode requires data")?;
            let s = match val {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.as_str(),
                    _ => return Err("invalid string".to_string()),
                },
                _ => return Err("expected string".to_string()),
            };
            let result = base64::engine::general_purpose::STANDARD.encode(s.as_bytes());
            Ok(make_string_owned(ctx.heap, result))
        }),
    })));

    funcs.push(("base64_decode".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.base64_decode>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("hashlib.base64_decode requires data")?;
            let s = match val {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.clone(),
                    _ => return Err("invalid string".to_string()),
                },
                _ => return Err("expected string".to_string()),
            };
            match base64::engine::general_purpose::STANDARD.decode(s.as_bytes()) {
                Ok(bytes) => Ok(make_string_owned(ctx.heap, String::from_utf8(bytes).map_err(|e| format!("base64_decode: invalid utf-8: {}", e))?)),
                Err(e) => Err(format!("hashlib.base64_decode: {}", e)),
            }
        }),
    })));

    funcs.push(("hex_encode".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.hex_encode>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("hashlib.hex_encode requires data")?;
            let s = match val {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.as_str(),
                    _ => return Err("invalid string".to_string()),
                },
                _ => return Err("expected string".to_string()),
            };
            Ok(make_string_owned(ctx.heap, hex_encode(s.as_bytes())))
        }),
    })));

    funcs.push(("hex_decode".to_string(), Value::NativeFunc(NativeFunc {
        name: "<hashlib.hex_decode>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("hashlib.hex_decode requires hex string")?;
            let s = match val {
                Value::String(r) => match ctx.heap.get(*r) {
                    GcObj::String(s) => s.clone(),
                    _ => return Err("invalid string".to_string()),
                },
                _ => return Err("expected string".to_string()),
            };
            match hex_decode(&s) {
                Ok(bytes) => Ok(make_string_owned(ctx.heap, String::from_utf8(bytes).map_err(|e| format!("hex_decode: invalid utf-8: {}", e))?)),
                Err(e) => Err(format!("hashlib.hex_decode: {}", e)),
            }
        }),
    })));

    funcs
}

fn hex_encode(data: &[u8]) -> String {
    let len = data.len();
    let mut buf = Vec::with_capacity(len * 2);
    for &byte in data {
        buf.push(HEX_TABLE[(byte >> 4) as usize]);
        buf.push(HEX_TABLE[(byte & 0xf) as usize]);
    }
    unsafe { String::from_utf8_unchecked(buf) }
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
    pub struct Sha1 {
        state: [u32; 5],
        count: u64,
        buffer: [u8; 64],
    }

    impl Sha1 {
        pub fn new() -> Self {
            Sha1 {
                state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0],
                count: 0,
                buffer: [0u8; 64],
            }
        }

        pub fn update(&mut self, data: &[u8]) {
            let offset = (self.count as usize) & 63;
            self.count += data.len() as u64;
            let mut remaining = data;

            if offset > 0 {
                let space = 64 - offset;
                if remaining.len() < space {
                    self.buffer[offset..offset + remaining.len()].copy_from_slice(remaining);
                    return;
                }
                self.buffer[offset..64].copy_from_slice(&remaining[..space]);
                let block = self.buffer; // Copy out before mutable borrow
                self.process_block(&block);
                remaining = &remaining[space..];
            }

            while remaining.len() >= 64 {
                self.process_block(remaining);
                remaining = &remaining[64..];
            }

            if !remaining.is_empty() {
                self.buffer[..remaining.len()].copy_from_slice(remaining);
            }
        }

        fn process_block(&mut self, block: &[u8]) {
            let mut w = [0u32; 80];
            let block32: &[u8; 64] = block.try_into().unwrap();
            for t in 0..16 {
                w[t] = u32::from_be_bytes([
                    block32[t * 4],
                    block32[t * 4 + 1],
                    block32[t * 4 + 2],
                    block32[t * 4 + 3],
                ]);
            }
            for t in 16..80 {
                w[t] = (w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16]).rotate_left(1);
            }

            let (mut a, mut b, mut c, mut d, mut e) = (
                self.state[0], self.state[1], self.state[2],
                self.state[3], self.state[4],
            );

            for t in 0..80 {
                let (f, k): (u32, u32) = match t {
                    0..=19 => ((b & c) | (!b & d), 0x5a827999),
                    20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                    _ => (b ^ c ^ d, 0xca62c1d6),
                };
                let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(w[t]);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = temp;
            }

            self.state[0] = self.state[0].wrapping_add(a);
            self.state[1] = self.state[1].wrapping_add(b);
            self.state[2] = self.state[2].wrapping_add(c);
            self.state[3] = self.state[3].wrapping_add(d);
            self.state[4] = self.state[4].wrapping_add(e);
        }

        pub fn finalize(mut self) -> Vec<u8> {
            let bits = self.count.wrapping_mul(8);
            let offset = (self.count as usize) & 63;
            let mut padding = vec![0u8; if offset < 56 { 56 - offset } else { 120 - offset }];
            padding[0] = 0x80;
            self.update(&padding);
            self.update(&bits.to_be_bytes());
            let mut result = Vec::with_capacity(20);
            for s in &self.state {
                result.extend_from_slice(&s.to_be_bytes());
            }
            result
        }

        pub fn digest(data: &[u8]) -> Vec<u8> {
            let mut hasher = Self::new();
            hasher.update(data);
            hasher.finalize()
        }
    }
}
