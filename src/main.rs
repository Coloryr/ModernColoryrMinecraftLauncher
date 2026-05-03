// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![windows_subsystem = "windows"]

use std::env;
use std::error::Error;

use mcml_core::core::{self, CoreInitObj};

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    core::init(CoreInitObj::new(
        String::from("./"),
        String::new(),
        String::new(),
    ));

    unsafe {
        env::set_var("SLINT_BACKEND", "winit-skia");
    }

    let ui = AppWindow::new()?;

    ui.run()?;

    Ok(())
}
