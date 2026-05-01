use mcml_skin_render::skin_animation::SkinAnimation;
use mcml_skin_render::texture::texture::SkinType;

#[test]
fn test_skin_animation_new() {
    let anim = SkinAnimation::new();
    assert_eq!(anim.get_frame(), 0);
    assert!(!anim.run);
    assert!((anim.arm.x - 40.0).abs() < f32::EPSILON);
    assert!((anim.arm.y).abs() < f32::EPSILON);
    assert!((anim.arm.z).abs() < f32::EPSILON);
    assert!((anim.leg.x).abs() < f32::EPSILON);
    assert!((anim.leg.y).abs() < f32::EPSILON);
    assert!((anim.leg.z).abs() < f32::EPSILON);
    assert!((anim.head.x).abs() < f32::EPSILON);
    assert!((anim.head.y).abs() < f32::EPSILON);
    assert!((anim.head.z).abs() < f32::EPSILON);
    assert!((anim.cape).abs() < f32::EPSILON);
}

#[test]
fn test_skin_animation_tick_not_running() {
    let mut anim = SkinAnimation::new();
    anim.run = false;
    let result = anim.tick(0.05);
    assert!(result); // 返回 true 因为 close 为 false
    assert_eq!(anim.get_frame(), 0);
}

#[test]
fn test_skin_animation_tick_running() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::New;

    // 运行一帧
    let result = anim.tick(0.05);
    assert!(result);
    assert!(anim.get_frame() > 0);
}

#[test]
fn test_skin_animation_tick_frame_cycle() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::New;

    // 模拟足够的时间让帧数超过 120
    for _ in 0..200 {
        anim.tick(0.05);
    }

    // 帧数应该在 0-119 之间循环
    assert!(anim.get_frame() >= 0 && anim.get_frame() < 120);
}

#[test]
fn test_skin_animation_close() {
    let mut anim = SkinAnimation::new();
    anim.run = true;

    anim.close();
    assert!(!anim.run);

    // close 后 tick 返回 false
    let result = anim.tick(0.05);
    assert!(!result);
}

#[test]
fn test_skin_animation_reset() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::New;

    // 运行一些帧
    anim.tick(0.5);
    assert!(anim.get_frame() > 0);

    // 重置
    anim.reset();
    assert_eq!(anim.get_frame(), 0);
    assert!((anim.arm.x - 40.0).abs() < f32::EPSILON);
    assert!((anim.arm.y).abs() < f32::EPSILON);
    assert!((anim.leg.x).abs() < f32::EPSILON);
    assert!((anim.leg.y).abs() < f32::EPSILON);
    assert!((anim.head.x).abs() < f32::EPSILON);
    assert!((anim.head.z).abs() < f32::EPSILON);
    assert!((anim.cape).abs() < f32::EPSILON);
}

#[test]
fn test_skin_animation_set_frame() {
    let mut anim = SkinAnimation::new();
    anim.set_frame(50);
    assert_eq!(anim.get_frame(), 50);

    anim.set_frame(150);
    assert_eq!(anim.get_frame(), 30); // 150 % 120 = 30

    anim.set_frame(0);
    assert_eq!(anim.get_frame(), 0);
}

#[test]
fn test_skin_animation_arm_rotation_new_skin() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::New;

    // 运行到 frame 30（前半段）
    anim.set_frame(30);
    anim.tick(0.0);

    // frame 30: arm.y = 30 * 6 - 180 = 0
    assert!((anim.arm.y).abs() < f32::EPSILON, "arm.y = {}", anim.arm.y);
}

#[test]
fn test_skin_animation_leg_rotation() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::New;

    // 运行到 frame 30（前半段）
    anim.set_frame(30);
    anim.tick(0.0);

    // frame 30: leg.y = 90 - 30 * 3 = 0
    assert!((anim.leg.y).abs() < f32::EPSILON, "leg.y = {}", anim.leg.y);
}

#[test]
fn test_skin_animation_cape_movement() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::New;

    // frame 10: cape = 10 / 10 = 1.0
    anim.set_frame(10);
    anim.tick(0.0);
    assert!((anim.cape - 1.0).abs() < f32::EPSILON, "cape = {}", anim.cape);

    // frame 70: cape = 6 - (70 - 60) / 10 = 5.0
    anim.set_frame(70);
    anim.tick(0.0);
    assert!((anim.cape - 5.0).abs() < f32::EPSILON, "cape = {}", anim.cape);
}

#[test]
fn test_skin_animation_head_new_slim() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::NewSlim;

    // frame 30: head.x = 30 - 30 = 0, head.z = 0
    anim.set_frame(30);
    anim.tick(0.0);
    assert!((anim.head.x).abs() < f32::EPSILON, "head.x = {}", anim.head.x);
    assert!((anim.head.z).abs() < f32::EPSILON, "head.z = {}", anim.head.z);
}

#[test]
fn test_skin_animation_head_new_normal() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::New;

    // frame 30: head.z = 30 - 30 = 0, head.x = 0
    anim.set_frame(30);
    anim.tick(0.0);
    assert!((anim.head.x).abs() < f32::EPSILON, "head.x = {}", anim.head.x);
    assert!((anim.head.z).abs() < f32::EPSILON, "head.z = {}", anim.head.z);
}

#[test]
fn test_skin_animation_second_half() {
    let mut anim = SkinAnimation::new();
    anim.run = true;
    anim.skin_type = SkinType::New;

    // frame 90（后半段）
    anim.set_frame(90);
    anim.tick(0.0);

    // arm.y = 540 - 90 * 6 = 0
    assert!((anim.arm.y).abs() < f32::EPSILON, "arm.y = {}", anim.arm.y);

    // leg.y = 90 * 3 - 270 = 0
    assert!((anim.leg.y).abs() < f32::EPSILON, "leg.y = {}", anim.leg.y);
}
