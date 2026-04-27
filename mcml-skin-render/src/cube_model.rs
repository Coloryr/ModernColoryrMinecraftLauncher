/// 一个方块模型数据
#[derive(Debug, Clone)]
pub struct CubeModelItemObj {
    pub model: Vec<f32>,
    pub point: Vec<u16>,
}

impl CubeModelItemObj {
    pub fn new(model: Vec<f32>, point: Vec<u16>) -> Self {
        Self { model, point }
    }
}

/// 一个史蒂夫模型数据
#[derive(Debug, Clone)]
pub struct SteveModelObj {
    pub head: CubeModelItemObj,
    pub body: CubeModelItemObj,
    pub left_arm: CubeModelItemObj,
    pub right_arm: CubeModelItemObj,
    pub left_leg: CubeModelItemObj,
    pub right_leg: CubeModelItemObj,
    pub cape: CubeModelItemObj,
}

impl SteveModelObj {
    pub fn new(
        head: CubeModelItemObj,
        body: CubeModelItemObj,
        left_arm: CubeModelItemObj,
        right_arm: CubeModelItemObj,
        left_leg: CubeModelItemObj,
        right_leg: CubeModelItemObj,
        cape: CubeModelItemObj,
    ) -> Self {
        Self {
            head,
            body,
            left_arm,
            right_arm,
            left_leg,
            right_leg,
            cape,
        }
    }
}

/// 模型贴图数据
#[derive(Debug, Clone)]
pub struct SteveTextureObj {
    pub head: Vec<f32>,
    pub body: Vec<f32>,
    pub left_arm: Vec<f32>,
    pub right_arm: Vec<f32>,
    pub left_leg: Vec<f32>,
    pub right_leg: Vec<f32>,
    pub cape: Vec<f32>,
}

impl SteveTextureObj {
    pub fn new(
        head: Vec<f32>,
        body: Vec<f32>,
        left_arm: Vec<f32>,
        right_arm: Vec<f32>,
        left_leg: Vec<f32>,
        right_leg: Vec<f32>,
        cape: Vec<f32>,
    ) -> Self {
        Self {
            head,
            body,
            left_arm,
            right_arm,
            left_leg,
            right_leg,
            cape,
        }
    }
}

impl Default for CubeModelItemObj {
    fn default() -> Self {
        Self {
            model: Vec::new(),
            point: Vec::new(),
        }
    }
}

impl Default for SteveTextureObj {
    fn default() -> Self {
        Self {
            head: Vec::new(),
            body: Vec::new(),
            left_arm: Vec::new(),
            right_arm: Vec::new(),
            left_leg: Vec::new(),
            right_leg: Vec::new(),
            cape: Vec::new(),
        }
    }
}
