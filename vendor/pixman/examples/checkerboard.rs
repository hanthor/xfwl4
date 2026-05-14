use pixman::{Color, Filter, FormatCode, Image, Operation, Repeat, Solid};

const WIDTH: usize = 400;
const HEIGHT: usize = 400;
const TILE_SIZE: usize = 25;

pub fn main() {
    let mut checkerboard = Image::new(FormatCode::A8R8G8B8, WIDTH, HEIGHT, false).unwrap();
    let mut destination = Image::new(FormatCode::A8R8G8B8, WIDTH, HEIGHT, false).unwrap();

    // let transform = Transform::new([
    //     [-1.96830, -1.82250, 512.12250],
    //     [0.00000, -7.29000, 1458.00000],
    //     [0.00000, -0.00911, 0.59231],
    // ]);

    for i in 0..(HEIGHT / TILE_SIZE) {
        for j in 0..(WIDTH / TILE_SIZE) {
            let u = (j + 1) as f64 / (WIDTH / TILE_SIZE) as f64;
            let v = (i + 1) as f64 / (HEIGHT / TILE_SIZE) as f64;
            let black = Color::new(0, 0, 0, 0xffff);
            let white = Color::new(
                (v * 0xffff as f64) as u16,
                (u * 0xffff as f64) as u16,
                ((1.0 - u) * 0xffff as f64) as u16,
                0xffff,
            );

            let c = if (j & 1) != (i & 1) { black } else { white };

            let fill = Solid::new(c).unwrap();

            checkerboard.composite(
                Operation::Src,
                &fill,
                None,
                (0, 0),
                (0, 0),
                ((j * TILE_SIZE) as i16, (i * TILE_SIZE) as i16),
                (TILE_SIZE as u16, TILE_SIZE as u16),
            );
        }
    }

    // NOTE: The transform from the original demo completely breaks the image
    // checkerboard.set_transform(transform).unwrap();
    checkerboard.set_filter(Filter::Best, &[]).unwrap();
    checkerboard.set_repeat(Repeat::None);

    destination.composite(
        Operation::Src,
        &checkerboard,
        None,
        (0, 0),
        (0, 0),
        (0, 0),
        (WIDTH as u16, HEIGHT as u16),
    );

    let mut out_img = Image::new(
        FormatCode::A8B8G8R8,
        destination.width(),
        destination.height(),
        false,
    )
    .unwrap();
    out_img.composite(
        Operation::Src,
        &destination,
        None,
        (0, 0),
        (0, 0),
        (0, 0),
        (destination.width() as u16, destination.height() as u16),
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
