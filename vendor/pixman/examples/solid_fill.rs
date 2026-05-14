use pixman::{FormatCode, Image, Operation, Repeat, Solid};

pub fn main() {
    let mut dst = Image::new(FormatCode::A8R8G8B8, 800, 600, true).unwrap();
    let mut solid = Solid::new([0xffff, 0xffff, 0xffff, 0xffff]).unwrap();
    solid.set_repeat(Repeat::Normal);
    dst.composite(
        Operation::Over,
        &solid,
        None,
        (0, 0),
        (0, 0),
        (50, 0),
        (50, 50),
    );

    let mut out_img = Image::new(FormatCode::A8B8G8R8, dst.width(), dst.height(), false).unwrap();
    out_img.composite(
        Operation::Src,
        &dst,
        None,
        (0, 0),
        (0, 0),
        (0, 0),
        (dst.width() as u16, dst.height() as u16),
    );

    let image_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        out_img.width() as u32,
        out_img.height() as u32,
        unsafe {
            std::slice::from_raw_parts(
                out_img.data() as *const u8,
                out_img.stride() * out_img.height(),
            )
        },
    )
    .unwrap();
    image_buffer
        .save_with_format("out.png", image::ImageFormat::Png)
        .unwrap();
}
