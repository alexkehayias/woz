use failure::Error;
use failure::ResultExt;
use handlebars::Handlebars;

use crate::config::{Config, LANDING_PAGE_CSS};
use super::AppComponent;
use crate::file_upload::FileUpload;


pub struct LandingPageComponent<'a> {
    conf: &'a Config,
    url: &'a str,
    templates: &'a Handlebars
}

impl<'a> LandingPageComponent<'a> {
    pub fn new(conf: &'a Config,
               url: &'a str,
               templates: &'a Handlebars) -> Self {
        Self { conf, url, templates }
    }
}

impl<'a> AppComponent for LandingPageComponent<'a> {
    fn files(&self, file_prefix: &str) -> Result<Vec<FileUpload>, Error> {
        let index_template = self.templates.render(
            "landing_page_index",
            &json!({
                "name": self.conf.name,
                "author": self.conf.author,
                "description": self.conf.description,
                "url": self.url,
            })
        );

        let uploads = vec![
            FileUpload::new(format!("{}/index.html", file_prefix),
                            String::from("text/html"),
                            index_template.context("Failed to render landing page index.html")?.into_bytes()),
            FileUpload::new(format!("{}/main.css", file_prefix),
                            String::from("text/css"),
                            LANDING_PAGE_CSS.as_bytes().to_vec()),
        ];
        Ok(uploads)
    }
}
