use std::fmt::Write as _;

use super::{compile_errors_with_runtime, run_ppl};

#[test]
fn multimedia_apis_require_runtime_402() {
    for runtime in [400, 401] {
        let errors = compile_errors_with_runtime("GfxInit GFX_SIXEL\nPRINTLN GfxBackend()", runtime);
        assert!(errors.iter().any(|error| error == "GfxInit needs runtime 402"), "runtime {runtime}: {errors:?}");
        assert!(
            errors.iter().any(|error| error == "GfxBackend needs runtime 402"),
            "runtime {runtime}: {errors:?}"
        );
    }
    assert!(compile_errors_with_runtime("GfxInit GFX_SIXEL\nPRINTLN GfxBackend()", 402).is_empty());
}

#[test]
fn rgb_colors_pack_channels_and_clamp_components() {
    let output = run_ppl(
        r"
        CONST UNSIGNED RED = Rgb(255, 0, 0)
        CONST UNSIGNED TRANSLUCENT = Rgb(1, 2, 3, 128)
        PrintLn RED
        PrintLn TRANSLUCENT
        PrintLn Rgb(-1, 256, 17, 999)
        PrintLn 0FF0000FFh
        ",
    );

    assert_eq!(output, "4278190335\n16909184\n16716287\n4278190335\n");
}

#[test]
fn reports_an_unavailable_backend_for_text_fallback() {
    let output = run_ppl(
        r"
        PrintLn GfxBackend()
        GfxInit 99
        PrintLn GfxBackend()
        ",
    );

    assert_eq!(output, "-1\n-1\n");
}

#[test]
fn surface_status_reports_dimensions_and_errors() {
    let output = run_ppl(
        r#"
        GfxInit GFX_SIXEL, FALSE
        SURFACE s = NewSurface(12, 7)
        PrintLn s.Valid
        PrintLn s.Width
        PrintLn s.Height
        PrintLn GfxError()
        SURFACE missing = LoadSurface("missing.png")
        PrintLn GfxError()
        GfxShutdown
        "#,
    );

    assert_eq!(output, "1\n12\n7\n0\n3\n");
}

#[test]
fn drawing_and_pinning_report_specific_errors() {
    let output = run_ppl(
        r"
        GfxInit GFX_SIXEL, FALSE
        SURFACE s = NewSurface(2, 2)
        s.Free()
        s.Clear(0)
        PrintLn GfxError()
        s.Pin()
        PrintLn GfxError()
        s.Free()
        PrintLn GfxError()
        ",
    );

    assert_eq!(output, "2\n6\n2\n");
}

#[test]
fn surface_count_is_bounded_and_freed_surfaces_can_be_reused() {
    let mut source = String::from("GfxInit GFX_SIXEL, FALSE\nSURFACE first, extra\nfirst = NewSurface(1, 1)\n");
    for _ in 1..256 {
        let _ = writeln!(source, "extra = NewSurface(1, 1)");
    }
    source.push_str("extra = NewSurface(1, 1)\nPRINTLN GfxError()\nfirst.Free()\nextra = NewSurface(1, 1)\nPRINTLN GfxError()\n");

    assert_eq!(run_ppl(&source), "5\n0\n");
}

#[test]
fn a_scaled_and_flipped_present_is_left_to_the_client() {
    let output = super::run_ppl_with_input(
        r"
        GfxInit GFX_AUTO, FALSE
        SURFACE s = NewSurface(8, 8)
        s.Clear(4278190335)
        s.PresentRect(0, 0, 8, 8, 10, 20, 64, 32, GFX_FLIP_X)
        GfxShutdown
        ",
        JXL_TERMINAL,
    );

    assert!(output.contains("SyncTERM:C;DrawJXLBlob;DX=10;DY=20;DW=64;DH=32;FX;"), "{output:?}");
}

#[test]
fn sixel_cannot_scale_and_says_so() {
    let output = run_ppl(
        r"
        GfxInit GFX_SIXEL, FALSE
        SURFACE s = NewSurface(8, 8)
        s.PresentRect(0, 0, 8, 8, 0, 0, 32, 32)
        PrintLn GfxError()
        GfxShutdown
        ",
    );

    assert_eq!(output, "6\n");
}

#[test]
fn a_surface_object_draws_and_reports_its_own_size() {
    let output = run_ppl(
        r#"
        GfxInit GFX_SIXEL, FALSE
        SURFACE s = NewSurface(4, 4)
        PrintLn s.Valid
        PrintLn s.Width, ",", s.Height
        s.Clear(Rgb(0, 0, 0))
        s.SetPixel(1, 1, Rgb(255, 0, 0))
        s.FillRect(0, 0, 2, 2, Rgb(0, 255, 0))
        s.Present()
        PrintLn GfxError()
        s.Free()
        PrintLn s.Valid
        GfxShutdown
        "#,
    );

    assert!(output.starts_with("1\n4,4\n"), "{output:?}");
    assert!(output.ends_with("0\n0\n"), "{output:?}");
}

#[test]
fn a_surface_can_be_blitted_onto_another() {
    let output = run_ppl(
        r"
        GfxInit GFX_SIXEL, FALSE
        SURFACE back = NewSurface(8, 8)
        SURFACE sprite = NewSurface(2, 2)
        sprite.Clear(Rgb(255, 0, 0))
        back.Blit(sprite, 3, 3)
        PrintLn GfxError()
        GfxShutdown
        ",
    );

    assert_eq!(output, "0\n");
}

#[test]
fn capability_and_geometry_queries_report_terminal_answers() {
    let output = super::run_ppl_with_input(
        r"
        PrintLn GfxCaps()
        PrintLn GfxCellWidth()
        PrintLn GfxCellHeight()
        PrintLn GfxScreenWidth()
        PrintLn GfxScreenHeight()
        ",
        b"\x1b[<1;4;7;8c\x1b[6;20;10t\x1b[4;600;800t\x1b[=1;1-n\x1b_SyncTERM:C;L\n\x1b\\",
    );

    assert!(output.ends_with("35\n10\n20\n800\n600\n"), "{output:?}");
}

#[test]
fn pacing_requests_an_acknowledgement_after_presenting() {
    let output = super::run_ppl_with_input(
        r"
        GfxInit GFX_SIXEL, FALSE
        SURFACE s = NewSurface(2, 2)
        GfxSetPacing 1
        s.Present()
        GfxShutdown
        ",
        b"\x1b[1;1R",
    );

    assert!(output.contains("\x1b[6n"), "{output:?}");
}

#[test]
fn a_silent_terminal_has_no_automatic_graphics_backend() {
    let output = run_ppl(
        r"
        GfxInit GFX_AUTO
        PrintLn GfxBackend()
        GfxShutdown
        ",
    );

    assert!(output.contains("\x1b_SyncTERM:Q;JXL\x1b\\"), "{output:?}");
    assert!(output.ends_with("-1\n"), "{output:?}");
}

#[test]
fn a_silent_terminal_refuses_an_explicit_jpeg_xl_request() {
    let output = run_ppl(
        r"
        GfxInit GFX_JXL
        PrintLn GfxBackend()
        ",
    );

    assert!(output.ends_with("-1\n"), "{output:?}");
}

#[test]
fn asking_for_sixel_does_not_query_the_terminal() {
    let output = run_ppl(
        r"
        GfxInit GFX_SIXEL
        PrintLn GfxBackend()
        GfxShutdown
        ",
    );

    assert!(!output.contains("SyncTERM:Q;JXL"), "{output:?}");
    assert!(output.contains("2\n"), "{output:?}");
}

#[test]
fn reports_the_selected_backend_after_initialization() {
    let output = run_ppl(
        r"
        GfxInit GFX_SIXEL
        PrintLn GfxBackend()
        GfxShutdown
        ",
    );

    assert!(output.contains("2\n"));
}

#[test]
fn creates_blits_and_presents_in_memory_surfaces() {
    let output = run_ppl(
        r"
        GfxInit GFX_SIXEL
        SURFACE back = NewSurface(4, 4)
        SURFACE sprite = NewSurface(2, 2)
        back.Clear(255)
        sprite.Clear(4278190335)
        sprite.Rect(0, 0, 2, 2, 16711935)
        back.Blit(sprite, 1, 1)
        back.Present()
        back.FillRect(2, 2, 1, 1, 16711935)
        back.PresentRect(2, 2, 1, 1)
        sprite.Free()
        GfxShutdown
        ",
    );

    assert!(output.starts_with("\x1b[2J\x1b[H"));
    assert_eq!(output.matches("\x1bP").count(), 2);
    assert!(output.ends_with("\x1b[?1070h\x1b[?80h\x1b[?7h\x1b[?25h"));
}

#[test]
fn inline_graphics_preserve_the_text_screen_and_cursor() {
    let output = run_ppl(
        r#"
        PrintLn "before"
        GfxInit GFX_SIXEL, FALSE
        SURFACE s = NewSurface(2, 2)
        s.Clear(4278190335)
        s.PresentAt(10, 3)
        s.Free()
        GfxShutdown
        PrintLn "after"
        "#,
    );

    assert!(output.starts_with("before\n"));
    assert!(output.contains("\x1b7\x1b[3;10H\x1bP"));
    assert!(output.contains("\x1b\\\x1b8after\n"));
    assert!(!output.contains("\x1b[2J"));
    assert!(!output.contains("\x1b[?25l"));
    assert!(!output.contains("\x1b[?25h"));
}

/// A terminal that answers the JPEG XL query, names a `CTerm` revision new enough for
/// inline blobs, and reports an empty cache.
const JXL_TERMINAL: &[u8] = b"\x1b[=67;84;101;114;109;1;332c\x1b[6;16;8t\x1b[=1;1-n\x1b_SyncTERM:C;L\n\x1b\\";

/// The same terminal without a revision, so inline blobs are not on the table.
const OLD_JXL_TERMINAL: &[u8] = b"\x1b[6;16;8t\x1b[=1;1-n\x1b_SyncTERM:C;L\n\x1b\\";

const BANNER: &[u8] = include_bytes!("../../../../../ppe/inline_gfx/banner.png");

#[test]
fn a_loaded_image_is_cached_once_and_drawn_by_name() {
    let output = super::run_ppl_with_files_and_input(
        r#"
        GfxInit GFX_AUTO, FALSE
        PrintLn GfxBackend()
        SURFACE banner = LoadSurface("./banner.png")
        banner.PresentAt(1, 1)
        banner.PresentAt(11, 3)
        GfxShutdown
        "#,
        &[("banner.png", BANNER)],
        JXL_TERMINAL,
    );

    assert!(output.contains("3\n"), "{output:?}");
    // Unchanged pixels keep their content hash, so the bytes only go out once.
    assert_eq!(output.matches("SyncTERM:C;S;gfx/").count(), 1, "{output:?}");
    assert_eq!(output.matches("SyncTERM:C;DrawJXL;").count(), 2, "{output:?}");
    // Cell size came back as 8x16, so column 11 row 3 is pixel 80,32.
    assert!(output.contains("SyncTERM:C;DrawJXL;DX=0;DY=0;"), "{output:?}");
    assert!(output.contains("SyncTERM:C;DrawJXL;DX=80;DY=32;"), "{output:?}");
    assert!(!output.contains("\x1bP"), "{output:?}");
}

#[test]
fn a_pinned_image_uses_the_client_pixel_buffer_for_partial_blits() {
    let output = super::run_ppl_with_files_and_input(
        r#"
        GfxInit GFX_AUTO, FALSE
        SURFACE banner = LoadSurface("./banner.png")
        banner.Pin()
        banner.PresentRect(2, 3, 4, 5, 20, 30)
        banner.Unpin()
        GfxShutdown
        "#,
        &[("banner.png", BANNER)],
        JXL_TERMINAL,
    );

    assert!(output.contains("SyncTERM:C;LoadJXLBlob;B=0;"), "{output:?}");
    assert!(output.contains("SyncTERM:P;Paste;B=0;SX=2;SY=3;SW=4;SH=5;DX=20;DY=30"), "{output:?}");
}

#[test]
fn drawing_on_a_pinned_surface_returns_to_normal_presentation() {
    let output = super::run_ppl_with_files_and_input(
        r#"
        GfxInit GFX_AUTO, FALSE
        SURFACE banner = LoadSurface("./banner.png")
        banner.Pin()
        banner.FillRect(0, 0, 1, 1, Rgb(255, 0, 0))
        banner.Present()
        GfxShutdown
        "#,
        &[("banner.png", BANNER)],
        JXL_TERMINAL,
    );

    assert!(output.contains("SyncTERM:C;LoadJXLBlob;B=0;"), "{output:?}");
    assert!(output.contains("SyncTERM:C;DrawJXLBlob;DX=0;DY=0;"), "{output:?}");
    assert!(!output.contains("SyncTERM:P;Paste"), "{output:?}");
}

#[test]
fn a_composed_frame_goes_inline_instead_of_into_the_cache() {
    let output = super::run_ppl_with_input(
        r"
        GfxInit GFX_AUTO, FALSE
        SURFACE s = NewSurface(8, 8)
        s.Clear(4278190335)
        s.Present()
        s.FillRect(2, 2, 2, 2, 16711935)
        s.PresentRect(2, 2, 2, 2)
        GfxShutdown
        ",
        JXL_TERMINAL,
    );

    assert_eq!(output.matches("SyncTERM:C;DrawJXLBlob;").count(), 2, "{output:?}");
    assert!(!output.contains("SyncTERM:C;S;"), "{output:?}");
    // A partial update carries only its own rectangle and says where it belongs.
    assert!(output.contains("SyncTERM:C;DrawJXLBlob;DX=2;DY=2;"), "{output:?}");
}

#[test]
fn a_terminal_without_inline_blobs_reuses_one_cache_name_per_surface() {
    let output = super::run_ppl_with_input(
        r"
        GfxInit GFX_AUTO, FALSE
        SURFACE s = NewSurface(8, 8)
        s.Clear(4278190335)
        s.Present()
        s.Clear(16711935)
        s.Present()
        GfxShutdown
        ",
        OLD_JXL_TERMINAL,
    );

    assert!(!output.contains("DrawJXLBlob"), "{output:?}");
    assert_eq!(output.matches("SyncTERM:C;S;gfx/n0s1.jxl;").count(), 2, "{output:?}");
    assert_eq!(output.matches("SyncTERM:C;DrawJXL;DX=0;DY=0;gfx/n0s1.jxl").count(), 2, "{output:?}");
}

#[test]
fn an_image_the_caller_already_cached_is_not_sent_again() {
    let mut terminal = b"\x1b[6;16;8t\x1b[=1;1-n\x1b_SyncTERM:C;L\n".to_vec();
    terminal.extend_from_slice(gfx_cache_name(BANNER).as_bytes());
    terminal.extend_from_slice(b"\td41d8cd98f00b204e9800998ecf8427e\n\x1b\\");

    let output = super::run_ppl_with_files_and_input(
        r#"
        GfxInit GFX_AUTO, FALSE
        SURFACE banner = LoadSurface("./banner.png")
        banner.Present()
        GfxShutdown
        "#,
        &[("banner.png", BANNER)],
        &terminal,
    );

    assert!(!output.contains("SyncTERM:C;S;"), "{output:?}");
    assert_eq!(output.matches("SyncTERM:C;DrawJXL;").count(), 1, "{output:?}");
}

/// The name a loaded image lands under, which is the hash of its encoded form.
fn gfx_cache_name(png: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use zune_core::{bit_depth::BitDepth, colorspace::ColorSpace, options::EncoderOptions};
    use zune_jpegxl::JxlSimpleEncoder;

    let image = image::load_from_memory(png).unwrap().to_rgba8();
    let (width, height) = (image.width() as usize, image.height() as usize);
    let rgb: Vec<u8> = image.into_raw().chunks_exact(4).flat_map(|pixel| [pixel[0], pixel[1], pixel[2]]).collect();
    let mut encoded = Vec::new();
    JxlSimpleEncoder::new(&rgb, EncoderOptions::new(width, height, ColorSpace::RGB, BitDepth::Eight))
        .encode(&mut encoded)
        .unwrap();
    format!("gfx/{}.jxl", &format!("{:x}", Sha256::digest(&encoded))[..32])
}

/// Shrunk to one zoom frame at a coarse resolution: the point is that the demo
/// runs, not that it looks like anything at this size.
fn fractal_source(backend: &str) -> String {
    include_str!("../../../../../ppe/fractal/src/fractal.pps")
        .replace("GfxInit GFX_AUTO", &format!("GfxInit {backend}"))
        .replace("CONST INTEGER ZOOM_FRAMES = 10", "CONST INTEGER ZOOM_FRAMES = 1")
        .replace("    fracW = 160\n    fracH = 80", "    fracW = 16\n    fracH = 8")
        .replace("    fracW = 320\n    fracH = 160", "    fracW = 16\n    fracH = 8")
        .replace("WHILE TRUE DO", "WHILE FALSE DO")
}

#[test]
fn the_fractal_demo_asks_the_terminal_to_scale_its_small_surface() {
    let output = super::run_ppl_with_input(&fractal_source("GFX_AUTO"), JXL_TERMINAL);

    // 16x8 pixels of fractal covering the 64x32 the viewport scales it to.
    assert!(output.contains("SyncTERM:C;DrawJXLBlob;DX=0;DY=0;DW=64;DH=32;"), "{output:?}");
    assert!(output.contains("surfaces, per-pixel drawing"), "{output:?}");
}

#[test]
fn the_fractal_demo_checks_for_input_between_mandelbrot_bands() {
    let source = fractal_source("GFX_AUTO").replace("CONST INTEGER BAND = 10", "CONST INTEGER BAND = 2");
    let output = super::run_ppl_with_input(&source, &[JXL_TERMINAL, b"q"].concat());

    // One HUD and the first fractal band are presented before the queued key stops the frame.
    assert_eq!(output.matches("SyncTERM:C;DrawJXLBlob;").count(), 2, "{output:?}");
    assert!(output.contains("surfaces, per-pixel drawing"), "{output:?}");
}

/// The demo skips the two parts of the set whose outline is known, which is only worth
/// doing if the picture is the same as iterating every pixel would give.
#[test]
fn the_mandelbrot_shortcuts_answer_what_iterating_every_pixel_would() {
    let escape = "WHILE i < maxIter & zx * zx + zy * zy < 4.0 DO\nt = zx * zx - zy * zy + ax\nzy = 2.0 * zx * zy + ay\nzx = t\ni += 1\nENDWHILE";
    let shortcut = "q = (ax - 0.25) * (ax - 0.25) + ay * ay\nIF q * (q + ax - 0.25) <= 0.25 * ay * ay THEN\nj = maxIter\nELSEIF ((ax + 1.0) * (ax + 1.0) + ay * ay <= 0.0625) THEN\nj = maxIter\nELSE\nWHILE j < maxIter & zx * zx + zy * zy < 4.0 DO\nt = zx * zx - zy * zy + ax\nzy = 2.0 * zx * zy + ay\nzx = t\nj += 1\nENDWHILE\nENDIF";

    // Deep in seahorse valley, where the interior points the shortcuts answer for are
    // the ones that would otherwise cost the whole budget.
    let source = format!(
        "INTEGER maxIter, px, py, i, j, differences\nDOUBLE scale, stepSize, minX, minY, ax, ay, zx, zy, t, q\nmaxIter = 120\nscale = 0.016935\nstepSize = scale * 2.0 / 30\nminX = -0.743643887 - scale * 2.0\nminY = 0.131825904 - scale\nFOR py = 0 TO 29\nay = minY + py * stepSize\nFOR px = 0 TO 59\nax = minX + px * stepSize\nzx = 0.0\nzy = 0.0\ni = 0\n{escape}\nzx = 0.0\nzy = 0.0\nj = 0\n{shortcut}\nIF i <> j differences = differences + 1\nNEXT px\nNEXT py\nPRINT differences\n"
    );

    assert_eq!(run_ppl(&source), "0");
}

#[test]
fn the_fractal_demo_renders_at_native_size_when_the_terminal_cannot_scale() {
    let output = run_ppl(&fractal_source("GFX_AUTO"));

    // No backend at all: the demo says so instead of drawing.
    assert!(output.contains("requires a graphics-capable terminal"), "{output:?}");

    let output = run_ppl(&fractal_source("GFX_SIXEL"));
    assert!(output.contains("\x1bP"), "{output:?}");
    assert!(output.contains("surfaces, per-pixel drawing"), "{output:?}");
}

#[test]
fn tetris_initializes_and_renders_without_leaking_call_frames() {
    let source = include_str!("../../../../../ppe/tetris/src/tetris.pps")
        .replace("GfxInit GFX_AUTO", "GfxInit GFX_SIXEL")
        .replace("running = TRUE", "running = FALSE");
    let output = super::run_ppl_with_files(
        &source,
        &[
            ("ui.png", include_bytes!("../../../../../ppe/tetris/ui.png")),
            ("tetris_music.ogg", include_bytes!("../../../../../ppe/tetris/tetris_music.ogg")),
            ("rotate.wav", include_bytes!("../../../../../ppe/tetris/rotate.wav")),
            ("lock.wav", include_bytes!("../../../../../ppe/tetris/lock.wav")),
            ("line.wav", include_bytes!("../../../../../ppe/tetris/line.wav")),
            ("gameover.wav", include_bytes!("../../../../../ppe/tetris/gameover.wav")),
        ],
    );

    assert!(output.contains("Loading assets..."));
    assert!(output.contains("\x1bP"));
}

#[test]
fn tetris_flashes_a_completed_line_before_compacting_it() {
    let mut source = include_str!("../../../../../ppe/tetris/src/tetris.pps")
        .replace("GfxInit GFX_AUTO", "GfxInit GFX_SIXEL")
        .replace("running = TRUE", "running = FALSE");
    let mut completed_row = String::new();
    for column in 0..10 {
        let _ = writeln!(completed_row, "board[{column}, 19] = 1");
    }
    source = source.replace(
        "SpawnPiece()\nRenderGame()",
        &format!("SpawnPiece()\n{completed_row}score = ClearLines()\nRenderGame()"),
    );
    let output = super::run_ppl_with_files(
        &source,
        &[
            ("ui.png", include_bytes!("../../../../../ppe/tetris/ui.png")),
            ("tetris_music.ogg", include_bytes!("../../../../../ppe/tetris/tetris_music.ogg")),
            ("rotate.wav", include_bytes!("../../../../../ppe/tetris/rotate.wav")),
            ("lock.wav", include_bytes!("../../../../../ppe/tetris/lock.wav")),
            ("line.wav", include_bytes!("../../../../../ppe/tetris/line.wav")),
            ("gameover.wav", include_bytes!("../../../../../ppe/tetris/gameover.wav")),
        ],
    );

    assert!(output.matches("\x1bP").count() >= 5);
}
