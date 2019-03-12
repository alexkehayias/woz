use failure::Error;
use handlebars::Handlebars;


const INDEX_TEMPLATE: &str = include_str!("templates/index.html");
const MANIFEST_TEMPLATE: &str = include_str!("templates/manifest.json");
const SERVICE_WORKER_JS_TEMPLATE: &str = include_str!("templates/serviceworker.js");

pub fn load_templates() -> Result<Handlebars, Error> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_template_string("index", INDEX_TEMPLATE)?;
    handlebars.register_template_string("manifest", MANIFEST_TEMPLATE)?;
    handlebars.register_template_string("sw.js", SERVICE_WORKER_JS_TEMPLATE)?;
    Ok(handlebars)
}

#[test]
fn test_index_templates() {
    let loader = load_templates().expect("Failed to load templates");
    let res = loader.render(
        "index",
        &json!({
            "name": "Test App",
            "author": "Alex Kehayias",
            "description": "Description here",
            "loader_js_path": "./loader.js",
            "sw_js_path": "./sw.js",
            "wasm_path": "./app.wasm",
        }));
    dbg!(res.expect("Failed to render"));
}
