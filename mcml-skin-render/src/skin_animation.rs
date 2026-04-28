use cgmath::Vector3;

use crate::texture::texture::SkinType;

/// 皮肤的动画
#[derive(Debug, Clone)]
pub struct SkinAnimation {
    frame: i32,
    count: f64,
    close: bool,
    pub run: bool,
    pub skin_type: SkinType,
    pub arm: Vector3<f32>,
    pub leg: Vector3<f32>,
    pub head: Vector3<f32>,
    pub cape: f32,
}

impl SkinAnimation {
    pub fn new() -> Self {
        Self {
            frame: 0,
            count: 0.0,
            close: false,
            run: false,
            skin_type: SkinType::Unknown,
            arm: Vector3::new(40.0, 0.0, 0.0),
            leg: Vector3::new(0.0, 0.0, 0.0),
            head: Vector3::new(0.0, 0.0, 0.0),
            cape: 0.0,
        }
    }

    /// 关闭动画
    pub fn close(&mut self) {
        self.run = false;
        self.close = true;
    }

    /// 进行动画演算
    /// 返回 false 表示动画已关闭
    pub fn tick(&mut self, time: f64) -> bool {
        if self.run {
            self.count += time;
            while self.count > 0.01 {
                self.count -= 0.01;
                self.frame += 1;
            }

            if self.frame >= 120 {
                self.frame = 0;
            }

            if self.frame <= 60 {
                // 0 360
                // -180 180
                self.arm.y = self.frame as f32 * 6.0 - 180.0;

                // 0 180
                // 90 -90
                self.leg.y = 90.0 - self.frame as f32 * 3.0;

                // 0 6
                self.cape = self.frame as f32 / 10.0;

                // -30 30
                if self.skin_type == SkinType::NewSlim {
                    self.head.z = 0.0;
                    self.head.x = self.frame as f32 - 30.0;
                } else {
                    self.head.x = 0.0;
                    self.head.z = self.frame as f32 - 30.0;
                }
            } else {
                // 61 120
                // 6 0
                self.cape = 6.0 - (self.frame as f32 - 60.0) / 10.0;

                // 360 720
                // 180 -180
                self.arm.y = 540.0 - self.frame as f32 * 6.0;

                // 180 360
                // -90 90
                self.leg.y = self.frame as f32 * 3.0 - 270.0;

                // 30 -30
                if self.skin_type == SkinType::NewSlim {
                    self.head.z = 0.0;
                    self.head.x = 90.0 - self.frame as f32;
                } else {
                    self.head.x = 0.0;
                    self.head.z = 90.0 - self.frame as f32;
                }
            }
        }

        !self.close
    }

    /// 重置动画状态
    pub fn reset(&mut self) {
        self.frame = 0;
        self.count = 0.0;
        self.close = false;
        self.arm = Vector3::new(40.0, 0.0, 0.0);
        self.leg = Vector3::new(0.0, 0.0, 0.0);
        self.head = Vector3::new(0.0, 0.0, 0.0);
        self.cape = 0.0;
    }

    /// 获取当前动画进度 (0-120)
    pub fn get_frame(&self) -> i32 {
        self.frame
    }

    /// 设置动画帧率
    pub fn set_frame(&mut self, frame: i32) {
        self.frame = frame % 120;
    }
}
