use std::io::Read;
use std::fs::File;

use failure::Error;
use failure::ResultExt;

use crate::config::{Config, DEFAULT_ICONS};
use super::AppComponent;
use crate::file_upload::FileUpload;


pub struct IconComponent<'a> {
    conf: &'a Config
}

impl<'a> IconComponent<'a> {
    pub fn new(conf: &'a Config) -> Self {
        Self { conf }
    }
}

impl<'a> AppComponent for IconComponent<'a> {
    fn files(&self, file_prefix: &String) -> Result<Vec<FileUpload>, Error> {
        let mut uploads = Vec::new();

        if let Some(icons) = &self.conf.icons {
            for (size, path) in icons.to_vec() {
                let mut f = File::open(path)
                    .context("Icon file does not exist")?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)
                    .context("Failed to read icon to bytes")?;
                uploads.push(
                    FileUpload::new(
                        format!("{}/app/img/icons/homescreen_{}.png", file_prefix, size),
                        String::from("image/png"),
                        buffer
                    )
                );
            }
        } else {
            for (size, bytes) in DEFAULT_ICONS.iter() {
                uploads.push(
                    FileUpload::new(
                        format!("{}/app/img/icons/homescreen_{}.png", file_prefix, size),
                        String::from("image/png"),
                        bytes.to_owned()
                    )
                );
            };
        }

        Ok(uploads)
    }
}
