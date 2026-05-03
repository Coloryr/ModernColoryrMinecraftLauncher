pub mod texture {
    use mcml_skin::SkinType;

    use crate::cube_model::SteveTexture;

    const HEAD_TEX: [f32; 48] = [
        // back
        32.0, 8.0, 32.0, 16.0, 24.0, 16.0, 24.0, 8.0, // front
        8.0, 8.0, 8.0, 16.0, 16.0, 16.0, 16.0, 8.0, // left
        0.0, 8.0, 0.0, 16.0, 8.0, 16.0, 8.0, 8.0, // right
        16.0, 8.0, 16.0, 16.0, 24.0, 16.0, 24.0, 8.0, // top
        8.0, 0.0, 8.0, 8.0, 16.0, 8.0, 16.0, 0.0, // bottom
        24.0, 0.0, 24.0, 8.0, 16.0, 8.0, 16.0, 0.0,
    ];

    const LEG_ARM_TEX: [f32; 48] = [
        // back
        12.0, 4.0, 12.0, 16.0, 16.0, 16.0, 16.0, 4.0, // front
        4.0, 4.0, 4.0, 16.0, 8.0, 16.0, 8.0, 4.0, // left
        0.0, 4.0, 0.0, 16.0, 4.0, 16.0, 4.0, 4.0, // right
        8.0, 4.0, 8.0, 16.0, 12.0, 16.0, 12.0, 4.0, // top
        4.0, 0.0, 4.0, 4.0, 8.0, 4.0, 8.0, 0.0, // bottom
        12.0, 0.0, 12.0, 4.0, 8.0, 4.0, 8.0, 0.0,
    ];

    const SLIM_ARM_TEX: [f32; 48] = [
        // back
        11.0, 4.0, 11.0, 16.0, 14.0, 16.0, 14.0, 4.0, // front
        4.0, 4.0, 4.0, 16.0, 7.0, 16.0, 7.0, 4.0, // left
        0.0, 4.0, 0.0, 16.0, 4.0, 16.0, 4.0, 4.0, // right
        7.0, 4.0, 7.0, 16.0, 10.0, 16.0, 10.0, 4.0, // top
        4.0, 0.0, 4.0, 4.0, 7.0, 4.0, 7.0, 0.0, // bottom
        10.0, 0.0, 10.0, 4.0, 7.0, 4.0, 7.0, 0.0,
    ];

    const BODY_TEX: [f32; 48] = [
        // back
        24.0, 4.0, 24.0, 16.0, 16.0, 16.0, 16.0, 4.0, // front
        4.0, 4.0, 4.0, 16.0, 12.0, 16.0, 12.0, 4.0, // left
        0.0, 4.0, 0.0, 16.0, 4.0, 16.0, 4.0, 4.0, // right
        12.0, 4.0, 12.0, 16.0, 16.0, 16.0, 16.0, 4.0, // top
        4.0, 0.0, 4.0, 4.0, 12.0, 4.0, 12.0, 0.0, // bottom
        20.0, 0.0, 20.0, 4.0, 12.0, 4.0, 12.0, 0.0,
    ];

    const CAPE_TEX: [f32; 48] = [
        // back
        11.0, 1.0, 11.0, 17.0, 1.0, 17.0, 1.0, 1.0, // front
        12.0, 1.0, 12.0, 17.0, 22.0, 17.0, 22.0, 1.0, // left
        11.0, 1.0, 11.0, 17.0, 12.0, 17.0, 12.0, 1.0, // right
        0.0, 1.0, 0.0, 17.0, 1.0, 17.0, 1.0, 1.0, // top
        1.0, 0.0, 1.0, 1.0, 11.0, 1.0, 11.0, 0.0, // bottom
        21.0, 0.0, 21.0, 1.0, 11.0, 1.0, 11.0, 0.0,
    ];

    /// 获取UV
    pub fn get_tex(input: &[f32], skin_type: SkinType, offset_u: f32, offset_v: f32) -> Vec<f32> {
        let mut temp = vec![0.0; input.len()];

        for (a, value) in temp.iter_mut().enumerate() {
            if a % 2 == 0 {
                *value = input[a] + offset_u;
            } else {
                *value = input[a] + offset_v;
            }

            if a % 2 != 0 && skin_type == SkinType::Old {
                *value /= 32.0;
            } else {
                *value /= 64.0;
            }
        }

        temp
    }

    /// 获取披风UV
    pub fn get_cap_tex(input: &[f32]) -> Vec<f32> {
        let mut temp = vec![0.0; input.len()];

        for (a, value) in temp.iter_mut().enumerate() {
            *value = input[a];
            if a % 2 == 0 {
                *value /= 64.0;
            } else {
                *value /= 32.0;
            }
        }

        temp
    }

    /// 顶层数据
    pub fn get_steve_texture_top(skin_type: SkinType) -> SteveTexture {
        let mut tex = SteveTexture::new();
        tex.head = get_tex(&HEAD_TEX, skin_type, 32.0, 0.0);

        if skin_type != SkinType::Old {
            tex.body = get_tex(&BODY_TEX, skin_type, 16.0, 32.0);

            let arm_tex = if skin_type == SkinType::NewSlim {
                &SLIM_ARM_TEX
            } else {
                &LEG_ARM_TEX
            };
            tex.left_arm = get_tex(arm_tex, skin_type, 48.0, 48.0);
            tex.right_arm = get_tex(arm_tex, skin_type, 40.0, 32.0);
            tex.left_leg = get_tex(&LEG_ARM_TEX, skin_type, 0.0, 48.0);
            tex.right_leg = get_tex(&LEG_ARM_TEX, skin_type, 0.0, 32.0);
        }

        tex
    }

    /// 本体数据
    pub fn get_steve_texture(skin_type: SkinType) -> SteveTexture {
        let mut tex = SteveTexture::new();
        tex.head = get_tex(&HEAD_TEX, skin_type, 0.0, 0.0);
        tex.body = get_tex(&BODY_TEX, skin_type, 16.0, 16.0);
        tex.cape = get_cap_tex(&CAPE_TEX);

        if skin_type == SkinType::Old {
            tex.left_arm = get_tex(&LEG_ARM_TEX, skin_type, 40.0, 16.0);
            tex.right_arm = get_tex(&LEG_ARM_TEX, skin_type, 40.0, 16.0);
            tex.left_leg = get_tex(&LEG_ARM_TEX, skin_type, 0.0, 16.0);
            tex.right_leg = get_tex(&LEG_ARM_TEX, skin_type, 0.0, 16.0);
        } else {
            let arm_tex = if skin_type == SkinType::NewSlim {
                &SLIM_ARM_TEX
            } else {
                &LEG_ARM_TEX
            };
            tex.left_arm = get_tex(arm_tex, skin_type, 32.0, 48.0);
            tex.right_arm = get_tex(arm_tex, skin_type, 40.0, 16.0);
            tex.left_leg = get_tex(&LEG_ARM_TEX, skin_type, 0.0, 16.0);
            tex.right_leg = get_tex(&LEG_ARM_TEX, skin_type, 16.0, 48.0);
        }

        tex
    }
}
