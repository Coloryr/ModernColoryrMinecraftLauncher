use mcml_skin_render::cube::cube;

#[test]
fn test_cube_value() {
    assert!((cube::VALUE - 0.5).abs() < f32::EPSILON);
}

#[test]
fn test_get_square_default_length() {
    let result = cube::get_square_default();
    // CUBE 常量有 24 个顶点，每个顶点 3 个坐标 = 72 个 f32
    assert_eq!(result.len(), 72);
}

#[test]
fn test_get_square_indices_default_length() {
    let result = cube::get_square_indices_default();
    // CUBE_INDICES 有 36 个索引
    assert_eq!(result.len(), 36);
}

#[test]
fn test_get_square_with_scale() {
    let result = cube::get_square(2.0, 2.0, 2.0, 0.0, 0.0, 0.0, 1.0);
    assert_eq!(result.len(), 72);

    // 验证缩放后的值范围
    let max_val = result.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_val = result.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!((max_val - 1.0).abs() < f32::EPSILON);
    assert!((min_val + 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_get_square_with_translation() {
    let result = cube::get_square(1.0, 1.0, 1.0, 5.0, 10.0, 15.0, 1.0);
    assert_eq!(result.len(), 72);

    // 验证平移后的值
    let max_val = result.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_val = result.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!((max_val - 15.5).abs() < f32::EPSILON);
    assert!((min_val - 4.5).abs() < f32::EPSILON);
}

#[test]
fn test_get_square_with_enlarge() {
    let result = cube::get_square(1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 2.0);
    assert_eq!(result.len(), 72);

    // CUBE 值是 0.5/-0.5，enlarge=2 后变成 1.0/-1.0
    // 再乘以 multiply=1 再加 add=0，所以 max=1.0, min=-1.0
    let max_val = result.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_val = result.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!((max_val - 1.0).abs() < f32::EPSILON, "max_val = {}", max_val);
    assert!((min_val + 1.0).abs() < f32::EPSILON, "min_val = {}", min_val);
}

#[test]
fn test_get_square_indices_with_offset() {
    let result = cube::get_square_indices(10);
    assert_eq!(result.len(), 36);

    // 验证偏移后的索引值
    assert_eq!(result[0], 10);
    assert_eq!(result[1], 11);
    assert_eq!(result[2], 12);
}

#[test]
fn test_get_square_indices_zero_offset() {
    let result = cube::get_square_indices(0);
    assert_eq!(result.len(), 36);

    // 验证无偏移时的索引值
    assert_eq!(result[0], 0);
    assert_eq!(result[1], 1);
    assert_eq!(result[2], 2);
}

#[test]
fn test_get_square_indices_large_offset() {
    let result = cube::get_square_indices(1000);
    assert_eq!(result.len(), 36);

    // 验证大偏移量
    // CUBE_INDICES 最大值是 23，所以 1000 + 23 = 1023
    assert_eq!(result[0], 1000);
    assert_eq!(result[35], 1023);
}

#[test]
fn test_get_square_asymmetric() {
    let result = cube::get_square(0.5, 1.5, 0.5, 0.0, 0.0, 0.0, 1.0);
    assert_eq!(result.len(), 72);

    // 验证非对称缩放
    let max_x = result.iter().step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);
    let max_y = result.iter().skip(1).step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);
    let max_z = result.iter().skip(2).step_by(3).cloned().fold(f32::NEG_INFINITY, f32::max);

    assert!((max_x - 0.25).abs() < f32::EPSILON);
    assert!((max_y - 0.75).abs() < f32::EPSILON);
    assert!((max_z - 0.25).abs() < f32::EPSILON);
}

#[test]
fn test_vertices_constant() {
    // VERTICES 应该有 72 个元素（24 个顶点 * 3 坐标）
    assert_eq!(cube::VERTICES.len(), 72);
}

#[test]
fn test_get_square_default_matches_get_square() {
    let default = cube::get_square_default();
    let explicit = cube::get_square(1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0);

    assert_eq!(default.len(), explicit.len());
    for (a, b) in default.iter().zip(explicit.iter()) {
        assert!((a - b).abs() < f32::EPSILON, "值 {} != {}", a, b);
    }
}
