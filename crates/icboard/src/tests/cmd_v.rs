use crate::tests::{setup_conference_with_messages, test_output};

#[test]
fn view_settings_shows_the_whole_pcboard_block() {
    let output = test_output("V\n".to_string(), setup_conference_with_messages);

    for line in ["Lst Date On", "Expire Date", "Page Length", "Security Lv", "Tr/Protocol"] {
        assert!(output.contains(line), "{line} is missing:\n{output}");
    }
}

/// PCBoard prints the line either way and says "None" when nothing expires.
#[test]
fn view_settings_reports_no_expiration_date() {
    let output = test_output("V\n".to_string(), setup_conference_with_messages);
    let line = output.lines().find(|line| line.contains("Expire Date")).unwrap_or_default();

    assert!(line.contains("None"), "an absent expiration was not reported:\n{output}");
}

#[test]
fn view_settings_shows_the_download_allowance() {
    let output = test_output("V\n".to_string(), setup_conference_with_messages);

    assert!(output.contains("Bytes Avail"), "the byte allowance is missing:\n{output}");
}

#[test]
fn view_settings_shows_the_message_base_stats() {
    let output = test_output("V\n".to_string(), setup_conference_with_messages);

    for line in ["L/Msg. Read", "High Msg. #", "Active Msgs"] {
        assert!(output.contains(line), "{line} is missing:\n{output}");
    }
    let active = output.lines().find(|line| line.contains("Active Msgs")).unwrap_or_default();
    assert!(active.contains('3'), "the three messages of the area were not counted:\n{output}");
}

/// A deleted message leaves a gap, so the count and the highest number part ways.
#[test]
fn view_settings_counts_active_messages_apart_from_the_high_number() {
    let output = test_output("V\n".to_string(), |board| {
        setup_conference_with_messages(board);
        let path = board.conferences[0].areas.as_ref().unwrap()[0].path.clone();
        let mut base = jamjam::jam::JamMessageBase::open(path).unwrap();
        base.delete_message(2).unwrap();
    });

    let high = output.lines().find(|line| line.contains("High Msg. #")).unwrap_or_default();
    assert!(high.contains('3'), "the highest number should survive the deletion:\n{output}");
    let active = output.lines().find(|line| line.contains("Active Msgs")).unwrap_or_default();
    assert!(active.contains('2'), "the deleted message should not be counted:\n{output}");
}

#[test]
fn view_settings_shows_the_ratios_when_the_level_sets_them() {
    let output = test_output("V\n".to_string(), |board| {
        setup_conference_with_messages(board);
        board.config.subscription_info.is_enabled = false;
    });

    // Nothing constrains the test level, so neither ratio line is printed.
    assert!(!output.contains("Byte Ratio"), "a ratio was shown although none is set:\n{output}");
    assert!(!output.contains("File Ratio"), "a ratio was shown although none is set:\n{output}");
}
