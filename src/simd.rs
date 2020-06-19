use packed_simd::*;

use crate::constants::*;

#[derive(Clone, Copy, PartialEq)]
// w must always be 0
struct Vec3(f32x4);

impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3(f32x4::new(x, y, z, 0.0))
    }

    fn length(self) -> f32 {
        (self.0 * self.0).sum().sqrt()
    }

    fn normalize(self) -> Vec3 {
        let l = self.length();
        Vec3(self.0 / l)
    }

    fn x(self) -> f32 {
        unsafe {
            self.0.extract_unchecked(0)
        }
    }
    fn y(self) -> f32 {
        unsafe {
            self.0.extract_unchecked(1)
        }
    }
    fn z(self) -> f32 {
        unsafe {
            self.0.extract_unchecked(2)
        }
    }

    fn sx(self, x: f32) -> Vec3 {
        unsafe {
            Vec3(self.0.replace_unchecked(0, x))
        }
    }
    fn sy(self, y: f32) -> Vec3 {
        unsafe {
            Vec3(self.0.replace_unchecked(1, y))
        }
    }
    fn sz(self, z: f32) -> Vec3 {
        unsafe {
            Vec3(self.0.replace_unchecked(2, z))
        }
    }

    fn dot(self, other: Vec3) -> f32 {
        (self.0 * other.0).sum()
    }

    const fn zero() -> Vec3 {
        Vec3(f32x4::splat(0.0))
    }
}

use core::ops::*;
impl Add<Vec3> for Vec3 {
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3 {
        Vec3(self.0 + other.0)
    }
}
impl Sub<Vec3> for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3 {
        Vec3(self.0 - other.0)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, other: f32) -> Vec3 {
        Vec3(self.0 * other)
    }
}
impl Sub<f32> for Vec3 {
    type Output = Vec3;
    fn sub(self, other: f32) -> Vec3 {
        unsafe {
            Vec3((self.0 - other).replace_unchecked(3, 0.0))
        }
    }
}

fn rgb(v: Vec3) -> u32 {
    let ones = Vec3::new(1.0, 1.0, 1.0);
    let v = Vec3(v.0.min(ones.0) * 255.0);
    // let r = (v.x().min(1.0) * 255.0) as u32;
    // let g = (v.y().min(1.0) * 255.0) as u32;
    // let b = (v.z().min(1.0) * 255.0) as u32;
    let r = v.x() as u32;
    let g = v.y() as u32;
    let b = v.z() as u32;
    255 << 24 // Alpha = 1.0
        | b << 16
        | g << 8
        | r
}

fn scene(p: Vec3) -> f32 {
    (p.length() - 1.0).min((p - Vec3::new(
        -2.0,
        0.0,
        1.0,
    )).length() - 1.0)
}

fn color(p: Vec3) -> Vec3 {
    let a = p.length() - 1.0;
    let b = (p - Vec3::new(
        -2.0,
        0.0,
        1.0,
    )).length() - 1.0;
    if a < b {
        Vec3::new(
            1.0,
            0.5,
            0.2,
        )
    } else {
        Vec3::new(
            0.3,
            0.7,
            0.4,
        )
    }
}

fn normal(p: Vec3) -> Vec3 {
    let e = 0.01;
    // TODO simd-ize
    Vec3::new(
        scene(p.sx(p.x() + e)) - scene(p.sx(p.x() - e)),
        scene(p.sy(p.y() + e)) - scene(p.sy(p.y() - e)),
        scene(p.sz(p.z() + e)) - scene(p.sz(p.z() - e)),
    ).normalize()
}

// Returns the t-value on a hit
fn trace(ro: Vec3, rd: Vec3) -> Option<f32> {
    let mut p = ro;
    let mut t = 0.0;

    for _ in 0..64 {
        let d = scene(p);
        if d < 0.01 {
            return Some(t);
        }
        t += d;
        p = p + rd * d;
    }

    None
}

fn shade(col: Vec3, l: Vec3, n: Vec3) -> Vec3 {
    let cos_theta = l.dot(n);

    // The SPIR-V version seems to handle NaN's slightly differently, so this is needed to make the output match
    if cos_theta < -0.05 {
        return Vec3::zero()
    }

    col * cos_theta // Lambertian. No 1/pi term because we're assuming colors are already adjusted for that
        + col * 0.05 // Ambient
}

fn run_pixel(id: usize) -> u32 {
    let x = id % SIZE;
    let y = id / SIZE;

    // Normalized coordinates on (-1, 1)
    let x = x as f32 / (SIZE as f32 * 0.5) - 1.0;
    let y = -(y as f32) / (SIZE as f32 * 0.5) + 1.0;

    let camera_pos = Vec3::new(0.0, 0.0, -2.0);
    let camera_dir = Vec3::new(0.0, 0.0, 1.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let right = Vec3::new(1.0, 0.0, 0.0);
    let rd = camera_dir
        + up * y
        + right * x;
    let rd = rd.normalize();
    let ro = camera_pos;

    let hit = trace(ro, rd);

    let f = hit.unwrap_or(0.0);

    let col = shade(color(ro + rd * f), up, normal(ro + rd * f));

    rgb(col)
}

pub fn run_simd() {
    let buf: Vec<_> = (0..SIZE * SIZE).map(|i| run_pixel(i)).collect();

    // print!(
    //     "{:?}",
    //     buf.iter().take(4).collect::<Vec<_>>(),
    // );

    // let image = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
    //     SIZE as u32,
    //     SIZE as u32,
    //     buf
    //         .iter()
    //         .flat_map(|x| x.to_le_bytes().to_vec())
    //         .collect::<Vec<_>>(),
    // )
    // .unwrap();
    // image.save("test.png").unwrap();
}

pub fn run_rayon() {
    use rayon::prelude::*;
    let buf: Vec<_> = (0..SIZE * SIZE).into_par_iter().map(|i| run_pixel(i)).collect();
}
