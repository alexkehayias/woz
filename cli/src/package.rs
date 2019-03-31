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

            let mut bindgen_proc = process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .stdout(process::Stdio::piped())
                .spawn()
                .context("Failed to spawn wasm-bindgen")?;
            let exit_code = bindgen_proc.wait().context("Failed to wait for bindings")?;
            if !exit_code.success() {
                return Err(format_err!("wasm-bindgen failed"))
            };

            let mut js_path = out_path.clone();
            js_path.push("app.js");

            let mut wasm_path = out_path.clone();
            wasm_path.push("app_bg.wasm");

            Ok(WasmPackage::new(lib, wasm_path, js_path))
        },
        _ => unimplemented!()
    }
}

// TODO add all the business logic here?
struct AppBuilder;

impl AppBuilder {

    fn size(&self) -> usize {
        unimplemented!("TODO");
    }
}

// TODO does all the business logic for producing all of the file
// bundle that can be uploaded which is currently stuffed inside main
