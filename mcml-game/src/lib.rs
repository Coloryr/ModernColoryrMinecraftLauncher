pub mod mojang;
pub mod game_arg;
pub mod launcher;

pub struct GameInstanceObj {
    pub setting: GameSettingObj,
    pub time: GameTimeObj,
    pub mods: HashMap<String, ModInfoObj>,
    pub custom: Vec<CustomGameArgObj>
}

impl GameInstanceObj {
    
}