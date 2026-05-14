use pixman::{
    Fixed, FormatCode, GradientStop, Image, Operation, Point, RadialGradient, Region32, Repeat,
    Transform,
};

const WIDTH: usize = 200;
const HEIGHT: usize = 200;

pub fn main() {
    let stops = [
        GradientStop::new(0, [0xffff, 0x0000, 0x0000, 0xffff]),
        GradientStop::new(1, [0xffff, 0xffff, 0x0000, 0xffff]),
    ];

    let trans = Transform::new([[1.3, 0.0, -0.5], [0.0, 1.0, -0.5], [0.0, 0.0, 1.0]]);

    let c_inner = Point::from((100.0, 100.0));
    let c_outer = Point::from((100.0, 100.0));
    let r_inner = Fixed::from(0);
    let r_outer = Fixed::from(100.0);

    let mut src = [0xff0000ffu32; WIDTH * HEIGHT];
    let mut src_img = Image::from_slice_mut(
        FormatCode::A8R8G8B8,
        WIDTH,
        HEIGHT,
        &mut src,
        WIDTH * 4,
        false,
    )
    .unwrap();

    let gradient_img = RadialGradient::new(c_inner, c_outer, r_inner, r_outer, &stops).unwrap();

    src_img.composite(
        Operation::Over,
        &gradient_img,
        None,
        (0, 0),
        (0, 0),
        (0, 0),
        (WIDTH as u16, HEIGHT as u16),
    );

    let clip_region = Region32::init_rect(50, 0, 100, 200);
    src_img.set_clip_region32(Some(&clip_region)).unwrap();
    src_img.set_source_clipping(true);
    src_img.set_has_client_clip(true);
    src_img.set_transform(trans).unwrap();
    src_img.set_repeat(Repeat::Normal);

    let mut dst = [0xffff0000u32; WIDTH * HEIGHT];
    let mut dst_img = Image::from_slice_mut(
        FormatCode::A8R8G8B8,
        WIDTH,
        HEIGHT,
        &mut dst,
        WIDTH * 4,
        false,
    )
    .unwrap();

    dst_img.composite(
        Operation::Over,
        &src_img,
        None,
        (0, 0),
        (0, 0),
        (0, 0),
        (WIDTH as u16, HEIGHT as u16),
    );

    let mut out_img = Image::new(
        FormatCode::A8B8G8R8,
        dst_img.width(),
        dst_img.height(),
        false,
    )
    .unwrap();
    out_img.composite(
        Operation::Src,
        &dst_img,
        None,
        (0, 0),
        (0, 0),
        (0, 0),
        (dst_img.width() as u16, dst_img.height() as u16),
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
