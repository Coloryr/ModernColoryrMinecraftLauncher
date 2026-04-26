fn main() {
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");

    // if cfg!(target_os = "windows") {
    //     let mut res = tauri_winres::WindowsResource::new();
    //     res.set_icon("test.ico")
    //         .set("InternalName", "TEST.EXE")
    //         // manually set version 1.0.0.0
    //         .set_version_info(
    //             tauri_winres::VersionInfo::PRODUCTVERSION,
    //             0x0001000000000000,
    //         );
    //     res.compile().unwrap();
    // }
}
