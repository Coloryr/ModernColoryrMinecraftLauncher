pub mod cube {
    /// 方块模型
    pub const VALUE: f32 = 0.5;

    pub const VERTICES: [f32; 72] = [
        0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
        1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0,
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
        1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0,
        0.0, -1.0, 0.0,
    ];

    const CUBE: [f32; 72] = [
        VALUE, VALUE, -VALUE, /* Back. */
        VALUE, -VALUE, -VALUE, -VALUE, -VALUE, -VALUE, -VALUE, VALUE, -VALUE, -VALUE, VALUE,
        VALUE, /* Front. */
        -VALUE, -VALUE, VALUE, VALUE, -VALUE, VALUE, VALUE, VALUE, VALUE, -VALUE, VALUE,
        -VALUE, /* Left. */
        -VALUE, -VALUE, -VALUE, -VALUE, -VALUE, VALUE, -VALUE, VALUE, VALUE, VALUE, VALUE,
        VALUE, /* Right. */
        VALUE, -VALUE, VALUE, VALUE, -VALUE, -VALUE, VALUE, VALUE, -VALUE, -VALUE, VALUE,
        -VALUE, /* Top. */
        -VALUE, VALUE, VALUE, VALUE, VALUE, VALUE, VALUE, VALUE, -VALUE, VALUE, -VALUE,
        -VALUE, /* Bottom. */
        VALUE, -VALUE, VALUE, -VALUE, -VALUE, VALUE, -VALUE, -VALUE, -VALUE,
    ];

    const CUBE_INDICES: [u16; 36] = [
        0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17,
        18, 16, 18, 19, 20, 21, 22, 20, 22, 23,
    ];

    /// 获得一个方块X Y Z坐标
    ///
    /// # Arguments
    /// * `multiply_x` - X轴乘数
    /// * `multiply_y` - Y轴乘数
    /// * `multiply_z` - Z轴乘数
    /// * `add_x` - X轴偏移
    /// * `add_y` - Y轴偏移
    /// * `add_z` - Z轴偏移
    /// * `enlarge` - 放大系数
    pub fn get_square(
        multiply_x: f32,
        multiply_y: f32,
        multiply_z: f32,
        add_x: f32,
        add_y: f32,
        add_z: f32,
        enlarge: f32,
    ) -> Vec<f32> {
        let mut temp = vec![0.0; CUBE.len()];

        for (a, value) in temp.iter_mut().enumerate() {
            *value = CUBE[a] * enlarge;

            match a % 3 {
                0 => *value = *value * multiply_x + add_x,
                1 => *value = *value * multiply_y + add_y,
                _ => *value = *value * multiply_z + add_z,
            }
        }

        temp
    }

    /// 获得一个标准方块顶点顺序
    ///
    /// # Arguments
    /// * `offset` - 顶点索引偏移量
    pub fn get_square_indices(offset: u16) -> Vec<u16> {
        let mut temp = vec![0; CUBE_INDICES.len()];

        for (a, value) in temp.iter_mut().enumerate() {
            *value = CUBE_INDICES[a] + offset;
        }

        temp
    }

    /// 使用默认参数获得方块坐标
    pub fn get_square_default() -> Vec<f32> {
        get_square(1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0)
    }

    /// 使用默认参数获得顶点顺序
    pub fn get_square_indices_default() -> Vec<u16> {
        get_square_indices(0)
    }
}
