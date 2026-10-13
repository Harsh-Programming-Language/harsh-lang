use serde_json::json;

fn main() {
    let name = "Ada";
    let langs = vec!["Rust", "Harsh"];
    let v = json!({
        "name": name.to_uppercase(),
        "age": 36,
        "langs": langs,
        "count": langs.len() + 1,
        "tags": [{ "id": 1, "main": langs.first() }]
    });
    println!("{v}");
}
