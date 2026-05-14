use pixman::{Color, FormatCode, Image, Operation, Solid, Triangle};

const WIDTH: usize = 200;
const HEIGHT: usize = 200;

pub fn main() {
    let tris = [
        Triangle::from([(100, 100), (10, 50), (110, 10)]),
        Triangle::from([(100, 100), (150, 10), (200, 50)]),
        Triangle::from([(100, 100), (10, 170), (90, 175)]),
        Triangle::from([(100, 100), (170, 150), (120, 190)]),
    ];

    let color = Color::new(0x4444, 0x4444, 0xffff, 0xffff);
    let mut bits = [0u32; WIDTH * HEIGHT];

    for (i, item) in bits.iter_mut().enumerate().take(WIDTH * HEIGHT) {
        *item = ((i / HEIGHT) as u32) * 0x01010000;
    }

    let src_img = Solid::new(color).unwrap();
    let mut dest_img = Image::from_slice_mut(
        FormatCode::A8R8G8B8,
        WIDTH,
        HEIGHT,
        &mut bits,
        WIDTH * 4,
        false,
    )
    .unwrap();

    dest_img.composite_triangles(
        Operation::AtopReverse,
        &src_img,
        FormatCode::A8,
        (200, 200),
        (-5, 5),
        &tris,
    );

    let mut out_img = Image::new(
        FormatCode::A8B8G8R8,
        dest_img.width(),
        dest_img.height(),
        false,
    )
    .unwrap();
    out_img.composite(
        Operation::Src,
        &dest_img,
        None,
        (0, 0),
        (0, 0),
        (0, 0),
        (dest_img.width() as u16, dest_img.height() as u16),
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
