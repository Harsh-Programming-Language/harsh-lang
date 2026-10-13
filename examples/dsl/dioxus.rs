use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);
    let items = vec!["apple", "pear"];
    rsx! {
        h1 { "High-five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, disabled: count() <= 0, "Down low!" }
        p { "Twice: {count() * 2}" }
        for item in items.iter() {
            li { key: "{item}", "{item}" }
        }
        if count() > 3 {
            p { class: if count() > 10 { "hot" } else { "warm" }, "Getting big" }
        }
    }
}
