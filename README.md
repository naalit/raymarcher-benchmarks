# Simple raytracer benchmarks

I wrote a raytracer in Rust, implemented the same thing in various ways, and measured the performance.
This is something in between a micro- and macro-benchmark, and not a great one, but it's still interesting to see the differences.

## Contenders

**Serial** is a simple, serial Rust implementation, compiled on release mode with target-cpu=native.

**Manual SIMD** is the same as **Serial**, but with `packed_simd::f32x4` for vectors.
It seems LLVM can vectorize **Serial** decently, because **Manual SIMD** performs worse.

**Parallel (Rayon)** is exactly the same as **Serial**, except for one call to `into_par_iter()`.
It runs pixels in a thread pool of 8 threads. For some reason, it uses slightly less memory than the other CPU implementations, maybe just because it's harder for top to measure across threads.

**ISPC** uses the Intel Implicit-SPMD Program Compiler to run 8 pixels at once on one CPU thread.
It performs very slightly better than using 8 different threads serially, which is quite impressive.

**ISPC Parallel** launches 64 ISPC tasks computing a subset of the image, each of which runs 8 pixels at once via SPMD.
I also tried it with different power-of-two numbers of tasks, but it peaked at 64.

**SPIR-V** is **Serial** compiled to WebAssembly, then to SPIR-V with `wasm-vk`, then optimized with `spirv-opt`, then run on the GPU with `vulkano`.
Despite including creation of pipelines, shader objects, transfer to and from GPU, etc. in its times, it outperforms everything else by 3x.
Note that even though the original SPIR-V is very bad code, it's only a couple ms slower than the `spirv-opt` version - most of the optimization that matters is probably in the driver's shader compiler.

The performance of **ISPC** and **SPIR-V** could probably be improved by running threads in a 2D tiled pattern instead of 1D, which is more coherent, and then arranging the image in memory so that threads in a group are still writing to contiguous memory.
I tried running **ISPC** tiled without the contiguous memory part, and got a slowdown, probably due to the scatter operation and associated cache misses.
**SPIR-V** could also be improved by using an actual Vulkan image instead of a 1D buffer, but the point of this benchmark is to measure the performance for general parallel workloads, not hardware image processing capabilities.

## Results

The `Space` measurement was very rough, running just one benchmark continuously and looking at the process memory in `top`, so take that with a grain of salt.
The main reason it's there is to show that Vulkan does have a ~10MB memory overhead, at least in this case.
Note that a 1024x1024 image of `u32`s, which these all create, takes up 4MB normally.

This was 1024x1024 pixels. All run on an AMD FX-8350 GPU and a Radeon RX 460 GPU with RADV, on Arch Linux.
`ISPC = 1.13.0` with `LLVM = 10.0.0`, `rustc = 1.44.1`.

| Benchmark        | Time (ms) | Space (MB) |
| ---------------- | ---------:| ----------:|
| SPIR-V           |        25 |         14 |
| ISPC Parallel    |        75 |          4 |
| ISPC             |       152 |          4 |
| Parallel (Rayon) |       156 |          3 |
| Serial           |       964 |          4 |
| Manual SIMD      |      1078 |          4 |

So what did we learn?
- GPUs are, in fact, very good at highly parallel workloads.
- ISPC is far better than normal compiler vectorization, or using SIMD manually for small parts of the program. For at least some parallel workloads, eight SIMD lanes with ISPC can beat eight CPU threads normally.
