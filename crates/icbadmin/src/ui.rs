use crate::dto::*;

pub const STYLESHEET: &str = include_str!("style.css");

pub enum Notice {
    Success(String),
    Failure(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SectionId {
    General,
    Messages,
    FileTransfer,
    SystemControl,
    Switches,
    Limits,
    NewUser,
    Events,
    Subscription,
    Connections,
    Paths,
    Accounting,
    FunctionKeys,
}

impl SectionId {
    pub fn from_slug(slug: &str) -> Option<Self> {
        Some(match slug {
            "general" => Self::General,
            "messages" => Self::Messages,
            "file-transfer" => Self::FileTransfer,
            "system-control" => Self::SystemControl,
            "switches" => Self::Switches,
            "limits" => Self::Limits,
            "new-user" => Self::NewUser,
            "events" => Self::Events,
            "subscription" => Self::Subscription,
            "connections" => Self::Connections,
            "paths" => Self::Paths,
            "accounting" => Self::Accounting,
            "function-keys" => Self::FunctionKeys,
            _ => return None,
        })
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Messages => "messages",
            Self::FileTransfer => "file-transfer",
            Self::SystemControl => "system-control",
            Self::Switches => "switches",
            Self::Limits => "limits",
            Self::NewUser => "new-user",
            Self::Events => "events",
            Self::Subscription => "subscription",
            Self::Connections => "connections",
            Self::Paths => "paths",
            Self::Accounting => "accounting",
            Self::FunctionKeys => "function-keys",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::General => "Board & Sysop",
            Self::Messages => "Messages",
            Self::FileTransfer => "File Transfer",
            Self::SystemControl => "System Control",
            Self::Switches => "Configuration Switches",
            Self::Limits => "Limits",
            Self::NewUser => "New User Options",
            Self::Events => "Event Setup",
            Self::Subscription => "Subscription",
            Self::Connections => "Connection Information",
            Self::Paths => "File Locations",
            Self::Accounting => "Accounting Configuration",
            Self::FunctionKeys => "Function Keys",
        }
    }

    /// Grouped in the order icbsetup presents them, so both tools read the same way.
    pub fn groups() -> &'static [(&'static str, &'static [SectionId])] {
        &[
            ("Board", &[Self::General, Self::Paths, Self::Connections, Self::Events, Self::Subscription]),
            (
                "Configuration options",
                &[
                    Self::Messages,
                    Self::FileTransfer,
                    Self::SystemControl,
                    Self::Switches,
                    Self::Limits,
                    Self::FunctionKeys,
                ],
            ),
            ("Accounts", &[Self::Accounting, Self::NewUser]),
        ]
    }
}

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Sidebar shared by the settings pages and the conference pages.
fn nav_links(active: Option<SectionId>, conferences_active: bool) -> String {
    let mut links = String::from(r#"<a href="/" class="nav-home">Overview</a>"#);
    for (group, sections) in SectionId::groups() {
        links.push_str(&format!(r#"<span class="nav-group">{}</span>"#, escape(group)));
        for section in *sections {
            let class = if active == Some(*section) { " class=\"active\"" } else { "" };
            links.push_str(&format!(r#"<a href="/settings/{}"{}>{}</a>"#, section.slug(), class, escape(section.title())));
        }
    }
    links.push_str(r#"<span class="nav-group">Conferences</span>"#);
    let class = if conferences_active { " class=\"active\"" } else { "" };
    links.push_str(&format!(r#"<a href="/conferences"{class}>Conferences</a>"#));
    links
}

fn shell(title: &str, active: Option<SectionId>, body: &str) -> String {
    let nav = if active.is_some() || title == "Overview" {
        let links = nav_links(active, false);
        format!(
            r#"<div class="layout"><aside class="sidebar"><div class="brand"><span class="brand-mark">IB</span><div><strong>IcyBoard</strong><small>Web Admin</small></div></div><nav class="side-nav">{links}</nav><form class="logout" method="post" action="/logout"><button type="submit">Log out</button></form></aside><div class="content"><header class="topbar"><div><p class="eyebrow">Configuration</p><h1>{title}</h1></div><span class="badge">local</span></header><main>{body}</main><footer>icbadmin {version} · icbsetup and icbsm remain fully supported.</footer></div></div>"#,
            title = escape(title),
            version = env!("CARGO_PKG_VERSION"),
            links = links,
            body = body
        )
    } else {
        format!(
            r#"<div class="login-shell"><main class="login-card">{body}</main><footer>icbadmin {version}</footer></div>"#,
            version = env!("CARGO_PKG_VERSION"),
            body = body
        )
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · IcyBoard Admin</title><link rel="stylesheet" href="/style.css"></head>
<body>{nav}</body></html>"#,
        title = escape(title),
        nav = nav
    )
}

fn notice_html(notice: &Option<Notice>) -> String {
    match notice {
        Some(Notice::Success(msg)) => format!(r#"<div class="notice ok"><strong>Saved</strong><p>{}</p></div>"#, escape(msg)),
        Some(Notice::Failure(msg)) => format!(r#"<div class="notice err"><strong>Could not save</strong><p>{}</p></div>"#, escape(msg)),
        None => String::new(),
    }
}

pub fn login_page(notice: Option<Notice>) -> String {
    let body = format!(
        r#"{notice}<div class="login-head"><span class="brand-mark large">IB</span><h1>IcyBoard Admin</h1><p>Enter the access token printed when icbadmin started.</p></div>
<form method="post" action="/login" class="stack">
<label>Access token<input type="password" name="token" autocomplete="off" autofocus required></label>
<button type="submit" class="primary">Sign in</button></form>"#,
        notice = notice_html(&notice)
    );
    shell("Sign in", None, &body)
}

pub fn overview_page(overview: &OverviewDto) -> String {
    let mut body = String::new();

    if !overview.config_loaded {
        body.push_str(&format!(
            r#"<div class="notice err"><strong>Configuration could not be loaded</strong><p>{}</p></div>"#,
            escape(overview.load_error.as_deref().unwrap_or("unknown error"))
        ));
    }

    body.push_str(r#"<section class="hero-grid">"#);
    body.push_str(&stat_card("Board", overview.board_name.as_deref().unwrap_or("—")));
    body.push_str(&stat_card("Sysop", overview.sysop_name.as_deref().unwrap_or("—")));
    body.push_str(&stat_card("Nodes", &overview.num_nodes.map(|n| n.to_string()).unwrap_or_else(|| "—".into())));
    body.push_str(&stat_card("Version", &overview.tool_version));
    body.push_str("</section>");

    body.push_str("<section class=\"panel\"><h2>Installation</h2><table class=\"kv\">");
    body.push_str(&row("Configuration file", &overview.board_file));
    body.push_str(&row("Board directory", &overview.root_path));
    body.push_str("</table></section>");

    if let Some(counts) = &overview.counts {
        body.push_str("<section class=\"panel\"><h2>Contents</h2><div class=\"metric-grid\">");
        body.push_str(&metric("Conferences", counts.conferences));
        body.push_str(&metric("Users", counts.users));
        body.push_str(&metric("Security levels", counts.security_levels));
        body.push_str(&metric("Commands", counts.commands));
        body.push_str(&metric("Languages", counts.languages));
        body.push_str(&metric("Protocols", counts.protocols));
        body.push_str("</div></section>");
    }

    if let Some(stats) = &overview.statistics {
        body.push_str("<section class=\"panel\"><h2>Statistics</h2><table><thead><tr><th></th><th>Today</th><th>Total</th></tr></thead><tbody>");
        body.push_str(&stat_row("Calls", stats.today_calls, stats.total_calls));
        body.push_str(&stat_row("Messages", stats.today_messages, stats.total_messages));
        body.push_str(&stat_row("Uploads", stats.today_uploads, stats.total_uploads));
        body.push_str(&stat_row("Downloads", stats.today_downloads, stats.total_downloads));
        body.push_str("</tbody></table></section>");
    }

    if !overview.paths.is_empty() {
        body.push_str("<section class=\"panel\"><h2>Path checks</h2><table><thead><tr><th>Entry</th><th>Location</th><th>State</th></tr></thead><tbody>");
        for check in &overview.paths {
            let state = match (check.expected, check.exists) {
                (PathKind::Unset, _) => r#"<span class="state unset">not set</span>"#,
                (_, true) => r#"<span class="state ok">ok</span>"#,
                (_, false) => r#"<span class="state missing">missing</span>"#,
            };
            body.push_str(&format!(
                "<tr><td>{}</td><td class=\"path\">{}</td><td>{}</td></tr>",
                escape(&check.label),
                escape(&check.path),
                state
            ));
        }
        body.push_str("</tbody></table></section>");
    }

    if !overview.warnings.is_empty() {
        body.push_str("<section class=\"panel\"><h2>Warnings</h2><ul class=\"warnings\">");
        for warning in &overview.warnings {
            body.push_str(&format!("<li>{}</li>", escape(warning)));
        }
        body.push_str("</ul></section>");
    }

    body.push_str(
        r#"<section class="panel soft"><h2>Still use icbsetup for</h2>
<ul class="plain"><li>Conference create/edit and menus</li><li>Security expressions and command levels</li><li>User maintenance (icbsm)</li><li>Sysop password changes</li><li>QWK / FTN message networking details</li></ul></section>"#,
    );

    shell("Overview", None, &body)
}

fn settings_shell(section: SectionId, fingerprint: &str, csrf: &str, notice: Option<Notice>, intro: &str, fields: &str) -> String {
    let body = format!(
        r#"{notice}<section class="panel">
<p class="hint">{intro}</p>
<form method="post" action="/settings/{slug}" class="settings-form">
<input type="hidden" name="csrf" value="{csrf}">
<input type="hidden" name="fingerprint" value="{fingerprint}">
{fields}
<div class="form-actions"><button type="submit" class="primary">Save changes</button></div>
</form></section>"#,
        notice = notice_html(&notice),
        intro = escape(intro),
        slug = section.slug(),
        csrf = escape(csrf),
        fingerprint = escape(fingerprint),
        fields = fields,
    );
    shell(section.title(), Some(section), &body)
}

pub fn general_page(settings: &GeneralSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let mut date_options = String::new();
    for (value, label) in DATE_FORMATS {
        let selected = if *value == s.date_format { " selected" } else { "" };
        date_options.push_str(&format!(r#"<option value="{}"{}>{}</option>"#, escape(value), selected, escape(label)));
    }
    let fields = format!(
        r#"<fieldset><legend>Board identity</legend>
<div class="grid-2">
{board_name}{location}{operator}{notice_field}{capabilities}
<label>Date format<select name="date_format">{date_options}</select></label>
{num_nodes}
</div>
<div class="check-grid">{allow_iemsi}{who_include_city}{who_show_alias}</div>
</fieldset>
<fieldset><legend>Sysop</legend>
<div class="grid-2">{sysop_name}{sysop_external_editor}{sysop_config_color_theme}</div>
<div class="check-grid">{sysop_use_real_name}{sysop_require_password_to_exit}</div>
<p class="hint">Sysop password: <strong>{password_state}</strong>. Change it with icbsetup.</p>
</fieldset>
<fieldset><legend>Web administration</legend>
<div class="check-grid">{web_admin_enabled}{web_admin_allow_remote}</div>
<div class="grid-2">{web_admin_address}{web_admin_port}</div>
<p class="hint">Remote bind should only be enabled behind an authenticated TLS reverse proxy.</p>
</fieldset>"#,
        board_name = text_field("Board name", "board_name", &s.board_name, 45, true),
        location = text_field("Location (IEMSI)", "location", &s.location, 54, false),
        operator = text_field("Operator (IEMSI)", "operator", &s.operator, 30, false),
        notice_field = text_field("Notice (IEMSI)", "notice", &s.notice, 30, false),
        capabilities = text_field("Capabilities (IEMSI)", "capabilities", &s.capabilities, 30, false),
        date_options = date_options,
        num_nodes = number_field("Nodes", "num_nodes", s.num_nodes as u64, Some(1), Some(256)),
        allow_iemsi = checkbox("Allow IEMSI logins", "allow_iemsi", s.allow_iemsi),
        who_include_city = checkbox("WHO shows city", "who_include_city", s.who_include_city),
        who_show_alias = checkbox("WHO shows alias", "who_show_alias", s.who_show_alias),
        sysop_name = text_field("Sysop name", "sysop_name", &s.sysop_name, 30, true),
        sysop_external_editor = text_field("External editor", "sysop_external_editor", &s.sysop_external_editor, 128, false),
        sysop_config_color_theme = text_field("Config color theme", "sysop_config_color_theme", &s.sysop_config_color_theme, 64, false),
        sysop_use_real_name = checkbox("Use real name instead of SYSOP", "sysop_use_real_name", s.sysop_use_real_name),
        sysop_require_password_to_exit = checkbox(
            "Require local password to exit",
            "sysop_require_password_to_exit",
            s.sysop_require_password_to_exit
        ),
        password_state = if settings.sysop_password_set { "set" } else { "not set" },
        web_admin_enabled = checkbox("Enable web admin with IcyBoard", "web_admin_enabled", s.web_admin_enabled),
        web_admin_allow_remote = checkbox("Allow remote bind", "web_admin_allow_remote", s.web_admin_allow_remote),
        web_admin_address = text_field("Listen address", "web_admin_address", &s.web_admin_address, 64, true),
        web_admin_port = number_field("Port", "web_admin_port", s.web_admin_port as u64, Some(1), Some(65535)),
    );
    settings_shell(
        SectionId::General,
        &settings.fingerprint,
        csrf,
        notice,
        "Board identity, sysop display options and web admin listener settings.",
        &fields,
    )
}

pub fn message_page(settings: &MessageSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let fields = format!(
        r#"<fieldset><legend>Message options</legend>
<div class="grid-2">{max_msg_lines}</div>
<div class="check-grid">
{scan_all}{disable_scan}{allow_esc}{allow_cc}{validate_to}{quick}{scan_all_confs}{prompt_mail}{force_comments}{update_ptr}
</div></fieldset>"#,
        max_msg_lines = number_field("Max message lines", "max_msg_lines", s.max_msg_lines as u64, Some(1), Some(500)),
        scan_all = checkbox("Scan all mail at login", "scan_all_mail_at_login", s.scan_all_mail_at_login),
        disable_scan = checkbox("Disable message scan prompt", "disable_message_scan_prompt", s.disable_message_scan_prompt),
        allow_esc = checkbox("Allow ESC codes in messages", "allow_esc_codes", s.allow_esc_codes),
        allow_cc = checkbox("Allow carbon copy", "allow_carbon_copy", s.allow_carbon_copy),
        validate_to = checkbox("Validate TO name", "validate_to_name", s.validate_to_name),
        quick = checkbox("Default quick personal scan", "default_quick_personal_scan", s.default_quick_personal_scan),
        scan_all_confs = checkbox(
            "Scan all selected conferences at login",
            "default_scan_all_selected_confs_at_login",
            s.default_scan_all_selected_confs_at_login
        ),
        prompt_mail = checkbox("Prompt to read mail", "prompt_to_read_mail", s.prompt_to_read_mail),
        force_comments = checkbox("Force comments to main board", "force_comments_to_main", s.force_comments_to_main),
        update_ptr = checkbox("Update last-read pointer while reading", "update_last_read_pointer", s.update_last_read_pointer),
    );
    settings_shell(
        SectionId::Messages,
        &settings.fingerprint,
        csrf,
        notice,
        "Message scanning and composition behaviour.",
        &fields,
    )
}

pub fn file_transfer_page(settings: &FileTransferSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let fields = format!(
        r#"<fieldset><legend>Transfers</legend>
<div class="grid-2">
{credit_time}{credit_bytes}{descr_lines}{free_space}
</div>
<div class="check-grid">
{disallow_batch}{promote}{verify}{display_uploader}{disable_drive}
</div></fieldset>"#,
        credit_time = number_field("Upload credit time", "upload_credit_time", s.upload_credit_time as u64, None, None),
        credit_bytes = number_field("Upload credit bytes", "upload_credit_bytes", s.upload_credit_bytes as u64, None, None),
        descr_lines = number_field("Upload description lines", "upload_descr_lines", s.upload_descr_lines as u64, Some(1), Some(99)),
        free_space = number_field(
            "Stop uploads free space",
            "stop_uploads_free_space",
            s.stop_uploads_free_space as u64,
            None,
            None
        ),
        disallow_batch = checkbox("Disallow batch uploads", "disallow_batch_uploads", s.disallow_batch_uploads),
        promote = checkbox("Promote to batch transfers", "promote_to_batch_transfers", s.promote_to_batch_transfers),
        verify = checkbox("Verify uploaded files", "verify_files_uploaded", s.verify_files_uploaded),
        display_uploader = checkbox("Display uploader", "display_uploader", s.display_uploader),
        disable_drive = checkbox("Disable drive size check", "disable_drive_size_check", s.disable_drive_size_check),
    );
    settings_shell(
        SectionId::FileTransfer,
        &settings.fingerprint,
        csrf,
        notice,
        "Upload credits, batch behaviour and free-space guards.",
        &fields,
    )
}

pub fn system_control_page(settings: &SystemControlSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let mut options = String::new();
    for (value, label) in PASSWORD_STORAGE_METHODS {
        let selected = if *value == s.password_storage_method { " selected" } else { "" };
        options.push_str(&format!(r#"<option value="{}"{}>{}</option>"#, value, selected, escape(label)));
    }
    let fields = format!(
        r#"<fieldset><legend>System control</legend>
<label>Password storage method<select name="password_storage_method">{options}</select></label>
<div class="check-grid">
{disable_ns}{disable_full}{alias}{multi}{closed}{daily}{pw_fail}{guard}{confirm}{reread}
</div></fieldset>"#,
        options = options,
        disable_ns = checkbox("Disable NS logon", "disable_ns_logon", s.disable_ns_logon),
        disable_full = checkbox("Disable full record updating", "disable_full_record_updating", s.disable_full_record_updating),
        alias = checkbox("Allow alias change", "allow_alias_change", s.allow_alias_change),
        multi = checkbox("Multi-lingual board", "is_multi_lingual", s.is_multi_lingual),
        closed = checkbox("Closed board / NewAsk mode", "is_closed_board", s.is_closed_board),
        daily = checkbox("Enforce daily time limit", "enforce_daily_time_limit", s.enforce_daily_time_limit),
        pw_fail = checkbox(
            "Allow password failure comment",
            "allow_password_failure_comment",
            s.allow_password_failure_comment
        ),
        guard = checkbox("Guard logoff (G asks)", "guard_logoff", s.guard_logoff),
        confirm = checkbox("Confirm caller name", "confirm_caller_name", s.confirm_caller_name),
        reread = checkbox("Re-read security level on join", "reread_sec_level_on_join", s.reread_sec_level_on_join),
    );
    settings_shell(
        SectionId::SystemControl,
        &settings.fingerprint,
        csrf,
        notice,
        "Logon policy, closed-board mode and password storage.",
        &fields,
    )
}

pub fn switches_page(settings: &SwitchesSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let mut options = String::new();
    for (value, label) in DISPLAY_NEWS_BEHAVIORS {
        let selected = if *value == s.display_news_behavior { " selected" } else { "" };
        options.push_str(&format!(r#"<option value="{}"{}>{}</option>"#, value, selected, escape(label)));
    }
    let fields = format!(
        r#"<fieldset><legend>Display &amp; registration</legend>
<label>Display news behaviour<select name="display_news_behavior">{options}</select></label>
<div class="check-grid">
{gfx}{non_gfx}{exclude_local}{disable_reg}{disable_hi}{userinfo}{force_intro}{scan_blt}{capture_chat}{handle_chat}
</div></fieldset>
<fieldset><legend>Logging &amp; doors</legend>
<div class="check-grid">
{give_pw}{call_log}{page_bell}{alarm}{log_caller}{log_connect}{log_sec}
</div></fieldset>"#,
        options = options,
        gfx = checkbox("Default graphics at login", "default_graphics_at_login", s.default_graphics_at_login),
        non_gfx = checkbox("Non-graphics / disable colors", "non_graphics", s.non_graphics),
        exclude_local = checkbox("Exclude local calls from stats", "exclude_local_calls_stats", s.exclude_local_calls_stats),
        disable_reg = checkbox("Disable registration edits", "disable_registration_edits", s.disable_registration_edits),
        disable_hi = checkbox("Disable high ASCII filter", "disable_high_ascii_filter", s.disable_high_ascii_filter),
        userinfo = checkbox("Display user info at login", "display_userinfo_at_login", s.display_userinfo_at_login),
        force_intro = checkbox("Force intro on join", "force_intro_on_join", s.force_intro_on_join),
        scan_blt = checkbox("Scan new bulletins", "scan_new_blt", s.scan_new_blt),
        capture_chat = checkbox("Capture group chat session", "capture_grp_chat_session", s.capture_grp_chat_session),
        handle_chat = checkbox("Allow handle in group chat", "allow_handle_in_grpchat", s.allow_handle_in_grpchat),
        give_pw = checkbox("Give user password to doors", "give_user_password_to_doors", s.give_user_password_to_doors),
        call_log = checkbox("Caller log", "call_log", s.call_log),
        page_bell = checkbox("Page bell", "page_bell", s.page_bell),
        alarm = checkbox("Alarm", "alarm", s.alarm),
        log_caller = checkbox("Log caller number", "log_caller_number", s.log_caller_number),
        log_connect = checkbox("Log connect string", "log_connect_string", s.log_connect_string),
        log_sec = checkbox("Log security level", "log_security_level", s.log_security_level),
    );
    settings_shell(
        SectionId::Switches,
        &settings.fingerprint,
        csrf,
        notice,
        "Config switches and board logging options.",
        &fields,
    )
}

pub fn limits_page(settings: &LimitsSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let fields = format!(
        r#"<fieldset><legend>Limits</legend>
<div class="grid-2">
{kb}{upload_lines}{min_pwd}{expire}{warn}{start}{stop}
</div>
<p class="hint">Times use HH:MM or HH:MM:SS. Leave blank for none.</p>
</fieldset>"#,
        kb = number_field("Keyboard timeout", "keyboard_timeout", s.keyboard_timeout as u64, None, None),
        upload_lines = number_field(
            "Max upload description lines",
            "max_number_upload_descr_lines",
            s.max_number_upload_descr_lines as u64,
            None,
            Some(99)
        ),
        min_pwd = number_field("Minimum password length", "min_pwd_length", s.min_pwd_length as u64, None, Some(64)),
        expire = number_field("Password expire days", "password_expire_days", s.password_expire_days as u64, None, None),
        warn = number_field(
            "Password expire warning days",
            "password_expire_warn_days",
            s.password_expire_warn_days as u64,
            None,
            None
        ),
        start = text_field("Sysop page start", "sysop_start", &s.sysop_start, 8, false),
        stop = text_field("Sysop page stop", "sysop_stop", &s.sysop_stop, 8, false),
    );
    settings_shell(
        SectionId::Limits,
        &settings.fingerprint,
        csrf,
        notice,
        "Timeouts, password ageing and sysop page window.",
        &fields,
    )
}

pub fn new_user_page(settings: &NewUserSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let fields = format!(
        r#"<fieldset><legend>Registration defaults</legend>
<div class="grid-2">{sec}{groups}</div>
<div class="check-grid">
{one_name}{use_newask}{auto_reg}
{city}{address}{verification}{biz}{home}{comment}{clr}{xfer}{date}{fse}{alias}{gender}{birth}{email}{web}{short}
</div></fieldset>"#,
        sec = number_field("Security level", "sec_level", s.sec_level as u64, None, Some(255)),
        groups = text_field("New user groups", "new_user_groups", &s.new_user_groups, 128, false),
        one_name = checkbox("Allow one-name users", "allow_one_name_users", s.allow_one_name_users),
        use_newask = checkbox("Use NewAsk survey and built-in questions", "use_newask_and_builtin", s.use_newask_and_builtin),
        auto_reg = checkbox("Auto-register public conferences", "auto_register_conferences", s.auto_register_conferences),
        city = checkbox("Ask city/state", "ask_city_or_state", s.ask_city_or_state),
        address = checkbox("Ask address", "ask_address", s.ask_address),
        verification = checkbox("Ask verification", "ask_verification", s.ask_verification),
        biz = checkbox("Ask business phone", "ask_business_phone", s.ask_business_phone),
        home = checkbox("Ask home phone", "ask_home_phone", s.ask_home_phone),
        comment = checkbox("Ask comment", "ask_comment", s.ask_comment),
        clr = checkbox("Ask clear message", "ask_clr_msg", s.ask_clr_msg),
        xfer = checkbox("Ask transfer protocol", "ask_xfer_protocol", s.ask_xfer_protocol),
        date = checkbox("Ask date format", "ask_date_format", s.ask_date_format),
        fse = checkbox("Ask full-screen editor", "ask_fse", s.ask_fse),
        alias = checkbox("Ask alias", "ask_alias", s.ask_alias),
        gender = checkbox("Ask gender", "ask_gender", s.ask_gender),
        birth = checkbox("Ask birthdate", "ask_birthdate", s.ask_birthdate),
        email = checkbox("Ask email", "ask_email", s.ask_email),
        web = checkbox("Ask web address", "ask_web_address", s.ask_web_address),
        short = checkbox("Ask short description preference", "ask_use_short_descr", s.ask_use_short_descr),
    );
    settings_shell(
        SectionId::NewUser,
        &settings.fingerprint,
        csrf,
        notice,
        "Default security, groups and registration questions.",
        &fields,
    )
}

pub fn event_page(settings: &EventSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let fields = format!(
        r#"<fieldset><legend>Timed events</legend>
<div class="check-grid">{enabled}{disallow}</div>
<div class="grid-2">{file}{suspend}{minutes}</div>
</fieldset>"#,
        enabled = checkbox("Events enabled", "enabled", s.enabled),
        disallow = checkbox("Disallow uploads near event", "disallow_uploads", s.disallow_uploads),
        file = text_field("Event file", "event_file", &s.event_file, 512, false),
        suspend = number_field("Suspend minutes", "suspend_minutes", s.suspend_minutes as u64, None, None),
        minutes = number_field(
            "Minutes uploads disallowed",
            "minutes_uploads_disallowed",
            s.minutes_uploads_disallowed as u64,
            None,
            None
        ),
    );
    settings_shell(
        SectionId::Events,
        &settings.fingerprint,
        csrf,
        notice,
        "EVENT.DAT style timed event controls.",
        &fields,
    )
}

pub fn subscription_page(settings: &SubscriptionSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let fields = format!(
        r#"<fieldset><legend>Subscription mode</legend>
<div class="check-grid">{enabled}</div>
<div class="grid-2">{length}{level}{warn}</div>
</fieldset>"#,
        enabled = checkbox("Subscription mode enabled", "is_enabled", s.is_enabled),
        length = number_field("Subscription length (days)", "subscription_length", s.subscription_length as u64, None, None),
        level = number_field(
            "Default expired level",
            "default_expired_level",
            s.default_expired_level as u64,
            None,
            Some(255)
        ),
        warn = number_field("Warning days", "warning_days", s.warning_days as u64, None, None),
    );
    settings_shell(
        SectionId::Subscription,
        &settings.fingerprint,
        csrf,
        notice,
        "Subscription period and expiry defaults.",
        &fields,
    )
}

pub fn connection_page(settings: &ConnectionSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let fields = format!(
        r#"<fieldset><legend>Telnet</legend>
<div class="check-grid">{t_en}</div>
<div class="grid-2">{t_port}{t_addr}{t_file}</div>
</fieldset>
<fieldset><legend>SSH</legend>
<div class="check-grid">{s_en}</div>
<div class="grid-2">{s_port}{s_addr}{s_file}</div>
</fieldset>
<fieldset><legend>Secure WebSockets</legend>
<div class="check-grid">{w_en}</div>
<div class="grid-2">{w_port}{w_addr}{w_file}{w_cert}{w_key}</div>
</fieldset>"#,
        t_en = checkbox("Telnet enabled", "telnet_is_enabled", s.telnet.is_enabled),
        t_port = number_field("Telnet port", "telnet_port", s.telnet.port as u64, Some(1), Some(65535)),
        t_addr = text_field("Telnet address", "telnet_address", &s.telnet.address, 64, false),
        t_file = text_field("Telnet display file", "telnet_display_file", &s.telnet.display_file, 512, false),
        s_en = checkbox("SSH enabled", "ssh_is_enabled", s.ssh.is_enabled),
        s_port = number_field("SSH port", "ssh_port", s.ssh.port as u64, Some(1), Some(65535)),
        s_addr = text_field("SSH address", "ssh_address", &s.ssh.address, 64, false),
        s_file = text_field("SSH display file", "ssh_display_file", &s.ssh.display_file, 512, false),
        w_en = checkbox("Secure WebSocket enabled", "wss_is_enabled", s.secure_websocket.is_enabled),
        w_port = number_field("WSS port", "wss_port", s.secure_websocket.port as u64, Some(1), Some(65535)),
        w_addr = text_field("WSS address", "wss_address", &s.secure_websocket.address, 64, false),
        w_file = text_field("WSS display file", "wss_display_file", &s.secure_websocket.display_file, 512, false),
        w_cert = text_field("Certificate PEM", "wss_cert_pem", &s.secure_websocket.cert_pem, 512, false),
        w_key = text_field("Key PEM", "wss_key_pem", &s.secure_websocket.key_pem, 512, false),
    );
    settings_shell(
        SectionId::Connections,
        &settings.fingerprint,
        csrf,
        notice,
        "Login listeners for Telnet, SSH and secure websockets.",
        &fields,
    )
}

pub fn paths_page(settings: &PathsSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let path = |label: &str, name: &str, value: &str| text_field(label, name, value, 512, false);
    let fields = format!(
        r#"<fieldset><legend>System files</legend><div class="grid-2">
{help}{sec}{email}{cmd_disp}{tmp}{icbtext}{conf}{user}{caller}{transfer}{stats}{cmd}{lang}{group}{proto}{pwrd}{ftn}
</div></fieldset>
<fieldset><legend>Display files</legend><div class="grid-2">
{welcome}{newuser}{closed}{expire_warning}{expired}{join}{chat_intro}{chat_menu}{chat_actions}{no_ansi}
</div></fieldset>
<fieldset><legend>Surveys &amp; trashcans</legend><div class="grid-2">
{logon_s}{logon_a}{logoff_s}{logoff_a}{newask_s}{newask_a}
{trash_up}{trash_user}{trash_email}{trash_pw}{vip}
</div></fieldset>"#,
        help = path("Help path", "help_path", &s.help_path),
        sec = path("Security file path", "security_file_path", &s.security_file_path),
        email = path("E-Mail message base", "email_msgbase", &s.email_msgbase),
        cmd_disp = path("Command display path", "command_display_path", &s.command_display_path),
        tmp = path("Temporary work path", "tmp_work_path", &s.tmp_work_path),
        icbtext = path("Display text (icbtext)", "icbtext", &s.icbtext),
        conf = path("Conferences", "conferences", &s.conferences),
        user = path("User file", "user_file", &s.user_file),
        caller = path("Caller log", "caller_log", &s.caller_log),
        transfer = path("Transfer log", "transfer_log", &s.transfer_log),
        stats = path("Statistics file", "statistics_file", &s.statistics_file),
        cmd = path("Command file", "command_file", &s.command_file),
        lang = path("Language file", "language_file", &s.language_file),
        group = path("Group file", "group_file", &s.group_file),
        proto = path("Protocol data file", "protocol_data_file", &s.protocol_data_file),
        pwrd = path("Security levels file", "pwrd_sec_level_file", &s.pwrd_sec_level_file),
        ftn = path("FTN file", "ftn_file", &s.ftn_file),
        welcome = path("Welcome", "welcome", &s.welcome),
        newuser = path("New user", "newuser", &s.newuser),
        closed = path("Closed", "closed", &s.closed),
        expire_warning = path("Expire warning", "expire_warning", &s.expire_warning),
        expired = path("Expired", "expired", &s.expired),
        join = path("Conference join menu", "conf_join_menu", &s.conf_join_menu),
        chat_intro = path("Chat intro", "chat_intro_file", &s.chat_intro_file),
        chat_menu = path("Chat menu", "chat_menu", &s.chat_menu),
        chat_actions = path("Chat actions menu", "chat_actions_menu", &s.chat_actions_menu),
        no_ansi = path("NOANSI warning", "no_ansi", &s.no_ansi),
        logon_s = path("Logon survey", "logon_survey", &s.logon_survey),
        logon_a = path("Logon answer", "logon_answer", &s.logon_answer),
        logoff_s = path("Logoff survey", "logoff_survey", &s.logoff_survey),
        logoff_a = path("Logoff answer", "logoff_answer", &s.logoff_answer),
        newask_s = path("NewAsk survey", "newask_survey", &s.newask_survey),
        newask_a = path("NewAsk answer", "newask_answer", &s.newask_answer),
        trash_up = path("Trashcan uploads", "trashcan_upload_files", &s.trashcan_upload_files),
        trash_user = path("Trashcan users", "trashcan_user", &s.trashcan_user),
        trash_email = path("Trashcan email", "trashcan_email", &s.trashcan_email),
        trash_pw = path("Trashcan passwords", "trashcan_passwords", &s.trashcan_passwords),
        vip = path("VIP users", "vip_users", &s.vip_users),
    );
    settings_shell(
        SectionId::Paths,
        &settings.fingerprint,
        csrf,
        notice,
        "All configuration path entries. Relative paths are resolved from the board directory.",
        &fields,
    )
}

pub fn accounting_page(settings: &AccountingSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let s = &settings.settings;
    let fields = format!(
        r#"<fieldset><legend>Accounting</legend>
<div class="check-grid">{enabled}{money}{concurrent}{ignore}</div>
<div class="grid-2">
{start}{end}{days}{holiday}{cfg}{track}{info}{warn}{logoff}
</div>
<p class="hint">Peak days of week is seven Y/N characters, Sunday first (example YYYYYNN).</p>
</fieldset>"#,
        enabled = checkbox("Accounting enabled", "enabled", s.enabled),
        money = checkbox("Use money", "use_money", s.use_money),
        concurrent = checkbox("Concurrent tracking", "concurrent_tracking", s.concurrent_tracking),
        ignore = checkbox("Ignore empty security level", "ignore_empty_sec_level", s.ignore_empty_sec_level),
        start = text_field("Peak usage start", "peak_usage_start", &s.peak_usage_start, 8, false),
        end = text_field("Peak usage end", "peak_usage_end", &s.peak_usage_end, 8, false),
        days = text_field("Peak days of week", "peak_days_of_week", &s.peak_days_of_week, 7, false),
        holiday = text_field("Peak holiday list file", "peak_holiday_list_file", &s.peak_holiday_list_file, 512, false),
        cfg = text_field("Accounting cfg file", "cfg_file", &s.cfg_file, 512, false),
        track = text_field("Tracking file", "tracking_file", &s.tracking_file, 512, false),
        info = text_field("Info file", "info_file", &s.info_file, 512, false),
        warn = text_field("Warning file", "warning_file", &s.warning_file, 512, false),
        logoff = text_field("Logoff file", "logoff_file", &s.logoff_file, 512, false),
    );
    settings_shell(
        SectionId::Accounting,
        &settings.fingerprint,
        csrf,
        notice,
        "Accounting mode, peak windows and related files.",
        &fields,
    )
}

pub fn function_keys_page(settings: &FunctionKeysSettingsResponse, csrf: &str, notice: Option<Notice>) -> String {
    let mut fields = String::from(r#"<fieldset><legend>Function keys F1–F10</legend><div class="grid-2">"#);
    for i in 0..10 {
        fields.push_str(&text_field(
            &format!("F{}", i + 1),
            &format!("f{}", i + 1),
            &settings.settings.keys[i],
            256,
            false,
        ));
    }
    fields.push_str("</div></fieldset>");
    settings_shell(
        SectionId::FunctionKeys,
        &settings.fingerprint,
        csrf,
        notice,
        "Local function key macros used by the board UI.",
        &fields,
    )
}

pub fn error_page(title: &str, message: &str) -> String {
    shell(
        title,
        None,
        &format!(
            r#"<section class="panel"><div class="notice err"><strong>{}</strong><p>{}</p></div></section>"#,
            escape(title),
            escape(message)
        ),
    )
}

fn row(label: &str, value: &str) -> String {
    format!("<tr><th>{}</th><td>{}</td></tr>", escape(label), escape(value))
}

fn stat_row(label: &str, today: u64, total: u64) -> String {
    format!("<tr><th>{}</th><td>{}</td><td>{}</td></tr>", escape(label), today, total)
}

fn stat_card(label: &str, value: &str) -> String {
    format!(
        r#"<article class="stat-card"><span>{}</span><strong>{}</strong></article>"#,
        escape(label),
        escape(value)
    )
}

fn metric(label: &str, value: usize) -> String {
    format!(r#"<div class="metric"><span>{}</span><strong>{}</strong></div>"#, escape(label), value)
}

fn text_field(label: &str, name: &str, value: &str, max_len: usize, required: bool) -> String {
    format!(
        r#"<label>{}<input type="text" name="{}" value="{}" maxlength="{}"{}></label>"#,
        escape(label),
        escape(name),
        escape(value),
        max_len,
        if required { " required" } else { "" }
    )
}

fn number_field(label: &str, name: &str, value: u64, min: Option<u64>, max: Option<u64>) -> String {
    let min_attr = min.map(|v| format!(r#" min="{v}""#)).unwrap_or_default();
    let max_attr = max.map(|v| format!(r#" max="{v}""#)).unwrap_or_default();
    format!(
        r#"<label>{}<input type="number" name="{}" value="{}"{}{}></label>"#,
        escape(label),
        escape(name),
        value,
        min_attr,
        max_attr
    )
}

fn checkbox(label: &str, name: &str, checked: bool) -> String {
    format!(
        r#"<label class="check"><input type="checkbox" name="{name}" value="true"{checked}><span>{label}</span></label>"#,
        name = escape(name),
        checked = if checked { " checked" } else { "" },
        label = escape(label)
    )
}

// ---------------------------------------------------------------- conferences

fn conference_shell(title: &str, body: &str) -> String {
    let nav = format!(
        r#"<div class="layout"><aside class="sidebar"><div class="brand"><span class="brand-mark">IB</span><div><strong>IcyBoard</strong><small>Web Admin</small></div></div><nav class="side-nav">{links}</nav><form class="logout" method="post" action="/logout"><button type="submit">Log out</button></form></aside><div class="content"><header class="topbar"><div><p class="eyebrow">Conferences</p><h1>{title}</h1></div><span class="badge">local</span></header><main>{body}</main><footer>icbadmin {version} · icbsetup and icbsm remain fully supported.</footer></div></div>"#,
        links = conference_nav_links(),
        title = escape(title),
        version = env!("CARGO_PKG_VERSION"),
        body = body
    );
    format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · IcyBoard Admin</title><link rel="stylesheet" href="/style.css"></head>
<body>{nav}</body></html>"#,
        title = escape(title),
        nav = nav
    )
}

fn conference_nav_links() -> String {
    nav_links(None, true)
}

pub fn conference_list_page(list: &ConferenceListResponse, csrf: &str, notice: Option<Notice>) -> String {
    let mut rows = String::new();
    for conf in &list.conferences {
        let name = if conf.name.is_empty() { "(unnamed)".to_string() } else { escape(&conf.name) };
        let security = if conf.required_security.is_empty() {
            "-".to_string()
        } else {
            escape(&conf.required_security)
        };
        let mut flags = Vec::new();
        if conf.is_public {
            flags.push(r#"<span class="state ok">public</span>"#.to_string());
        } else {
            flags.push(r#"<span class="state unset">private</span>"#.to_string());
        }
        if conf.is_read_only {
            flags.push(r#"<span class="state unset">read only</span>"#.to_string());
        }
        if conf.password_set {
            flags.push(r#"<span class="state unset">password</span>"#.to_string());
        }
        rows.push_str(&format!(
            r#"<tr><td>{index}</td><td><a href="/conferences/{index}">{name}</a></td><td>{kind}</td><td>{security}</td><td>{flags}</td>
<td class="row-actions"><form method="post" action="/conferences/{index}/delete" onsubmit="return confirm('Delete conference {index}?')">
<input type="hidden" name="csrf" value="{csrf}"><input type="hidden" name="fingerprint" value="{fingerprint}">
<button type="submit" class="danger">Delete</button></form></td></tr>"#,
            index = conf.index,
            name = name,
            kind = escape(&conf.conference_type),
            security = security,
            flags = flags.join(" "),
            csrf = escape(csrf),
            fingerprint = escape(&list.fingerprint),
        ));
    }

    if rows.is_empty() {
        rows = r#"<tr><td colspan="6">No conferences defined.</td></tr>"#.to_string();
    }

    let body = format!(
        r#"{notice}<section class="panel">
<div class="panel-head"><h2>Conferences</h2><a class="button primary" href="/conferences/new">New conference</a></div>
<p class="hint">Stored in <span class="path">{file}</span>. Conference sub lists (areas, directories, doors, bulletins, surveys) are still edited with icbsetup.</p>
<table><thead><tr><th>#</th><th>Name</th><th>Type</th><th>Security</th><th>Flags</th><th></th></tr></thead><tbody>{rows}</tbody></table>
</section>"#,
        notice = notice_html(&notice),
        file = escape(&list.file),
        rows = rows
    );
    conference_shell("Conferences", &body)
}

pub fn conference_new_page(list: &ConferenceListResponse, csrf: &str, notice: Option<Notice>) -> String {
    let dto = ConferenceDto {
        conference_type: "Normal".to_string(),
        is_public: true,
        use_main_commands: true,
        ..Default::default()
    };
    let body = format!(
        r#"{notice}<section class="panel">
<div class="panel-head"><h2>New conference</h2><a class="button" href="/conferences">Back to list</a></div>
<p class="hint">The new conference is appended at the end of <span class="path">{file}</span>.</p>
<form method="post" action="/conferences/new" class="settings-form">
<input type="hidden" name="csrf" value="{csrf}">
<input type="hidden" name="fingerprint" value="{fingerprint}">
{fields}
<div class="form-actions"><button type="submit" class="primary">Create conference</button></div>
</form></section>"#,
        notice = notice_html(&notice),
        file = escape(&list.file),
        csrf = escape(csrf),
        fingerprint = escape(&list.fingerprint),
        fields = conference_fields(&dto, false),
    );
    conference_shell("New conference", &body)
}

pub fn conference_page(conf: &ConferenceResponse, csrf: &str, notice: Option<Notice>) -> String {
    let title = if conf.settings.name.is_empty() {
        format!("Conference {}", conf.index)
    } else {
        conf.settings.name.clone()
    };
    let body = format!(
        r#"{notice}<section class="panel">
<div class="panel-head"><h2>Conference {index}</h2><a class="button" href="/conferences">Back to list</a></div>
<p class="hint">Stored in <span class="path">{file}</span>.</p>
<form method="post" action="/conferences/{index}" class="settings-form">
<input type="hidden" name="csrf" value="{csrf}">
<input type="hidden" name="fingerprint" value="{fingerprint}">
{fields}
<div class="form-actions"><button type="submit" class="primary">Save changes</button></div>
</form></section>"#,
        notice = notice_html(&notice),
        index = conf.index,
        file = escape(&conf.file),
        csrf = escape(csrf),
        fingerprint = escape(&conf.fingerprint),
        fields = conference_fields(&conf.settings, conf.password_set),
    );
    conference_shell(&title, &body)
}

fn conference_fields(dto: &ConferenceDto, password_set: bool) -> String {
    let mut types = String::new();
    for (value, label) in CONFERENCE_TYPES {
        let selected = if *value == dto.conference_type { " selected" } else { "" };
        types.push_str(&format!(r#"<option value="{}"{}>{}</option>"#, value, selected, escape(label)));
    }

    let password_state = if password_set { "set" } else { "not set" };
    let path = |label: &str, name: &str, value: &str| text_field(label, name, value, 512, false);

    format!(
        r#"<fieldset><legend>Conference</legend>
<div class="grid-2">
{name}
<label>Type<select name="conference_type">{types}</select></label>
</div>
<div class="check-grid">{is_public}{is_read_only}{auto_rejoin}{allow_view}{allow_aliases}{show_intro}{use_main}</div>
</fieldset>

<fieldset><legend>Join password</legend>
<p class="hint">Current password: <strong>{password_state}</strong>. Leave the field empty to keep it.</p>
<div class="grid-2">{new_password}</div>
<div class="check-grid">{clear_password}</div>
</fieldset>

<fieldset><legend>Security</legend>
<p class="hint">Security expressions accept a level such as <code>20</code> or an expression such as <code>20 &amp; !GROUP("banned")</code>. Leave empty for no restriction.</p>
<div class="grid-2">{required_security}{sec_attachments}{sec_write_message}{sec_request_rr}{sec_carbon_copy}{carbon_limit}</div>
</fieldset>

<fieldset><legend>Messages</legend>
<div class="check-grid">{echo_mail}{force_echomail}{private_msgs}{disallow_private}{record_origin}{prompt_routing}{long_to_names}</div>
<div class="grid-2">{add_sec}{add_time}</div>
</fieldset>

<fieldset><legend>Uploads</legend>
<div class="check-grid">{private_uploads}</div>
<div class="grid-2">{pub_loc}{pub_meta}{pub_sort}{priv_loc}{priv_meta}{priv_sort}</div>
</fieldset>

<fieldset><legend>Charges</legend>
<div class="grid-2">{charge_time}{charge_read}{charge_write}</div>
</fieldset>

<fieldset><legend>Menus and files</legend>
<div class="grid-2">{users_menu}{sysop_menu}{news_file}{intro_file}{attachment}{command_file}</div>
</fieldset>

<fieldset><legend>Sub lists</legend>
<p class="hint">These point at the conference list files. Their contents are still edited with icbsetup.</p>
<div class="grid-2">{doors_menu}{doors_file}{blt_menu}{blt_file}{survey_menu}{survey_file}{dir_menu}{dir_file}{area_menu}{area_file}</div>
</fieldset>"#,
        name = text_field("Name", "name", &dto.name, 60, true),
        types = types,
        is_public = checkbox("Public conference", "is_public", dto.is_public),
        is_read_only = checkbox("Read only", "is_read_only", dto.is_read_only),
        auto_rejoin = checkbox("Auto rejoin", "auto_rejoin", dto.auto_rejoin),
        allow_view = checkbox("Allow viewing conference members", "allow_view_conf_members", dto.allow_view_conf_members),
        allow_aliases = checkbox("Allow aliases", "allow_aliases", dto.allow_aliases),
        show_intro = checkbox("Show intro in scan", "show_intro_in_scan", dto.show_intro_in_scan),
        use_main = checkbox("Use main board commands", "use_main_commands", dto.use_main_commands),
        password_state = password_state,
        new_password = text_field("New join password", "new_password", "", 60, false),
        clear_password = checkbox("Remove the join password", "clear_password", false),
        required_security = text_field("Required security", "required_security", &dto.required_security, 128, false),
        sec_attachments = text_field("Attachment security", "sec_attachments", &dto.sec_attachments, 128, false),
        sec_write_message = text_field("Write message security", "sec_write_message", &dto.sec_write_message, 128, false),
        sec_request_rr = text_field("Return receipt security", "sec_request_rr", &dto.sec_request_rr, 128, false),
        sec_carbon_copy = text_field("Carbon copy security", "sec_carbon_copy", &dto.sec_carbon_copy, 128, false),
        carbon_limit = number_field("Carbon list limit", "carbon_list_limit", dto.carbon_list_limit as u64, None, Some(255)),
        echo_mail = checkbox("Echo mail in conference", "echo_mail_in_conference", dto.echo_mail_in_conference),
        force_echomail = checkbox("Force echo mail", "force_echomail", dto.force_echomail),
        private_msgs = checkbox("Private messages", "private_msgs", dto.private_msgs),
        disallow_private = checkbox("Disallow private messages", "disallow_private_msgs", dto.disallow_private_msgs),
        record_origin = checkbox("Record origin", "record_origin", dto.record_origin),
        prompt_routing = checkbox("Prompt for routing", "prompt_for_routing", dto.prompt_for_routing),
        long_to_names = checkbox("Long TO names", "long_to_names", dto.long_to_names),
        add_sec = number_field(
            "Security level given on join",
            "add_conference_security",
            dto.add_conference_security.max(0) as u64,
            None,
            Some(255)
        ),
        add_time = number_field("Extra minutes on join", "add_conference_time", dto.add_conference_time as u64, None, None),
        private_uploads = checkbox("Private uploads", "private_uploads", dto.private_uploads),
        pub_loc = path("Public upload location", "pub_upload_location", &dto.pub_upload_location),
        pub_meta = path("Public upload metadata", "pub_upload_metadata", &dto.pub_upload_metadata),
        pub_sort = select_field("Public upload sort", "pub_upload_sort", &dto.pub_upload_sort.to_string(), SORT_ORDERS),
        priv_loc = path("Private upload location", "private_upload_location", &dto.private_upload_location),
        priv_meta = path("Private upload metadata", "private_upload_metadata", &dto.private_upload_metadata),
        priv_sort = select_field("Private upload sort", "private_upload_sort", &dto.private_upload_sort.to_string(), SORT_ORDERS),
        charge_time = decimal_field("Charge per minute", "charge_time", dto.charge_time),
        charge_read = decimal_field("Charge per message read", "charge_msg_read", dto.charge_msg_read),
        charge_write = decimal_field("Charge per message written", "charge_msg_write", dto.charge_msg_write),
        users_menu = path("User menu", "users_menu", &dto.users_menu),
        sysop_menu = path("Sysop menu", "sysop_menu", &dto.sysop_menu),
        news_file = path("News file", "news_file", &dto.news_file),
        intro_file = path("Intro file", "intro_file", &dto.intro_file),
        attachment = path("Attachment location", "attachment_location", &dto.attachment_location),
        command_file = path("Command file", "command_file", &dto.command_file),
        doors_menu = path("Doors menu", "doors_menu", &dto.doors_menu),
        doors_file = path("Doors file", "doors_file", &dto.doors_file),
        blt_menu = path("Bulletin menu", "blt_menu", &dto.blt_menu),
        blt_file = path("Bulletin file", "blt_file", &dto.blt_file),
        survey_menu = path("Survey menu", "survey_menu", &dto.survey_menu),
        survey_file = path("Survey file", "survey_file", &dto.survey_file),
        dir_menu = path("Directory menu", "dir_menu", &dto.dir_menu),
        dir_file = path("Directory file", "dir_file", &dto.dir_file),
        area_menu = path("Area menu", "area_menu", &dto.area_menu),
        area_file = path("Area file", "area_file", &dto.area_file),
    )
}

fn select_field(label: &str, name: &str, value: &str, options: &[(&str, &str)]) -> String {
    let mut rendered = String::new();
    for (option_value, option_label) in options {
        let selected = if *option_value == value { " selected" } else { "" };
        rendered.push_str(&format!(
            r#"<option value="{}"{}>{}</option>"#,
            escape(option_value),
            selected,
            escape(option_label)
        ));
    }
    format!(r#"<label>{}<select name="{}">{}</select></label>"#, escape(label), escape(name), rendered)
}

fn decimal_field(label: &str, name: &str, value: f64) -> String {
    format!(
        r#"<label>{}<input type="number" step="0.01" min="0" name="{}" value="{}"></label>"#,
        escape(label),
        escape(name),
        value
    )
}
