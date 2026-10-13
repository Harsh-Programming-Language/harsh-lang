use maud::{html, Markup};

fn page(title: &str, items: &[&str]) -> Markup {
    html! {
        h1 { (title) }
        p.intro { "Items: " (items.len()) }
        a href=(format!("/items/{}", items.len())) { "next" }
        ul {
            @for item in items {
                li { (item.to_uppercase()) }
            }
        }
        @if items.is_empty() {
            p { "none" }
        }
    }
}

fn main() {
    println!("{}", page("Fruit", &["apple", "pear"]).into_string());
}
