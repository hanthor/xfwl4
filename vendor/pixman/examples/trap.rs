use pixman::{Color, Fixed, FormatCode, Image, Operation, Solid, Span, Trap};

const WIDTH: usize = 200;
const HEIGHT: usize = 200;

pub fn main() {
    let white = Color::from([0x0000, 0xffff, 0x0000, 0xffff]);

    let mut mbits = [0u32; WIDTH * HEIGHT];
    let mut bits = [0xffu8; WIDTH * HEIGHT * 4];
    let bits =
        unsafe { std::slice::from_raw_parts_mut(&mut bits as *mut u8 as *mut u32, bits.len() / 4) };

    let trap = Trap::new(
        Span::new(
            Fixed::from_int(50) + Fixed::from_raw(0x8000),
            Fixed::from_int(150) + Fixed::from_raw(0x8000),
            30,
        ),
        Span::new(
            Fixed::from_int(50) + Fixed::from_raw(0x8000),
            Fixed::from_int(150) + Fixed::from_raw(0x8000),
            150,
        ),
    );

    let mut mask_img =
        Image::from_slice_mut(FormatCode::A8, WIDTH, HEIGHT, &mut mbits, WIDTH, false).unwrap();
    let src_img = Solid::new(white).unwrap();
    let mut dest_img =
        Image::from_slice_mut(FormatCode::A8R8G8B8, WIDTH, HEIGHT, bits, WIDTH * 4, false).unwrap();

    mask_img.add_traps((0, 0), &[trap]);

    dest_img.composite(
        Operation::Over,
        &src_img,
        Some(&mask_img),
        (0, 0),
        (0, 0),
        (0, 0),
        (WIDTH as u16, HEIGHT as u16),
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
