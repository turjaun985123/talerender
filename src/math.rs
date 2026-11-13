//! Minimal column-major 4x4 matrix math (WGSL/wgpu convention:
//! right-handed view space, clip z in [0, 1]). Written in-house to keep
//! the dependency surface at zero for Milestone 1; every function is
//! covered by unit tests with hand-derived expectations.

pub type Mat4 = [[f32; 4]; 4]; // [column][row]

pub fn identity() -> Mat4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Returns a * b (b is applied to vectors first).
pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [[0.0f32; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            out[col][row] = a[0][row] * b[col][0]
                + a[1][row] * b[col][1]
                + a[2][row] * b[col][2]
                + a[3][row] * b[col][3];
        }
    }
    out
}

pub fn mul_vec4(m: &Mat4, v: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for row in 0..4 {
        out[row] = m[0][row] * v[0] + m[1][row] * v[1] + m[2][row] * v[2] + m[3][row] * v[3];
    }
    out
}

pub fn translate(x: f32, y: f32, z: f32) -> Mat4 {
    let mut m = identity();
    m[3] = [x, y, z, 1.0];
    m
}

pub fn rotate_y(rad: f32) -> Mat4 {
    let (s, c) = rad.sin_cos();
    let mut m = identity();
    m[0] = [c, 0.0, -s, 0.0];
    m[2] = [s, 0.0, c, 0.0];
    m
}

pub fn rotate_x(rad: f32) -> Mat4 {
    let (s, c) = rad.sin_cos();
    let mut m = identity();
    m[1] = [0.0, c, s, 0.0];
    m[2] = [0.0, -s, c, 0.0];
    m
}

pub fn rotate_z(rad: f32) -> Mat4 {
    let (s, c) = rad.sin_cos();
    let mut m = identity();
    m[0] = [c, s, 0.0, 0.0];
    m[1] = [-s, c, 0.0, 0.0];
    m
}

/// three.js Euler order 'XYZ' (the default for Object3D.rotation):
/// the composed rotation is Rx * Ry * Rz, i.e. Z is applied to the
/// vector first, then Y, then X (matches Quaternion.setFromEuler XYZ,
/// q = qx * qy * qz).
pub fn euler_xyz(x: f32, y: f32, z: f32) -> Mat4 {
    mul(&rotate_x(x), &mul(&rotate_y(y), &rotate_z(z)))
}

pub fn scale(sx: f32, sy: f32, sz: f32) -> Mat4 {
    let mut m = identity();
    m[0][0] = sx;
    m[1][1] = sy;
    m[2][2] = sz;
    m
}

/// Perspective projection for wgpu clip space (z in [0, 1]), right-handed
/// view space looking down -Z.
pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y_rad * 0.5).tan();
    let mut m = [[0.0f32; 4]; 4];
    m[0][0] = f / aspect;
    m[1][1] = f;
    m[2][2] = far / (near - far);
    m[2][3] = -1.0;
    m[3][2] = (near * far) / (near - far);
    m
}

/// Orthographic projection for wgpu clip space (z in [0, 1]): the
/// GL-style three.js OrthographicCamera matrix with the z row folded
/// through z_wgpu = 0.5 * z_gl + 0.5, so a view-space point at the
/// near plane maps to 0 and at the far plane to 1 — exactly the
/// fragCoordZ three.js packs into shadow maps.
pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    let mut m = [[0.0f32; 4]; 4];
    m[0][0] = 2.0 / (right - left);
    m[1][1] = 2.0 / (top - bottom);
    m[2][2] = -1.0 / (far - near);
    m[3][0] = -(right + left) / (right - left);
    m[3][1] = -(top + bottom) / (top - bottom);
    m[3][2] = -near / (far - near);
    m[3][3] = 1.0;
    m
}

/// The 3x3 normal matrix: inverse-transpose of the model's upper 3x3,
/// returned as a Mat4 with the linear part in the upper block. three.js
/// transforms normals by the inverse-transpose (its `normalMatrix` is
/// the view-space form; composed with the view transform the net
/// world-space effect is this matrix).
pub fn normal_matrix(m: &Mat4) -> Mat4 {
    // M[r][c] = m[c][r]
    let mm = |r: usize, c: usize| m[c][r];
    let c00 = mm(1, 1) * mm(2, 2) - mm(1, 2) * mm(2, 1);
    let c01 = -(mm(1, 0) * mm(2, 2) - mm(1, 2) * mm(2, 0));
    let c02 = mm(1, 0) * mm(2, 1) - mm(1, 1) * mm(2, 0);
    let det = mm(0, 0) * c00 + mm(0, 1) * c01 + mm(0, 2) * c02;
    let inv_det = if det != 0.0 { 1.0 / det } else { 0.0 };
    // (A^-1)^T [r][c] = cofactor[r][c] / det; storage out[c][r].
    let cof = [
        [c00, c01, c02],
        [
            -(mm(0, 1) * mm(2, 2) - mm(0, 2) * mm(2, 1)),
            mm(0, 0) * mm(2, 2) - mm(0, 2) * mm(2, 0),
            -(mm(0, 0) * mm(2, 1) - mm(0, 1) * mm(2, 0)),
        ],
        [
            mm(0, 1) * mm(1, 2) - mm(0, 2) * mm(1, 1),
            -(mm(0, 0) * mm(1, 2) - mm(0, 2) * mm(1, 0)),
            mm(0, 0) * mm(1, 1) - mm(0, 1) * mm(1, 0),
        ],
    ];
    let mut out = identity();
    for r in 0..3 {
        for c in 0..3 {
            out[c][r] = cof[r][c] * inv_det;
        }
    }
    out
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm3(a: [f32; 3]) -> [f32; 3] {
    let l = dot3(a, a).sqrt();
    [a[0] / l, a[1] / l, a[2] / l]
}

/// Right-handed look-at view matrix (camera looks down its -Z).
pub fn look_at(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> Mat4 {
    let z = norm3(sub3(eye, center)); // backward
    let x = norm3(cross3(up, z)); // right
    let y = cross3(z, x); // true up
    let mut m = identity();
    m[0] = [x[0], y[0], z[0], 0.0];
    m[1] = [x[1], y[1], z[1], 0.0];
    m[2] = [x[2], y[2], z[2], 0.0];
    m[3] = [-dot3(x, eye), -dot3(y, eye), -dot3(z, eye), 1.0];
    m
}

/// Row-pitch for GPU readback: each row padded up to the 256-byte
/// COPY_BYTES_PER_ROW_ALIGNMENT boundary.
pub fn padded_row_bytes(width_px: u32) -> u32 {
    (width_px * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn identity_is_neutral() {
        let t = translate(3.0, -2.0, 5.0);
        let out = mul(&identity(), &t);
        assert_eq!(out, t);
        let p = mul_vec4(&identity(), [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(p, [1.0, 2.0, 3.0, 1.0]);
    }

    #[test]
    fn translate_then_rotate_order() {
        // mul(a, b) applies b first: rotate (1,0,0) by 90 deg about Y
        // -> (0,0,-1); then translate by (10,0,0) -> (10,0,-1).
        let m = mul(&translate(10.0, 0.0, 0.0), &rotate_y(std::f32::consts::FRAC_PI_2));
        let p = mul_vec4(&m, [1.0, 0.0, 0.0, 1.0]);
        assert!(approx(p[0], 10.0) && approx(p[1], 0.0) && approx(p[2], -1.0));
    }

    #[test]
    fn perspective_maps_near_to_zero_far_to_one() {
        let proj = perspective(36f32.to_radians(), 1080.0 / 1920.0, 0.1, 100.0);
        let near = mul_vec4(&proj, [0.0, 0.0, -0.1, 1.0]);
        assert!(approx(near[2] / near[3], 0.0), "near z: {}", near[2] / near[3]);
        let far = mul_vec4(&proj, [0.0, 0.0, -100.0, 1.0]);
        assert!(approx(far[2] / far[3], 1.0), "far z: {}", far[2] / far[3]);
        // A point on the frustum's vertical edge at the near plane -> NDC y = 1.
        let half_h = 0.1 * (18f32.to_radians()).tan();
        let edge = mul_vec4(&proj, [0.0, half_h, -0.1, 1.0]);
        assert!(approx(edge[1] / edge[3], 1.0));
    }

    #[test]
    fn look_at_puts_center_on_negative_z() {
        let view = look_at([0.0, 0.0, 5.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let c = mul_vec4(&view, [0.0, 0.0, 0.0, 1.0]);
        assert!(approx(c[0], 0.0) && approx(c[1], 0.0) && approx(c[2], -5.0));
        let e = mul_vec4(&view, [0.0, 0.0, 5.0, 1.0]);
        assert!(approx(e[0], 0.0) && approx(e[1], 0.0) && approx(e[2], 0.0));
    }

    #[test]
    fn rotate_x_tips_z_toward_negative_y() {
        // Rotating +Z by +90 deg about X: (0,0,1) -> (0,-1,0)? No:
        // right-handed about +X, +Y -> +Z, +Z -> -Y. Check +Y -> +Z.
        let p = mul_vec4(&rotate_x(std::f32::consts::FRAC_PI_2), [0.0, 1.0, 0.0, 1.0]);
        assert!(approx(p[0], 0.0) && approx(p[1], 0.0) && approx(p[2], 1.0), "{p:?}");
        let q = mul_vec4(&rotate_x(std::f32::consts::FRAC_PI_2), [0.0, 0.0, 1.0, 1.0]);
        assert!(approx(q[1], -1.0) && approx(q[2], 0.0), "{q:?}");
    }

    #[test]
    fn euler_xyz_applies_z_first() {
        // euler_xyz(x=90deg, y=0, z=90deg) on (1,0,0): Rz first sends
        // (1,0,0)->(0,1,0); Rx then sends (0,1,0)->(0,0,1).
        let m = euler_xyz(std::f32::consts::FRAC_PI_2, 0.0, std::f32::consts::FRAC_PI_2);
        let p = mul_vec4(&m, [1.0, 0.0, 0.0, 1.0]);
        assert!(approx(p[0], 0.0) && approx(p[1], 0.0) && approx(p[2], 1.0), "{p:?}");
    }

    #[test]
    fn orthographic_maps_box_corners() {
        // three.js DirectionalLightShadow camera for the ab1 rig:
        // +-17, near 0.1, far 80 (world.ts/engine.ts).
        let p = orthographic(-17.0, 17.0, -17.0, 17.0, 0.1, 80.0);
        let near = mul_vec4(&p, [0.0, 0.0, -0.1, 1.0]);
        assert!(approx(near[2], 0.0), "near: {}", near[2]);
        let far = mul_vec4(&p, [0.0, 0.0, -80.0, 1.0]);
        assert!(approx(far[2], 1.0), "far: {}", far[2]);
        let corner = mul_vec4(&p, [-17.0, 17.0, -40.0, 1.0]);
        assert!(approx(corner[0], -1.0) && approx(corner[1], 1.0));
        assert!(approx(corner[2], (40.0 - 0.1) / 79.9));
    }

    #[test]
    fn normal_matrix_undoes_nonuniform_scale() {
        // Scale (2, 1, 1): normals must shrink along x — a normal of
        // (1, 1, 0)/sqrt(2) maps to direction (0.5, 1, 0).
        let m = mul(&translate(5.0, 0.0, 0.0), &scale(2.0, 1.0, 1.0));
        let nm = normal_matrix(&m);
        let n = mul_vec4(&nm, [0.70710678, 0.70710678, 0.0, 0.0]);
        let expect = [0.35355339, 0.70710678, 0.0];
        assert!(
            approx(n[0], expect[0]) && approx(n[1], expect[1]) && approx(n[2], expect[2]),
            "{n:?}"
        );
        // Rotation only: normal matrix equals the rotation.
        let r = rotate_y(0.7);
        let nm2 = normal_matrix(&r);
        for c in 0..3 {
            for row in 0..3 {
                assert!(approx(nm2[c][row], r[c][row]), "nm2[{c}][{row}]");
            }
        }
    }

    #[test]
    fn padded_rows_align_to_256() {
        assert_eq!(padded_row_bytes(1080), 4352); // 4320 -> 4352
        assert_eq!(padded_row_bytes(64), 256);
        assert_eq!(padded_row_bytes(65), 512);
    }
}
