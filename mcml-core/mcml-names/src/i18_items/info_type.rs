#[derive(Clone, Debug)]
pub enum InfoType {
    CoreStart,
    CoreStop,

    TempFile,

    AuthTypeOffline,
    AuthTypeOAuth,
    AuthTypeNide8,
    AuthTypeAuthlibInjector,
    AuthTypeLittleSkin,
    AuthTypeSelfLittleSkin,
}