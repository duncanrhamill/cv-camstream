//! # Stereo Camera Test
//!
//! Designed to test stereo camera display.

use cv_camstream::prelude::*;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640 * 2;
const HEIGHT: usize = 480;

// -----------------------------------------------------------------------------------------------
// MAIN
// -----------------------------------------------------------------------------------------------  

#[test]
fn stereo() -> Result<(), Box<dyn std::error::Error>> {

    let mut camstream = CamStreamBuilder::new()
        .stereo()
        .left_path("/dev/video0")?
        .right_path("/dev/video2")?
        .interval((1, 30))
        .resolution((640, 480))
        .format(b"MJPG")?
        .build()?;

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Stereo Camera Stream",
        WIDTH,
        HEIGHT,
        WindowOptions::default()
    ).unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (left, right) = camstream
            .capture()?
            .to_luma_pair();

        for y in 0..(HEIGHT) {
            for x in 0..(WIDTH) {
                if x > (WIDTH / 2) - 1 {
                    buffer[x + y * WIDTH] = luma_to_u32(right.get_pixel(
                        (x - (WIDTH / 2)) as u32, 
                        y as u32
                    ));
                }
                else {
                    buffer[x + y * WIDTH] = luma_to_u32(left.get_pixel(x as u32, y as u32));
                }
            }
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }

    Ok(())
}

fn luma_to_u32(luma: &image::Luma<u8>) -> u32 {
    (luma[0] as u32) << 24 | (luma[0] as u32) << 16 | (luma[0] as u32) << 8 | luma[0] as u32 
}