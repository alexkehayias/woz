use std::io::Read;
use std::fs;
use std::fs::File;
use std::process;
use std::path::PathBuf;

use failure::Error;
use failure::ResultExt;
use crate::file_upload::FileUpload;
use super::AppComponent;


pub struct WasmComponent<'a> {
    wasm_path: PathBuf,
    out_path: &'a PathBuf
}

impl<'a> WasmComponent<'a> {
    pub fn new(wasm_path: PathBuf, out_path: &'a PathBuf) -> Self {
        Self { wasm_path, out_path }
    }
}

impl<'a> AppComponent for WasmComponent<'a> {
    fn files(&self, file_prefix: &String) -> Result<Vec<FileUpload>, Error> {
        let command = format!(
            "wasm-bindgen {} --no-typescript --no-modules --out-dir {} --out-name app",
            self.wasm_path.clone().into_os_string().into_string().unwrap(),
            self.out_path.clone().into_os_string().into_string().unwrap()
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

        let mut js_path = self.out_path.clone();
        js_path.push("app.js");

        let mut wasm_path = self.out_path.clone();
        wasm_path.push("app_bg.wasm");

        let uploads = vec![
            FileUpload::new(
                format!("{}/app/app.js", &file_prefix),
                String::from("application/javascript"),
                fs::read_to_string(js_path).context("Failed to read js file")?.into_bytes()
            ),
            FileUpload::new(
                format!("{}/app/app.wasm", &file_prefix),
                String::from("application/wasm"),
                {
                    let mut f = File::open(wasm_path).context("Failed to read wasm file")?;
                    let mut buffer = Vec::new();
                    f.read_to_end(&mut buffer).context("Failed to read to bytes")?;
                    buffer
                }
            ),
        ];

        Ok(uploads)
    }
}
