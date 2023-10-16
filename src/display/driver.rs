use heapless::String;

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
use u8g2_fonts::fonts::u8g2_font_profont15_mf as Profont15;

use super::{WIDTH, HEIGHT, SCREEN_SIZE, Command, COMMAND_QUEUE};

use crate::utils::random;

type FrameBuffer<'a> = FrameBuf<Rgb565, &'a mut [Rgb565; SCREEN_SIZE]>;

pub fn drive<D>(mut display: D) -> !
where
    D: DrawTarget<Color = Rgb565, Error = DisplayError>,
{
    let mut data = [Rgb565::BLACK; SCREEN_SIZE];

    let mut fbuf = FrameBuf::new(
        &mut data,
        WIDTH as usize,
        HEIGHT as usize
    );
    
    let area = Rectangle::new(
        Point::default(),
        fbuf.size()
    );

    loop {
        use Command::*;

        if let Some(command) = COMMAND_QUEUE.dequeue() {
            match command {
                Splash => splash(&mut fbuf),
                Panic { message } => panic(&mut fbuf, message),
                _ => unimplemented!()
            };

            display.fill_contiguous(
                &area, 
                fbuf.data.iter().copied()
            ).unwrap();

            fbuf.clear(Rgb565::BLACK).unwrap();
        }
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

    let bounds = fbuf.bounding_box().offset(-20);
    
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
        bounds.anchor_point(AnchorPoint::Center),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(Rgb565::WHITE),
        fbuf
    )
    .unwrap();
}

fn panic(fbuf: &mut FrameBuffer, message: String<64>) {
    fbuf.clear(Rgb565::CSS_DARK_RED).unwrap();

    let bounds = fbuf.bounding_box().offset(-20);
    
    let lg_font_renderer = FontRenderer::new::<Profont29>();
    let sm_font_renderer = FontRenderer::new::<Profont15>();


    lg_font_renderer.render_aligned(
        "SYSTEM PANIC",
        bounds.anchor_point(AnchorPoint::TopCenter),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(Rgb565::WHITE),
        fbuf
    )
    .unwrap();

    sm_font_renderer.render_aligned(
        "Hyperdeck firmware halted. \n Power cycle to reset.",
        bounds.anchor_point(AnchorPoint::Center),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(Rgb565::WHITE),
        fbuf
    )
    .unwrap();

    sm_font_renderer.render_aligned(
        message.as_str(),
        bounds.anchor_point(AnchorPoint::BottomCenter),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(Rgb565::WHITE),
        fbuf
    )
    .unwrap();
}