// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![windows_subsystem = "windows"]

use std::error::Error;
use std::env;

use mcml_core::core::{CoreInitObj, core};

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        env::set_var("SLINT_BACKEND", "winit-skia");
    }

    let ui = AppWindow::new()?;

    ui.run()?;

    core::init(CoreInitObj::new(String::new(), String::new(), String::new()));

    Ok(())
}
