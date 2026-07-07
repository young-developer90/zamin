use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(cuda_support)");
    // Try to find CUDA toolkit
    let cuda_path = find_cuda();

    if let Some(cuda_path) = cuda_path {
        let nvcc = if cfg!(target_os = "windows") {
            cuda_path.join("bin/nvcc.exe")
        } else {
            cuda_path.join("bin/nvcc")
        };

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let kernel_src = PathBuf::from("src/cuda_kernels.cu");
        let ptx_out = out_dir.join("cuda_kernels.ptx");

        if nvcc.exists() {
            // Compile CUDA kernels to PTX
            let status = Command::new(&nvcc)
                .args([
                    "--ptx",
                    "-o", ptx_out.to_str().unwrap(),
                    kernel_src.to_str().unwrap(),
                ])
                .status()
                .expect("nvcc failed to compile CUDA kernels");
            assert!(status.success(), "nvcc compilation failed");

            println!("cargo:rustc-cfg=cuda_support");

            // Tell cargo where to find CUDA libs
            let lib_dir = if cfg!(target_os = "windows") {
                cuda_path.join("lib/x64")
            } else if cuda_path.join("lib64").exists() {
                cuda_path.join("lib64")
            } else if cuda_path.join("lib/x86_64-linux-gnu").exists() {
                cuda_path.join("lib/x86_64-linux-gnu")
            } else {
                cuda_path.join("lib")
            };
            println!("cargo:rustc-link-search={}", lib_dir.display());

            if cfg!(target_os = "windows") {
                println!("cargo:rustc-link-lib=dylib=cudart");
                println!("cargo:rustc-link-lib=dylib=cublas");
                println!("cargo:rustc-link-lib=dylib=cuda");
            } else {
                println!("cargo:rustc-link-lib=dylib=cudart");
                println!("cargo:rustc-link-lib=dylib=cublas");
                println!("cargo:rustc-link-lib=dylib=cuda");
            }

            // Rerun if kernels change
            println!("cargo:rerun-if-changed=src/cuda_kernels.cu");
        } else {
            println!("cargo:warning=CUDA toolkit found at {} but nvcc not found, skipping CUDA kernel compilation", cuda_path.display());
        }
    } else {
        println!("cargo:warning=CUDA toolkit not found, skipping CUDA kernel compilation");
    }
}

fn find_cuda() -> Option<PathBuf> {
    // Check CUDA_PATH env var first (set by CUDA installer on Windows, can be set manually)
    if let Ok(path) = env::var("CUDA_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    if cfg!(target_os = "windows") {
        // Common Windows install paths
        let program_files = env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
        let base = PathBuf::from(program_files).join("NVIDIA GPU Computing Toolkit").join("CUDA");
        if base.exists() {
            if let Ok(entries) = std::fs::read_dir(&base) {
                let mut versions: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .collect();
                versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
                if let Some(latest) = versions.into_iter().next() {
                    return Some(latest.path());
                }
            }
        }
    } else {
        // Common Linux paths
        let candidates = vec![
            PathBuf::from("/opt/cuda"),
            PathBuf::from("/usr/local/cuda"),
            PathBuf::from("/usr/cuda"),
        ];
        for p in candidates {
            if p.exists() {
                return Some(p);
            }
        }

        // Fallback: detect system-packaged CUDA (e.g. nvidia-cuda-toolkit on Ubuntu)
        // where nvcc is in PATH but no /usr/local/cuda directory exists.
        if let Ok(output) = std::process::Command::new("which").arg("nvcc").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let nvcc_path = PathBuf::from(&path);
                // Walk up to find the CUDA root: nvcc lives at <root>/bin/nvcc
                if let Some(parent) = nvcc_path.parent().and_then(|p| p.parent()) {
                    if parent.join("include").join("cuda.h").exists()
                        || parent.join("targets").exists()
                    {
                        return Some(parent.to_path_buf());
                    }
                    // For system packages, nvcc is at /usr/bin/nvcc.
                    // No CUDA root dir exists, so use /usr and link against system libs.
                    return Some(parent.to_path_buf());
                }
            }
        }
    }

    None
}
