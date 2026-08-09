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

**38 of 125 options and 21 of 29 sysop security levels do nothing today.**

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

Twenty of twenty-one are used, both in the new user questionnaire and in the
matching questions of the `W` command.

| Option | Status | Note |
|---|---|---|
| `auto_register_conferences` | ✅ | new user, registers in every public conference without a security requirement |
| `new_user_groups` | ❌ | a new user is never put into the group named here |

## message

| Option | Status | Note |
|---|---|---|
| `disable_message_scan_prompt` | ✅ | join |
| `default_scan_all_selected_confs_at_login` | ✅ | join |
| `prompt_to_read_mail` | ✅ | logon mail scan |
| `force_comments_to_main` | ✅ | `C` |
| `update_last_read_pointer` | ✅ | message reader |
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
| `confirm_caller_name` | ✅ | login |
| `reread_sec_level_on_join` | ✅ | join, when the conference changes the level |
| `disable_ns_logon` | ✅ | login |
| `allow_alias_change` | 📥 | `W` lets the alias be changed regardless |
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

All seven are used: `give_user_password_to_doors`, `call_log`, `page_bell`,
`alarm`, `log_caller_number`, `log_connect_string`, `log_security_level`.
`call_log` switches the caller log on, the three `log_` options decide what
goes into it beyond the plain logon line.

## event

| Option | Status | Note |
|---|---|---|
| `enabled` | ✅ | |
| `event_file` | ✅ | the event list, `events.toml` |
| `suspend_minutes` | ✅ | turns callers away and caps the session time |
| `disallow_uploads` | ✅ | |
| `minutes_uploads_disallowed` | ✅ | |

The scheduler lives in `crates/icboard/src/event_scheduler.rs`, the event list
in `crates/icy_board_engine/src/icy_board/events.rs`. PCBoard's binary
`EVENT.DAT` is not read; the per-node and expedited/fido/mail event modes have
no equivalent.

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

## The other direction: PCBoard options with no home here

`PCBOARD.DAT` has 194 top level entries in our model of it. The importer reads
87 into `icboard.toml`; the remaining 107 have nowhere to go. Most of that is
right — half of `PCBOARD.DAT` describes a UART, a swap file or an OS/2 thread
priority — but not all of it.

### Obsolete by construction, do not port

The machine underneath is gone, so the setting has no meaning: `eliminate_snow`,
`disable_ctsdrop`, `no16550`, `force16550_a`, `os2_driver`, `monitor_modem`,
`auto_reset`, `verify_cdloss`, `no_carrier_exit`, `parallel_port_num`,
`upload_buf_size`, `env_size`, `swap`, `swap_during_bat`, `slow_drives`,
`slow_drive_bat`, `exit_to_dos`, `allow_shell`, `disable_password`, `slaves`,
`fast_text`, `fast_cnames`, `network`, `node_num`, `net_timeout`, `net_copy`,
`float_node_number`, `low_baud_sec_override`, `max_scroll_back`,
`view_batch`/`view_ext` (unarc-rs reads the archives directly),
`auto_make_msgs` (a JAM base creates itself), `encrypt` (passwords are hashed),
`user_sys_during_bat`, the seven `minimize_*` and the six `priority_*`.

That is 47 of the 107, and the honest answer for all of them is no.

### Ported since this audit was written

`event_active`, `event_time`, `event_slide` — when the nightly event runs and
whether it waits for a caller to hang up. They become `event.enabled` and the
single daily entry the importer writes to `events.toml`.

`auto_reg_conf`, `force_main`, `conf_pwrd_adjust`, `confirm_caller`,
`disable_quick`, `last_read_update`, `log_caller_number`, `log_connect_str`,
`log_sec_level` — imported, written back out by `icbsetup export`, editable in
ICBSetup and read at runtime. `conf_pwrd_adjust` turned out to be about PWRD,
the security level file, and not about the conference password: a conference
may raise or lower the level of the caller, and the option decides whether the
limits of the new level are applied. `disable_quick` is PCBoard's switch for
the `NS` token stacked onto the logon prompt, so it landed on the existing
`system_control.disable_ns_logon`.

`qwk_file`, `cap_file`, `max_total_msgs`, `max_conf_msgs` — `cap_file` no
longer names a capture file in PCBoard 15, `getqwkroot()` only falls back to it
when `qwk_file` is empty, so both feed `qwk_settings.bbs_id`.

### No counterpart here

| PCBoard option | Why not |
|---|---|
| `stop_clock_on_cap` | there is no session clock to stop; a session is granted a time limit and nothing is charged against it per transfer |
| `chat_delay` | group chat is pushed over a channel, no node polls for it |
| `pub_conf` | PCBoard 15 dropped the 40 character mask, the flag lives in the conference record and `Conference::is_public` already carries it. The exporter rebuilds the mask for older readers |

### Worth porting

Nothing outside the FidoNet block is left on this list.

### The FidoNet block — 25 options, newly relevant

PCBoard grew a whole FidoNet configuration; icy_board's `ftn.toml` has six
keys. Now that the tosser exists, several of these describe decisions the
tosser is currently making on its own with no way to change them:

`fido_check_dupe_msg_id`, `fido_check_dupe_path`, `fido_num_msgs_to_track`
(the duplicate window — currently every MSGID ever seen is kept),
`fido_secure` (where mail for an unknown user goes), `fido_auto_add`
(create an area for an unknown tag instead of counting it),
`fido_sysop_change`, `fido_make_response`, `fido_crash_sec`,
`fido_enable_area_fix`, `fido_enable_pass_thru`, `fido_route_echo_mail`,
`fido_re_address`, `fido_enable_routing`, `fido_import_after_xfer`,
`fido_pkt_freq`/`fido_export_freq`/`fido_mail_freq` (the scheduler that does
not exist), `fido_default_zone`/`fido_default_net`, `fido_log_level`,
`fido_process_in`/`_out`/`_orphan`, `fido_dial_out`, `fido_create_msg`.

Not all of these are worth having, but the duplicate window, the unknown-area
policy and the unknown-user policy are decisions the code makes today by
accident rather than by configuration.

### The UUCP block — 22 options, a whole missing feature

`uucp_*`, `organization`, `comp_bat_file`, `de_comp_bat_file`. This is
PCBoard's Usenet and internet mail gateway: newsgroups appear as conferences,
mail is exchanged over UUCP. Porting the options makes no sense; porting the
*feature* over NNTP and SMTP instead of UUCP might, and that is a roadmap
question rather than a configuration one.

### Options with a home the importer does not fill

These are worse than missing, because a converted board silently loses a
setting the sysop made:

| PCBoard option | icy_board field | State |
|---|---|---|
| `account_track` | `accounting.tracking_file` | left empty |
| `num_areas` | — | conferences are counted, areas are not |

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
   `switches.default_graphics_at_login`.
3. **Mark it.** Where the feature is planned but distant — the peak-rate half
   of `accounting` — ICBSetup should say so rather than presenting a
   live-looking toggle.

## Verification

This audit is a grep over field reads. It answers "does anything read this",
not "does it do the right thing". An option marked ✅ can still diverge from
PCBoard, and those divergences are not tracked here.

Re-run it with:

```sh
grep -rn "\.<field>\b" --include=*.rs crates/ \
  | grep -v icb_config.rs | grep -v /icbsetup/ | grep -v pcboard_data.rs
```
