use super::{run_ppl_with_files_and_input, run_ppl_with_input};

#[test]
fn sound_capabilities_are_queried_and_cached() {
    let output = run_ppl_with_input(
        r"
        PrintLn SndAvailable()
        PrintLn SndAvailable()
        PrintLn SndSupports(SND_FMT_WAV)
        PrintLn SndSupports(SND_FMT_WAV)
        ",
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );

    assert_eq!(output.matches("1\n").count(), 4, "{output:?}");
    assert!(!output.contains("0\n"), "{output:?}");
    assert_eq!(output.matches("SyncTERM:Q;libsndfile\x1b\\").count(), 1, "{output:?}");
    assert_eq!(output.matches("SyncTERM:Q;libsndfileFormat;1;0").count(), 1, "{output:?}");
}

#[test]
fn logical_sound_channel_zero_maps_to_apc_channel_two() {
    let output = run_ppl_with_files_and_input(
        r#"
        SndPlay 0, "tone.wav"
        PrintLn SndPlaying(0)
        SndFade 0, 50, 250
        SndStopAll
        "#,
        &[("tone.wav", b"RIFFxxxxWAVEfmt ")],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n\x1b[=7;2;1n",
    );

    assert!(output.contains("Load;S=2;"), "{output:?}");
    assert!(output.contains("Queue;C=2;S=2"), "{output:?}");
    assert!(output.contains("Volume;C=2;"), "{output:?}");
    assert!(output.contains("T=250"), "{output:?}");
    assert!(output.contains("Flush;C=2;O=0"), "{output:?}");
    assert!(output.contains("1\n"), "{output:?}");
}

#[test]
fn unsupported_sound_is_not_uploaded() {
    let output = run_ppl_with_files_and_input(r#"SndPlay 0, "tone.wav""#, &[("tone.wav", b"RIFFxxxxWAVEfmt ")], b"\x1b[=7;100;0n");

    assert!(!output.contains("SyncTERM:C;S;"), "{output:?}");
}

#[test]
fn an_invalid_channel_is_reported_instead_of_redirected() {
    let output = super::run_ppl(
        r"
        SndStop 14
        PrintLn SndError()
        PrintLn SndPlaying(-1)
        PrintLn SndError()
        ",
    );

    assert_eq!(output, "2\n0\n2\n");
}

#[test]
fn sound_failures_are_reported_through_snderror() {
    let unavailable = run_ppl_with_files_and_input(
        r#"
        SndPlay 0, "tone.wav"
        PrintLn SndError()
        "#,
        &[("tone.wav", b"RIFFxxxxWAVEfmt ")],
        b"\x1b[=7;100;0n",
    );
    assert!(unavailable.ends_with("1\n"), "{unavailable:?}");

    let unsupported = run_ppl_with_files_and_input(
        r#"
        SndPlay 0, "tone.wav"
        PrintLn SndError()
        "#,
        &[("tone.wav", b"RIFFxxxxWAVEfmt ")],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;0n",
    );
    assert!(unsupported.ends_with("4\n"), "{unsupported:?}");

    let missing = run_ppl_with_input(
        r#"
        SndPlay 0, "nope.wav"
        PrintLn SndError()
        "#,
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );
    assert!(missing.ends_with("3\n"), "{missing:?}");
}

#[test]
fn a_missing_channel_state_reply_does_not_report_stale_playback() {
    let output = run_ppl_with_files_and_input(
        r#"
        SndPlay 0, "tone.wav"
        PrintLn SndPlaying(0)
        "#,
        &[("tone.wav", b"RIFFxxxxWAVEfmt ")],
        b"\x1b[=7;100;1n\x1b[=7;101;1;0;1n",
    );

    assert!(output.ends_with("0\n"), "{output:?}");
}
