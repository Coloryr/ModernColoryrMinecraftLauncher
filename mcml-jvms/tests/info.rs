use mcml_jvms::java_helper;

#[test]
fn get_java_list() {
    let list2 = java_helper::find_java();
    if list2.is_some() {
        for item in list2.unwrap() {
            println!(
                "find java {}: {} {} {} {}",
                item.major_version, item.name, item.version, item.arch, item.path
            );
        }
    }
}
