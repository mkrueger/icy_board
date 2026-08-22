use super::{run_ppl_with_files_and_input, run_ppl_with_input};

const TONE: &[u8] = b"RIFFxxxxWAVEfmt ";

#[test]
fn sound_capabilities_are_queried_and_cached() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO first = LoadAudio("tone.wav")
        AUDIO second = LoadAudio("tone.wav")
        PrintLn first.Valid, second.Valid
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );

    assert!(output.ends_with("11\n"), "{output:?}");
    assert_eq!(output.matches("SyncTERM:Q;libsndfile\x1b\\").count(), 1, "{output:?}");
    assert_eq!(output.matches("SyncTERM:Q;libsndfileFormat;1;0").count(), 1, "{output:?}");
}

#[test]
fn the_first_sound_takes_the_lowest_apc_channel() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO tone = LoadAudio("tone.wav")
        tone.Play()
        PrintLn tone.Channel, ":", tone.Playing
        tone.Stop()
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );

    assert!(output.contains("SyncTERM:Q;libsndfileFormat;1;0"), "{output:?}");
    assert!(output.contains("Load;S=2;"), "{output:?}");
    assert!(output.contains("Queue;C=2;S=2"), "{output:?}");
    assert!(output.contains("0:1\n"), "{output:?}");
    assert!(output.contains("Flush;C=2;O=0"), "{output:?}");
}

#[test]
fn a_sound_object_carries_its_own_channel() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO music = LoadAudio("tone.wav")
        AUDIO effect = LoadAudio("tone.wav")
        PRINTLN music.Valid, ":", music.Channel, ":", effect.Channel
        music.SetVolume(70)
        music.Play(TRUE)
        PRINTLN music.Playing, ":", music.Volume
        music.Fade(0, 250)
        music.Stop()
        PRINTLN music.Playing
        music.Free()
        PRINTLN music.Valid
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );

    // One upload is enough, because both objects name the same cached file.
    assert_eq!(output.matches("SyncTERM:C;S;").count(), 1, "{output:?}");
    assert!(output.contains("1:0:1\n"), "{output:?}");
    assert!(output.contains("Queue;C=2;S=2;L"), "{output:?}");
    assert!(output.contains("1:70\n"), "{output:?}");
    assert!(output.contains("T=250"), "{output:?}");
    assert_eq!(output.matches("Flush;C=2;O=0").count(), 2, "{output:?}");
    assert!(output.ends_with("0\n"), "{output:?}");
}

#[test]
fn a_freed_sound_gives_its_channel_back() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO first = LoadAudio("tone.wav")
        first.Free()
        AUDIO second = LoadAudio("tone.wav")
        PRINTLN second.Channel
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );

    assert!(output.ends_with("0\n"), "{output:?}");
}

#[test]
fn an_unavailable_terminal_is_not_sent_a_sound() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO tone = LoadAudio("tone.wav")
        PrintLn tone.Valid, ":", tone.Error
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;0n",
    );

    assert!(!output.contains("SyncTERM:C;S;"), "{output:?}");
    assert!(output.ends_with("0:1\n"), "{output:?}");
}

#[test]
fn sound_failures_are_reported_by_the_audio_object() {
    let unsupported = run_ppl_with_files_and_input(
        r#"
        AUDIO tone = LoadAudio("tone.wav")
        PrintLn tone.Error
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;0n",
    );
    assert!(unsupported.ends_with("4\n"), "{unsupported:?}");

    let missing = run_ppl_with_input(
        r#"
        AUDIO tone = LoadAudio("nope.wav")
        PrintLn tone.Error
        "#,
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );
    assert!(missing.ends_with("3\n"), "{missing:?}");
}

#[test]
fn a_sound_that_cannot_be_loaded_stays_callable() {
    let output = run_ppl_with_input(
        r#"
        AUDIO missing = LoadAudio("nope.wav")
        PRINTLN missing.Valid, ":", missing.Playing
        PRINTLN missing.Play(FALSE)
        PRINTLN missing.Error
        "#,
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );

    assert!(output.contains("0:0\n0\n"), "{output:?}");
    assert!(output.ends_with("3\n"), "{output:?}");
}

#[test]
fn a_second_format_is_probed_and_uploaded_after_the_first() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO music = LoadAudio("music.ogg")
        AUDIO effect = LoadAudio("rotate.wav")
        PrintLn music.Valid, effect.Valid
        "#,
        &[("music.ogg", b"OggS\x00\x02\x01\x13OpusHead"), ("rotate.wav", TONE)],
        b"\x1b[=7;100;1n\x1b[=7;101;32;100;1n\x1b[=7;101;1;0;1n",
    );

    assert!(output.contains("SyncTERM:Q;libsndfileFormat;32;100"), "{output:?}");
    assert!(output.contains("SyncTERM:Q;libsndfileFormat;1;0"), "{output:?}");
    assert_eq!(output.matches("SyncTERM:C;S;").count(), 2, "{output:?}");
}

#[test]
fn a_format_probe_that_goes_unanswered_does_not_mute_the_channel() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO tone = LoadAudio("tone.wav")
        tone.Play()
        PrintLn tone.Error
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;1n",
    );

    assert!(output.contains("SyncTERM:C;S;"), "{output:?}");
    assert!(output.contains("Queue;C=2;S=2"), "{output:?}");
    assert!(output.ends_with("0\n"), "{output:?}");
}

#[test]
fn a_sound_the_caller_already_cached_is_not_sent_again() {
    let mut terminal = b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n\x1b[6;16;8t\x1b[=1;1-n\x1b_SyncTERM:C;L\n".to_vec();
    terminal.extend_from_slice(snd_cache_name(TONE, "wav").as_bytes());
    terminal.extend_from_slice(b"\td41d8cd98f00b204e9800998ecf8427e\n\x1b\\");

    let output = run_ppl_with_files_and_input(
        r#"
        PrintLn GfxCaps()
        AUDIO tone = LoadAudio("tone.wav")
        tone.Play()
        "#,
        &[("tone.wav", TONE)],
        &terminal,
    );

    assert!(!output.contains("SyncTERM:C;S;"), "{output:?}");
    assert!(output.contains("Queue;C=2;S=2"), "{output:?}");
}

#[test]
fn a_finished_sound_arrives_as_an_event() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO tone = LoadAudio("tone.wav")
        tone.Play()
        EVENT e = EventWait(-1)
        PRINTLN e.Kind, ":", e.Code, ":", tone.Playing
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n\x1b[=7;2;0n",
    );

    assert!(output.contains("Update;C=2"), "{output:?}");
    assert!(output.ends_with("5:0:0\n"), "{output:?}");
}

#[test]
fn a_looping_sound_is_not_watched_for_its_end() {
    let output = run_ppl_with_files_and_input(
        r#"
        AUDIO tone = LoadAudio("tone.wav")
        tone.Play(TRUE)
        "#,
        &[("tone.wav", TONE)],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );

    assert!(output.contains("Queue;C=2;S=2;L"), "{output:?}");
    assert!(!output.contains("Update;C=2"), "{output:?}");
}

/// The name a sound lands under, which is the hash of the file it was read from.
fn snd_cache_name(data: &[u8], extension: &str) -> String {
    use sha2::{Digest, Sha256};

    format!("snd/{}.{extension}", &format!("{:x}", Sha256::digest(data))[..32])
}
