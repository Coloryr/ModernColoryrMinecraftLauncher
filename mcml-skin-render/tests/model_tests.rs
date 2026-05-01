use mcml_skin_render::cube_model::{CubeModelItemObj, SteveModel, SteveTexture};
use mcml_skin_render::model::model;
use mcml_skin_render::texture::texture::SkinType;

#[test]
fn test_cube_model_item_obj_new() {
    let model = CubeModelItemObj::new(vec![1.0, 2.0, 3.0], vec![0, 1, 2]);
    assert_eq!(model.model, vec![1.0, 2.0, 3.0]);
    assert_eq!(model.point, vec![0, 1, 2]);
}

#[test]
fn test_cube_model_item_obj_default() {
    let model = CubeModelItemObj::default();
    assert!(model.model.is_empty());
    assert!(model.point.is_empty());
}

#[test]
fn test_steve_model_new() {
    let head = CubeModelItemObj::new(vec![1.0], vec![0]);
    let body = CubeModelItemObj::new(vec![2.0], vec![1]);
    let left_arm = CubeModelItemObj::new(vec![3.0], vec![2]);
    let right_arm = CubeModelItemObj::new(vec![4.0], vec![3]);
    let left_leg = CubeModelItemObj::new(vec![5.0], vec![4]);
    let right_leg = CubeModelItemObj::new(vec![6.0], vec![5]);
    let cape = CubeModelItemObj::new(vec![7.0], vec![6]);

    let steve = SteveModel::new(head, body, left_arm, right_arm, left_leg, right_leg, cape);

    assert_eq!(steve.head.model, vec![1.0]);
    assert_eq!(steve.body.model, vec![2.0]);
    assert_eq!(steve.left_arm.model, vec![3.0]);
    assert_eq!(steve.right_arm.model, vec![4.0]);
    assert_eq!(steve.left_leg.model, vec![5.0]);
    assert_eq!(steve.right_leg.model, vec![6.0]);
    assert_eq!(steve.cape.model, vec![7.0]);
}

#[test]
fn test_steve_texture_new() {
    let tex = SteveTexture::new();
    assert!(tex.head.is_empty());
    assert!(tex.body.is_empty());
    assert!(tex.left_arm.is_empty());
    assert!(tex.right_arm.is_empty());
    assert!(tex.left_leg.is_empty());
    assert!(tex.right_leg.is_empty());
    assert!(tex.cape.is_empty());
}

#[test]
fn test_steve_texture_default() {
    let tex = SteveTexture::default();
    assert!(tex.head.is_empty());
    assert!(tex.body.is_empty());
    assert!(tex.left_arm.is_empty());
    assert!(tex.right_arm.is_empty());
    assert!(tex.left_leg.is_empty());
    assert!(tex.right_leg.is_empty());
    assert!(tex.cape.is_empty());
}

#[test]
fn test_get_steve_new_skin() {
    let steve = model::get_steve(SkinType::New);
    assert_eq!(steve.head.model.len(), 72);
    assert_eq!(steve.head.point.len(), 36);
    assert_eq!(steve.body.model.len(), 72);
    assert_eq!(steve.left_arm.model.len(), 72);
    assert_eq!(steve.right_arm.model.len(), 72);
    assert_eq!(steve.left_leg.model.len(), 72);
    assert_eq!(steve.right_leg.model.len(), 72);
    assert_eq!(steve.cape.model.len(), 72);
}

#[test]
fn test_get_steve_slim_skin() {
    let steve = model::get_steve(SkinType::NewSlim);
    assert_eq!(steve.head.model.len(), 72);
    assert_eq!(steve.left_arm.model.len(), 72);
    assert_eq!(steve.right_arm.model.len(), 72);

    // 纤细手臂的宽度不同，验证左右手臂数据相同（clone）
    for (a, b) in steve.left_arm.model.iter().zip(steve.right_arm.model.iter()) {
        assert!((a - b).abs() < f32::EPSILON);
    }
}

#[test]
fn test_get_steve_old_skin() {
    let steve = model::get_steve(SkinType::Old);
    assert_eq!(steve.head.model.len(), 72);
    assert_eq!(steve.body.model.len(), 72);
    assert_eq!(steve.left_arm.model.len(), 72);
    assert_eq!(steve.right_arm.model.len(), 72);
    assert_eq!(steve.left_leg.model.len(), 72);
    assert_eq!(steve.right_leg.model.len(), 72);
    assert_eq!(steve.cape.model.len(), 72);
}

#[test]
fn test_get_steve_top_new_skin() {
    let steve = model::get_steve_top(SkinType::New);
    assert_eq!(steve.head.model.len(), 72);
    assert_eq!(steve.body.model.len(), 72);
    assert_eq!(steve.left_arm.model.len(), 72);
    assert_eq!(steve.right_arm.model.len(), 72);
    assert_eq!(steve.left_leg.model.len(), 72);
    assert_eq!(steve.right_leg.model.len(), 72);
    // 披风在顶层为空
    assert!(steve.cape.model.is_empty());
}

#[test]
fn test_get_steve_top_old_skin() {
    let steve = model::get_steve_top(SkinType::Old);
    assert_eq!(steve.head.model.len(), 72);
    // 旧版皮肤顶层只有头部
    assert!(steve.body.model.is_empty());
    assert!(steve.left_arm.model.is_empty());
    assert!(steve.right_arm.model.is_empty());
    assert!(steve.left_leg.model.is_empty());
    assert!(steve.right_leg.model.is_empty());
    assert!(steve.cape.model.is_empty());
}

#[test]
fn test_get_steve_top_slim_skin() {
    let steve = model::get_steve_top(SkinType::NewSlim);
    assert_eq!(steve.head.model.len(), 72);
    assert_eq!(steve.body.model.len(), 72);
    assert_eq!(steve.left_arm.model.len(), 72);
    assert_eq!(steve.right_arm.model.len(), 72);
    assert_eq!(steve.left_leg.model.len(), 72);
    assert_eq!(steve.right_leg.model.len(), 72);
    assert!(steve.cape.model.is_empty());
}

#[test]
fn test_steve_model_body_dimensions() {
    let steve = model::get_steve(SkinType::New);

    // 身体应该是 1.0 x 1.5 x 0.5 的缩放
    let body_max_x = steve.body.model.iter().step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);
    let body_max_y = steve.body.model.iter().skip(1).step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);
    let body_max_z = steve.body.model.iter().skip(2).step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);

    assert!((body_max_x - 0.5).abs() < f32::EPSILON, "body max_x = {}", body_max_x);
    assert!((body_max_y - 0.75).abs() < f32::EPSILON, "body max_y = {}", body_max_y);
    assert!((body_max_z - 0.25).abs() < f32::EPSILON, "body max_z = {}", body_max_z);
}

#[test]
fn test_steve_model_leg_dimensions() {
    let steve = model::get_steve(SkinType::New);

    // 腿应该是 0.5 x 1.5 x 0.5 的缩放
    let leg_max_x = steve.left_leg.model.iter().step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);
    let leg_max_y = steve.left_leg.model.iter().skip(1).step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);

    assert!((leg_max_x - 0.25).abs() < f32::EPSILON, "leg max_x = {}", leg_max_x);
    assert!((leg_max_y - 0.75).abs() < f32::EPSILON, "leg max_y = {}", leg_max_y);
}

#[test]
fn test_steve_model_cape_dimensions() {
    let steve = model::get_steve(SkinType::New);

    // 披风应该是 1.25 x 2.0 x 0.1 的缩放
    let cape_max_x = steve.cape.model.iter().step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);
    let cape_max_y = steve.cape.model.iter().skip(1).step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);
    let cape_max_z = steve.cape.model.iter().skip(2).step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);

    assert!((cape_max_x - 0.625).abs() < f32::EPSILON, "cape max_x = {}", cape_max_x);
    assert!((cape_max_y - 1.0).abs() < f32::EPSILON, "cape max_y = {}", cape_max_y);
    assert!((cape_max_z - 0.05).abs() < f32::EPSILON, "cape max_z = {}", cape_max_z);
}
