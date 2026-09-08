//! Read-only menu checks. No runtime state, dispatch, PPE loading or writes.
//!
//! Local menu keywords use the FIRST ASCII-case-insensitive exact match in
//! `state/menu_runner.rs`, not the global command list's prefix matching.
//! File warnings use a Graphics, security 0, no-language profile. They are not
//! proof that a file is absent in every possible caller/session profile.

use std::path::{Path, PathBuf};

use icy_board_engine::{
    icy_board::{
        IcyBoard,
        commands::{ActionTrigger, AutoRun, CommandType},
        menu::{Menu, MenuType},
        security_expr::{SecurityExpression, Value},
        state::{Session, functions::MASK_COMMAND},
    },
    tokens::tokenize,
};
use icy_board_tui::{get_text, pcb_line::get_styled_pcb_line};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueSeverity {
    Warning,
    Error,
}

impl IssueSeverity {
    pub fn label(self) -> String {
        get_text(match self {
            Self::Warning => "mnu_check_warning",
            Self::Error => "mnu_check_error",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuIssue {
    /// Zero-based command index; None denotes a menu-wide issue.
    pub command: Option<usize>,
    pub message: String,
    pub severity: IssueSeverity,
}

fn issue(issues: &mut Vec<MenuIssue>, command: Option<usize>, severity: IssueSeverity, id: &str, detail: impl AsRef<str>) {
    let detail = detail.as_ref();
    issues.push(MenuIssue {
        command,
        severity,
        message: if detail.is_empty() {
            get_text(id)
        } else {
            format!("{}: {detail}", get_text(id))
        },
    });
}

/// Pure ordering shared by validation and preview, mirroring
/// `find_more_specific_file` for Graphics/no language. Explicit extended files
/// win; extensionless bases probe security, graphics, extensions, then bare.
/// Existence probing is injectable so precedence regressions need no files.
fn display_candidate(base: &Path, security: u8, exists: impl Fn(&Path) -> bool) -> PathBuf {
    if base.extension().is_some() && exists(base) {
        return base.to_path_buf();
    }
    let name = base.to_string_lossy();
    for prefix in [format!("{name}{security}"), name.into_owned()] {
        for graphics in [format!("{prefix}g"), prefix] {
            for extension in [".ans", ".avt", ".pcb", ".asc", ""] {
                let file = PathBuf::from(format!("{graphics}{extension}"));
                if exists(&file) {
                    return file;
                }
            }
        }
    }
    base.to_path_buf()
}

pub(crate) fn resolve_display_file(board: &IcyBoard, file: &Path, security: u8) -> PathBuf {
    let base = board.resolve_file(&file);
    if base.as_os_str().is_empty() {
        return base;
    }
    // Resolve the base exactly once, as display_file_with_error does. Do not
    // introduce extra case-insensitive lookup for the runtime's appended names.
    display_candidate(&base, security, Path::exists)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreviewAccess {
    Allowed,
    Denied,
    Unknown,
    Invalid,
}

// Do not evaluate session-dependent functions with invented user/group/time
// data. Also protect the engine's U_GROUP evaluator, which indexes args[0].
fn expression_profile(expr: &SecurityExpression) -> PreviewAccess {
    match expr {
        SecurityExpression::Constant(_) => PreviewAccess::Allowed,
        SecurityExpression::Parens(inner) | SecurityExpression::UnaryExpression(_, inner) => expression_profile(inner),
        SecurityExpression::BinaryExpression(_, left, right) => {
            let l = expression_profile(left);
            let r = expression_profile(right);
            if l == PreviewAccess::Invalid || r == PreviewAccess::Invalid {
                PreviewAccess::Invalid
            } else if l == PreviewAccess::Unknown || r == PreviewAccess::Unknown {
                PreviewAccess::Unknown
            } else {
                PreviewAccess::Allowed
            }
        }
        SecurityExpression::Call(name, args) => match name.to_ascii_uppercase().as_str() {
            // These engine calls ignore surplus arguments without evaluating
            // them. Do not invent a hard arity error absent from the runtime.
            "U_SEC" => PreviewAccess::Allowed,
            "U_AGE" | "TIME" | "TIME_LEFT" | "DOW" => PreviewAccess::Unknown,
            "U_GROUP" if !args.is_empty() => {
                if expression_profile(&args[0]) == PreviewAccess::Invalid {
                    PreviewAccess::Invalid
                } else {
                    PreviewAccess::Unknown
                }
            }
            _ => PreviewAccess::Invalid,
        },
    }
}

pub(crate) fn preview_access(expr: &SecurityExpression, security: u8) -> PreviewAccess {
    let profile = expression_profile(expr);
    if profile != PreviewAccess::Allowed {
        return profile;
    }
    let mut session = Session::new();
    session.cur_security = security;
    match expr.eval(&session) {
        Ok(Value::Bool(_) | Value::Integer(_)) => {
            // Use the actual engine admission method, including its integer
            // conversion semantics, rather than approximating with a threshold.
            if expr.session_can_access(&session) {
                PreviewAccess::Allowed
            } else {
                PreviewAccess::Denied
            }
        }
        _ => PreviewAccess::Invalid,
    }
}

fn check_display(issues: &mut Vec<MenuIssue>, board: &IcyBoard, command: Option<usize>, file: &Path) {
    if !file.as_os_str().is_empty() {
        let resolved = resolve_display_file(board, file, 0);
        if !resolved.is_file() {
            issue(
                issues,
                command,
                IssueSeverity::Warning,
                "mnu_check_display_missing",
                resolved.display().to_string(),
            );
        }
    }
}

pub fn validate_menu(board: &IcyBoard, menu: &Menu) -> Vec<MenuIssue> {
    let mut issues = Vec::new();
    if menu.commands.is_empty() {
        issue(&mut issues, None, IssueSeverity::Error, "mnu_check_empty_menu", "");
    }
    check_display(&mut issues, board, None, &menu.display_file);
    check_display(&mut issues, board, None, &menu.help_file);
    if !menu.prompts.is_empty() || menu.force_display || menu.pass_through {
        issue(&mut issues, None, IssueSeverity::Warning, "mnu_check_unused_options", "");
    }
    for (i, cmd) in menu.commands.iter().enumerate() {
        let at = Some(i);
        if !cmd.charge_per_use.is_finite() || !cmd.charge_per_minute.is_finite() || cmd.charge_per_use < 0.0 || cmd.charge_per_minute < 0.0 {
            issue(&mut issues, at, IssueSeverity::Error, "mnu_check_fees", "");
        }
        if cmd.keyword.is_empty() {
            if cmd.auto_run == AutoRun::Disabled && cmd.lighbar_display.is_empty() {
                issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_empty_keyword", "");
            }
        } else {
            if let Some(first) = menu.commands[..i].iter().position(|other| other.keyword.eq_ignore_ascii_case(&cmd.keyword)) {
                issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_duplicate", (first + 1).to_string());
            }
            let tokens = tokenize(&cmd.keyword);
            if tokens.len() != 1 || tokens[0] != cmd.keyword || cmd.keyword.len() > 13 || cmd.keyword.chars().any(|ch| !MASK_COMMAND.contains(ch)) {
                issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_untypable", &cmd.keyword);
            } else if menu.menu_type == MenuType::Hotkey && cmd.keyword.len() != 1 {
                issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_hotkey", &cmd.keyword);
            }
        }
        if cmd.actions.is_empty() {
            issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_no_actions", "");
        } else if !cmd
            .actions
            .iter()
            .any(|action| action.trigger == ActionTrigger::Activation && !matches!(action.command_type, CommandType::Disabled | CommandType::DisableMenuOption))
        {
            issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_no_activation", "");
        }
        if cmd.auto_run == AutoRun::Loop && cmd.autorun_time == 0 {
            issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_loop_zero", "");
        }
        let width = get_styled_pcb_line(&cmd.display).width().max(get_styled_pcb_line(&cmd.lighbar_display).width());
        if cmd.position.x >= 80 || cmd.position.y >= 25 || usize::from(cmd.position.x).saturating_add(width) > 80 {
            issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_position", cmd.position.to_string());
        }
        if width > 0
            && menu.commands[..i].iter().any(|other| {
                let other_width = get_styled_pcb_line(&other.display)
                    .width()
                    .max(get_styled_pcb_line(&other.lighbar_display).width());
                other_width > 0
                    && other.position.y == cmd.position.y
                    && usize::from(other.position.x) < usize::from(cmd.position.x).saturating_add(width)
                    && usize::from(cmd.position.x) < usize::from(other.position.x).saturating_add(other_width)
            })
        {
            issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_overlap", "");
        }
        match preview_access(&cmd.security, 0) {
            PreviewAccess::Invalid => issue(&mut issues, at, IssueSeverity::Error, "mnu_check_security_invalid", cmd.security.to_string()),
            PreviewAccess::Unknown => issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_security_context", cmd.security.to_string()),
            _ => {}
        }
        check_display(&mut issues, board, at, Path::new(&cmd.help));
        for (a, action) in cmd.actions.iter().enumerate() {
            let parameter = &action.parameter;
            let action_detail = format!("{} ({:?})", a + 1, action.command_type);
            if action.command_type == CommandType::GotoXY {
                let coordinates = parameter.split(',').map(|part| part.trim().parse::<u16>()).collect::<Vec<_>>();
                if coordinates.len() != 2 || coordinates[0].as_ref().map_or(true, |x| *x >= 80) || coordinates[1].as_ref().map_or(true, |y| *y >= 25) {
                    issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_action_position", &action_detail);
                }
            }
            if parameter.is_empty()
                && matches!(
                    action.command_type,
                    CommandType::PrintText
                        | CommandType::StuffText
                        | CommandType::StuffTextSilent
                        | CommandType::StuffTextAndExitMenu
                        | CommandType::StuffTextAndExitMenuSilent
                        | CommandType::Command
                        | CommandType::GlobalCommand
                )
            {
                // Exit-menu variants still leave the menu, and Command may
                // consume caller tokens. Warn, rather than falsely rejecting.
                issue(&mut issues, at, IssueSeverity::Warning, "mnu_check_empty_action", &action_detail);
            }
            // Script is a one-based conference survey NUMBER, not a script path.
            if action.command_type == CommandType::Script && parameter.trim().parse::<usize>().ok().filter(|n| *n > 0).is_none() {
                issue(&mut issues, at, IssueSeverity::Error, "mnu_check_survey_number", &action_detail);
            }
            if matches!(
                action.command_type,
                CommandType::Menu | CommandType::DisplayFile | CommandType::StuffFile | CommandType::StuffFileSilent
            ) && parameter.trim().is_empty()
            {
                issue(&mut issues, at, IssueSeverity::Error, "mnu_check_parameter", &action_detail);
                continue;
            }
            if action.command_type == CommandType::DisplayFile {
                check_display(&mut issues, board, at, Path::new(parameter));
                continue;
            }
            if action.command_type == CommandType::RunPPE && tokenize(parameter).first().is_none_or(|token| token.is_empty()) {
                issue(&mut issues, at, IssueSeverity::Error, "mnu_check_parameter", &action_detail);
                continue;
            }
            let file = match action.command_type {
                CommandType::Menu => Some(board.resolve_file(&PathBuf::from(parameter)).with_extension("mnu")),
                CommandType::StuffFile | CommandType::StuffFileSilent => Some(board.resolve_file(&PathBuf::from(parameter))),
                CommandType::RunPPE => tokenize(parameter).first().filter(|s| !s.is_empty()).map(|token| {
                    let file = board.resolve_file(token);
                    if file.exists() { file } else { file.with_extension("ppe") }
                }),
                _ => None,
            };
            if let Some(file) = file
                && !file.is_file()
            {
                issue(
                    &mut issues,
                    at,
                    IssueSeverity::Warning,
                    "mnu_check_file_missing",
                    format!("{action_detail}: {}", file.display()),
                );
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_engine::icy_board::commands::{Command, CommandAction, Position};
    use std::str::FromStr;

    fn command(keyword: &str) -> Command {
        Command {
            keyword: keyword.into(),
            actions: vec![CommandAction {
                command_type: CommandType::QuitMenu,
                ..CommandAction::default()
            }],
            ..Command::default()
        }
    }

    #[test]
    fn local_keywords_are_exact_ascii_first_match_not_prefixes() {
        let menu = Menu {
            menu_type: MenuType::Command,
            commands: vec![command("GO"), command("GOTO"), command("go")],
            ..Menu::default()
        };
        let issues = validate_menu(&IcyBoard::default(), &menu);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].command, Some(2));
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn empty_ppe_filename_is_an_error_but_literal_space_output_is_valid() {
        for parameter in ["", " ", ";argument"] {
            let mut cmd = command("A");
            cmd.actions[0] = CommandAction {
                command_type: CommandType::RunPPE,
                parameter: parameter.into(),
                ..Default::default()
            };
            let issues = validate_menu(
                &IcyBoard::default(),
                &Menu {
                    commands: vec![cmd],
                    ..Default::default()
                },
            );
            assert!(issues.iter().any(|issue| issue.severity == IssueSeverity::Error));
        }
        let mut cmd = command("A");
        cmd.actions[0] = CommandAction {
            command_type: CommandType::PrintText,
            parameter: " ".into(),
            ..Default::default()
        };
        assert!(
            validate_menu(
                &IcyBoard::default(),
                &Menu {
                    commands: vec![cmd],
                    ..Default::default()
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn fees_empty_menu_and_survey_numbers_are_errors() {
        assert_eq!(validate_menu(&IcyBoard::default(), &Menu::default())[0].severity, IssueSeverity::Error);
        for fee in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1] {
            let mut cmd = command("A");
            cmd.charge_per_use = fee;
            let menu = Menu {
                commands: vec![cmd],
                ..Menu::default()
            };
            assert!(validate_menu(&IcyBoard::default(), &menu).iter().any(|i| i.severity == IssueSeverity::Error));
        }
        let mut cmd = command("A");
        cmd.actions[0].command_type = CommandType::Script;
        cmd.actions[0].parameter = "survey.pps".into();
        assert!(
            validate_menu(
                &IcyBoard::default(),
                &Menu {
                    commands: vec![cmd],
                    ..Menu::default()
                }
            )
            .iter()
            .any(|i| i.severity == IssueSeverity::Error)
        );
    }

    #[test]
    fn positions_and_hotkeys_warn_without_rejecting_autoruns() {
        let mut cmd = command("AB");
        cmd.position = Position { x: u16::MAX, y: u16::MAX };
        let issues = validate_menu(
            &IcyBoard::default(),
            &Menu {
                commands: vec![cmd],
                ..Menu::default()
            },
        );
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().all(|i| i.severity == IssueSeverity::Warning));
        let mut cmd = command("");
        cmd.auto_run = AutoRun::FirstCmd;
        assert!(
            validate_menu(
                &IcyBoard::default(),
                &Menu {
                    commands: vec![cmd],
                    ..Menu::default()
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn security_uses_engine_expressions_and_does_not_invent_context() {
        let expr = SecurityExpression::from_str("U_SEC() >= 20 & U_SEC() < 100").unwrap();
        assert_eq!(preview_access(&expr, 30), PreviewAccess::Allowed);
        assert_eq!(preview_access(&expr, 100), PreviewAccess::Denied);
        assert_eq!(preview_access(&SecurityExpression::from_req_security(20), 19), PreviewAccess::Denied);
        assert_eq!(preview_access(&SecurityExpression::Call("U_GROUP".into(), vec![]), 255), PreviewAccess::Invalid);
        assert_eq!(
            preview_access(
                &SecurityExpression::Call("U_SEC".into(), vec![SecurityExpression::Call("U_GROUP".into(), vec![])]),
                30
            ),
            PreviewAccess::Allowed
        );
        assert_eq!(preview_access(&SecurityExpression::Call("U_AGE".into(), vec![]), 255), PreviewAccess::Unknown);
        assert_eq!(
            preview_access(&SecurityExpression::Constant(Value::String("x".into())), 255),
            PreviewAccess::Invalid
        );
    }

    #[test]
    fn display_precedence_matches_explicit_and_extensionless_runtime_rules() {
        let choose = |base: &str, files: &[&str]| display_candidate(Path::new(base), 30, |p| files.iter().any(|f| p == Path::new(f)));
        assert_eq!(choose("MENU.TOP", &["MENU.TOP", "MENU.TOP30g.ans", "MENU.TOP.pcb"]), PathBuf::from("MENU.TOP"));
        assert_eq!(choose("MENU.TOP", &["MENU.TOP.pcb"]), PathBuf::from("MENU.TOP.pcb"));
        assert_eq!(choose("MENU", &["MENU", "MENU.pcb", "MENUg.ans", "MENU30g.pcb"]), PathBuf::from("MENU30g.pcb"));
        assert_eq!(choose("MENU", &["MENU", "MENU.pcb", "MENU.ans"]), PathBuf::from("MENU.ans"));
        assert_eq!(choose("MENU", &[]), PathBuf::from("MENU"));
    }

    #[test]
    fn finite_zero_fees_and_selection_only_are_not_hard_failures() {
        let mut cmd = command("A");
        cmd.actions[0].trigger = ActionTrigger::Selection;
        let issues = validate_menu(
            &IcyBoard::default(),
            &Menu {
                commands: vec![cmd],
                ..Menu::default()
            },
        );
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn malformed_action_coordinates_and_empty_text_warn_but_do_not_fail() {
        let mut cmd = command("A");
        cmd.actions = vec![
            CommandAction {
                command_type: CommandType::GotoXY,
                parameter: "1".into(),
                ..CommandAction::default()
            },
            CommandAction {
                command_type: CommandType::PrintText,
                ..CommandAction::default()
            },
        ];
        let issues = validate_menu(
            &IcyBoard::default(),
            &Menu {
                commands: vec![cmd],
                ..Menu::default()
            },
        );
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().all(|i| i.severity == IssueSeverity::Warning));
    }

    #[test]
    fn missing_background_help_and_file_actions_are_profile_warnings() {
        let missing = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/validation.rs/missing"));
        let mut cmd = command("A");
        cmd.help = missing.display().to_string();
        cmd.actions = vec![CommandAction {
            command_type: CommandType::StuffFile,
            parameter: cmd.help.clone(),
            ..CommandAction::default()
        }];
        let menu = Menu {
            display_file: missing.clone(),
            help_file: missing,
            commands: vec![cmd],
            ..Menu::default()
        };
        let issues = validate_menu(&IcyBoard::default(), &menu);
        assert_eq!(issues.len(), 4);
        assert!(issues.iter().all(|i| i.severity == IssueSeverity::Warning));
    }

    #[test]
    fn overlaps_measure_pcb_color_codes_as_zero_width() {
        let mut a = command("A");
        a.display = "@X0FABC".into();
        let mut b = command("B");
        b.display = "B".into();
        b.position.x = 3;
        let mut menu = Menu {
            commands: vec![a, b],
            ..Menu::default()
        };
        assert!(validate_menu(&IcyBoard::default(), &menu).is_empty());
        menu.commands[1].position.x = 2;
        assert_eq!(validate_menu(&IcyBoard::default(), &menu).len(), 1);
    }
}
