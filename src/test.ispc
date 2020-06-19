// SIZExSIZE
uniform const uint SIZE = 1024;

inline float length(float<3> s) {
  return sqrt(s.x*s.x + s.y*s.y + s.z*s.z);
}
inline float<3> normalize(float<3> s) {
  float l = length(s);
  return s / l;
}
inline float dot(float<3> a, float<3> b) {
  return a.x*b.x + a.y*b.y + a.z*b.z;
}
const float<3> zero() {
  uniform float<3> f = { 0.0, 0.0, 0.0 };
  return f;
}

uint rgb(float<3> v) {
  uint r = (uint) (min(v.x, 1.0) * 255.0);
  uint g = (uint) (min(v.y, 1.0) * 255.0);
  uint b = (uint) (min(v.z, 1.0) * 255.0);
  return 255u << 24
    | b << 16
    | g << 8
    | r;
}

float scene(float<3> p) {
  uniform float<3> s2 = { -2.0, 0.0, 1.0 };
  return min(length(p) - 1.0, length(p - s2) - 1.0);
}

float<3> color(float<3> p) {
  float a = length(p) - 1.0;
  uniform float<3> s2 = { -2.0, 0.0, 1.0 };
  float b = length(p - s2) - 1.0;

  uniform float<3> ca = { 1.0, 0.5, 0.2 };
  uniform float<3> cb = { 0.3, 0.7, 0.4 };

  if (a < b)
    return ca;
  else
    return cb;
}

inline float<3> vec3(float x, float y, float z) {
  float<3> r = { x, y, z };
  return r;
}

float<3> normal(float<3> p) {
  uniform float e = 0.01;
  float<3> n = {
    scene(vec3(p.x+e, p.y, p.z)) - scene(vec3(p.x-e, p.y, p.z)),
    scene(vec3(p.x, p.y+e, p.z)) - scene(vec3(p.x, p.y-e, p.z)),
    scene(vec3(p.x, p.y, p.z+e)) - scene(vec3(p.x, p.y, p.z-e))
  };
  return normalize(n);
}

// t on hit, 0.0 on no hit
float trace(float<3> ro, float<3> rd) {
  float<3> p = ro;
  float t = 0.0;

  for (uniform int i = 0; i < 64; i++) {
    float d = scene(p);
    cif (d < 0.01) {
      return t;
    }
    t += d;
    p = p + rd * d;
  }

  return 0.0;
}

float<3> shade(float<3> col, float<3> l, float<3> n) {
  float cos_theta = dot(l, n);

  // The SPIR-V version seems to handle NaN's slightly differently, so this is needed to make the output match
  if (cos_theta < -0.05) {
      return zero();
  }

  return col * cos_theta + col * 0.05;
}

uint run_pixel(uint ix, uint iy) {
  float x = ((float) ix) / ((float) SIZE * 0.5) - 1.0;
  float y = -((float) iy) / ((float) SIZE * 0.5) + 1.0;

  uniform float<3> camera_pos = { 0.0, 0.0, -2.0 };
  uniform float<3> camera_dir = { 0.0, 0.0, 1.0  };
  uniform float<3> up         = { 0.0, 1.0, 0.0  };
  uniform float<3> right      = { 1.0, 0.0, 0.0  };
  float<3> rd = normalize(camera_dir
    + up * y
    + right * x);

  float hit = trace(camera_pos, rd);

  float<3> col = shade(color(camera_pos + rd * hit), up, normal(camera_pos + rd * hit));

  return rgb(col);
}

export void ispc_main(uniform uint output[]) {
  foreach(i = 0 ... SIZE * SIZE) {
    output[i] = run_pixel(i % SIZE, i / SIZE);
  }
  // foreach_tiled (i = 0 ... SIZE, j = 0 ... SIZE) {
  //   output[j * SIZE + i] = run_pixel(i, j);
  // }
}

task void subset(uniform uint output[]) {
  uniform uint start = SIZE * SIZE * taskIndex / taskCount;
  uniform uint end = start + SIZE * SIZE / taskCount;
  foreach(i = start ... end) {
    output[i] = run_pixel(i % SIZE, i / SIZE);
  }
}

export void ispc_task(uniform uint output[]) {
  launch[64] subset(output);
}
