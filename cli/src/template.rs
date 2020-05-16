use failure::Error;
use handlebars::Handlebars;


const LANDING_PAGE_INDEX_TEMPLATE: &str = include_str!("templates/landing_page/index.html");
const APP_INDEX_TEMPLATE: &str = include_str!("templates/app/index.html");
const MANIFEST_TEMPLATE: &str = include_str!("templates/app/manifest.json");
const SERVICE_WORKER_JS_TEMPLATE: &str = include_str!("templates/app/serviceworker.js");

pub fn load_templates() -> Result<Handlebars<'static>, Error> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_template_string("landing_page_index", LANDING_PAGE_INDEX_TEMPLATE)?;
    handlebars.register_template_string("app_index", APP_INDEX_TEMPLATE)?;
    handlebars.register_template_string("manifest", MANIFEST_TEMPLATE)?;
    handlebars.register_template_string("sw.js", SERVICE_WORKER_JS_TEMPLATE)?;
    Ok(handlebars)
}

#[test]
fn test_index_templates() {
    let loader = load_templates().expect("Failed to load templates");
    let res = loader.render(
        "app_index",
        &json!({
            "name": "Test App",
            "author": "Alex Kehayias",
            "description": "Description here",
            "url": "http://localhost",
            "manifest_path": "./manifest.json",
            "app_js_path": "./app.js",
            "sw_js_path": "./sw.js",
            "wasm_path": "./app.wasm",
            "bg_color": "#000000",
        }));
    dbg!(res.expect("Failed to render"));
}
