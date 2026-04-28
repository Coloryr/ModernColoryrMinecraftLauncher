use glam::{Mat4, Vec2, Vec3, Vec4};
use skia_safe::Bitmap;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

use crate::cube::cube;
use crate::cube_model::{SteveModel, SteveTexture};
use crate::skin_animation::SkinAnimation;
use crate::texture::texture::SkinType;

/// 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    InvalidSkin,
    UnknownSkin,
    RenderError,
    TextureError,
}

/// 状态类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateType {
    Initialized,
    SkinLoaded,
    CapeLoaded,
    RenderStarted,
    RenderCompleted,
    Disposed,
}

/// 按键类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    Left,
    Right,
    Middle,
}

/// 模型部件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPartType {
    Head,
    Body,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
    Cape,
    Proj,
    View,
    Model,
}

/// 渲染类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinRenderType {
    Normal,
    FXAA,
    MSAA,
}

/// 抽象皮肤渲染器
pub trait SkinRender: Send + Sync {
    // 属性获取器
    fn have_cape(&self) -> bool;
    fn have_skin(&self) -> bool;
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn info(&self) -> &str;

    // 属性设置器
    fn set_animation(&mut self, value: bool);
    fn get_animation(&self) -> bool;

    fn set_skin_type(&mut self, value: SkinType);
    fn get_skin_type(&self) -> SkinType;

    fn set_back_color(&mut self, color: Vec4);
    fn get_back_color(&self) -> Vec4;

    fn set_render_type(&mut self, value: SkinRenderType);
    fn get_render_type(&self) -> SkinRenderType;

    fn set_enable_cape(&mut self, value: bool);
    fn get_enable_cape(&self) -> bool;

    fn set_enable_top(&mut self, value: bool);
    fn get_enable_top(&self) -> bool;

    fn set_arm_rotate(&mut self, rotate: Vec3);
    fn get_arm_rotate(&self) -> Vec3;

    fn set_leg_rotate(&mut self, rotate: Vec3);
    fn get_leg_rotate(&self) -> Vec3;

    fn set_head_rotate(&mut self, rotate: Vec3);
    fn get_head_rotate(&self) -> Vec3;

    // 交互方法
    fn pointer_pressed(&mut self, key_type: KeyType, point: Vec2);
    fn pointer_released(&mut self, key_type: KeyType, point: Vec2);
    fn pointer_moved(&mut self, key_type: KeyType, point: Vec2);
    fn pointer_wheel_changed(&mut self, is_post: bool);

    fn rotate(&mut self, x: f32, y: f32);
    fn position(&mut self, x: f32, y: f32);
    fn add_distance(&mut self, x: f32);

    fn set_skin_tex(&mut self, skin: Option<Bitmap>) -> Result<(), ErrorType>;
    fn set_cape_tex(&mut self, cape: Option<Bitmap>) -> Result<(), ErrorType>;

    fn reset_position(&mut self);
    fn tick(&mut self, time: f64);

    // 事件
    fn on_error(&self, error: ErrorType);
    fn on_state_change(&self, state: StateType);
    fn on_fps_update(&self, fps: i32);

    // 渲染方法
    fn render(&mut self, model: &SteveModel, texture: &SteveTexture) -> Result<(), ErrorType>;
    fn get_matrix(&self, part_type: ModelPartType) -> Mat4;
}

/// 抽象皮肤渲染器基类
pub struct BaseSkinRender {
    // 标志位
    pub enable_cape: bool,
    pub enable_top: bool,
    pub switch_model: bool,
    pub switch_skin: bool,
    pub switch_type: bool,
    pub switch_back: bool,
    pub animation: bool,

    // 渲染状态
    pub render_type: SkinRenderType,
    pub back_color: Vec4,
    pub skin_type: SkinType,

    // 贴图
    pub skin_tex: Option<Bitmap>,
    pub cape: Option<Bitmap>,

    // 时间和性能
    pub time: f64,
    pub fps: i32,

    // 位置和旋转
    pub distance: f32,
    pub rot_xy: Vec2,
    pub diff_xy: Vec2,
    pub xy: Vec2,
    pub save_xy: Vec2,
    pub last_xy: Vec2,

    // 变换矩阵
    pub last: Mat4,

    // 动画
    pub skin_animation: SkinAnimation,

    // 状态
    pub have_cape: bool,
    pub have_skin: bool,
    pub info: String,

    // 旋转角度
    pub arm_rotate: Vec3,
    pub leg_rotate: Vec3,
    pub head_rotate: Vec3,

    // 事件回调 (使用Arc<Mutex<dyn Fn...>> 或者闭包)
    pub error_callback: Option<Arc<Mutex<dyn Fn(ErrorType) + Send + Sync>>>,
    pub state_callback: Option<Arc<Mutex<dyn Fn(StateType) + Send + Sync>>>,
    pub fps_callback: Option<Arc<Mutex<dyn Fn(i32) + Send + Sync>>>,

    // 画布尺寸
    pub width: i32,
    pub height: i32,
}

impl BaseSkinRender {
    pub fn new() -> Self {
        Self {
            enable_cape: false,
            enable_top: false,
            switch_model: false,
            switch_skin: false,
            switch_type: false,
            switch_back: false,
            animation: false,
            render_type: SkinRenderType::Normal,
            back_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            skin_type: SkinType::Unknown,
            skin_tex: None,
            cape: None,
            time: 0.0,
            fps: 0,
            distance: 1.0,
            rot_xy: Vec2::new(0.0, 0.0),
            diff_xy: Vec2::new(0.0, 0.0),
            xy: Vec2::new(0.0, 0.0),
            save_xy: Vec2::new(0.0, 0.0),
            last_xy: Vec2::new(0.0, 0.0),
            last: Mat4::default(),
            skin_animation: SkinAnimation::new(),
            have_cape: false,
            have_skin: false,
            info: String::new(),
            arm_rotate: Vec3::new(0.0, 0.0, 0.0),
            leg_rotate: Vec3::new(0.0, 0.0, 0.0),
            head_rotate: Vec3::new(0.0, 0.0, 0.0),
            error_callback: None,
            state_callback: None,
            fps_callback: None,
            width: 800,
            height: 600,
        }
    }

    pub fn pointer_pressed(&mut self, key_type: KeyType, point: Vec2) {
        match key_type {
            KeyType::Left => {
                self.diff_xy.x = point.x;
                self.diff_xy.y = -point.y;
            }
            KeyType::Right => {
                self.last_xy.x = point.x;
                self.last_xy.y = point.y;
            }
            _ => {}
        }
    }

    pub fn pointer_released(&mut self, key_type: KeyType, _point: Vec2) {
        if let KeyType::Right = key_type {
            self.save_xy.x = self.xy.x;
            self.save_xy.y = self.xy.y;
        }
    }

    pub fn pointer_moved(&mut self, key_type: KeyType, point: Vec2) {
        match key_type {
            KeyType::Left => {
                self.rot_xy.y = point.x - self.diff_xy.x;
                self.rot_xy.x = point.y + self.diff_xy.y;
                self.rot_xy.y *= 2.0;
                self.rot_xy.x *= 2.0;
                self.diff_xy.x = point.x;
                self.diff_xy.y = -point.y;
            }
            KeyType::Right => {
                self.xy.x = -(self.last_xy.x - point.x) / 100.0 + self.save_xy.x;
                self.xy.y = (self.last_xy.y - point.y) / 100.0 + self.save_xy.y;
            }
            _ => {}
        }
    }

    pub fn pointer_wheel_changed(&mut self, is_post: bool) {
        if is_post {
            self.distance += 0.1;
        } else {
            self.distance -= 0.1;
        }
    }

    pub fn rotate(&mut self, x: f32, y: f32) {
        self.rot_xy.x += x;
        self.rot_xy.y += y;
    }

    pub fn position(&mut self, x: f32, y: f32) {
        self.xy.x += x;
        self.xy.y += y;
    }

    pub fn add_distance(&mut self, x: f32) {
        self.distance += x;
    }

    pub fn set_skin_tex(&mut self, skin: Option<Bitmap>) -> Result<(), ErrorType> {
        if let Some(skin_tex) = skin {
            if skin_tex.width() != 64 {
                return Err(ErrorType::InvalidSkin);
            }

            self.skin_tex = Some(skin_tex.clone());
            // 需要访问皮肤类型检测器
            // self.skin_type = skin_type_checker::get_text_type(&skin_tex);
            self.switch_skin = true;
            self.have_skin = true;

            self.on_state_change(StateType::SkinLoaded);
            Ok(())
        } else {
            self.have_skin = false;
            Ok(())
        }
    }

    pub fn set_cape_tex(&mut self, cape: Option<Bitmap>) -> Result<(), ErrorType> {
        if let Some(cape_tex) = cape {
            self.cape = Some(cape_tex);
            self.switch_skin = true;
            self.have_cape = true;

            self.on_state_change(StateType::CapeLoaded);
            Ok(())
        } else {
            self.have_cape = false;
            Ok(())
        }
    }

    pub fn reset_position(&mut self) {
        self.distance = 1.0;
        self.diff_xy = Vec2::new(0.0, 0.0);
        self.xy = Vec2::new(0.0, 0.0);
        self.save_xy = Vec2::new(0.0, 0.0);
        self.last_xy = Vec2::new(0.0, 0.0);
        self.last = Mat4::default();
    }

    pub fn tick(&mut self, time: f64) {
        if self.animation {
            self.skin_animation.tick(time);

            // 同步动画旋转到当前旋转值
            if self.animation {
                self.head_rotate = self.skin_animation.head;
                self.arm_rotate = self.skin_animation.arm;
                self.leg_rotate = self.skin_animation.leg;
            }
        }

        if self.rot_xy.x != 0.0 || self.rot_xy.y != 0.0 {
            let rot_x = Mat4::from_rotation_x(self.rot_xy.x / 360.0);
            let rot_y = Mat4::from_rotation_y(self.rot_xy.y / 360.0);
            self.last = self.last * rot_x * rot_y;
            self.rot_xy = Vec2::new(0.0, 0.0);
        }

        self.fps += 1;
        self.time += time;

        if self.time >= 1.0 {
            self.time -= 1.0;
            self.on_fps_update(self.fps);
            self.fps = 0;
        }
    }

    pub fn on_error(&self, error: ErrorType) {
        if let Some(callback) = &self.error_callback {
            if let Ok(cb) = callback.lock() {
                cb(error);
            }
        }
    }

    pub fn on_state_change(&self, state: StateType) {
        if let Some(callback) = &self.state_callback {
            if let Ok(cb) = callback.lock() {
                cb(state);
            }
        }
    }

    pub fn on_fps_update(&self, fps: i32) {
        if let Some(callback) = &self.fps_callback {
            if let Ok(cb) = callback.lock() {
                cb(fps);
            }
        }
    }

    pub fn get_matrix(&self, part_type: ModelPartType) -> Mat4 {
        let enable = self.animation;
        let is_slim = self.skin_type == SkinType::NewSlim;
        let arm_width = if is_slim { 1.375 } else { 1.5 };

        match part_type {
            ModelPartType::Head => {
                let head_rot = if enable {
                    self.skin_animation.head
                } else {
                    self.head_rotate
                };
                Mat4::from_translation(Vec3::new(0.0, cube::VALUE, 0.0))
                    * Mat4::from_rotation_z(head_rot.x / 360.0)
                    * Mat4::from_rotation_x(head_rot.y / 360.0)
                    * Mat4::from_rotation_y(head_rot.z / 360.0)
                    * Mat4::from_translation(Vec3::new(0.0, cube::VALUE * 1.5, 0.0))
            }
            ModelPartType::LeftArm => {
                let arm_rot = if enable {
                    self.skin_animation.arm
                } else {
                    self.arm_rotate
                };
                Mat4::from_translation(Vec3::new(
                    cube::VALUE / 2.0,
                    -(arm_width * cube::VALUE),
                    0.0,
                )) * Mat4::from_rotation_z(arm_rot.x / 360.0)
                    * Mat4::from_rotation_x(arm_rot.y / 360.0)
                    * Mat4::from_translation(Vec3::new(
                        arm_width * cube::VALUE - cube::VALUE / 2.0,
                        arm_width * cube::VALUE,
                        0.0,
                    ))
            }
            ModelPartType::RightArm => {
                let arm_rot = if enable {
                    self.skin_animation.arm
                } else {
                    self.arm_rotate
                };
                Mat4::from_translation(Vec3::new(
                    -cube::VALUE / 2.0,
                    -(arm_width * cube::VALUE),
                    0.0,
                )) * Mat4::from_rotation_z(-arm_rot.x / 360.0)
                    * Mat4::from_rotation_x(-arm_rot.y / 360.0)
                    * Mat4::from_translation(Vec3::new(
                        -arm_width * cube::VALUE + cube::VALUE / 2.0,
                        arm_width * cube::VALUE,
                        0.0,
                    ))
            }
            ModelPartType::LeftLeg => {
                let leg_rot = if enable {
                    self.skin_animation.leg
                } else {
                    self.leg_rotate
                };
                Mat4::from_translation(Vec3::new(0.0, -1.5 * cube::VALUE, 0.0))
                    * Mat4::from_rotation_z(leg_rot.x / 360.0)
                    * Mat4::from_rotation_x(leg_rot.y / 360.0)
                    * Mat4::from_translation(Vec3::new(cube::VALUE * 0.5, -cube::VALUE * 1.5, 0.0))
            }
            ModelPartType::RightLeg => {
                let leg_rot = if enable {
                    self.skin_animation.leg
                } else {
                    self.leg_rotate
                };
                Mat4::from_translation(Vec3::new(0.0, -1.5 * cube::VALUE, 0.0))
                    * Mat4::from_rotation_z(-leg_rot.x / 360.0)
                    * Mat4::from_rotation_x(-leg_rot.y / 360.0)
                    * Mat4::from_translation(Vec3::new(-cube::VALUE * 0.5, -cube::VALUE * 1.5, 0.0))
            }
            ModelPartType::Proj => {
                let aspect = self.width as f32 / self.height as f32;
                Mat4::perspective_lh(PI / 4.0, aspect, 0.1, 10.0)
            }
            ModelPartType::View => Mat4::look_at_rh(
                Vec3::new(0.0, 0.0, 7.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            ModelPartType::Model => {
                let translation = Mat4::from_translation(Vec3::new(self.xy.x, self.xy.y, 0.0));
                let scale = Mat4::from_scale(Vec3::new(self.distance, self.distance, self.distance));
                self.last * translation * scale
            }
            ModelPartType::Cape => {
                let cape_rot = if enable {
                    11.8 + self.skin_animation.cape
                } else {
                    6.3
                };
                Mat4::from_translation(Vec3::new(0.0, -2.0 * cube::VALUE, -cube::VALUE * 0.1))
                    * Mat4::from_rotation_x(cape_rot * std::f32::consts::PI / 180.0)
                    * Mat4::from_translation(Vec3::new(0.0, 1.6 * cube::VALUE, -cube::VALUE * 0.5))
            }
            ModelPartType::Body => Mat4::default(),
        }
    }
}
