
#[test]
fn get_java_list() {
    mcml_jvms::scan_java();
    let list2 = mcml_jvms::get_all_java();
    for item in list2 {
        println!(
            "find java {}: {} {} {} {}",
            item.major_version, item.name, item.version, item.arch, item.path
        );
    }
}
