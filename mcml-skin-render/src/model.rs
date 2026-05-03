pub mod model {
    use mcml_skin::SkinType;

    use crate::{
        cube::cube,
        cube_model::{CubeModelItemObj, SteveModel},
    };

    /// 生成一个模型
    pub fn get_steve(skin_type: SkinType) -> SteveModel {
        let is_slim = skin_type == SkinType::NewSlim;

        // 公共部件
        let head = CubeModelItemObj::new(
            cube::get_square_default(),
            cube::get_square_indices_default(),
        );

        let body = CubeModelItemObj::new(
            cube::get_square(1.0, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        let left_leg = CubeModelItemObj::new(
            cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        let right_leg = CubeModelItemObj::new(
            cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        let cape = CubeModelItemObj::new(
            cube::get_square(1.25, 2.0, 0.1, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        // 手臂 (根据类型决定宽度)
        let arm_width = if is_slim { 0.375 } else { 0.5 };
        let arm = CubeModelItemObj::new(
            cube::get_square(arm_width, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0),
            cube::get_square_indices_default(),
        );

        SteveModel::new(head, body, arm.clone(), arm, left_leg, right_leg, cape)
    }

    /// 生成第二层模型
    pub fn get_steve_top(skin_type: SkinType) -> SteveModel {
        let is_slim = skin_type == SkinType::NewSlim;
        let is_old = skin_type == SkinType::Old;

        // 头部 (总是存在)
        let head = CubeModelItemObj::new(
            cube::get_square(1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.125),
            cube::get_square_indices_default(),
        );

        let mut body = CubeModelItemObj::new(Vec::new(), Vec::new());
        let mut left_arm = CubeModelItemObj::new(Vec::new(), Vec::new());
        let mut right_arm = CubeModelItemObj::new(Vec::new(), Vec::new());
        let mut left_leg = CubeModelItemObj::new(Vec::new(), Vec::new());
        let mut right_leg = CubeModelItemObj::new(Vec::new(), Vec::new());
        let cape = CubeModelItemObj::new(Vec::new(), Vec::new());

        if !is_old {
            body = CubeModelItemObj::new(
                cube::get_square(1.0, 1.5, 0.5, 0.0, 0.0, 0.0, 1.125),
                cube::get_square_indices_default(),
            );

            let arm_width = if is_slim { 0.375 } else { 0.5 };
            let arm_model = cube::get_square(arm_width, 1.5, 0.5, 0.0, 0.0, 0.0, 1.125);
            let arm_indices = cube::get_square_indices_default();

            left_arm = CubeModelItemObj::new(arm_model.clone(), arm_indices.clone());
            right_arm = CubeModelItemObj::new(arm_model, arm_indices);

            left_leg = CubeModelItemObj::new(
                cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.125),
                cube::get_square_indices_default(),
            );

            right_leg = CubeModelItemObj::new(
                cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.125),
                cube::get_square_indices_default(),
            );
        }

        SteveModel::new(head, body, left_arm, right_arm, left_leg, right_leg, cape)
    }
}
