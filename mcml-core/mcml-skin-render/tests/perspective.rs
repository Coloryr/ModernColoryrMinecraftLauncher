//! 验证 `glam::camera::lh::proj::directx::perspective` 与旧 `Mat4::perspective_lh` 行为一致。
//!
//! 两者的矩阵完全相同（glam 0.33.1 起 `Mat4::perspective_lh` 被弃用，官方指向
//! `directx::perspective` 作为替代），弃用只是 API 组织上的重命名。

use glam::camera::lh::proj::directx;
use glam::{Mat4, Vec4};
use std::f32::consts::PI;

fn assert_mat4_eq(a: Mat4, b: Mat4) {
    let aa = a.to_cols_array();
    let bb = b.to_cols_array();
    for i in 0..16 {
        assert!(
            (aa[i] - bb[i]).abs() < 1e-6,
            "element {i} differs: {} vs {}",
            aa[i],
            bb[i]
        );
    }
}

/// `directx::perspective` 与旧的 `Mat4::perspective_lh` 输出完全一致
#[test]
fn perspective_lh_matches_directx_perspective() {
    let fov = PI / 4.0;
    let aspect = 1.5;
    let (near, far) = (0.1, 10.0);

    #[allow(deprecated)]
    let old = Mat4::perspective_lh(fov, aspect, near, far);
    let new = directx::perspective(fov, aspect, near, far);

    assert_mat4_eq(old, new);
}

/// 验证左撇子投影的 NDC 深度映射：near 映射到 0，far 映射到 1
#[test]
fn directx_perspective_maps_depth_to_0_1() {
    let (near, far) = (0.1, 10.0);
    let proj = directx::perspective(PI / 4.0, 1.5, near, far);

    // 左撇子视空间沿 +Z 为前方，取 Z 轴上的两个点做透视除法
    let clip_near = proj * Vec4::new(0.0, 0.0, near, 1.0);
    let clip_far = proj * Vec4::new(0.0, 0.0, far, 1.0);
    let ndc_near = clip_near.z / clip_near.w;
    let ndc_far = clip_far.z / clip_far.w;

    assert!((ndc_near - 0.0).abs() < 1e-6, "near depth: {ndc_near}");
    assert!((ndc_far - 1.0).abs() < 1e-6, "far depth: {ndc_far}");
}

/// 验证 FOV/宽高比换算：fov 越大视野越宽，矩阵缩放因子 h = 1/tan(fov/2) 越小
#[test]
fn directx_perspective_fov_scale() {
    let aspect = 1.5;
    let (near, far) = (0.1, 10.0);

    let wide = directx::perspective(PI / 3.0, aspect, near, far);
    let narrow = directx::perspective(PI / 6.0, aspect, near, far);

    let wide_scale = wide.to_cols_array()[5]; // m[1][1] = h
    let narrow_scale = narrow.to_cols_array()[5];
    assert!(wide_scale < narrow_scale, "wide fov should scale less");
    assert!(
        (wide_scale - (PI / 3.0 / 2.0).tan().recip()).abs() < 1e-6,
        "h = 1/tan(fov/2)"
    );
}
