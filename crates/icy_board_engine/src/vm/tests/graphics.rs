use std::fmt::Write as _;

use super::run_ppl;

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
        GfxCreate 4, 12, 7
        PrintLn GfxValid(4)
        PrintLn GfxWidth(4)
        PrintLn GfxHeight(4)
        PrintLn GfxError()
        GfxLoad 5, "missing.png"
        PrintLn GfxError()
        GfxShutdown
        "#,
    );

    assert_eq!(output, "1\n12\n7\n0\n3\n");
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
        GfxCreate 0, 2, 2
        GfxSetPacing 1
        GfxPresent 0
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
        GfxCreate 0, 4, 4
        GfxCreate 1, 2, 2
        GfxClear 0, 255
        GfxClear 1, 4278190335
        GfxRect 1, 0, 0, 2, 2, 16711935
        GfxBlit 0, 1, 1, 1
        GfxPresent 0
        GfxFillRect 0, 2, 2, 1, 1, 16711935
        GfxPresentRect 0, 2, 2, 1, 1
        GfxWaitFrame 240
        GfxFree 1
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
        GfxCreate 1, 2, 2
        GfxClear 1, 4278190335
        GfxPresentAt 1, 10, 3
        GfxFree 1
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
        GfxLoad 1, "./banner.png"
        GfxPresentAt 1, 1, 1
        GfxPresentAt 1, 11, 3
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
        GfxLoad 1, "./banner.png"
        GfxPin 1
        GfxPresentRect 1, 2, 3, 4, 5, 20, 30
        GfxPin 1, FALSE
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
        GfxLoad 1, "./banner.png"
        GfxPin 1
        GfxFillRect 1, 0, 0, 1, 1, Rgb(255, 0, 0)
        GfxPresent 1
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
        GfxCreate 0, 8, 8
        GfxClear 0, 4278190335
        GfxPresent 0
        GfxFillRect 0, 2, 2, 2, 2, 16711935
        GfxPresentRect 0, 2, 2, 2, 2
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
        GfxCreate 3, 8, 8
        GfxClear 3, 4278190335
        GfxPresent 3
        GfxClear 3, 16711935
        GfxPresent 3
        GfxShutdown
        ",
        OLD_JXL_TERMINAL,
    );

    assert!(!output.contains("DrawJXLBlob"), "{output:?}");
    assert_eq!(output.matches("SyncTERM:C;S;gfx/n0s3.jxl;").count(), 2, "{output:?}");
    assert_eq!(output.matches("SyncTERM:C;DrawJXL;DX=0;DY=0;gfx/n0s3.jxl").count(), 2, "{output:?}");
}

#[test]
fn an_image_the_caller_already_cached_is_not_sent_again() {
    let mut terminal = b"\x1b[6;16;8t\x1b[=1;1-n\x1b_SyncTERM:C;L\n".to_vec();
    terminal.extend_from_slice(gfx_cache_name(BANNER).as_bytes());
    terminal.extend_from_slice(b"\td41d8cd98f00b204e9800998ecf8427e\n\x1b\\");

    let output = super::run_ppl_with_files_and_input(
        r#"
        GfxInit GFX_AUTO, FALSE
        GfxLoad 1, "./banner.png"
        GfxPresent 1
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
