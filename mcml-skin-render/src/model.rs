pub mod model {
    use crate::{cube::cube, texture::texture::SkinType};
    
    /// 史蒂夫模型部件
    #[derive(Debug, Clone)]
    pub struct SteveModelPart {
        pub model: Vec<f32>,
        pub point: Vec<u16>,
    }

    impl SteveModelPart {
        pub fn new(model: Vec<f32>, point: Vec<u16>) -> Self {
            Self { model, point }
        }
    }

    /// 史蒂夫模型对象
    #[derive(Debug, Clone)]
    pub struct SteveModelObj {
        pub head: SteveModelPart,
        pub body: SteveModelPart,
        pub left_arm: SteveModelPart,
        pub right_arm: SteveModelPart,
        pub left_leg: SteveModelPart,
        pub right_leg: SteveModelPart,
        pub cape: SteveModelPart,
    }

    impl SteveModelObj {
        pub fn new(
            head: SteveModelPart,
            body: SteveModelPart,
            left_arm: SteveModelPart,
            right_arm: SteveModelPart,
            left_leg: SteveModelPart,
            right_leg: SteveModelPart,
            cape: SteveModelPart,
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

    /// 生成一个模型
    pub fn get_steve(skin_type: SkinType) -> SteveModelObj {
        let is_slim = skin_type == SkinType::NewSlim;

        // 公共部件
        let head = SteveModelPart::new(
            cube::get_square_default(),
            cube::get_square_indices_default(),
        );

        let body = SteveModelPart::new(
            cube::get_square(1.0, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        let left_leg = SteveModelPart::new(
            cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        let right_leg = SteveModelPart::new(
            cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        let cape = SteveModelPart::new(
            cube::get_square(1.25, 2.0, 0.1, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        // 手臂 (根据类型决定宽度)
        let arm_width = if is_slim { 0.375 } else { 0.5 };
        let arm = SteveModelPart::new(
            cube::get_square(arm_width, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        SteveModelObj::new(head, body, arm.clone(), arm, left_leg, right_leg, cape)
    }

    /// 生成第二层模型
    pub fn get_steve_top(skin_type: SkinType) -> SteveModelObj {
        let is_slim = skin_type == SkinType::NewSlim;
        let is_old = skin_type == SkinType::Old;

        // 头部 (总是存在)
        let head = SteveModelPart::new(
            cube::get_square(1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.125),
            cube::get_square_indices_default(),
        );

        let mut body = SteveModelPart::new(Vec::new(), Vec::new());
        let mut left_arm = SteveModelPart::new(Vec::new(), Vec::new());
        let mut right_arm = SteveModelPart::new(Vec::new(), Vec::new());
        let mut left_leg = SteveModelPart::new(Vec::new(), Vec::new());
        let mut right_leg = SteveModelPart::new(Vec::new(), Vec::new());
        let cape = SteveModelPart::new(Vec::new(), Vec::new());

        if !is_old {
            body = SteveModelPart::new(
                cube::get_square(1.0, 1.5, 0.5, 0.0, 0.0, 0.0, 1.125),
                cube::get_square_indices_default(),
            );

            let arm_width = if is_slim { 0.375 } else { 0.5 };
            let arm_model = cube::get_square(arm_width, 1.5, 0.5, 0.0, 0.0, 0.0, 1.125);
            let arm_indices = cube::get_square_indices_default();

            left_arm = SteveModelPart::new(arm_model.clone(), arm_indices.clone());
            right_arm = SteveModelPart::new(arm_model, arm_indices);

            left_leg = SteveModelPart::new(
                cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.125),
                cube::get_square_indices_default(),
            );

            right_leg = SteveModelPart::new(
                cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.125),
                cube::get_square_indices_default(),
            );
        }

        SteveModelObj::new(head, body, left_arm, right_arm, left_leg, right_leg, cape)
    }
}
