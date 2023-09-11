use stylist::yew::styled_component;
use wasm_bindgen::prelude::*;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct FrameProps {
    pub file: AttrValue,
}

#[styled_component]
pub fn BrowserSvgFrame(props: &FrameProps) -> Html {
    let ctr_css = css! {
        display: inline-grid;
        margin: 10px;

        #browser-frame {
            width: 400px;
            height: 400px;
            border: 1px solid black;
        }
    };

    html! {
        <div class={ctr_css}>
            <h1>{"Browser"}</h1>
            <img
                id="browser-frame"
                src={&props.file}
            />
        </div>
    }
}
