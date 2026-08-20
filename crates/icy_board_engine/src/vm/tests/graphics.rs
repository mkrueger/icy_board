use super::run_ppl;

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
fn reports_the_selected_backend_after_initialization() {
    let output = run_ppl(
        r"
        GfxInit 0
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
        GfxInit 0
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
    assert!(output.ends_with("\x1b[?1070h\x1b[?7h\x1b[?25h"));
}

#[test]
fn inline_graphics_preserve_the_text_screen_and_cursor() {
    let output = run_ppl(
        r#"
        PrintLn "before"
        GfxInit 0, FALSE
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

#[test]
fn tetris_initializes_and_renders_without_leaking_call_frames() {
    let source = include_str!("../../../../../ppe/tetris/src/tetris.pps").replace("running = TRUE", "running = FALSE");
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
    let mut source = include_str!("../../../../../ppe/tetris/src/tetris.pps").replace("running = TRUE", "running = FALSE");
    let mut completed_row = String::new();
    for column in 0..10 {
        completed_row.push_str(&format!("board[{column}, 19] = 1\n"));
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
