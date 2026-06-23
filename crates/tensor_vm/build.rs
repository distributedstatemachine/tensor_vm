use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=kernels/cuda/field_matmul.cu");
    println!("cargo:rerun-if-env-changed=NVCC");
    println!("cargo:rerun-if-env-changed=TVM_CUDA_ARCH");

    if env::var_os("CARGO_FEATURE_CUDA_KERNELS").is_none() {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let source = manifest_dir.join("kernels/cuda/field_matmul.cu");
    let library = out_dir.join("libtensor_vm_cuda_kernels.a");
    let nvcc = env::var_os("NVCC")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("nvcc"));
    let cuda_arch = cuda_arch();

    let status = Command::new(&nvcc)
        .arg("--lib")
        .arg("-std=c++17")
        .arg("-O3")
        .arg("-arch")
        .arg(cuda_arch)
        .arg("-cudart=shared")
        .arg("--compiler-options")
        .arg("-fPIC")
        .arg("-o")
        .arg(&library)
        .arg(&source)
        .status()
        .unwrap_or_else(|error| panic!("failed to invoke nvcc for TensorVM CUDA kernels: {error}"));

    if !status.success() {
        panic!("nvcc failed while compiling TensorVM CUDA kernels");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=tensor_vm_cuda_kernels");

    if let Some(cuda_home) = cuda_home(&nvcc) {
        let lib64 = cuda_home.join("lib64");
        if lib64.exists() {
            println!("cargo:rustc-link-search=native={}", lib64.display());
        }
    }

    println!("cargo:rustc-link-lib=dylib=cudart");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}

fn cuda_arch() -> String {
    if let Ok(value) = env::var("TVM_CUDA_ARCH") {
        return value;
    }
    detected_cuda_arch().unwrap_or_else(|| "sm_80".to_owned())
}

fn detected_cuda_arch() -> Option<String> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let capability = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    let (major, minor) = capability.split_once('.')?;
    Some(format!("sm_{major}{minor}"))
}

fn cuda_home(nvcc: &Path) -> Option<PathBuf> {
    if let Some(value) = env::var_os("CUDA_HOME").or_else(|| env::var_os("CUDA_PATH")) {
        return Some(PathBuf::from(value));
    }
    if nvcc.components().count() > 1 {
        return nvcc.parent().and_then(Path::parent).map(Path::to_path_buf);
    }
    Some(PathBuf::from("/usr/local/cuda"))
}
