use pixman::FormatCode;

const WIDTH: usize = 200;
const HEIGHT: usize = 200;

pub fn main() {
    let mut bits = [0u32; WIDTH * HEIGHT];
    pixman::fill(
        &mut bits,
        WIDTH as u32,
        FormatCode::bpp(FormatCode::A8R8G8B8),
        0,
        0,
        WIDTH as u32,
        HEIGHT as u32,
        0xff000000, // ABGR
    )
    .unwrap();
    let bits =
        unsafe { std::slice::from_raw_parts_mut(&mut bits as *mut _ as *mut _, bits.len() * 4) };
    let image_buffer =
        image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(WIDTH as u32, HEIGHT as u32, bits)
            .unwrap();
    image_buffer
        .save_with_format("out.png", image::ImageFormat::Png)
        .unwrap();
}
