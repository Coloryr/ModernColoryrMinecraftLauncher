// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![windows_subsystem = "windows"]

use std::{env};
use std::error::Error;

use mcml_core::{self, CoreInitObj};

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // let exe_path = env::current_exe().expect("Failed to get exe path");
    // let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    // let run_dir = exe_dir.parent().unwrap();

    // mcml_core::init(CoreInitObj::new(
    //     run_dir.to_path_buf(),
    //     String::new(),
    //     String::new(),
    // ));

    unsafe {
        env::set_var("SLINT_BACKEND", "winit-skia");
    }

    let ui = AppWindow::new()?;

    ui.run()?;

    Ok(())
}
