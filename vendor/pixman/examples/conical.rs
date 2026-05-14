use pixman::{
    ConicalGradient, FormatCode, GradientStop, Image, Operation, Repeat, Solid, Transform,
};

const SIZE: usize = 128;
const GRADIENTS_PER_ROW: usize = 7;
const NUM_ROWS: usize = (NUM_GRADIENTS + GRADIENTS_PER_ROW - 1) / GRADIENTS_PER_ROW;
const WIDTH: usize = SIZE * GRADIENTS_PER_ROW;
const HEIGHT: usize = SIZE * NUM_ROWS;
const NUM_GRADIENTS: usize = 35;

pub fn main() {
    let mut dest_img = Image::new(FormatCode::A8R8G8B8, WIDTH, HEIGHT, false).unwrap();

    draw_checkerboard(&mut dest_img, 25, 0xffaaaaaa, 0xff888888);

    let transform = Transform::identity()
        .translate(0.5, 0.5, true)
        .unwrap()
        .scale(SIZE as f32, SIZE as f32, true)
        .unwrap()
        .translate(0.5, 0.5, true)
        .unwrap();

    for i in 0..NUM_GRADIENTS {
        let column = i % GRADIENTS_PER_ROW;
        let row = i / GRADIENTS_PER_ROW;

        let mut src_img = create_conical(i);
        src_img.set_repeat(Repeat::Normal);
        src_img.set_transform(transform).unwrap();

        dest_img.composite32(
            Operation::Over,
            &src_img,
            None,
            (0, 0),
            (0, 0),
            ((column * SIZE) as i32, (row * SIZE) as i32),
            (SIZE as i32, SIZE as i32),
        );
    }

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

fn create_conical(index: usize) -> ConicalGradient<'static> {
    let angle = (0.5 / NUM_GRADIENTS as f64 + index as f64 / NUM_GRADIENTS as f64) * 720.0 - 180.0;
    ConicalGradient::new(
        (0.0, 0.0),
        angle,
        &[
            GradientStop::new(0.25, [1.0, 0.0, 0.0, 0.7]),
            GradientStop::new(0.5, [1.0, 1.0, 0.0, 0.7]),
            GradientStop::new(0.75, [0.0, 1.0, 0.0, 0.7]),
            GradientStop::new(1.0, [0.0, 0.0, 1.0, 0.7]),
        ],
    )
    .unwrap()
}

fn draw_checkerboard(image: &mut Image<'_, '_>, check_size: usize, color1: u32, color2: u32) {
    let c1 = Solid::new(color1).unwrap();
    let c2 = Solid::new(color2).unwrap();

    let n_checks_x = (image.width() + check_size - 1) / check_size;
    let n_checks_y = (image.height() + check_size - 1) / check_size;

    for j in 0..n_checks_y {
        for i in 0..n_checks_x {
            let src = if ((i ^ j) & 1) == 1 { &c1 } else { &c2 };

            image.composite32(
                Operation::Src,
                src,
                None,
                (0, 0),
                (0, 0),
                ((i * check_size) as i32, (j * check_size) as i32),
                (check_size as i32, check_size as i32),
            );
        }
    }
}
