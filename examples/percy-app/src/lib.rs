#![feature(proc_macro_hygiene)]

use wasm_bindgen::prelude::*;
use web_sys;

use css_rs_macro::css;
use virtual_dom_rs::prelude::*;
use std::rc::Rc;

#[wasm_bindgen]
pub fn render() {
    let start_view = html! { <div> Hello </div> };

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    let mut dom_updater = DomUpdater::new_append_to_mount(start_view, &body);
    // TODO this even handler throws a runtime exception
    let end_view = html! {
       <div>
          <h1>Hello, World!</h1>
          <button
            onclick=move |_event: web_sys::MouseEvent| {
                web_sys::console::log_1(&"Button Clicked!".into());
            }
          >
            Click me and check your console
          </button>
       </div>
    };
    dom_updater.update(end_view);
}

static MY_COMPONENT_CSS: &'static str = css!{r#"
:host {
    font-size: 24px;
    font-weight: bold;
}
"#};

static _MORE_CSS: &'static str = css!{r#"
.big {
  font-size: 30px;
}

.blue {
  color: blue;
}
"#};
