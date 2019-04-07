use failure::Error;
use failure::ResultExt;
use handlebars::Handlebars;

use crate::config::Config;
use super::AppComponent;
use crate::file_upload::FileUpload;


pub struct PwaComponent<'a> {
    conf: &'a Config,
    templates: Handlebars,
}

impl<'a> PwaComponent<'a> {
    pub fn new(conf: &'a Config, templates: Handlebars) -> Self {
        Self { conf, templates }
    }
}

impl<'a> AppComponent for PwaComponent<'a> {
    fn files(&self, file_prefix: &String) -> Result<Vec<FileUpload>, Error> {
        let index_template = self.templates.render("index", &json!({
            "name": self.conf.name,
            "author": self.conf.author,
            "description": self.conf.description,
            "manifest_path": "./manifest.json",
            "app_js_path": "./app.js",
            "sw_js_path": "./sw.js",
            "wasm_path": "./app.wasm",
        }));
        let manifest_template = self.templates.render("manifest", &json!({
            "name": self.conf.name,
            "short_name": self.conf.short_name,
            "bg_color": "#ffffff",
            "description": self.conf.description
        }));
        let service_worker_template = self.templates.render("sw.js", &json!({}));

        let uploads = vec![
            FileUpload::new(format!("{}/index.html", file_prefix),
                            String::from("text/html"),
                            index_template.context("Failed to render index.html")?.into_bytes()),
            FileUpload::new(format!("{}/manifest.json", file_prefix),
                            String::from("application/manifest+json"),
                            manifest_template.context("Failed to render manifest.json")?.into_bytes()),
            FileUpload::new(format!("{}/sw.js", file_prefix),
                            String::from("application/javascript"),
                            service_worker_template.context("Failed to render sw.js")?.into_bytes()),
        ];
        Ok(uploads)
    }
}
