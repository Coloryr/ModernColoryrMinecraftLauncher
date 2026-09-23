#[test]
#[ignore = "skip"]
fn lang_locale() {
    let locale = mml_names::get_current_locale();
    
    println!("locale: {}", locale);
}