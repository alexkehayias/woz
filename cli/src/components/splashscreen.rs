use std::io::Read;
use std::fs::File;

use failure::Error;
use failure::ResultExt;

use crate::config::{Config, DEFAULT_SPLASHSCREENS};
use super::AppComponent;
use crate::file_upload::FileUpload;


pub struct SplashscreenComponent<'a> {
    conf: &'a Config
}

impl<'a> SplashscreenComponent<'a> {
    pub fn new(conf: &'a Config) -> Self {
        Self { conf }
    }
}

impl<'a> AppComponent for SplashscreenComponent<'a> {
    fn files(&self, file_prefix: &String) -> Result<Vec<FileUpload>, Error> {
        let mut uploads = Vec::new();

        if let Some(splashscreens) = &self.conf.splashscreens {
            for (device, path) in splashscreens.to_vec() {
                let mut f = File::open(path)
                    .context("Splashscreen file does not exist")?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)
                    .context("Failed to read splashscreen to bytes")?;
                uploads.push(
                    FileUpload::new(
                        format!("{}/img/splashscreens/{}.png", file_prefix, device),
                        String::from("image/png"),
                        buffer
                    )
                );
            };
        } else {
            for (device, bytes) in DEFAULT_SPLASHSCREENS.iter() {
                uploads.push(
                    FileUpload::new(
                        format!("{}/img/splashscreens/{}.png", file_prefix, device),
                        String::from("image/png"),
                        bytes.to_owned()
                    )
                );
            };
        }

        Ok(uploads)
    }
}
