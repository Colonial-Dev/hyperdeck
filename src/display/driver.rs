use embedded_graphics::geometry::AnchorPoint;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

use display_interface::DisplayError;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::Rgb565;

use embedded_graphics_framebuf::FrameBuf;

use u8g2_fonts::FontRenderer;
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};
use u8g2_fonts::fonts::u8g2_font_profont29_mf as Profont29;


use super::{WIDTH, HEIGHT, SCREEN_SIZE, Command, COMMAND_QUEUE};

use crate::utils::random;

type FrameBuffer = FrameBuf<Rgb565, &'static mut [Rgb565; SCREEN_SIZE]>;

pub fn drive<D>(mut display: D) -> !
where
    D: DrawTarget<Color = Rgb565, Error = DisplayError>,
{
    static mut FRAME_BUFFER_DATA: [Rgb565; SCREEN_SIZE]= [Rgb565::BLACK; SCREEN_SIZE];
    let mut fbuf = FrameBuf::new(unsafe { &mut FRAME_BUFFER_DATA }, WIDTH as usize, HEIGHT as usize);
    let area = Rectangle::new(Point::new(0, 0), fbuf.size());

    loop {
        use Command::*;

        COMMAND_QUEUE.dequeue().map(|command| match command {
            Clear => fbuf.clear(Rgb565::BLACK).unwrap(),
            Splash => splash(&mut fbuf),
            _ => unimplemented!()
        });

        display.fill_contiguous(
            &area, 
            unsafe { FRAME_BUFFER_DATA.iter().copied() }
        ).unwrap();
    }
}

/// Display the splash screen.
fn splash(fbuf: &mut FrameBuffer) 
{
    // Draw 255 random white pixels (stars) on the background.
    for _ in 0..255 {
        let x_pos = random(0, WIDTH as u32) as i32;
        let y_pos = random(0, HEIGHT as u32) as i32;

        Pixel(Point::new(x_pos, y_pos), Rgb565::WHITE)
            .draw(fbuf)
            .unwrap();
    }

    // Randomly pick an accent color combo for the wordmark.
    let (color_a, color_b) = match random(0, 3) {
        0 => (Rgb565::CSS_DARK_BLUE, Rgb565::CSS_DARK_RED),
        1 => (Rgb565::CSS_DARK_BLUE, Rgb565::CSS_DARK_GOLDENROD),
        2 => (Rgb565::CSS_PURPLE, Rgb565::CSS_DARK_GREEN),
        3 => (Rgb565::CSS_PURPLE, Rgb565::CSS_DARK_CYAN),
        _ => unreachable!()
    };

    let bb = fbuf.bounding_box().offset(-20);

    let font_renderer = FontRenderer::new::<Profont29>();

    font_renderer.render_aligned(
        "HYPERDECK",
        Point::new(119, 60),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(color_a),
        fbuf
    )
    .unwrap();

    font_renderer.render_aligned(
        "HYPERDECK",
        Point::new(119, 75),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(color_b),
        fbuf
    )
    .unwrap();

    font_renderer.render_aligned(
        "HYPERDECK",
        bb.anchor_point(AnchorPoint::Center),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(Rgb565::WHITE),
        fbuf
    )
    .unwrap();
}