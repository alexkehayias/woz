use std::process;
use std::path::PathBuf;

use failure::Error;
use failure::ResultExt;

use crate::config::Lib;

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
                    -> Result<WasmPackage, Error> {
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
                .context("Failed to generate wasm bindings")?;

            let mut js_path = out_path.clone();
            js_path.push("app.js");

            let mut wasm_path = out_path.clone();
            wasm_path.push("app_bg.wasm");

            Ok(WasmPackage::new(lib, wasm_path, js_path))
        },
        _ => unimplemented!()
    }
}
