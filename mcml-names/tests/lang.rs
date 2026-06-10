#[test]
#[ignore = "skip"]
fn lang_locale() {
    let locale = mcml_names::get_current_locale();
    
    println!("locale: {}", locale);
}