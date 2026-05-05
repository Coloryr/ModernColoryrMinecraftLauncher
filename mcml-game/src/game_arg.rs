use crate::mojang::game_obj::{Argument, GameObj};

fn make_v1_game_arg(game: &GameObj) -> Vec<String> {
    match &game.minecraft_arguments {
        None => Vec::new(),
        Some(arg) => arg.split(' ').map(|s| s.to_string()).collect(),
    }
}

fn make_v2_game_arg(game: &GameObj) -> Vec<String> {
    match &game.arguments {
        None => Vec::new(),
        Some(args) => {
            let mut vec: Vec<String> = Vec::new();

            for item in args.game.iter() {
                if let Argument::Plain(item) = item {
                    vec.push(item.to_string());
                }
            }

            vec
        }
    }
}

fn make_loader_v1_game_arg() -> Vec<String> {
    Vec::new()
}