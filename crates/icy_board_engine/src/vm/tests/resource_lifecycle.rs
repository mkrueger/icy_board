//! Serialized resource-only PPEs sharing the production session and nesting boundary.

use super::{compile, run_ppl_at_boundary_with_files_and_input};
use crate::icy_board::state::IcyBoardState;

const TONE: &[u8] = b"RIFFxxxxWAVEfmt ";
const AUDIO_CAPABILITIES: &[u8] = b"\x1b[=7;100;1n\x1b[=7;101;1;2;1n";
const FULLSCREEN_OFF: &str = "\x1b[?1070h\x1b[?80h\x1b[?7h\x1b[?25h";
const FLUSH_0: &str = "\x1b_SyncTERM:A;Flush;C=2;O=0\x1b\\";
const FLUSH_1: &str = "\x1b_SyncTERM:A;Flush;C=3;O=0\x1b\\";

#[derive(Debug, PartialEq)]
struct MediaAfterPpe {
    graphics: bool,
    audio: usize,
    playing: bool,
    volumes: [i32; 14],
    gfx_error: i32,
    keyboard: bool,
    mouse: bool,
    update_depth: usize,
    margins: bool,
}

impl MediaAfterPpe {
    fn capture(state: &mut IcyBoardState) -> Self {
        Self {
            graphics: state.ppl_graphics.is_some(),
            audio: (0..14).filter(|&channel| state.ppl_audio_file(channel).is_some()).count(),
            playing: state.sound_active.iter().any(|&active| active),
            volumes: state.sound_volume,
            gfx_error: state.gfx_error,
            keyboard: state.ppl_keys.is_enabled(),
            mouse: state.ppl_mouse.is_enabled(),
            update_depth: state.ppl_terminal.take_update_depth(),
            margins: state.ppl_terminal.take_margins_changed(),
        }
    }

    fn assert_clean(&self) {
        assert_eq!(
            *self,
            Self {
                graphics: false,
                audio: 0,
                playing: false,
                volumes: [100; 14],
                gfx_error: -1,
                keyboard: false,
                mouse: false,
                update_depth: 0,
                margins: false,
            }
        );
    }
}

fn child_ppe(source: &str) -> Vec<u8> {
    let executable = compile(source);
    assert!(executable.user_types.is_empty(), "resource fixtures must not introduce record layouts");
    executable.to_buffer().expect("resource-only PPE must retain its existing encoding")
}

#[test]
fn r1_nested_ppe_restart_rejects_parent_aliases_after_child_slot_reuse() {
    for shutdown in ["", "Terminal.Gfx.Shutdown()"] {
        let child = child_ppe(&format!(
            r#"
            {shutdown}
            Terminal.Gfx.Init(GfxBackend.Sixel, TRUE)
            SURFACE replacement = Surface.New(3, 4)
            replacement.Clear(Rgb(10, 20, 30))
            AUDIO replacementTone = Audio.Load("tone.wav")
            replacementTone.Play(TRUE)
            PRINTLN "child:", replacement.Valid, replacement.Width = 3, replacement.Height = 4, ":", replacementTone.Channel
            replacement.Present()
            PRINTLN "child-done"
            "#
        ));
        let (kept, output, media) = run_ppl_at_boundary_with_files_and_input(
            r#"
            Terminal.Gfx.Init(GfxBackend.Sixel, TRUE)
            SURFACE original = Surface.New(2, 2)
            SURFACE surfaceAlias = original
            original.Clear(Rgb(1, 2, 3))
            AUDIO originalTone = Audio.Load("tone.wav")
            AUDIO toneAlias = originalTone
            originalTone.Play(TRUE)
            originalTone.Free()
            CALL PPEPATH() + "child.ppe"
            PRINTLN "stale:", surfaceAlias.Valid, original.Valid, toneAlias.Valid, toneAlias.Playing
            PRINTLN "surface:", surfaceAlias.Clear(0), surfaceAlias.SetPixel(0, 0, 0), surfaceAlias.Present(), surfaceAlias.Free(), original.Free()
            PRINTLN "gfx-error:", Error.Last().Kind = ErrKind.Gfx, Error.Last().Code = ErrCode.Invalid
            PRINTLN "audio:", toneAlias.Play(), toneAlias.Stop(), toneAlias.SetVolume(17), toneAlias.Fade(0, 250), toneAlias.Free()
            PRINTLN "audio-error:", Error.Last().Kind = ErrKind.Audio, Error.Last().Code = ErrCode.Invalid
            AUDIO probe = Audio.Load("tone.wav")
            PRINTLN "occupied:", probe.Channel
            probe.Free()
            PRINTLN "outer-done"
            "#,
            &[("child.ppe", &child), ("tone.wav", TONE)],
            AUDIO_CAPABILITIES,
            MediaAfterPpe::capture,
        );
        assert!(kept, "{output:?}");
        assert!(output.contains("child:111:0\n"), "{output:?}");
        assert!(output.contains("stale:0000\n"), "{output:?}");
        assert!(output.contains("surface:00000\ngfx-error:11\n"), "{output:?}");
        assert!(output.contains("audio:00000\naudio-error:11\noccupied:1\n"), "{output:?}");
        let parent = output.split_once("child-done\n").unwrap().1.split_once("outer-done\n").unwrap().0;
        assert!(!parent.contains("\x1bP"), "stale surface emitted graphics: {parent:?}");
        assert!(!parent.contains(FLUSH_0), "stale audio stopped the child's channel: {parent:?}");
        assert!(!parent.contains("Volume;C=2;"), "stale audio changed the child's volume: {parent:?}");
        assert_eq!(output.matches("\x1bP").count(), 1, "only the child replacement may draw: {output:?}");
        assert_eq!(output.matches("Queue;C=2;S=2;L").count(), 2, "{output:?}");
        assert_eq!(output.matches(FLUSH_0).count(), 2, "{output:?}");
        assert!(
            output.split_once("outer-done\n").unwrap().1.starts_with(&format!("{FULLSCREEN_OFF}{FLUSH_0}")),
            "{output:?}"
        );
        media.assert_clean();
    }
}

#[test]
fn r1_nested_ppe_normal_return_and_free_preserve_parent_resources() {
    let child = child_ppe(
        r#"
        SURFACE temporary = Surface.New(3, 4)
        AUDIO temporaryTone = Audio.Load("tone.wav")
        temporaryTone.Play(TRUE)
        PRINTLN "child-channel:", temporaryTone.Channel
        BOOLEAN freedSurface = temporary.Free()
        BOOLEAN freedTone = temporaryTone.Free()
        PRINTLN "child-free:", freedSurface, freedTone, temporary.Valid, temporaryTone.Valid
        "#,
    );
    let (kept, output, media) = run_ppl_at_boundary_with_files_and_input(
        r#"
        Terminal.Gfx.Init(GfxBackend.Sixel, TRUE)
        SURFACE parent = Surface.New(2, 2)
        parent.Clear(Rgb(1, 2, 3))
        AUDIO parentTone = Audio.Load("tone.wav")
        parentTone.Play(TRUE)
        parentTone.SetVolume(37)
        CALL PPEPATH() + "child.ppe"
        PRINTLN "parent:", parent.Valid, parent.GetPixel(0, 0) = Rgb(1, 2, 3), parentTone.Valid, parentTone.Playing
        parent.Present()
        PRINTLN "outer-done"
        "#,
        &[("child.ppe", &child), ("tone.wav", TONE)],
        AUDIO_CAPABILITIES,
        MediaAfterPpe::capture,
    );
    assert!(kept, "{output:?}");
    assert!(output.contains("child-channel:1\n"), "{output:?}");
    assert!(output.contains("child-free:1100\n"), "{output:?}");
    assert!(output.contains("parent:1111\n"), "{output:?}");
    let (body, cleanup) = output.split_once("outer-done\n").unwrap();
    assert!(body.contains("\x1bP"), "parent did not emit sixel: {body:?}");
    assert!(!body.contains(FULLSCREEN_OFF), "child cleaned the parent's graphics: {body:?}");
    assert!(!body.contains(FLUSH_0), "child cleaned the parent's audio: {body:?}");
    assert_eq!(output.matches(FLUSH_1).count(), 1, "explicit child free must not be repeated: {output:?}");
    assert_eq!(output.matches(FLUSH_0).count(), 1, "{output:?}");
    assert!(cleanup.starts_with(&format!("{FULLSCREEN_OFF}{FLUSH_0}")), "{output:?}");
    media.assert_clean();
}

#[test]
fn r1_outer_ppe_completion_stop_and_vm_error_reset_media() {
    for (ending, expected_kept) in [("EXIT", true), ("STOP", false), ("INTEGER unused\nPOP unused", false)] {
        let source = format!(
            r#"
            Terminal.Gfx.Init(GfxBackend.Sixel, TRUE)
            SURFACE picture = Surface.New(2, 2)
            picture.Clear(Rgb(1, 2, 3))
            picture.Present()
            AUDIO tone = Audio.Load("tone.wav")
            tone.Play(TRUE)
            tone.SetVolume(37)
            Terminal.Input.KeyboardOn(TRUE)
            Terminal.BeginUpdate()
            Terminal.Margins.SetVertical(1, 20)
            PRINTLN "outer-ending"
            {ending}
            PRINTLN "unreachable"
            "#
        );
        let (kept, output, media) = run_ppl_at_boundary_with_files_and_input(&source, &[("tone.wav", TONE)], AUDIO_CAPABILITIES, MediaAfterPpe::capture);
        assert_eq!(kept, expected_kept, "{ending}: {output:?}");
        assert!(!output.contains("unreachable"), "{ending}: {output:?}");
        assert!(output.contains("\x1bP"), "surface was never presented: {output:?}");
        let cleanup = output.split_once("outer-ending\n").unwrap().1;
        assert!(
            cleanup.starts_with(&format!("\x1b[=2l\x1b[=1l\x1b[?2026l\x1b[r\x1b[?69l{FULLSCREEN_OFF}{FLUSH_0}")),
            "{ending}: {cleanup:?}"
        );
        assert_eq!(output.matches(FLUSH_0).count(), 1, "{ending}: {output:?}");
        assert_eq!(output.matches(FULLSCREEN_OFF).count(), 1, "{ending}: {output:?}");
        media.assert_clean();
    }
}
