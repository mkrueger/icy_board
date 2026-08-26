//! Runtime 4.00 shipped only the object API; the earlier flat draft is not source syntax.

use super::compile_errors;

#[test]
fn flat_terminal_statements_are_retired() {
    for source in [
        "GfxInit 0",
        "GfxShutdown",
        "GfxSetPacing 1",
        "SetVMargins 1, 25",
        "SetHMargins 1, 80",
        "ResetVMargins",
        "ResetHMargins",
        "ResetMargins",
        "SetFont 0, 5",
        "LoadFont 43, \"font.psf\"",
        "SetPaletteColor 1, 0",
        "ResetPaletteColor 1",
        "ResetPalette",
        "BeginTerminalUpdate",
        "EndTerminalUpdate",
        "RecordMacro 0",
        "EndMacro",
        "PlayMacro 0",
        "DeleteMacro 0",
        "ClearMacros",
    ] {
        assert!(!compile_errors(source).is_empty(), "retired statement still compiles: {source}");
    }
}

#[test]
fn flat_terminal_functions_are_retired() {
    for source in [
        "PRINTLN GfxBackend()",
        "PRINTLN GfxCaps()",
        "PRINTLN GfxCellWidth()",
        "PRINTLN GfxCellHeight()",
        "PRINTLN GfxScreenWidth()",
        "PRINTLN GfxScreenHeight()",
        "SURFACE s = NewSurface(1, 1)",
        "SURFACE s = LoadSurface(\"image.png\")",
        "AUDIO a = LoadAudio(\"sound.wav\")",
        "TERMINFO i = TermInfo()",
        "TERMINPUT i = TermInput()",
        "TERMSTATE s = TermState()",
    ] {
        assert!(!compile_errors(source).is_empty(), "retired function still compiles: {source}");
    }
}

#[test]
fn flat_terminal_constants_are_retired() {
    for name in [
        "GFX_AUTO",
        "GFX_CAP_SIXEL",
        "MOUSE_TEXT",
        "MOUSE_PRESS",
        "MOUSE_LEFT",
        "MOUSE_TRACK_ALL",
        "EVENT_KEY",
        "EVENT_CTRL",
        "MOUSE_BUTTON_LEFT",
        "ERR_INVALID",
        "ERR_KIND_TERM",
        "FONT_ALL",
    ] {
        let source = format!("PRINTLN {name}");
        assert!(!compile_errors(&source).is_empty(), "retired constant still compiles: {name}");
    }
}
