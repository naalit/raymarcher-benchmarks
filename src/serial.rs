use crate::constants::*;

#[derive(Clone, Copy, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn length(self) -> f32 {
        let Vec3 { x, y, z } = self;
        (x*x + y*y + z*z).sqrt()
    }

    fn normalize(self) -> Vec3 {
        let l = self.length();
        let Vec3 { x, y, z } = self;
        Vec3 {
            x: x / l,
            y: y / l,
            z: z / l,
        }
    }

    fn sx(self, x: f32) -> Vec3 {
        Vec3 {
            x,
            ..self
        }
    }

    fn sy(self, y: f32) -> Vec3 {
        Vec3 {
            y,
            ..self
        }
    }

    fn sz(self, z: f32) -> Vec3 {
        Vec3 {
            z,
            ..self
        }
    }

    fn dot(self, other: Vec3) -> f32 {
        self.x*other.x + self.y*other.y + self.z*other.z
    }

    const fn zero() -> Vec3 {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

use core::ops::*;
impl Add<Vec3> for Vec3 {
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}
impl Sub<Vec3> for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, other: f32) -> Vec3 {
        Vec3 {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
        }
    }
}
impl Sub<f32> for Vec3 {
    type Output = Vec3;
    fn sub(self, other: f32) -> Vec3 {
        Vec3 {
            x: self.x - other,
            y: self.y - other,
            z: self.z - other,
        }
    }
}

fn rgb(r: f32, g: f32, b: f32) -> u32 {
    let r = (r.min(1.0) * 255.0) as u32;
    let g = (g.min(1.0) * 255.0) as u32;
    let b = (b.min(1.0) * 255.0) as u32;
    255 << 24 // Alpha = 1.0
        | b << 16
        | g << 8
        | r
}

fn scene(p: Vec3) -> f32 {
    (p.length() - 1.0).min((p - Vec3 {
        x: -2.0,
        y: 0.0,
        z: 1.0,
    }).length() - 1.0)
}

fn color(p: Vec3) -> Vec3 {
    let a = p.length() - 1.0;
    let b = (p - Vec3 {
        x: -2.0,
        y: 0.0,
        z: 1.0,
    }).length() - 1.0;
    if a < b {
        Vec3 {
            x: 1.0,
            y: 0.5,
            z: 0.2,
        }
    } else {
        Vec3 {
            x: 0.3,
            y: 0.7,
            z: 0.4,
        }
    }
}

fn normal(p: Vec3) -> Vec3 {
    let e = 0.01;
    Vec3 {
        x: scene(p.sx(p.x + e)) - scene(p.sx(p.x - e)),
        y: scene(p.sy(p.y + e)) - scene(p.sy(p.y - e)),
        z: scene(p.sz(p.z + e)) - scene(p.sz(p.z - e)),
    }.normalize()
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

    let camera_pos = Vec3 {
        z: -2.0,
        ..Vec3::zero()
    };
    let camera_dir = Vec3 {
        z: 1.0,
        ..Vec3::zero()
    };
    let up = Vec3 {
        y: 1.0,
        ..Vec3::zero()
    };
    let right = Vec3 {
        x: 1.0,
        ..Vec3::zero()
    };
    let rd = camera_dir
        + up * y
        + right * x;
    let rd = rd.normalize();
    let ro = camera_pos;

    let hit = trace(ro, rd);
    let f = hit.unwrap_or(0.0);

    let col = shade(color(ro + rd * f), up, normal(ro + rd * f));

    rgb(col.x, col.y, col.z)
}

pub fn run_serial() {
    let buf: Vec<_> = (0..SIZE * SIZE).map(|i| run_pixel(i)).collect();
    // let mut buf = vec![0; SIZE * SIZE];
    // for i in 0..SIZE * SIZE {
    //     run_pixel(&mut buf, i);
    // }

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
