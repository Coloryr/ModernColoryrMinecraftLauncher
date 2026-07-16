use std::{env, path::Path};

use mcml_base::path_helper;
use mcml_game::game_shaderpacks;

fn start(run_dir: &Path) {
    mcml_names::init(run_dir);
}

#[test]
fn read_shaderpacks() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let run_dir = exe_dir.parent().unwrap().to_path_buf();

    start(&run_dir);

    let files = path_helper::get_files("./tests/shaderpacks/");
    for item in files.iter() {
        let obj = game_shaderpacks::read_shaderpacks(item);
        match obj {
            Ok(obj) => {
                println!("name = {}, comment = {}", obj.name, obj.comment);
            }
            Err(err) => {
                println!("error file: {}", item.to_string_lossy());
                println!("error: {}", err);
            }
        }
    }
}
