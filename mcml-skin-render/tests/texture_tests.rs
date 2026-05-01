use mcml_skin_render::texture::texture::{SkinType, get_tex, get_cap_tex, get_steve_texture, get_steve_texture_top};

#[test]
fn test_skin_type_variants() {
    assert_eq!(SkinType::Old as u8, 0);
    assert_eq!(SkinType::New as u8, 1);
    assert_eq!(SkinType::NewSlim as u8, 2);
    assert_eq!(SkinType::Unknown as u8, 3);
}

#[test]
fn test_skin_type_debug() {
    assert_eq!(format!("{:?}", SkinType::Old), "Old");
    assert_eq!(format!("{:?}", SkinType::New), "New");
    assert_eq!(format!("{:?}", SkinType::NewSlim), "NewSlim");
    assert_eq!(format!("{:?}", SkinType::Unknown), "Unknown");
}

#[test]
fn test_skin_type_clone_eq() {
    let a = SkinType::New;
    let b = a;
    assert_eq!(a, b);
    assert_eq!(a, SkinType::New);
    assert_ne!(a, SkinType::Old);
}

#[test]
fn test_get_tex_new_skin() {
    // HEAD_TEX 有 48 个元素
    let input = vec![0.0; 48];
    let result = get_tex(&input, SkinType::New, 0.0, 0.0);
    assert_eq!(result.len(), 48);

    // 新皮肤除以 64
    for val in &result {
        assert!((*val).abs() < f32::EPSILON);
    }
}

#[test]
fn test_get_tex_old_skin() {
    let input = vec![32.0; 48];
    let result = get_tex(&input, SkinType::Old, 0.0, 0.0);

    assert_eq!(result.len(), 48);
    // 旧皮肤的 V 坐标除以 32，U 坐标除以 64
    for (i, val) in result.iter().enumerate() {
        if i % 2 == 0 {
            // U 坐标除以 64
            assert!((val - 0.5).abs() < f32::EPSILON, "U[{}] = {}", i, val);
        } else {
            // V 坐标除以 32
            assert!((val - 1.0).abs() < f32::EPSILON, "V[{}] = {}", i, val);
        }
    }
}

#[test]
fn test_get_tex_with_offset() {
    let input = vec![8.0; 48];
    let result = get_tex(&input, SkinType::New, 16.0, 32.0);

    assert_eq!(result.len(), 48);
    for (i, val) in result.iter().enumerate() {
        if i % 2 == 0 {
            // U = (8 + 16) / 64 = 24/64 = 0.375
            assert!((val - 0.375).abs() < f32::EPSILON, "U[{}] = {}", i, val);
        } else {
            // V = (8 + 32) / 64 = 40/64 = 0.625
            assert!((val - 0.625).abs() < f32::EPSILON, "V[{}] = {}", i, val);
        }
    }
}

#[test]
fn test_get_cap_tex() {
    let input = vec![32.0; 48];
    let result = get_cap_tex(&input);

    assert_eq!(result.len(), 48);
    for (i, val) in result.iter().enumerate() {
        if i % 2 == 0 {
            // U 除以 64
            assert!((val - 0.5).abs() < f32::EPSILON, "U[{}] = {}", i, val);
        } else {
            // V 除以 32
            assert!((val - 1.0).abs() < f32::EPSILON, "V[{}] = {}", i, val);
        }
    }
}

#[test]
fn test_get_steve_texture_new() {
    let tex = get_steve_texture(SkinType::New);
    assert_eq!(tex.head.len(), 48);
    assert_eq!(tex.body.len(), 48);
    assert_eq!(tex.cape.len(), 48);
    assert_eq!(tex.left_arm.len(), 48);
    assert_eq!(tex.right_arm.len(), 48);
    assert_eq!(tex.left_leg.len(), 48);
    assert_eq!(tex.right_leg.len(), 48);
}

#[test]
fn test_get_steve_texture_old() {
    let tex = get_steve_texture(SkinType::Old);
    assert_eq!(tex.head.len(), 48);
    assert_eq!(tex.body.len(), 48);
    assert_eq!(tex.cape.len(), 48);
    assert_eq!(tex.left_arm.len(), 48);
    assert_eq!(tex.right_arm.len(), 48);
    assert_eq!(tex.left_leg.len(), 48);
    assert_eq!(tex.right_leg.len(), 48);

    // 旧版左右手臂纹理相同
    for (a, b) in tex.left_arm.iter().zip(tex.right_arm.iter()) {
        assert!((a - b).abs() < f32::EPSILON);
    }
    // 旧版左右腿纹理相同
    for (a, b) in tex.left_leg.iter().zip(tex.right_leg.iter()) {
        assert!((a - b).abs() < f32::EPSILON);
    }
}

#[test]
fn test_get_steve_texture_slim() {
    let tex = get_steve_texture(SkinType::NewSlim);
    assert_eq!(tex.head.len(), 48);
    assert_eq!(tex.body.len(), 48);
    assert_eq!(tex.cape.len(), 48);
    assert_eq!(tex.left_arm.len(), 48);
    assert_eq!(tex.right_arm.len(), 48);
    assert_eq!(tex.left_leg.len(), 48);
    assert_eq!(tex.right_leg.len(), 48);

    // 纤细手臂纹理与普通手臂不同
    let normal_tex = get_steve_texture(SkinType::New);
    let mut is_different = false;
    for (a, b) in tex.left_arm.iter().zip(normal_tex.left_arm.iter()) {
        if (a - b).abs() > f32::EPSILON {
            is_different = true;
            break;
        }
    }
    assert!(is_different, "纤细手臂纹理应与普通手臂不同");
}

#[test]
fn test_get_steve_texture_top_new() {
    let tex = get_steve_texture_top(SkinType::New);
    assert_eq!(tex.head.len(), 48);
    assert_eq!(tex.body.len(), 48);
    assert_eq!(tex.left_arm.len(), 48);
    assert_eq!(tex.right_arm.len(), 48);
    assert_eq!(tex.left_leg.len(), 48);
    assert_eq!(tex.right_leg.len(), 48);
    // 顶层披风为空
    assert!(tex.cape.is_empty());
}

#[test]
fn test_get_steve_texture_top_old() {
    let tex = get_steve_texture_top(SkinType::Old);
    assert_eq!(tex.head.len(), 48);
    // 旧版顶层只有头部
    assert!(tex.body.is_empty());
    assert!(tex.left_arm.is_empty());
    assert!(tex.right_arm.is_empty());
    assert!(tex.left_leg.is_empty());
    assert!(tex.right_leg.is_empty());
    assert!(tex.cape.is_empty());
}

#[test]
fn test_get_steve_texture_top_slim() {
    let tex = get_steve_texture_top(SkinType::NewSlim);
    assert_eq!(tex.head.len(), 48);
    assert_eq!(tex.body.len(), 48);
    assert_eq!(tex.left_arm.len(), 48);
    assert_eq!(tex.right_arm.len(), 48);
    assert_eq!(tex.left_leg.len(), 48);
    assert_eq!(tex.right_leg.len(), 48);
    assert!(tex.cape.is_empty());
}

#[test]
fn test_texture_values_in_range() {
    let tex = get_steve_texture(SkinType::New);

    // 所有纹理坐标应该在 0.0 到 1.0 之间
    for val in tex.head.iter() {
        assert!(*val >= 0.0 && *val <= 1.0, "head value {} out of range", val);
    }
    for val in tex.body.iter() {
        assert!(*val >= 0.0 && *val <= 1.0, "body value {} out of range", val);
    }
    for val in tex.left_arm.iter() {
        assert!(*val >= 0.0 && *val <= 1.0, "arm value {} out of range", val);
    }
}

#[test]
fn test_head_texture_specific_values() {
    // 验证头部纹理的特定值
    let tex = get_steve_texture(SkinType::New);

    // HEAD_TEX 第一个值是 32.0，偏移 0.0，除以 64 = 0.5
    assert!((tex.head[0] - 0.5).abs() < f32::EPSILON, "head[0] = {}", tex.head[0]);
    // HEAD_TEX 第二个值是 8.0，偏移 0.0，除以 64 = 0.125
    assert!((tex.head[1] - 0.125).abs() < f32::EPSILON, "head[1] = {}", tex.head[1]);
}
