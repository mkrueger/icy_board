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
