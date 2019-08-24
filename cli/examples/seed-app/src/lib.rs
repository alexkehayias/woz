#[macro_use] extern crate seed;
use seed::prelude::*;

struct Model;

impl Default for Model {
    fn default() -> Self {
        Self { }
    }
}

#[derive(Clone)]
enum Msg {}

fn update(_msg: Msg, _model: &mut Model, _orders: &mut impl Orders<Msg>) {
    // This would be where you update state. In this example there is
    // no stateful interactivity so this is a noop
    ()
}


fn view(_model: &Model) -> impl View<Msg> {
    let font_family = "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, Helvetica, Arial, sans-serif, \"Apple Color Emoji\", \"Segoe UI Emoji\", \"Segoe UI Symbol\"";

    let outer_style = style!{
        "padding" => "0 20px";
        "box-sizing" => "border-box";
        "position" => "absolute";
        "width" => "100%";
        "height" => "100%";
        "display" => "flex";
        "align-items" => "center";
        "justify-content" => "center";
        "background" => "#f5f6fa";
    };

    div![outer_style,
         div![
             style!{
                 "padding" => "50px 30px";
                 "background" => "white";
                 "border-radius" => "5px";
                 "box-shadow" => "0px 0px 20px 0px #d8d8d8"
             },
             div![
                 style!{
                     "text-align" => "center";
                     "font-size" => "1.8em";
                     "line-height" => "90px";
                     "border-radius" => "50%";
                     "width" => "80px";
                     "height" => "80px";
                     "background" => "#f6e58d";
                     "margin" => "auto auto 20px auto";
                 },
                 "🎉"
             ],
             h1![
                 style!{
                     "font-size" => "1.5em";
                     "font-family" => { font_family };
                     "font-weight" => "700";
                     "line-height" => "1.35em";
                     "color" => "#30336b";
                     "margin-bottom" => "20px";
                     "text-align" => "center";
                 },
                 "Hello from WebAssembly!"],
             p![
                 style!{
                     "font-family" => { font_family };
                     "font-size" => "1.1em";
                     "text-align" => "center";
                     "line-height" => "1.2em";
                     "color" => "#535c68";
                     "margin-bottom" => "30px";
                 },
                 "This app is written entirely using the Rust programming \
                  language and packaged as a progressive web app (PWA) \
                  using Woz."
             ],
             a![attrs!{"href" => "https://woz.sh"},
                style!{
                    "width" => "100%";
                    "background" => "#6ab04c";
                    "padding" => "16px 8px";
                    "font-family" => { font_family };
                    "font-size" => "1em";
                    "font-weight" => "bold";
                    "color" => "white";
                    "display" => "block";
                    "text-decoration" => "none";
                    "text-align" => "center";
                    "border-bottom" => "2px solid green";
                    "border-radius" => "5px";
                    "box-sizing" => "border-box";
                },
                "Learn more"
             ]
         ],
    ]
}

#[wasm_bindgen]
pub fn render() {
    seed::App::build(|_, _| Model::default(), update, view)
        .finish()
        .run();
}
