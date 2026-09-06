pub mod cube;
pub mod cube_model;
pub mod model;
pub mod renders;
pub mod skin_animation;
pub mod texture;

use glam::{
    camera::{lh::proj::directx, rh::view},
    Mat4, Vec2, Vec3, Vec4,
};
use mcml_skin::SkinType;
use skia_safe::Bitmap;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

use crate::skin_animation::SkinAnimation;

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

    // 旋转角度
    pub arm_rotate: Vec3,
    pub leg_rotate: Vec3,
    pub head_rotate: Vec3,

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
                directx::perspective(PI / 4.0, aspect, 0.1, 10.0)
            }
            ModelPartType::View => view::look_at_mat4(
                Vec3::new(0.0, 0.0, 7.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            ModelPartType::Model => {
                let translation = Mat4::from_translation(Vec3::new(self.xy.x, self.xy.y, 0.0));
                let scale =
                    Mat4::from_scale(Vec3::new(self.distance, self.distance, self.distance));
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

    pub fn set_animation(&mut self, value: bool) {
        self.skin_animation.run = value;
        self.animation = value;
    }

    pub fn get_animation(&self) -> bool {
        self.animation
    }

    pub fn set_skin_type(&mut self, value: SkinType) {
        if self.skin_type != value {
            self.skin_animation.skin_type = value;
            self.switch_model = true;
            self.skin_type = value;
        }
    }

    pub fn get_skin_type(&self) -> SkinType {
        self.skin_type
    }

    pub fn set_back_color(&mut self, color: Vec4) {
        self.back_color = color;
        self.switch_back = true;
    }

    pub fn get_back_color(&self) -> Vec4 {
        self.back_color
    }

    pub fn set_enable_cape(&mut self, value: bool) {
        self.enable_cape = value;
        self.switch_type = true;
    }

    pub fn get_enable_cape(&self) -> bool {
        self.enable_cape
    }

    pub fn set_enable_top(&mut self, value: bool) {
        self.enable_top = value;
        self.switch_type = true;
    }

    pub fn get_enable_top(&self) -> bool {
        self.enable_top
    }

    pub fn set_arm_rotate(&mut self, rotate: Vec3) {
        self.arm_rotate = rotate;
    }

    pub fn get_arm_rotate(&self) -> Vec3 {
        self.arm_rotate
    }

    pub fn set_leg_rotate(&mut self, rotate: Vec3) {
        self.leg_rotate = rotate;
    }

    pub fn get_leg_rotate(&self) -> Vec3 {
        self.leg_rotate
    }

    pub fn set_head_rotate(&mut self, rotate: Vec3) {
        self.head_rotate = rotate;
    }

    pub fn get_head_rotate(&self) -> Vec3 {
        self.head_rotate
    }

    pub fn have_cape(&self) -> bool {
        self.have_cape
    }

    pub fn have_skin(&self) -> bool {
        self.have_skin
    }
}

impl Drop for BaseSkinRender {
    fn drop(&mut self) {
        self.skin_animation.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::{AlphaType, ColorType, ImageInfo};
    use std::sync::Mutex;

    /// 创建指定位图的辅助函数（RGBA8888，纯色填充）
    fn make_bitmap(w: i32, h: i32, r: u8, g: u8, b: u8) -> Bitmap {
        let info = ImageInfo::new((w, h), ColorType::RGBA8888, AlphaType::Premul, None);
        let mut bm = Bitmap::new();
        assert!(bm.set_info(&info, None));
        bm.alloc_pixels();
        let row = bm.row_bytes() as usize;
        let bpp = bm.bytes_per_pixel() as usize;
        let ptr = bm.pixels() as *mut u8;
        assert!(!ptr.is_null());
        unsafe {
            for y in 0..h {
                for x in 0..w {
                    let off = y as usize * row + x as usize * bpp;
                    let p = std::slice::from_raw_parts_mut(ptr.add(off), bpp);
                    p.copy_from_slice(&[r, g, b, 255]);
                }
            }
        }
        bm
    }

    /// new() 的默认值
    #[test]
    fn test_default_state() {
        let render = BaseSkinRender::new();
        assert_eq!(render.width, 800);
        assert_eq!(render.height, 600);
        assert_eq!(render.distance, 1.0);
        assert_eq!(render.skin_type, SkinType::Unknown);
        assert!(!render.have_skin);
        assert!(!render.have_cape);
        assert!(!render.animation);
        assert_eq!(render.back_color, Vec4::new(0.0, 0.0, 0.0, 1.0));
    }

    /// 左键拖拽旋转：rot_xy = (当前点 - 按下点) * 2，y 轴取反
    #[test]
    fn test_pointer_left_drag_rotation() {
        let mut render = BaseSkinRender::new();
        render.pointer_pressed(KeyType::Left, Vec2::new(10.0, 20.0));
        // 按下时 diff_xy = (10, -20)
        assert_eq!(render.diff_xy, Vec2::new(10.0, -20.0));

        render.pointer_moved(KeyType::Left, Vec2::new(15.0, 25.0));
        // rot_xy.y = (15-10)*2 = 10；rot_xy.x = (25 + (-20))*2 = 10
        // （按下时 diff_xy.y = -20，相当于以按下点 y 为基准计算增量）
        assert_eq!(render.rot_xy, Vec2::new(10.0, 10.0));
        // diff_xy 更新为当前点
        assert_eq!(render.diff_xy, Vec2::new(15.0, -25.0));
    }

    /// 右键拖拽平移 + 松开保存位置
    #[test]
    fn test_pointer_right_drag_pan() {
        let mut render = BaseSkinRender::new();
        render.pointer_pressed(KeyType::Right, Vec2::new(5.0, 5.0));
        assert_eq!(render.last_xy, Vec2::new(5.0, 5.0));

        render.pointer_moved(KeyType::Right, Vec2::new(105.0, 5.0));
        // xy.x = -(5-105)/100 = 1.0
        assert_eq!(render.xy.x, 1.0);
        assert_eq!(render.xy.y, 0.0);

        render.pointer_released(KeyType::Right, Vec2::new(105.0, 5.0));
        assert_eq!(render.save_xy, render.xy);
    }

    /// 滚轮缩放与位置重置
    #[test]
    fn test_wheel_and_reset_position() {
        let mut render = BaseSkinRender::new();
        render.pointer_wheel_changed(true);
        assert_eq!(render.distance, 1.1);
        render.pointer_wheel_changed(false);
        render.pointer_wheel_changed(false);
        assert_eq!(render.distance, 0.9);

        render.position(0.5, -0.5);
        assert_eq!(render.xy, Vec2::new(0.5, -0.5));

        render.reset_position();
        assert_eq!(render.distance, 1.0);
        assert_eq!(render.xy, Vec2::ZERO);
        assert_eq!(render.diff_xy, Vec2::ZERO);
    }

    /// add_distance 与 rotate 的增量语义
    #[test]
    fn test_add_distance_and_rotate() {
        let mut render = BaseSkinRender::new();
        render.add_distance(0.5);
        assert_eq!(render.distance, 1.5);
        render.rotate(10.0, 20.0);
        assert_eq!(render.rot_xy, Vec2::new(10.0, 20.0));
    }

    /// 皮肤/披风贴图设置与状态回调
    #[test]
    fn test_set_skin_tex_and_callbacks() {
        let mut render = BaseSkinRender::new();

        // 记录状态回调
        let states = Arc::new(Mutex::new(Vec::new()));
        let sink = states.clone();
        render.state_callback = Some(Arc::new(Mutex::new(move |s: StateType| {
            sink.lock().unwrap().push(s);
        })));

        // 非 64 宽度的皮肤应被拒绝
        let bad = make_bitmap(32, 32, 255, 0, 0);
        assert_eq!(render.set_skin_tex(Some(bad)), Err(ErrorType::InvalidSkin));
        assert!(!render.have_skin);

        // None 表示清除皮肤
        render.set_skin_tex(None).unwrap();
        assert!(!render.have_skin);

        // 64x64 皮肤应成功加载并触发 SkinLoaded 回调
        let good = make_bitmap(64, 64, 0, 255, 0);
        render.set_skin_tex(Some(good)).unwrap();
        assert!(render.have_skin);
        assert!(render.switch_skin);

        // 披风
        let cape = make_bitmap(64, 32, 0, 0, 255);
        render.set_cape_tex(Some(cape)).unwrap();
        assert!(render.have_cape);
        assert!(render.switch_skin);

        let recorded = states.lock().unwrap();
        assert!(recorded.contains(&StateType::SkinLoaded));
        assert!(recorded.contains(&StateType::CapeLoaded));
    }

    /// 错误回调
    #[test]
    fn test_error_callback() {
        let mut render = BaseSkinRender::new();
        let errors = Arc::new(Mutex::new(Vec::new()));
        let sink = errors.clone();
        render.error_callback = Some(Arc::new(Mutex::new(move |e: ErrorType| {
            sink.lock().unwrap().push(e);
        })));
        render.on_error(ErrorType::TextureError);
        assert_eq!(*errors.lock().unwrap(), vec![ErrorType::TextureError]);
    }

    /// set_skin_type 只在类型变化时置位 switch_model
    #[test]
    fn test_set_skin_type() {
        let mut render = BaseSkinRender::new();
        assert!(!render.switch_model);
        render.set_skin_type(SkinType::NewSlim);
        assert_eq!(render.get_skin_type(), SkinType::NewSlim);
        assert!(render.switch_model);
        // 相同类型不再置位（先复位标志再设置一次）
        render.switch_model = false;
        render.set_skin_type(SkinType::NewSlim);
        assert!(!render.switch_model);
    }

    /// tick 应把 rot_xy 累积到 last 矩阵并清零 rot_xy
    #[test]
    fn test_tick_accumulates_rotation() {
        let mut render = BaseSkinRender::new();
        render.rotate(360.0, 720.0);
        render.tick(0.016);
        assert_eq!(render.rot_xy, Vec2::ZERO);
        assert_ne!(render.last, Mat4::IDENTITY, "last 应记录了旋转");
        // 再次 tick：rot_xy 已为 0，last 不变
        let last = render.last;
        render.tick(0.016);
        assert_eq!(render.last, last);
    }

    /// tick 的 FPS 统计：每累计 1 秒触发一次 fps 回调并清零计数
    #[test]
    fn test_tick_fps_callback() {
        let mut render = BaseSkinRender::new();
        let fps_values = Arc::new(Mutex::new(Vec::new()));
        let sink = fps_values.clone();
        render.fps_callback = Some(Arc::new(Mutex::new(move |f: i32| {
            sink.lock().unwrap().push(f);
        })));

        // 0.016 * 63 = 1.008 >= 1.0，第 63 次 tick 触发回调
        for _ in 0..63 {
            render.tick(0.016);
        }
        assert_eq!(*fps_values.lock().unwrap(), vec![63]);
        assert_eq!(render.fps, 0, "触发后 fps 计数应清零");
        assert!(render.time < 1.0, "剩余时间应小于 1 秒");
    }

    /// 部件矩阵：头部矩阵在零旋转下等价于平移 (0, VALUE + 1.5*VALUE, 0)
    #[test]
    fn test_get_matrix_head() {
        let render = BaseSkinRender::new();
        let m = render.get_matrix(ModelPartType::Head);
        let w = m.w_axis;
        assert!((w.x - 0.0).abs() < 1e-5);
        assert!((w.y - (cube::VALUE + 1.5 * cube::VALUE)).abs() < 1e-5);
        assert!((w.z - 0.0).abs() < 1e-5);
    }

    /// 手臂矩阵在零旋转下的落点：肩部枢轴 x = arm_width * VALUE
    /// （get_matrix 中宽臂 arm_width = 1.5，纤细 = 1.375）
    #[test]
    fn test_get_matrix_arms_width() {
        let mut render = BaseSkinRender::new();
        render.set_skin_type(SkinType::New);
        let wide = render.get_matrix(ModelPartType::LeftArm);
        assert!((wide.w_axis.x - 1.5 * cube::VALUE).abs() < 1e-5);
        assert!((wide.w_axis.y - 0.0).abs() < 1e-5);

        render.set_skin_type(SkinType::NewSlim);
        let slim = render.get_matrix(ModelPartType::LeftArm);
        assert!((slim.w_axis.x - 1.375 * cube::VALUE).abs() < 1e-5);
    }

    /// Model 矩阵应包含平移与等比缩放
    #[test]
    fn test_get_matrix_model() {
        let mut render = BaseSkinRender::new();
        render.position(0.3, -0.2);
        render.distance = 2.0;
        let m = render.get_matrix(ModelPartType::Model);
        assert!((m.w_axis.x - 0.3).abs() < 1e-5, "平移不应被缩放");
        assert!((m.w_axis.y - -0.2).abs() < 1e-5);
        assert!((m.x_axis.x - 2.0).abs() < 1e-5, "x 基向量应缩放 2 倍");
    }

    /// Proj / View / Cape 矩阵应全部为有限值
    #[test]
    fn test_get_matrix_finite() {
        let mut render = BaseSkinRender::new();
        render.skin_animation.cape = 1.5;
        render.set_animation(true);
        for part in [
            ModelPartType::Proj,
            ModelPartType::View,
            ModelPartType::Cape,
            ModelPartType::Body,
            ModelPartType::Head,
            ModelPartType::LeftArm,
            ModelPartType::RightArm,
            ModelPartType::LeftLeg,
            ModelPartType::RightLeg,
        ] {
            let m = render.get_matrix(part);
            for v in m.to_cols_array() {
                assert!(v.is_finite(), "{part:?} 矩阵含非有限值 {v}");
            }
        }
    }

    /// set_animation / getter 一致性；动画开启时 tick 会同步动画角度到部件旋转
    #[test]
    fn test_animation_sync_on_tick() {
        let mut render = BaseSkinRender::new();
        render.set_animation(true);
        assert!(render.get_animation());
        assert!(render.skin_animation.run);

        render.skin_animation.set_frame(30);
        render.skin_animation.run = true;
        render.skin_animation.tick(0.02); // frame 31
        render.tick(0.0);
        assert!(
            (render.head_rotate.z - (31.0 - 30.0)).abs() < 1e-5,
            "动画 head 角度应同步到 head_rotate"
        );

        render.set_animation(false);
        assert!(!render.get_animation());
    }

    /// 各 setter/getter 往返
    #[test]
    fn test_setters_getters() {
        let mut render = BaseSkinRender::new();

        render.set_back_color(Vec4::new(0.2, 0.4, 0.6, 1.0));
        assert_eq!(render.get_back_color(), Vec4::new(0.2, 0.4, 0.6, 1.0));
        assert!(render.switch_back);

        render.set_enable_cape(true);
        assert!(render.get_enable_cape());
        assert!(render.switch_type);
        render.set_enable_top(true);
        assert!(render.get_enable_top());

        let rot = Vec3::new(1.0, 2.0, 3.0);
        render.set_arm_rotate(rot);
        assert_eq!(render.get_arm_rotate(), rot);
        render.set_leg_rotate(rot);
        assert_eq!(render.get_leg_rotate(), rot);
        render.set_head_rotate(rot);
        assert_eq!(render.get_head_rotate(), rot);
    }
}
