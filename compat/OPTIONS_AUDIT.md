# Configuration option audit: icy_board vs PCBoard 15.x

Which of the switches a sysop can set actually change what the board does.

A switch that is offered in ICBSetup, written to `icboard.toml` and then read
by nobody is worse than a missing feature: the sysop sets it, believes the
board behaves that way, and finds out otherwise from a user.

Derived mechanically on 2026-08-09 from
`crates/icy_board_engine/src/icy_board/icb_config.rs`: for every field of every
option struct, is there a read outside the config definition, ICBSetup and the
`PCBOARD.DAT` importer.

Legend:

- ✅ read at runtime and acted on
- ❌ stored and editable, but nothing reads it
- 📥 filled in by the `PCBOARD.DAT` importer, but nothing reads it afterwards

**43 of 117 options and 21 of 29 sysop security levels do nothing today.**

## board — general information

All ten are used: `name`, `allow_iemsi`, `location`, `operator`, `notice`,
`capabilities`, `date_format`, `num_nodes`, `who_include_city`,
`who_show_alias`.

## sysop

| Option | Status | Where |
|---|---|---|
| `name`, `password`, `require_password_to_exit`, `use_real_name` | ✅ | login, door drop files, sysop functions |
| `external_editor` | ✅ | ICBSetup only, which is where it belongs |
| `config_color_theme` | ❌ | the TUI theme is not chosen from it |

## new_user_settings

Nineteen of twenty are used, both in the new user questionnaire and in the
matching questions of the `W` command.

| Option | Status | Note |
|---|---|---|
| `new_user_groups` | ❌ | a new user is never put into the group named here |

## message

| Option | Status | Note |
|---|---|---|
| `disable_message_scan_prompt` | ✅ | join |
| `default_scan_all_selected_confs_at_login` | ✅ | join |
| `prompt_to_read_mail` | ✅ | logon mail scan |
| `max_msg_lines` | 📥 | the editor has its own limit |
| `allow_esc_codes` | 📥 | ESC is filtered or not without asking this |
| `scan_all_mail_at_login` | ❌ | |
| `allow_carbon_copy` | ❌ | `E` never offers a carbon copy |
| `validate_to_name` | ❌ | a message to a name nobody carries is accepted |
| `default_quick_personal_scan` | ❌ | |

## file_transfer

Eight of nine do nothing. This is the worst section, and the one a sysop is
most likely to touch.

| Option | Status | Note |
|---|---|---|
| `display_uploader` | ✅ | file listing |
| `disallow_batch_uploads` | ❌ | `BU` is a stub anyway |
| `promote_to_batch_transfers` | ❌ | |
| `upload_credit_time` | ❌ | uploading earns neither time nor bytes |
| `upload_credit_bytes` | ❌ | |
| `verify_files_uploaded` | ❌ | uploads are never test-extracted |
| `upload_descr_lines` | ❌ | duplicated by `limits.max_number_upload_descr_lines`, both dead |
| `disable_drive_size_check` | ❌ | |
| `stop_uploads_free_space` | ❌ | the board uploads until the disk is full |

## system_control

| Option | Status | Note |
|---|---|---|
| `is_closed_board` | ✅ | login |
| `guard_logoff` | ✅ | `G` |
| `password_storage_method` | ✅ | |
| `allow_alias_change` | 📥 | `W` lets the alias be changed regardless |
| `disable_ns_logon` | ❌ | the `NS` token is always honoured |
| `disable_full_record_updating` | ❌ | `W` always asks everything |
| `is_multi_lingual` | ❌ | `LANG` works whether or not this is set |
| `enforce_daily_time_limit` | ❌ | only session limits exist |
| `allow_password_failure_comment` | ❌ | |

## switches

| Option | Status | Note |
|---|---|---|
| `non_graphics`, `exclude_local_calls_stats`, `display_news_behavior`, `disable_registration_edits`, `disable_high_ascii_filter`, `display_userinfo_at_login`, `force_intro_on_join`, `scan_new_blt` | ✅ | |
| `default_graphics_at_login` | ❌ | graphics mode is decided by the terminal handshake |
| `capture_grp_chat_session` | ❌ | group chat is never logged |
| `allow_handle_in_grpchat` | ❌ | group chat always uses the handle |

## limits

| Option | Status | Note |
|---|---|---|
| `min_pwd_length`, `password_expire_days`, `password_expire_warn_days`, `sysop_start`, `sysop_stop` | ✅ | |
| `keyboard_timeout` | ❌ | an idle user is never disconnected |
| `max_number_upload_descr_lines` | ❌ | |

`keyboard_timeout` is the one with a real consequence: without it a dropped
connection holds its node until the process is restarted.

## options

All four are used: `give_user_password_to_doors`, `call_log`, `page_bell`,
`alarm`.

## event

| Option | Status | Note |
|---|---|---|
| `enabled` | ✅ | but only as a flag; nothing schedules |
| `event_dat_path` | ❌ | the event file is never read |
| `suspend_minutes` | ❌ | |
| `disallow_uploads` | ❌ | |
| `minutes_uploads_disallowed` | ❌ | |

There is no event scheduler at all, so the whole section is decoration apart
from the flag.

## accounting

| Option | Status | Note |
|---|---|---|
| `enabled`, `cfg_file`, `tracking_file`, `warning_file`, `accounting_config` | ✅ | |
| `use_money` | ❌ | amounts are always shown as units |
| `concurrent_tracking` | ❌ | |
| `ignore_empty_sec_level` | ❌ | |
| `peak_usage_start`, `peak_usage_end`, `peak_days_of_week`, `peak_holiday_list_file` | ❌ | peak rates are never applied |
| `info_file`, `logoff_file` | ❌ | only the warning file is displayed |

## subs — subscription mode

| Option | Status | Note |
|---|---|---|
| `is_enabled`, `warning_days` | ✅ | |
| `subscription_length` | 📥 | a new subscription period is never set |
| `default_expired_level` | 📥 | an expired user keeps their level |

## qwk_settings

| Option | Status | Note |
|---|---|---|
| the five `bbs_*` fields, `welcome_screen`, `max_msgs`, `max_msgs_per_conf` | ✅ | |
| `goodbye_screen`, `news_sceen` | ❌ | not packed into the QWK archive |

`news_sceen` is also a typo in the key name; fixing it needs a migration.

## sysop_sec — sysop security levels

Eight of twenty-nine are checked. The rest are read by nobody, which means the
privilege they name is either always granted or the feature does not exist.

| Level | Status |
|---|---|
| `copy_move_messages`, `edit_any_message`, `use_broadcast_command`, `view_private_uploads`, `edit_message_headers`, `protect_unprotect_messages` | ✅ |
| `sec_4_recover_deleted_msg`, `sec_10_shelled_dos_func` | ✅ |
| `read_all_comments`, `read_all_mail` | ❌ always granted to whoever passes the sysop level |
| `enter_color_codes_in_messages`, `not_update_msg_read`, `enter_generic_messages`, `overwrite_files_on_uploads`, `set_pack_out_date_on_messages`, `see_all_return_receipts` | ❌ |
| `sec_1`, `sec_2`, `sec_3`, `sec_5`, `sec_6`, `sec_7`, `sec_8`, `sec_9`, `sec_11`, `sec_12`, `sec_13`, `sec_14` | ❌ the numeric command itself is missing, see COMMAND_AUDIT.md |
| `edit_own_messages` (in `user_sec`) | ❌ |

`user_sec` is otherwise complete: `security_for` maps every built-in command
to its level.

## What to do about it

Three answers are defensible per option, and each needs a deliberate choice:

1. **Implement it.** The ones worth it first, in the order a sysop notices
   them: `limits.keyboard_timeout`, `file_transfer.stop_uploads_free_space`,
   `file_transfer.upload_credit_time`/`upload_credit_bytes`,
   `message.validate_to_name`, `new_user_settings.new_user_groups`,
   `system_control.enforce_daily_time_limit`.
2. **Remove it.** An option that describes a DOS-era problem the port does not
   have should not be offered. Candidates:
   `file_transfer.disable_drive_size_check`,
   `switches.default_graphics_at_login`,
   `system_control.disable_ns_logon`.
3. **Mark it.** Where the feature is planned but distant — the whole `event`
   section, the peak-rate half of `accounting` — ICBSetup should say so rather
   than presenting a live-looking toggle.

## Verification

This audit is a grep over field reads. It answers "does anything read this",
not "does it do the right thing". An option marked ✅ can still diverge from
PCBoard, and those divergences are not tracked here.

Re-run it with:

```sh
grep -rn "\.<field>\b" --include=*.rs crates/ \
  | grep -v icb_config.rs | grep -v /icbsetup/ | grep -v pcboard_data.rs
```
