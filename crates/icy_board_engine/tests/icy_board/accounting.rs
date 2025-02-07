use icy_board_engine::icy_board::{accounting_cfg::AccountingConfig, PCBoardBinImporter};

#[test]
fn test_accounting_load() {
    let bytes = include_bytes!("accounting_test.cfg");
    let cfg = AccountingConfig::import_data(bytes).unwrap();

    assert_eq!(1.0, cfg.new_user_balance);
    assert_eq!(2.0, cfg.warn_level);
    assert_eq!(3.0, cfg.charge_per_logon);
    assert_eq!(4.0, cfg.charge_per_time);
    assert_eq!(5.0, cfg.charge_per_peak_time);
    assert_eq!(6.0, cfg.charge_per_group_chat_time);
    assert_eq!(7.0, cfg.charge_per_msg_read);
    assert_eq!(8.0, cfg.charge_per_msg_read_captured);
    assert_eq!(9.0, cfg.charge_per_msg_written);
    assert_eq!(10.0, cfg.charge_per_msg_write_echoed);
    assert_eq!(11.0, cfg.charge_per_msg_write_private);
    assert_eq!(12.0, cfg.charge_per_download_file);
    assert_eq!(13.0, cfg.charge_per_download_bytes);
    assert_eq!(14.0, cfg.pay_back_for_upload_file);
    assert_eq!(15.0, cfg.pay_back_for_upload_bytes);
}

#[test]
fn test_accounting_roundtrip() {
    let bytes = include_bytes!("accounting_test.cfg");
    let cfg = AccountingConfig::import_data(bytes).unwrap();
    let export = cfg.export_pcboard();

    assert_eq!(bytes, &export[..]);
}
