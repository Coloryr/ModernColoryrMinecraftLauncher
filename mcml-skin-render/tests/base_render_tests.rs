use glam::{Vec2, Vec3, Vec4};
use mcml_skin_render::base_render::{
    BaseSkinRender, ErrorType, KeyType, ModelPartType, SkinRenderType, StateType,
};
use mcml_skin_render::texture::texture::SkinType;

#[test]
fn test_base_skin_render_new() {
    let render = BaseSkinRender::new();
    assert!(!render.enable_cape);
    assert!(!render.enable_top);
    assert!(!render.animation);
    assert_eq!(render.render_type, SkinRenderType::Normal);
    assert_eq!(render.back_color, Vec4::new(0.0, 0.0, 0.0, 1.0));
    assert_eq!(render.skin_type, SkinType::Unknown);
    assert!(render.skin_tex.is_none());
    assert!(render.cape.is_none());
    assert!(!render.have_cape);
    assert!(!render.have_skin);
    assert_eq!(render.width, 800);
    assert_eq!(render.height, 600);
    assert!((render.distance - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_set_get_animation() {
    let mut render = BaseSkinRender::new();
    assert!(!render.get_animation());

    render.set_animation(true);
    assert!(render.get_animation());
    assert!(render.animation);

    render.set_animation(false);
    assert!(!render.get_animation());
}

#[test]
fn test_set_get_skin_type() {
    let mut render = BaseSkinRender::new();
    assert_eq!(render.get_skin_type(), SkinType::Unknown);

    render.set_skin_type(SkinType::New);
    assert_eq!(render.get_skin_type(), SkinType::New);
    assert!(render.switch_model);

    render.set_skin_type(SkinType::New);
    // 相同类型不会触发 switch_model
    assert!(render.switch_model);
}

#[test]
fn test_set_get_back_color() {
    let mut render = BaseSkinRender::new();
    assert_eq!(render.get_back_color(), Vec4::new(0.0, 0.0, 0.0, 1.0));

    render.set_back_color(Vec4::new(0.5, 0.5, 0.5, 1.0));
    assert_eq!(render.get_back_color(), Vec4::new(0.5, 0.5, 0.5, 1.0));
    assert!(render.switch_back);
}

#[test]
fn test_set_get_render_type() {
    let mut render = BaseSkinRender::new();
    assert_eq!(render.get_render_type(), SkinRenderType::Normal);

    render.set_render_type(SkinRenderType::FXAA);
    assert_eq!(render.get_render_type(), SkinRenderType::FXAA);
    assert!(render.switch_type);
}

#[test]
fn test_set_get_enable_cape() {
    let mut render = BaseSkinRender::new();
    assert!(!render.get_enable_cape());

    render.set_enable_cape(true);
    assert!(render.get_enable_cape());
    assert!(render.switch_type);
}

#[test]
fn test_set_get_enable_top() {
    let mut render = BaseSkinRender::new();
    assert!(!render.get_enable_top());

    render.set_enable_top(true);
    assert!(render.get_enable_top());
    assert!(render.switch_type);
}

#[test]
fn test_set_get_arm_rotate() {
    let mut render = BaseSkinRender::new();
    assert_eq!(render.get_arm_rotate(), Vec3::new(0.0, 0.0, 0.0));

    render.set_arm_rotate(Vec3::new(10.0, 20.0, 30.0));
    assert_eq!(render.get_arm_rotate(), Vec3::new(10.0, 20.0, 30.0));
}

#[test]
fn test_set_get_leg_rotate() {
    let mut render = BaseSkinRender::new();
    assert_eq!(render.get_leg_rotate(), Vec3::new(0.0, 0.0, 0.0));

    render.set_leg_rotate(Vec3::new(5.0, 10.0, 15.0));
    assert_eq!(render.get_leg_rotate(), Vec3::new(5.0, 10.0, 15.0));
}

#[test]
fn test_set_get_head_rotate() {
    let mut render = BaseSkinRender::new();
    assert_eq!(render.get_head_rotate(), Vec3::new(0.0, 0.0, 0.0));

    render.set_head_rotate(Vec3::new(15.0, 30.0, 45.0));
    assert_eq!(render.get_head_rotate(), Vec3::new(15.0, 30.0, 45.0));
}

#[test]
fn test_have_cape_skin() {
    let mut render = BaseSkinRender::new();
    assert!(!render.have_cape());
    assert!(!render.have_skin());

    render.have_cape = true;
    render.have_skin = true;
    assert!(render.have_cape());
    assert!(render.have_skin());
}

#[test]
fn test_pointer_pressed_left() {
    let mut render = BaseSkinRender::new();
    render.pointer_pressed(KeyType::Left, Vec2::new(100.0, 200.0));

    assert!((render.diff_xy.x - 100.0).abs() < f32::EPSILON);
    assert!((render.diff_xy.y - (-200.0)).abs() < f32::EPSILON);
}

#[test]
fn test_pointer_pressed_right() {
    let mut render = BaseSkinRender::new();
    render.pointer_pressed(KeyType::Right, Vec2::new(300.0, 400.0));

    assert!((render.last_xy.x - 300.0).abs() < f32::EPSILON);
    assert!((render.last_xy.y - 400.0).abs() < f32::EPSILON);
}

#[test]
fn test_pointer_moved_left() {
    let mut render = BaseSkinRender::new();
    render.pointer_pressed(KeyType::Left, Vec2::new(100.0, 200.0));
    render.pointer_moved(KeyType::Left, Vec2::new(150.0, 250.0));

    // rot_xy.y = (150 - 100) * 2 = 100
    // rot_xy.x = (250 + (-200)) * 2 = 100
    assert!((render.rot_xy.x - 100.0).abs() < f32::EPSILON, "rot_xy.x = {}", render.rot_xy.x);
    assert!((render.rot_xy.y - 100.0).abs() < f32::EPSILON, "rot_xy.y = {}", render.rot_xy.y);
}

#[test]
fn test_pointer_moved_right() {
    let mut render = BaseSkinRender::new();
    render.pointer_pressed(KeyType::Right, Vec2::new(100.0, 200.0));
    render.pointer_moved(KeyType::Right, Vec2::new(200.0, 300.0));

    // xy.x = -(100 - 200) / 100 + 0 = 1.0
    // xy.y = (200 - 300) / 100 + 0 = -1.0
    assert!((render.xy.x - 1.0).abs() < f32::EPSILON, "xy.x = {}", render.xy.x);
    assert!((render.xy.y - (-1.0)).abs() < f32::EPSILON, "xy.y = {}", render.xy.y);
}

#[test]
fn test_pointer_released_right() {
    let mut render = BaseSkinRender::new();
    render.xy = Vec2::new(2.0, 3.0);
    render.pointer_released(KeyType::Right, Vec2::new(0.0, 0.0));

    assert!((render.save_xy.x - 2.0).abs() < f32::EPSILON);
    assert!((render.save_xy.y - 3.0).abs() < f32::EPSILON);
}

#[test]
fn test_pointer_wheel_changed() {
    let mut render = BaseSkinRender::new();
    let initial_distance = render.distance;

    render.pointer_wheel_changed(true);
    assert!((render.distance - initial_distance - 0.1).abs() < f32::EPSILON);

    render.pointer_wheel_changed(false);
    assert!((render.distance - initial_distance).abs() < f32::EPSILON);
}

#[test]
fn test_rotate() {
    let mut render = BaseSkinRender::new();
    render.rotate(10.0, 20.0);

    assert!((render.rot_xy.x - 10.0).abs() < f32::EPSILON);
    assert!((render.rot_xy.y - 20.0).abs() < f32::EPSILON);
}

#[test]
fn test_position() {
    let mut render = BaseSkinRender::new();
    render.position(1.5, 2.5);

    assert!((render.xy.x - 1.5).abs() < f32::EPSILON);
    assert!((render.xy.y - 2.5).abs() < f32::EPSILON);
}

#[test]
fn test_add_distance() {
    let mut render = BaseSkinRender::new();
    render.add_distance(0.5);
    assert!((render.distance - 1.5).abs() < f32::EPSILON);

    render.add_distance(-0.3);
    assert!((render.distance - 1.2).abs() < f32::EPSILON);
}

#[test]
fn test_reset_position() {
    let mut render = BaseSkinRender::new();
    render.distance = 2.0;
    render.xy = Vec2::new(1.0, 2.0);
    render.rot_xy = Vec2::new(3.0, 4.0);

    render.reset_position();
    assert!((render.distance - 1.0).abs() < f32::EPSILON);
    assert_eq!(render.xy, Vec2::new(0.0, 0.0));
    // reset_position 不重置 rot_xy，rot_xy 在 tick 中被消耗
    assert_eq!(render.rot_xy, Vec2::new(3.0, 4.0));
}

#[test]
fn test_error_type_variants() {
    assert_eq!(format!("{:?}", ErrorType::InvalidSkin), "InvalidSkin");
    assert_eq!(format!("{:?}", ErrorType::UnknownSkin), "UnknownSkin");
    assert_eq!(format!("{:?}", ErrorType::RenderError), "RenderError");
    assert_eq!(format!("{:?}", ErrorType::TextureError), "TextureError");
}

#[test]
fn test_state_type_variants() {
    assert_eq!(format!("{:?}", StateType::Initialized), "Initialized");
    assert_eq!(format!("{:?}", StateType::SkinLoaded), "SkinLoaded");
    assert_eq!(format!("{:?}", StateType::CapeLoaded), "CapeLoaded");
    assert_eq!(format!("{:?}", StateType::RenderStarted), "RenderStarted");
    assert_eq!(format!("{:?}", StateType::RenderCompleted), "RenderCompleted");
    assert_eq!(format!("{:?}", StateType::Disposed), "Disposed");
}

#[test]
fn test_key_type_variants() {
    assert_eq!(format!("{:?}", KeyType::Left), "Left");
    assert_eq!(format!("{:?}", KeyType::Right), "Right");
    assert_eq!(format!("{:?}", KeyType::Middle), "Middle");
}

#[test]
fn test_model_part_type_variants() {
    assert_eq!(format!("{:?}", ModelPartType::Head), "Head");
    assert_eq!(format!("{:?}", ModelPartType::Body), "Body");
    assert_eq!(format!("{:?}", ModelPartType::Cape), "Cape");
    assert_eq!(format!("{:?}", ModelPartType::Model), "Model");
}

#[test]
fn test_skin_render_type_variants() {
    assert_eq!(format!("{:?}", SkinRenderType::Normal), "Normal");
    assert_eq!(format!("{:?}", SkinRenderType::FXAA), "FXAA");
    assert_eq!(format!("{:?}", SkinRenderType::MSAA), "MSAA");
}

#[test]
fn test_get_matrix_proj() {
    let render = BaseSkinRender::new();
    let proj = render.get_matrix(ModelPartType::Proj);
    // 透视矩阵应该不是单位矩阵
    assert_ne!(proj, glam::Mat4::IDENTITY);
}

#[test]
fn test_get_matrix_view() {
    let render = BaseSkinRender::new();
    let view = render.get_matrix(ModelPartType::View);
    assert_ne!(view, glam::Mat4::IDENTITY);
}

#[test]
fn test_get_matrix_body() {
    let render = BaseSkinRender::new();
    let body = render.get_matrix(ModelPartType::Body);
    assert_eq!(body, glam::Mat4::default());
}

#[test]
fn test_get_matrix_model() {
    let mut render = BaseSkinRender::new();
    // 设置一些偏移和缩放，使矩阵不是单位矩阵
    render.xy = Vec2::new(1.0, 2.0);
    render.distance = 1.5;
    let model = render.get_matrix(ModelPartType::Model);
    // 模型矩阵应该包含平移和缩放
    assert_ne!(model, glam::Mat4::IDENTITY);
}

#[test]
fn test_tick_fps_counting() {
    let mut render = BaseSkinRender::new();
    render.tick(0.016); // ~60 FPS
    assert_eq!(render.fps, 1);
    assert!((render.time - 0.016).abs() < f64::EPSILON);
}

#[test]
fn test_tick_rotation_accumulation() {
    let mut render = BaseSkinRender::new();
    render.rot_xy = Vec2::new(90.0, 180.0);
    render.tick(0.016);

    // 旋转被应用后重置为 0
    assert_eq!(render.rot_xy, Vec2::new(0.0, 0.0));
    // last 矩阵应该被更新
    assert_ne!(render.last, glam::Mat4::default());
}

#[test]
fn test_set_skin_tex_none() {
    let mut render = BaseSkinRender::new();
    let result = render.set_skin_tex(None);
    assert!(result.is_ok());
    assert!(!render.have_skin);
}

#[test]
fn test_error_type_clone_copy() {
    let error = ErrorType::InvalidSkin;
    let cloned = error;
    assert_eq!(error, cloned);
    assert_eq!(error as u8, 0);
    assert_eq!(ErrorType::UnknownSkin as u8, 1);
    assert_eq!(ErrorType::RenderError as u8, 2);
    assert_eq!(ErrorType::TextureError as u8, 3);
}
