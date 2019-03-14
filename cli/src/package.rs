use std::fmt;
use std::process;
use std::error::Error;
use std::path::PathBuf;

use crate::config::Lib;


#[derive(Debug)]
pub struct WasmPackageError;

impl fmt::Display for WasmPackageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed to generate wasm package")
    }
}

impl Error for WasmPackageError {
    fn description(&self) -> &str {
        "Failed to generate wasm package"
    }
}

pub struct WasmPackage {
    pub lib: Lib,
    pub js: PathBuf,
    pub wasm: PathBuf,
}

impl WasmPackage {
    fn new(lib: Lib, wasm_path: PathBuf, js_path: PathBuf) -> Self {
        WasmPackage {lib, wasm: wasm_path, js: js_path}
    }
}

/// Generates a js file that manages the interop between js and wasm
pub fn wasm_package(lib: Lib, wasm_path: PathBuf, out_path: PathBuf)
                    -> Result<WasmPackage, WasmPackageError> {
    match lib {
        Lib::WasmBindgen => {
            let command = format!(
                "wasm-bindgen {} --no-typescript --no-modules --out-dir {} --out-name app",
                wasm_path.into_os_string().into_string().unwrap(),
                out_path.clone().into_os_string().into_string().unwrap()
            );

            process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .expect("failed to execute process");

            let mut js_path = out_path.clone();
            js_path.push("app.js");

            let mut wasm_path = out_path.clone();
            wasm_path.push("app_bg.wasm");

            Ok(WasmPackage::new(lib, wasm_path, js_path))
        },
        _ => Err(WasmPackageError)
    }
}
