//! # Stereo Camera Test
//!
//! Designed to test stereo camera display.

use std::io::Write;
use cv_camstream::prelude::*;
use image::GrayImage;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640 * 2;
const HEIGHT: usize = 480;
const NUM_FRAMES: usize = 100;

// -----------------------------------------------------------------------------------------------
// MAIN
// -----------------------------------------------------------------------------------------------  

#[test]
fn stereo() -> Result<(), Box<dyn std::error::Error>> {

    println!("Starting...");

    let mut camstream = CamStreamBuilder::new()
        .stereo()
        .left_path("/dev/video0")?
        .right_path("/dev/video2")?
        .rectif_params_from_file("tests/stereo_bench_drh_01.toml")?
        .interval((1, 30))
        .resolution((640, 480))
        .format(b"MJPG")?
        .build()?;

    println!("Cameras built");

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Stereo Camera Stream",
        WIDTH,
        HEIGHT,
        WindowOptions::default()
    ).unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut frame_num = 0;

    let mut left = GrayImage::new(WIDTH as u32, HEIGHT as u32);
    let mut right = GrayImage::new(WIDTH as u32, HEIGHT as u32);

    while window.is_open() && !window.is_key_down(Key::Escape) && frame_num < NUM_FRAMES {
        let pair = camstream
            .capture()?
            .to_luma8_pair();
        left = pair.0;
        right = pair.1;

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

        print!("\rFrame count: {}/{}", frame_num, NUM_FRAMES);
        std::io::stdout().flush()?;
        frame_num += 1;
    }

    // Save last frame
    left.save("left.png")?;
    right.save("right.png")?;

    Ok(())
}

fn luma_to_u32(luma: &image::Luma<u8>) -> u32 {
    (luma[0] as u32) << 24 | (luma[0] as u32) << 16 | (luma[0] as u32) << 8 | luma[0] as u32 
}