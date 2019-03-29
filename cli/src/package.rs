use std::process;
use std::path::PathBuf;

use failure::Error;
use failure::ResultExt;

use image;
use image::DynamicImage;
use image::GenericImageView;
use image::imageops::FilterType;

use crate::config::Lib;


/// Generates a set of icon images from the source file
fn icons(source_file: PathBuf) -> Result<Vec<DynamicImage>, Error> {
    let src_img = image::open(source_file).unwrap();
    let src_img_width = src_img.width();
    let src_img_height = src_img.height();
    let x = src_img_width / 2;
    let y = src_img_height / 2;

    let img_dimensions = vec![
        (192, 192),
        (168, 168),
        (144, 144),
        (96, 96),
        (72, 72),
        (48, 48),
    ];

    let images = img_dimensions.iter().map(|(width, height)| {
        // TODO how much should we scale the image to make the icon?
        let scale_width = src_img_width + width;
        let scale_height = src_img_height + height;
        src_img.clone()
            .resize(scale_width, scale_height, FilterType::Nearest)
            .crop(x, y, *width, *height)
    }).collect();
    Ok(images)
}

#[test]
fn icons_work() {
    let result = icons(PathBuf::from("resources/test-src.png"))
        .expect("Failed to generate icons");
    result[0].save("resources/crop.png").unwrap();
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
