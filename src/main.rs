use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, DeviceExtensions};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::pipeline::ComputePipeline;
use vulkano::sync;
use vulkano::sync::GpuFuture;

use wasm_vk::*;

use std::sync::Arc;
use std::time::Instant;

mod serial;
mod simd;
mod simt_ispc;
mod constants;
use constants::*;

fn load_wasm(test: &str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::path::PathBuf;

    // let test = format!("{}.wat", test);
    // let test: PathBuf = ["tests", &test].iter().collect();
    // let test_name = test.file_name().unwrap().to_str().unwrap().to_string();
    let f = File::open(&test)?;
    let mut buf_reader = BufReader::new(f);

    let mut buf = Vec::new();
    buf_reader.read_to_end(&mut buf)?;
    match wabt::wat2wasm(buf) {
        Ok(binary) => {
            let w = wasm::deserialize_buffer(&binary).unwrap();

            let ctx = spirv::Ctx::new();
            let m = ctx.module(&w);
            let spv = spirv::module_bytes(m);
            let mut f = std::fs::File::create("comp.spv")?;
            f.write_all(&spv)?;
            Ok(())
        }
        Err(e) => {
            eprintln!("Test {} failed verification: {:?}", test, e);
            Err(std::io::Error::from(std::io::ErrorKind::Other))
        }
    }
}

fn run_spirv() {
    // Read SPIR-V from file instead of generating it - for debugging
    let spv = {
        use std::io::Read;
        let mut f = std::fs::File::open(if OPT { "comp2.spv" } else { "comp.spv" }).unwrap();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        buf
    };

    // First, we generate SPIR-V
    // let spv = spirv::to_spirv(w.clone());

    // We write the SPIR-V to disk so we can disassemble it later if we want
    // use std::io::Write;
    // let mut f = std::fs::File::create("examples/image.spv").unwrap();
    // f.write_all(&spv).unwrap();

    // println!("Written generated spirv to 'examples/image.spv'");

    // Here's the data we'll be using, it's just BUFFER_SIZE consecutive u32s, starting at 0
    let data_iter = 0..BUFFER_SIZE as u32;

    // Now we'll run the SPIR-V on the GPU with Vulkano.
    // This is a bunch of boilerplate, see the Vulkano examples for explanations.

    let instance = Instance::new(None, &InstanceExtensions::none(), None).unwrap();
    let physical = PhysicalDevice::enumerate(&instance).next().unwrap();
    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_compute())
        .unwrap();

    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &DeviceExtensions::none(),
        [(queue_family, 0.5)].iter().cloned(),
    )
    .unwrap();

    let queue = queues.next().unwrap();

    // This is pretty messy, but is pretty much what you need to do to get your own SPIR-V loaded with Vulkano
    let pipeline = Arc::new({
        #[derive(Copy, Clone)]
        struct PLayout;
        unsafe impl vulkano::descriptor::pipeline_layout::PipelineLayoutDesc for PLayout {
            fn num_sets(&self) -> usize {
                1
            }
            fn num_bindings_in_set(&self, set: usize) -> Option<usize> {
                assert_eq!(set, 0);
                Some(1)
            }
            fn descriptor(
                &self,
                set: usize,
                _binding: usize,
            ) -> Option<vulkano::descriptor::descriptor::DescriptorDesc> {
                assert_eq!(self.num_bindings_in_set(set), Some(1));
                Some(vulkano::descriptor::descriptor::DescriptorDesc {
                    ty: vulkano::descriptor::descriptor::DescriptorDescTy::Buffer(
                        vulkano::descriptor::descriptor::DescriptorBufferDesc {
                            // I have no idea what these do
                            dynamic: Some(false),
                            storage: true,
                        },
                    ),
                    array_count: 1,
                    stages: vulkano::descriptor::descriptor::ShaderStages::compute(),
                    readonly: false,
                })
            }
            fn num_push_constants_ranges(&self) -> usize {
                0
            }
            fn push_constants_range(
                &self,
                _num: usize,
            ) -> Option<vulkano::descriptor::pipeline_layout::PipelineLayoutDescPcRange>
            {
                None
            }
        }

        let shader =
            unsafe { vulkano::pipeline::shader::ShaderModule::new(device.clone(), &spv).unwrap() };

        let entry_str = std::ffi::CString::new("main").unwrap();

        let entry = unsafe { shader.compute_entry_point(&entry_str, PLayout) };

        ComputePipeline::new(device.clone(), &entry, &()).unwrap()
    });

    let data_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), data_iter.clone())
            .unwrap();

    let set = Arc::new(
        PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_buffer(data_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    let command_buffer =
        AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
            .unwrap()
            // Our workgroups are 64x1x1
            .dispatch([BUFFER_SIZE as u32 / 64, 1, 1], pipeline.clone(), set.clone(), ())
            .unwrap()
            // Finish building the command buffer by calling `build`.
            .build()
            .unwrap();

    // We time it from command buffer submission to fence signaling
    // let time = std::time::Instant::now();

    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    // let gpu_time = Instant::now() - time;

    // Here's the data the GPU got
    let data_buffer_content = data_buffer.read().unwrap();

    // print!(
    //     "{:?}",
    //     data_buffer_content.iter().take(4).collect::<Vec<_>>(),
    // );
    //
    // let image = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
    //     SIZE as u32,
    //     SIZE as u32,
    //     data_buffer_content
    //         .iter()
    //         .flat_map(|x| x.to_le_bytes().to_vec())
    //         .collect::<Vec<_>>(),
    // )
    // .unwrap();
    // image.save("test.png").unwrap();
}

fn bench_impl(name: &str, fun: impl Fn()) {
    let start = Instant::now();
    for _ in 0..NUM_RUNS {
        fun();
    }
    let end = Instant::now();
    println!("{}: Average time was {:?}", name, (end - start) / NUM_RUNS as u32);
}

fn main() {
    println!("####RUN 1####");
    bench_impl("serial", serial::run_serial);
    bench_impl("rayon", serial::run_rayon);
    bench_impl("spirv", run_spirv);
    bench_impl("simd", simd::run_simd);
    bench_impl("simd_rayon", simd::run_rayon);
    bench_impl("ispc", simt_ispc::run_ispc);
    bench_impl("ispc_parallel", simt_ispc::run_ispc_parallel);

    println!("####RUN 2####");
    bench_impl("serial", serial::run_serial);
    bench_impl("rayon", serial::run_rayon);
    bench_impl("spirv", run_spirv);
    bench_impl("simd", simd::run_simd);
    bench_impl("simd_rayon", simd::run_rayon);
    bench_impl("ispc", simt_ispc::run_ispc);
    bench_impl("ispc_parallel", simt_ispc::run_ispc_parallel);
}
