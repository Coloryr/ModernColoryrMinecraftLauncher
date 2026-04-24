pub struct CoreInitObj {
    pub local: String,
    pub oauth_key: String,
    pub curseforge_key: String,
}

impl CoreInitObj {
    pub fn new(local: String, oauth_key: String, curseforge_key: String) -> Self {
        CoreInitObj {
            local,
            oauth_key,
            curseforge_key,
        }
    }
}
