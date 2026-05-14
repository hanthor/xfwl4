use pixman::{
    Fixed, FormatCode, GradientStop, Image, LinearGradient, Operation, Point, Repeat, Transform,
};

const WIDTH: usize = 400;
const HEIGHT: usize = 200;

pub fn main() {
    let stops = [
        GradientStop::new(0, [0x0000, 0x0000, 0xffff, 0xffff]),
        GradientStop::new(1, [0xffff, 0x1111, 0x1111, 0xffff]),
    ];

    let p1 = Point::from((50f64, 0f64));
    let p2 = Point::from((200f64, 0f64));

    let transform = Transform::new([
        [Fixed::ONE, Fixed::ZERO, Fixed::ZERO],
        [Fixed::ZERO, Fixed::ONE, Fixed::ZERO],
        [Fixed::ZERO, Fixed::ZERO, Fixed::ONE],
    ]);

    let mut dest = [0xff00ff00u32; WIDTH * HEIGHT];
    let mut dest_img = Image::from_slice_mut(
        FormatCode::A8R8G8B8,
        WIDTH,
        HEIGHT,
        &mut dest,
        WIDTH * 4,
        false,
    )
    .unwrap();

    let mut src_img = LinearGradient::new(p1, p2, &stops).unwrap();

    src_img.set_transform(transform).unwrap();
    src_img.set_repeat(Repeat::None);

    dest_img.composite(
        Operation::Over,
        &src_img,
        None,
        (0, 0),
        (0, 0),
        (0, 0),
        ((10 * WIDTH) as u16, HEIGHT as u16),
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
