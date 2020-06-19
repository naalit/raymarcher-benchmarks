use ispc::*;
use crate::constants::*;

ispc_module!(test);

pub fn run_ispc() {
    let mut buf = vec![0; SIZE * SIZE];
    unsafe {
        test::ispc_main(buf.as_mut_ptr());
    }

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

pub fn run_ispc_parallel() {
    let mut buf = vec![0; SIZE * SIZE];
    unsafe {
        test::ispc_task(buf.as_mut_ptr());
    }

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
