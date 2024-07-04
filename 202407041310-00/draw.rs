use std::{fs::OpenOptions, io::Write};

use parley::{
    fontique::{Collection, CollectionOptions},
    layout::{Alignment, Glyph, GlyphRun},
    style::{FontFamily, StyleProperty},
    swash::zeno::{Command, PathData},
    FontContext, LayoutContext,
};
use parley::{
    style::FontStack,
    swash::{
        scale::{ScaleContext, Scaler},
        FontRef,
    },
};
use peniko::Color as PenikoColor;
use tiny_skia::{
    Color as TinySkiaColor, FillRule, Paint, Path as SkiaPath, PathBuilder, Pixmap, PixmapMut,
    Shader, Transform as TinySkiaTransform,
};

// const DIGIT_FONT: &[u8] = include_bytes!("../../SourceHanSansSC-Bold-subset.otf");
// const EMOJI_FONT: &[u8] = include_bytes!("../../NotoEmoji-VariableFont_wght-subset.ttf");
// const MATH_FONT: &[u8] = include_bytes!("../../NotoSansMath-Regular-subset.otf");

// const _SUB: &str = "123456ABC♥↺";

fn main() {
    let display_scale = 1.0;

    // also weird
    // let text = "Some text long longlong very long with longlonglongwordwordword";
    // let max_advance = 28.0 * display_scale;

    // weird
    let text = "ABCDABCDABCDABCDABCD";
    let max_advance = 56.0 * display_scale;

    let fg_color = PenikoColor::BLACK;
    let mut font_ctx = FontContext {
        collection: Collection::new(CollectionOptions {
            system_fonts: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut layout_ctx = LayoutContext::new();
    let mut scale_ctx = ScaleContext::new();

    // font_ctx.collection.register_fonts(DIGIT_FONT.to_owned());
    // font_ctx.collection.register_fonts(EMOJI_FONT.to_owned());
    // font_ctx.collection.register_fonts(MATH_FONT.to_owned());

    // for i in font_ctx.collection.family_names() {
    //     println!("{i}")
    // }

    let mut builder = layout_ctx.ranged_builder(&mut font_ctx, &text, display_scale);

    let _ = FontFamily::Named("Source Han Sans SC");
    // builder.push_default(&StyleProperty::FontStack(FontStack::List(&[
    //     FontFamily::Named("Source Han Sans SC"),
    //     FontFamily::Named("Noto Sans Math"),
    //     FontFamily::Named("Noto Emoji"),
    // ])));
    builder.push_default(&StyleProperty::FontStack(FontStack::Source("system-ui")));
    builder.push_default(&StyleProperty::Brush(to_rgba8(fg_color)));
    builder.push_default(&StyleProperty::LineHeight(1.0));
    builder.push_default(&StyleProperty::FontSize(16.0));

    let mut layout = builder.build();

    {
        let mut breaker = layout.break_lines();
        while let Some((_w, _)) = breaker.break_next(max_advance, Alignment::Start) {
            //println!("{_w}");
        }
        breaker.finish();
    }
    let height = layout.height().ceil();
    let width = layout.width().ceil();

    let mut img = Pixmap::new(width as _, height as _).unwrap();
    img.fill(TinySkiaColor::WHITE);

    for line in layout.lines() {
        for glyph_run in line.glyph_runs() {
            render_glyph_run(&mut scale_ctx, &mut img.as_mut(), glyph_run);
        }
    }

    let mut out_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open("demo.png")
        .unwrap();
    let png_data = img.encode_png().unwrap();
    out_file.write(&png_data).unwrap();
}

fn render_glyph_run(
    scale_ctx: &mut ScaleContext,
    canvas: &mut PixmapMut,
    glyph_run: GlyphRun<[u8; 4]>,
) {
    let mut run_x = glyph_run.offset();
    let run_y = glyph_run.baseline();
    let style = glyph_run.style();
    let color = style.brush;

    let run = glyph_run.run();
    let font = run.font();
    let font_size = run.font_size();
    let normalized_coords = run.normalized_coords();

    println!("{:?}", run.advance());

    let font_ref = FontRef::from_index(font.data.as_ref(), font.index as usize).unwrap();

    let mut scaler = scale_ctx
        .builder(font_ref)
        .size(font_size)
        .hint(true)
        .normalized_coords(normalized_coords)
        .build();

    for glyph in glyph_run.glyphs() {
        let glyph_x = run_x + glyph.x;
        let glyph_y = run_y - glyph.y;
        run_x += glyph.advance;
        render_glyph(
            canvas,
            &mut scaler,
            glyph,
            PenikoColor::from(color),
            glyph_x,
            glyph_y,
        );
    }
}

fn render_glyph(
    canvas: &mut PixmapMut,
    scaler: &mut Scaler,
    glyph: Glyph,
    color: PenikoColor,
    glyph_x: f32,
    glyph_y: f32,
) {
    if let Some(outlines) = scaler.scale_outline(glyph.id) {
        if let Some(path) = build_path(outlines.path(), glyph_x, glyph_y) {
            canvas.fill_path(
                &path,
                &Paint {
                    shader: Shader::SolidColor(to_tscolor(color)),
                    ..Default::default()
                },
                FillRule::Winding,
                TinySkiaTransform::identity(),
                None,
            );
        }
    } else {
        unimplemented!()
    }
}

fn to_tscolor(t: PenikoColor) -> TinySkiaColor {
    TinySkiaColor::from_rgba8(t.r, t.g, t.b, t.a)
}

fn to_rgba8(t: PenikoColor) -> [u8; 4] {
    [t.r, t.g, t.b, t.a]
}

fn build_path(path_data: impl PathData, x: f32, y: f32) -> Option<SkiaPath> {
    let mut pb = PathBuilder::new();
    for cmd in path_data.commands() {
        match cmd {
            Command::MoveTo(p) => pb.move_to(x + p.x, y - p.y),
            Command::LineTo(p) => pb.line_to(x + p.x, y - p.y),
            Command::CurveTo(p1, p2, p) => {
                pb.cubic_to(x + p1.x, y - p1.y, x + p2.x, y - p2.y, x + p.x, y - p.y)
            }
            Command::QuadTo(p1, p) => pb.quad_to(x + p1.x, y - p1.y, x + p.x, y - p.y),
            Command::Close => pb.close(),
        }
    }
    pb.finish()
}
