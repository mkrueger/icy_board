// Every opcode handler is called with `.await` from the dispatch table in `mod.rs`,
// so the signature stays async even where a given handler never awaits.
#![allow(clippy::unused_async)]

use std::{
    env,
    fs::{self},
    io::Write,
};

use crate::{
    Res,
    datetime::{IcbDate, IcbTime},
    executable::{GenericVariableData, PPEExpr, VariableType, VariableValue},
    icy_board::{
        ftn::queue,
        icb_config::IcbColor,
        state::{
            GraphicsMode, KeySource, NodeState, NodeStatus,
            functions::{MASK_ASCII, PPECall, display_flags},
        },
        user_base::ConferenceFlags,
        user_inf::{BankUserInf, QwkConfigUserInf},
    },
    vm::{MAX_FILE_CHANNELS, dbase, get_file_channel},
};
use bstr::BString;
use chrono::{DateTime, Utc};
use codepages::tables::CP437_TO_UNICODE;
use icy_engine::BufferType;
use jamjam::jam::{JamMessage, JamMessageBase, attributes as jam_attributes, msg_header::SubfieldType};

use crate::{
    icy_board::icb_text::IceText,
    icy_board::state::ppl_error::{
        ERR_FORMAT, ERR_INVALID, ERR_IO, ERR_KIND_FILE, ERR_KIND_FONT, ERR_KIND_GFX, ERR_KIND_SOUND, ERR_KIND_STRING, ERR_KIND_TERM, ERR_LIMIT,
        ERR_UNAVAILABLE, PplError,
    },
    icy_board::state::screen_to_pcboard_text,
    vm::{TerminalTarget, VMError, VirtualMachine},
};

use super::super::errors::IcyError;
use super::super::expressions::predefined_functions::message_status;
use std::fmt::Write as _;

/// A statement that is not implemented yet. `PCBoard` never aborted a PPE over one,
/// so the call is logged and skipped rather than killing the session.
macro_rules! unimplemented_stmt {
    ($name:expr) => {{
        log::warn!("{} statement is not implemented, ignoring the call", $name);
        return Ok(());
    }};
}

/// Should never be called. But some op codes are invalid as statement call (like if or return)
/// and are handled by it's own `PPECommands` and will point to this function.
///
/// # Panics
///
/// Always
pub async fn invalid(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    panic!("Invalid statement");
}

pub async fn eval(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.eval_expr(&args[0]).await?;
    Ok(())
}

pub async fn end(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.is_running = false;
    Ok(())
}

pub async fn cls(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.clear_screen(TerminalTarget::Both).await?;
    Ok(())
}

pub async fn clreol(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.clear_eol(TerminalTarget::Both).await?;
    Ok(())
}

pub async fn more(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.more_promt().await?;
    Ok(())
}

pub async fn wait(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.press_enter().await?;
    Ok(())
}

pub async fn color(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let color = vm.eval_expr(&args[0]).await?.as_int() as u8;
    vm.icy_board_state.set_color(TerminalTarget::Both, color.into()).await?;
    Ok(())
}

pub async fn confflag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conf = vm.eval_expr(&args[0]).await?.as_int() as u16;
    let flags = vm.eval_expr(&args[0]).await?.as_int();
    // 1 = registered
    // 2 = expired
    // 4 = selected
    // 8 = conference sysop
    //16 = mail waiting
    //32 = net status

    if let Some(session_user) = &mut vm.icy_board_state.session.current_user {
        let mut value = session_user.conference_flags.get(&(conf as usize)).copied().unwrap_or(ConferenceFlags::None);
        value |= ConferenceFlags::from_bits(flags as u8).unwrap_or(ConferenceFlags::None);
        session_user.conference_flags.insert(conf as usize, value);
    }

    Ok(())
}

pub async fn confunflag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conf = vm.eval_expr(&args[0]).await?.as_int() as u16;
    let flags = vm.eval_expr(&args[1]).await?.as_int();

    // Flag values to clear:
    // 1 = registered
    // 2 = expired
    // 4 = selected
    // 8 = conference sysop
    // 16 = mail waiting
    // 32 = net status

    if let Some(session_user) = &mut vm.icy_board_state.session.current_user {
        // Get existing flags or None if conference has no flags set
        let mut value = session_user.conference_flags.get(&(conf as usize)).copied().unwrap_or(ConferenceFlags::None);

        // Clear the specified flags using bitwise operations
        let flags_to_clear = ConferenceFlags::from_bits(flags as u8).unwrap_or(ConferenceFlags::None);
        value &= !flags_to_clear; // Remove the specified flags

        // Update the conference flags
        if value == ConferenceFlags::None {
            // If no flags remain, remove the entry from the map
            session_user.conference_flags.remove(&(conf as usize));
        } else {
            // Otherwise update with the new value
            session_user.conference_flags.insert(conf as usize, value);
        }
    }

    Ok(())
}

/// # Errors
/// Errors if
pub async fn dispfile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let file_name = &vm.eval_expr(&args[0]).await?.as_string();
    let file_name = vm.resolve_file(&file_name).await;
    vm.icy_board_state.display_file(&file_name).await?;
    Ok(())
}

pub async fn fcreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    vm.io.fcreate(channel, &file, am, sm);
    Ok(())
}

pub async fn fopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    vm.io.fopen(channel, &file, am, sm)?;
    Ok(())
}

pub async fn fappend(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    vm.io.fappend(channel, &file);
    Ok(())
}

pub async fn fclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    if channel == -1 {
        // READLINE uses -1 as a special value
        return Ok(());
    }
    vm.io.fclose(channel)?;
    Ok(())
}

pub async fn fget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let value = VariableValue::new_string(vm.io.fget(channel)?);
    vm.set_variable(&args[1], value).await?;
    Ok(())
}

pub async fn fput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;

    for value in &args[1..] {
        let text = vm.eval_expr(value).await?.as_string();
        vm.io.fput(channel, text)?;
    }
    Ok(())
}

pub async fn fputln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;

    for value in &args[1..] {
        let text = vm.eval_expr(value).await?.as_string();
        vm.io.fput(channel, text)?;
    }
    vm.io.fput(channel, "\n".to_string())?;
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn resetdisp(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // TODO?: unused
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn startdisp(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    const FORCE_NS: i32 = 1;
    const FORCE_COUNTLINES: i32 = 2;
    let channel = vm.eval_expr(&args[0]).await?.as_int();
    if channel == FORCE_NS {
        vm.icy_board_state.session.disp_options.force_non_stop();
    } else if channel == FORCE_COUNTLINES {
        vm.icy_board_state.session.disp_options.force_count_lines();
    } else {
        vm.icy_board_state.session.disp_options.no_change();
    }
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn fputpad(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let text = vm.eval_expr(&args[1]).await?.as_string();
    let width = vm.eval_expr(&args[2]).await?.as_int();
    fputpad_internal(vm, channel, text, width)
}

fn fputpad_internal(vm: &mut VirtualMachine<'_>, channel: i32, text: String, width: i32) -> Res<()> {
    let abs_width = width.unsigned_abs() as usize;
    let padded = match width.cmp(&0) {
        std::cmp::Ordering::Greater => {
            // Positive width: right-justify (left-pad with spaces)
            if text.len() >= abs_width { text } else { format!("{text:>abs_width$}") }
        }
        std::cmp::Ordering::Less => {
            // Negative width: left-justify (right-pad with spaces)
            if text.len() >= abs_width { text } else { format!("{text:<abs_width$}") }
        }
        std::cmp::Ordering::Equal => {
            // Width of 0: just the text as-is
            text
        }
    };
    vm.io.fput(channel, padded)?;
    vm.io.fput(channel, "\n".to_string())?;
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn hangup(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.goodbye().await?;
    vm.is_running = false;
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn getuser(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.user = if let Some(user) = &mut vm.icy_board_state.session.current_user {
        user.clone()
    } else {
        return Err(Box::new(IcyError::UserNotFound(String::new())));
    };
    vm.set_user_variables()?;
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn putuser(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    if let Some(mut user) = vm.icy_board_state.session.current_user.take() {
        vm.put_user_variables(&mut user).await;
        vm.icy_board_state.session.current_user = Some(user);
    }
    Ok(())
}

pub async fn defcolor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.reset_color(TerminalTarget::Both).await
}

pub async fn delete(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let file = &vm.eval_expr(&args[0]).await?.as_string();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    if let Err(err) = vm.io.delete(&file)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        log::error!("Error deleting file'{file}': {err}");
    }
    Ok(())
}

pub async fn deluser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.user.flags.delete_flag = true;
    Ok(())
}

pub async fn adjtime(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let min = vm.eval_expr(&args[0]).await?.as_int();
    // Once an event has trimmed the session there is no room to give any back.
    if min > 0 && vm.icy_board_state.session.time_adjusted_for_event {
        return Ok(());
    }
    vm.icy_board_state.session.time_limit += min;
    Ok(())
}

pub async fn log(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut msg = vm.eval_expr(&args[0]).await?.as_string();
    let left_justify = vm.eval_expr(&args[1]).await?.as_bool();
    if left_justify {
        msg = msg.trim_start().to_string();
    }
    log::info!("{msg}");
    Ok(())
}

pub async fn input(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();
    let color = IcbColor::dos_yellow();
    let d = get_default_string(vm, &args[1]).await;
    let output = vm
        .icy_board_state
        .input_string(
            color,
            prompt,
            60,
            &MASK_ASCII,
            "",
            d,
            display_flags::FIELDLEN | display_flags::GUIDE | display_flags::HIGHASCII,
        )
        .await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}

pub async fn inputstr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();
    // 1 Output Variable
    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = vm.eval_expr(&args[3]).await?.as_int();
    let valid = vm.eval_expr(&args[4]).await?.as_string();
    let flags = vm.eval_expr(&args[5]).await?.as_int();
    let d = get_default_string(vm, &args[1]).await;
    let output = vm.icy_board_state.input_string(color.into(), prompt, len, &valid, "", d, flags).await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}

pub async fn inputtext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();
    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = vm.eval_expr(&args[3]).await?.as_int();
    let output = vm
        .icy_board_state
        .input_string(
            color.into(),
            prompt,
            len,
            &MASK_ASCII,
            "",
            None,
            display_flags::FIELDLEN | display_flags::GUIDE | display_flags::HIGHASCII,
        )
        .await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}
pub async fn inputyn(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();
    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 1;
    let d = get_default_string(vm, &args[1]).await;
    let output = vm
        .icy_board_state
        .input_string(
            color.into(),
            prompt,
            len,
            "",
            "",
            d,
            display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::GUIDE,
        )
        .await?;

    vm.set_variable(&args[1], VariableValue::new_string(output.to_ascii_uppercase())).await?;
    Ok(())
}

async fn get_default_string(vm: &mut VirtualMachine<'_>, args: &PPEExpr) -> Option<String> {
    let default = vm.eval_expr(args).await.unwrap().as_string();
    if default.is_empty() { None } else { Some(default) }
}

pub async fn inputmoney(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 13;
    let valid = "01234567890+-$.";
    let d = get_default_string(vm, &args[1]).await;
    let output = vm
        .icy_board_state
        .input_string(
            color.into(),
            prompt,
            len,
            valid,
            "",
            d,
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::GUIDE,
        )
        .await?;
    // PCBoard assigns the text and lets the variable's type convert it (SCREXEC.CPP).
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}

pub async fn inputint(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();
    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 11;
    let valid = "01234567890+-";
    let d = get_default_string(vm, &args[1]).await;
    let output = vm
        .icy_board_state
        .input_string(
            color.into(),
            prompt,
            len,
            valid,
            "",
            d,
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::GUIDE,
        )
        .await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}
pub async fn inputcc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 16;
    let valid = "01234567890";
    let d = get_default_string(vm, &args[1]).await;
    let output = vm
        .icy_board_state
        .input_string(
            color.into(),
            prompt,
            len,
            valid,
            "",
            d,
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::GUIDE,
        )
        .await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}
pub async fn inputdate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 8;
    let valid = "01234567890-/";
    let d = get_default_string(vm, &args[1]).await;
    let output = vm
        .icy_board_state
        .input_string(
            color.into(),
            prompt,
            len,
            valid,
            "",
            d,
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::GUIDE,
        )
        .await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}

pub async fn inputtime(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();
    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 8;
    let valid = "01234567890:";
    let d = get_default_string(vm, &args[1]).await;
    let output = vm
        .icy_board_state
        .input_string(
            color.into(),
            prompt,
            len,
            valid,
            "",
            d,
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::GUIDE,
        )
        .await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}
pub async fn promptstr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_int();
    // 1 Output Variable
    let len = vm.eval_expr(&args[2]).await?.as_int();
    let valid = vm.eval_expr(&args[3]).await?.as_string();
    let flags = vm.eval_expr(&args[4]).await?.as_int();
    let Some(prompt) = IceText::try_from_number(prompt.max(0) as usize) else {
        return Err(format!("PROMPTSTR: there is no text record {prompt}").into());
    };
    let output = vm.icy_board_state.input_field(prompt, len, &valid, "", None, flags).await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}
pub async fn dtron(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // IGNORE
    Ok(())
}

pub async fn dtroff(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.goodbye().await?;
    Ok(())
}

pub async fn cdchkon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // IGNORE
    log::info!("ignore PPL statement CDCHKON");
    Ok(())
}
pub async fn cdchkoff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // IGNORE
    log::info!("ignore PPL statement CDCHKOFF");
    Ok(())
}

pub async fn delay(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // 1 tick is ~1/18.2s
    let ticks = vm.eval_expr(&args[0]).await?.as_int();
    if ticks > 0 {
        let ms = (ticks as f32 * 1000.0 / 18.2) as u64;
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    }
    Ok(())
}

pub async fn sendmodem(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // IGNORE
    log::info!("ignore PPL statement SENDMODEM");
    Ok(())
}

pub async fn inc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let new_value = vm.eval_expr(&args[0]).await? + VariableValue::new_int(1);
    vm.set_variable(&args[0], new_value).await?;
    Ok(())
}

pub async fn dec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let new_value = vm.eval_expr(&args[0]).await?.clone() - VariableValue::new_int(1);
    vm.set_variable(&args[0], new_value).await?;
    Ok(())
}

pub async fn newline(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.write_raw(TerminalTarget::Both, &['\n']).await
}

pub async fn newlines(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let count = vm.eval_expr(&args[0]).await?.as_int();
    for _ in 0..count {
        newline(vm, args).await?;
    }
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn tokenize(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let str: String = vm.eval_expr(&args[0]).await?.to_string();
    vm.icy_board_state.session.push_tokens(&str);
    Ok(())
}

pub async fn gettoken(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let var = if let Some(token) = vm.icy_board_state.session.tokens.pop_front() {
        VariableValue::new_string(token)
    } else {
        VariableValue::new_string(String::new())
    };

    vm.set_variable(&args[0], var).await?;
    Ok(())
}

pub async fn shell(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let use_shell = vm.eval_expr(&args[0]).await?.as_bool();

    let mut cmd = vm.eval_expr(&args[2]).await?.as_string();
    let arguments = vm.eval_expr(&args[3]).await?.as_string();
    let mut exit_code = 1;

    let mut command_args = Vec::new();
    if use_shell {
        // Only COMMAND.COM ever took /C, every shell here takes -c.
        let (shell, flag) = if cfg!(windows) {
            (env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string()), "/C")
        } else {
            (env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()), "-c")
        };
        command_args.push(flag.to_string());
        command_args.push(format!("{} '{}'", cmd.trim(), arguments));
        cmd = shell;
    } else {
        command_args.push(arguments);
    }

    match std::process::Command::new(cmd).args(command_args).spawn() {
        Ok(mut child) => match child.wait() {
            Ok(code) => {
                if let Some(ec) = code.code() {
                    exit_code = ec;
                }
            }
            Err(e) => {
                log::error!("Error running process: {e}");
            }
        },
        Err(e) => {
            log::error!("Error starting process: {e}");
        }
    }

    vm.set_variable(&args[1], VariableValue::new_int(exit_code)).await?;
    Ok(())
}

pub async fn disptext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let rec = vm.eval_expr(&args[0]).await?.as_int();
    let flags = vm.eval_expr(&args[1]).await?.as_int();

    let Some(rec) = IceText::try_from_number(rec.max(0) as usize) else {
        return Err(format!("DISPTEXT: there is no text record {rec}").into());
    };
    vm.icy_board_state.display_text(rec, flags).await
}

pub async fn stop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.is_running = false;
    Ok(())
}

pub async fn beep(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.bell().await
}

pub async fn push(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for p in args {
        let value = vm.eval_expr(p).await?;
        vm.push_pop_stack.push(value);
    }
    Ok(())
}

pub async fn pop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for arg in args {
        if let Some(val) = vm.push_pop_stack.pop() {
            vm.set_variable(arg, val).await?;
        } else {
            return Err(Box::new(VMError::PushPopStackEmpty));
        }
    }
    Ok(())
}

pub async fn call(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.run_ppe(&file_name, None).await?;
    Ok(())
}

pub async fn join(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conf = vm.eval_expr(&args[0]).await?.as_int();
    if conf >= 0 {
        vm.icy_board_state.join_conference(conf as u16, true, true).await?;
    }
    Ok(())
}
pub async fn quest(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let nr = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(surveys) = &vm.icy_board_state.session.current_conference.surveys
        && let Some(survey) = surveys.get(nr as usize)
    {
        vm.icy_board_state.start_survey(&survey.clone()).await?;
    }
    Ok(())
}

pub async fn blt(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let nr = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(bulletins) = &vm.icy_board_state.session.current_conference.bulletins
        && let Some(bulletin) = bulletins.get(nr as usize)
    {
        vm.icy_board_state.display_file(&bulletin.path.clone()).await?;
    }
    Ok(())
}

pub async fn dir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let arg = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.session.push_tokens(&arg);
    vm.icy_board_state.show_file_directories_cmd().await?;
    Ok(())
}

pub async fn kbdstuff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.stuff_keyboard_buffer(&value, false)?;
    Ok(())
}
pub async fn kbdstring(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.print(TerminalTarget::Both, &value).await?;
    vm.icy_board_state.stuff_keyboard_buffer(&value, true)?;
    Ok(())
}
pub async fn kbdfile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    let file_name = vm.resolve_file(&file_name).await;
    let contents = fs::read_to_string(file_name)?;
    vm.icy_board_state.stuff_keyboard_buffer_from(&contents, KeySource::StuffedFile)?;

    Ok(())
}

pub async fn bye(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.goodbye().await?;
    vm.is_running = false;
    Ok(())
}

pub async fn goodbye(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.goodbye().await?;
    vm.is_running = false;
    Ok(())
}

/// Broadcast a single line message to a range of nodes.
/// # Arguments
///  * `lonode` - The low node number to which the message should be broadcast.
///  * `hinode` - The high node number to which the message should be broadcast.
///  * `message` - The message text which should be broadcast to the specified nodes.
/// # Remarks
/// This statement allows you to programatically broadcast a message to a range of nodes
/// without giving users the ability to manually broadcast
/// at any time they choose.
pub async fn broadcast(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let lonode = vm.eval_expr(&args[0]).await?.as_int().saturating_sub(1).max(0) as u16;
    let hinode = vm.eval_expr(&args[1]).await?.as_int().saturating_sub(1).min(65536) as u16;
    let message = vm.eval_expr(&args[2]).await?.as_string();
    vm.icy_board_state.broadcast(lonode, hinode, &message).await?;
    Ok(())
}

pub async fn waitfor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    use std::time::{Duration, Instant};

    // WAITFOR(STRING str, VAR BOOLEAN flag, INTEGER sec)
    let search_str = vm.eval_expr(&args[0]).await?.as_string();
    let timeout_secs = vm.eval_expr(&args[2]).await?.as_int();

    // Default result is FALSE
    let mut result = false;

    // Only wait if we have a non-empty string and remote connection
    if !search_str.is_empty() && !vm.icy_board_state.session.is_local {
        let search_str_lower = search_str.to_lowercase();
        let search_len = search_str_lower.len();
        let mut buffer = String::new();

        // Set up timer
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs.max(0) as u64);

        // Wait for the string or timeout
        while start.elapsed() < timeout {
            // Try to get a character from the modem/remote
            if let Some(key_char) = vm.icy_board_state.get_char(TerminalTarget::User).await? {
                // Add character to buffer
                buffer.push(key_char.ch);

                // Keep buffer size manageable (only keep last search_len chars)
                if buffer.len() > search_len {
                    buffer = buffer.chars().skip(1).collect();
                }

                // Check if buffer matches (case-insensitive)
                if buffer.len() >= search_len {
                    let buffer_lower = buffer.to_lowercase();
                    if buffer_lower == search_str_lower || buffer_lower.ends_with(&search_str_lower) {
                        result = true;
                        break;
                    }
                }
            } else {
                // No character available, yield to prevent tight loop
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    // Set the flag variable
    vm.set_variable(&args[1], VariableValue::new_bool(result)).await?;

    Ok(())
}

pub async fn kbdchkon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.reset_keyboard_check_timer();
    Ok(())
}

pub async fn kbdchkoff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.keyboard_timer_check = false;
    Ok(())
}

pub async fn optext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.op_text = vm.eval_expr(&args[0]).await?.as_string();
    Ok(())
}
pub async fn dispstr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0]).await?.as_string();
    // PCBoard looks for a file spec only after stripping the padding, but prints the string as it came.
    if PPECall::try_parse_line(value.trim_end()).is_some() {
        vm.icy_board_state.display_line(value.trim_end()).await
    } else {
        vm.icy_board_state.print(TerminalTarget::Both, &value).await
    }
}

pub async fn rdunet(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let node = vm.eval_expr(&args[0]).await?.as_int() - 1;
    if let Some(Some(node)) = vm.icy_board_state.node_state.lock().await.get(node as usize) {
        vm.pcb_node = Some(NodeState {
            sysop_connection: None,
            bbs_channel: None,
            cur_user: node.cur_user,
            cur_conference: node.cur_conference,
            graphics_mode: node.graphics_mode,
            operation: node.operation.clone(),
            user_name: node.user_name.clone(),
            city: node.city.clone(),
            status: node.status,
            enabled_chat: node.enabled_chat,
            node_number: node.node_number,
            connection_type: node.connection_type,
            logon_time: node.logon_time,
            handle: None,
        });
    } else {
        vm.pcb_node = Some(NodeState {
            sysop_connection: None,
            bbs_channel: None,
            cur_user: -1,
            cur_conference: 0,
            graphics_mode: GraphicsMode::Graphics,
            operation: String::new(),
            user_name: String::new(),
            city: String::new(),
            status: NodeStatus::NoCaller,
            enabled_chat: false,
            node_number: node as usize,
            connection_type: icy_net::ConnectionType::Channel,
            logon_time: DateTime::<Utc>::MIN_UTC,
            handle: None,
        });
    }
    Ok(())
}

pub async fn wrunet(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let node = vm.eval_expr(&args[0]).await?.as_int() - 1;
    let stat = vm.eval_expr(&args[1]).await?.as_string();
    let name: String = vm.eval_expr(&args[2]).await?.as_string();
    let city = vm.eval_expr(&args[3]).await?.as_string();
    let operation = vm.eval_expr(&args[4]).await?.as_string();
    let _broadcast = vm.eval_expr(&args[5]).await?.as_string();

    // PCBoard's WRUNET (SCREXEC.CPP) only calls updateusernetrecord - it never sends
    // anything itself. A node message is delivered when the *target* node's own
    // polling loop later finds Status == NODEMESSAGE in its record (USERNET.C); we
    // do not run that poll, so the message text has nowhere to go.
    if let Some(Some(node)) = vm.icy_board_state.node_state.lock().await.get_mut(node as usize) {
        // PCBoard writes the status byte unconditionally; an empty string clears it.
        if let Some(ch) = stat.chars().next() {
            if let Some(stat) = NodeStatus::from_char(ch) {
                node.status = stat;
            }
        } else {
            node.status = NodeStatus::NoCaller;
        }
        node.operation = operation;
        node.user_name = name;
        node.city = city;
    } else {
        log::error!("PPE wrunet - node invalid: {node}");
    }

    Ok(())
}

pub async fn dointr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("DOINTR");
}
pub async fn varseg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("VARSEG");
}
pub async fn varoff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("VAROFF");
}
pub async fn pokeb(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("POKEB");
}
pub async fn pokew(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("POKEW");
}
pub async fn varaddr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("VARADDR");
}

pub async fn ansipos(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let x = vm.eval_expr(&args[0]).await?.as_int();
    let y = vm.eval_expr(&args[1]).await?.as_int();
    vm.icy_board_state.gotoxy(TerminalTarget::Both, x, y).await
}

/// Margins and fonts are VT features: AVATAR and plain ASCII callers have no equivalent,
/// so they must not receive the escape sequence as literal text.
fn supports_vt_sequences(vm: &VirtualMachine<'_>) -> bool {
    matches!(
        vm.icy_board_state.session.disp_options.grapics_mode,
        GraphicsMode::Ansi | GraphicsMode::Graphics | GraphicsMode::Rip
    )
}

async fn write_vt_sequence(vm: &mut VirtualMachine<'_>, sequence: &str) -> Res<()> {
    if supports_vt_sequences(vm) {
        vm.icy_board_state.print(TerminalTarget::Both, sequence).await
    } else {
        Ok(())
    }
}
pub async fn set_v_margins(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let top = vm.eval_expr(&args[0]).await?.as_int();
    let bottom = vm.eval_expr(&args[1]).await?.as_int();
    margins_set_vertical(vm, top, bottom).await?;
    Ok(())
}

pub(crate) async fn margins_set_vertical(vm: &mut VirtualMachine<'_>, top: i32, bottom: i32) -> Res<bool> {
    // A terminal ignores an empty or out of range region; sending it would only risk a
    // malformed sequence, so it stops here instead.
    if top < 1 || bottom <= top {
        log::warn!("vertical margins ignored: invalid region {top};{bottom}");
        return Ok(false);
    }
    write_vt_sequence(vm, &format!("\x1B[{top};{bottom}r")).await?;
    vm.icy_board_state.ppl_terminal.mark_margins_changed();
    Ok(true)
}

pub async fn set_h_margins(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let left = vm.eval_expr(&args[0]).await?.as_int();
    let right = vm.eval_expr(&args[1]).await?.as_int();
    margins_set_horizontal(vm, left, right).await?;
    Ok(())
}

pub(crate) async fn margins_set_horizontal(vm: &mut VirtualMachine<'_>, left: i32, right: i32) -> Res<bool> {
    if left < 1 || right <= left {
        log::warn!("horizontal margins ignored: invalid region {left};{right}");
        return Ok(false);
    }
    write_vt_sequence(vm, &format!("\x1B[?69h\x1B[{left};{right}s")).await?;
    vm.icy_board_state.ppl_terminal.mark_margins_changed();
    Ok(true)
}

pub async fn reset_v_margins(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    // DECSTBM leaves DECLRMM alone on the wire, so a horizontal margin may still stand.
    write_vt_sequence(vm, "\x1B[r").await
}

pub async fn reset_h_margins(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    write_vt_sequence(vm, "\x1B[?69l").await
}

pub async fn reset_margins(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    write_vt_sequence(vm, "\x1B[r\x1B[?69l").await?;
    vm.icy_board_state.ppl_terminal.forget_margins();
    Ok(())
}

const DOS_TO_ANSI_PALETTE: [u8; 16] = [0, 4, 2, 6, 1, 5, 3, 7, 8, 12, 10, 14, 9, 13, 11, 15];

fn dos_palette_sequence() -> String {
    let mut sequence = "\x1b]4".to_string();
    for (ansi_index, dos_index) in DOS_TO_ANSI_PALETTE.into_iter().enumerate() {
        let (red, green, blue) = icy_engine::DOS_DEFAULT_PALETTE[usize::from(dos_index)].rgb();
        sequence.push_str(&format!(";{ansi_index};rgb:{red:02X}/{green:02X}/{blue:02X}"));
    }
    sequence.push_str("\x1b\\");
    sequence
}

pub async fn set_palette_color(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let color = vm.eval_expr(&args[0]).await?.as_int();
    let components = if args.len() == 2 {
        let rgba = vm.eval_expr(&args[1]).await?.as_unsigned() as u32;
        PaletteComponents::Packed(rgba)
    } else {
        let red = vm.eval_expr(&args[1]).await?.as_int();
        let green = vm.eval_expr(&args[2]).await?.as_int();
        let blue = vm.eval_expr(&args[3]).await?.as_int();
        PaletteComponents::Separate(red, green, blue)
    };
    palette_set(vm, color, components).await?;
    Ok(())
}

/// How a palette colour was written down. The alpha of a packed value is ignored.
pub(crate) enum PaletteComponents {
    Packed(u32),
    Separate(i32, i32, i32),
}

pub(crate) async fn palette_set(vm: &mut VirtualMachine<'_>, color: i32, components: PaletteComponents) -> Res<bool> {
    if !(0..16).contains(&color) {
        vm.set_error(PplError::new(ERR_KIND_GFX, ERR_INVALID, format!("invalid palette color {color}")));
        return Ok(false);
    }
    if !supports_vt_sequences(vm) {
        vm.set_error(PplError::new(ERR_KIND_GFX, ERR_UNAVAILABLE, "the terminal has no palette support"));
        return Ok(false);
    }

    let (red, green, blue) = match components {
        PaletteComponents::Packed(rgba) => ((rgba >> 24) as u8, (rgba >> 16) as u8, (rgba >> 8) as u8),
        PaletteComponents::Separate(red, green, blue) => {
            if !(0..=255).contains(&red) || !(0..=255).contains(&green) || !(0..=255).contains(&blue) {
                vm.set_error(PplError::new(ERR_KIND_GFX, ERR_INVALID, "palette RGB components must be between 0 and 255"));
                return Ok(false);
            }
            (red as u8, green as u8, blue as u8)
        }
    };

    let index = DOS_TO_ANSI_PALETTE[color as usize];
    write_vt_sequence(vm, &format!("\x1B]4;{index};rgb:{red:02X}/{green:02X}/{blue:02X}\x1B\\")).await?;
    vm.operation_succeeded();
    Ok(true)
}

pub async fn reset_palette_color(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let color = vm.eval_expr(&args[0]).await?.as_int();
    palette_reset(vm, color).await?;
    Ok(())
}

pub(crate) async fn palette_reset(vm: &mut VirtualMachine<'_>, color: i32) -> Res<bool> {
    if !(0..16).contains(&color) {
        vm.set_error(PplError::new(ERR_KIND_GFX, ERR_INVALID, format!("invalid palette color {color}")));
        return Ok(false);
    }
    if !supports_vt_sequences(vm) {
        vm.set_error(PplError::new(ERR_KIND_GFX, ERR_UNAVAILABLE, "the terminal has no palette support"));
        return Ok(false);
    }

    let index = DOS_TO_ANSI_PALETTE[color as usize];
    write_vt_sequence(vm, &format!("\x1B]104;{index}\x1B\\")).await?;
    vm.operation_succeeded();
    Ok(true)
}

pub async fn reset_palette(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    palette_reset_all(vm).await?;
    Ok(())
}

pub(crate) async fn palette_reset_all(vm: &mut VirtualMachine<'_>) -> Res<bool> {
    if !supports_vt_sequences(vm) {
        vm.set_error(PplError::new(ERR_KIND_GFX, ERR_UNAVAILABLE, "the terminal has no palette support"));
        return Ok(false);
    }

    write_vt_sequence(vm, "\x1B]104\x1B\\").await?;
    vm.operation_succeeded();
    Ok(true)
}

pub async fn begin_terminal_update(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    terminal_begin_update(vm).await?;
    Ok(())
}

pub(crate) async fn terminal_begin_update(vm: &mut VirtualMachine<'_>) -> Res<bool> {
    if !supports_vt_sequences(vm) || vm.icy_board_state.session.term_caps.synchronized_output == Some(false) {
        vm.set_error(PplError::new(
            ERR_KIND_TERM,
            ERR_UNAVAILABLE,
            "the terminal does not support synchronized output",
        ));
        return Ok(false);
    }
    if vm.icy_board_state.ppl_terminal.begin_update() {
        write_vt_sequence(vm, "\x1B[?2026h").await?;
    }
    vm.operation_succeeded();
    Ok(true)
}

pub async fn end_terminal_update(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    terminal_end_update(vm).await?;
    Ok(())
}

pub(crate) async fn terminal_end_update(vm: &mut VirtualMachine<'_>) -> Res<bool> {
    let Some(outermost) = vm.icy_board_state.ppl_terminal.end_update() else {
        vm.set_error(PplError::new(ERR_KIND_TERM, ERR_INVALID, "no terminal update is active"));
        return Ok(false);
    };
    if outermost {
        write_vt_sequence(vm, "\x1B[?2026l").await?;
    }
    vm.operation_succeeded();
    Ok(true)
}

fn macro_slot(vm: &mut VirtualMachine<'_>, slot: i32) -> Option<usize> {
    if let Ok(slot) = usize::try_from(slot)
        && slot < crate::icy_board::state::ppl_terminal_control::MACRO_SLOTS
    {
        return Some(slot);
    }
    vm.set_error(PplError::new(ERR_KIND_TERM, ERR_INVALID, format!("invalid terminal macro slot {slot}")));
    None
}

fn terminal_macros_available(vm: &mut VirtualMachine<'_>) -> bool {
    if supports_vt_sequences(vm) && vm.icy_board_state.session.term_caps.terminal_macros != Some(false) {
        return true;
    }
    vm.set_error(PplError::new(ERR_KIND_TERM, ERR_UNAVAILABLE, "the terminal does not support DEC macros"));
    false
}

pub async fn record_macro(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    macros_record(vm, slot).await?;
    Ok(())
}

pub(crate) async fn macros_record(vm: &mut VirtualMachine<'_>, slot: i32) -> Res<bool> {
    let Some(slot) = macro_slot(vm, slot) else {
        return Ok(false);
    };
    if !terminal_macros_available(vm) {
        return Ok(false);
    }
    if !vm.icy_board_state.ppl_terminal.start_recording(slot) {
        vm.set_error(PplError::new(ERR_KIND_TERM, ERR_INVALID, "a terminal macro recording is already active"));
        return Ok(false);
    }
    vm.operation_succeeded();
    Ok(true)
}

pub async fn end_macro(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    macros_end(vm).await?;
    Ok(())
}

pub(crate) async fn macros_end(vm: &mut VirtualMachine<'_>) -> Res<bool> {
    match vm.icy_board_state.finish_ppl_macro().await? {
        None => {
            vm.set_error(PplError::new(ERR_KIND_TERM, ERR_INVALID, "no terminal macro recording is active"));
            return Ok(false);
        }
        Some(false) => {
            vm.set_error(PplError::new(ERR_KIND_TERM, ERR_LIMIT, "terminal macro exceeds the 512 KiB limit"));
            return Ok(false);
        }
        Some(true) => {}
    }
    vm.operation_succeeded();
    Ok(true)
}

pub async fn play_macro(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    macros_play(vm, slot).await?;
    Ok(())
}

pub(crate) async fn macros_play(vm: &mut VirtualMachine<'_>, slot: i32) -> Res<bool> {
    let Some(slot) = macro_slot(vm, slot) else {
        return Ok(false);
    };
    if !terminal_macros_available(vm) {
        return Ok(false);
    }
    if !vm.icy_board_state.play_ppl_macro(slot).await? {
        vm.set_error(PplError::new(ERR_KIND_TERM, ERR_INVALID, format!("terminal macro slot {slot} is empty")));
        return Ok(false);
    }
    vm.operation_succeeded();
    Ok(true)
}

pub async fn delete_macro(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    macros_delete(vm, slot).await?;
    Ok(())
}

pub(crate) async fn macros_delete(vm: &mut VirtualMachine<'_>, slot: i32) -> Res<bool> {
    let Some(slot) = macro_slot(vm, slot) else {
        return Ok(false);
    };
    if !terminal_macros_available(vm) {
        return Ok(false);
    }
    if vm.icy_board_state.ppl_terminal.is_recording() {
        vm.set_error(PplError::new(ERR_KIND_TERM, ERR_INVALID, "cannot delete a terminal macro while recording"));
        return Ok(false);
    }
    vm.icy_board_state.delete_ppl_macro(slot).await?;
    vm.operation_succeeded();
    Ok(true)
}

pub async fn clear_macros(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    macros_clear(vm).await?;
    Ok(())
}

pub(crate) async fn macros_clear(vm: &mut VirtualMachine<'_>) -> Res<bool> {
    if !terminal_macros_available(vm) {
        return Ok(false);
    }
    if vm.icy_board_state.ppl_terminal.is_recording() {
        vm.set_error(PplError::new(ERR_KIND_TERM, ERR_INVALID, "cannot clear terminal macros while recording"));
        return Ok(false);
    }
    vm.icy_board_state.clear_ppl_macros().await?;
    vm.operation_succeeded();
    Ok(true)
}

/// Stops every channel a PPE started, the way ending the PPE would.
pub(crate) async fn sound_stop_all(vm: &mut VirtualMachine<'_>) -> Res<bool> {
    for logical_channel in 0..vm.icy_board_state.sound_active.len() {
        if vm.icy_board_state.sound_active[logical_channel] {
            let channel = logical_channel + 2;
            let command = format!("\x1b_SyncTERM:A;Flush;C={channel};O=0\x1b\\");
            vm.icy_board_state.connection.send(command.as_bytes()).await?;
            vm.icy_board_state.sound_active[logical_channel] = false;
        }
    }
    vm.operation_succeeded();
    Ok(true)
}

/// 256 glyphs of at most 32 rows is 8K, and PSF2 adds a header and a Unicode table.
const MAX_FONT_FILE_BYTES: u64 = 64 * 1024;

/// The attribute classes `CTerm` binds fonts to: normal, bold, blink, bold+blink.
const FONT_SLOT_COUNT: i32 = 4;

/// `FONT_ALL`: binds every attribute class at once, so colour cannot change the typeface.
const FONT_ALL_SLOTS: i32 = -1;

/// `CTerm`'s `CONIO_FIRST_FREE_FONT`: it drops an upload below this silently, so the
/// built-in fonts are read-only.
const FIRST_FREE_FONT: i32 = 43;
const LAST_FONT: i32 = 255;

pub async fn set_font(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let font = vm.eval_expr(&args[1]).await?.as_int();
    font_set(vm, slot, font).await?;
    Ok(())
}

pub(crate) async fn font_set(vm: &mut VirtualMachine<'_>, slot: i32, font: i32) -> Res<bool> {
    let slot_valid = slot == FONT_ALL_SLOTS || (0..FONT_SLOT_COUNT).contains(&slot);
    if !slot_valid || !(0..=LAST_FONT).contains(&font) {
        log::warn!("font selection ignored: invalid slot {slot} / font {font}");
        vm.set_error(PplError::new(ERR_KIND_FONT, ERR_INVALID, format!("invalid font slot {slot} or font {font}")));
        return Ok(false);
    }
    if !supports_vt_sequences(vm) {
        vm.set_error(PplError::new(ERR_KIND_FONT, ERR_UNAVAILABLE, "the terminal has no fonts"));
        return Ok(false);
    }
    let (first_slot, last_slot) = if slot == FONT_ALL_SLOTS { (0, FONT_SLOT_COUNT - 1) } else { (slot, slot) };
    for slot in first_slot..=last_slot {
        write_vt_sequence(vm, &format!("\x1B[{slot};{font} D")).await?;
    }
    vm.operation_succeeded();
    Ok(true)
}

pub async fn load_font(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let font = vm.eval_expr(&args[0]).await?.as_int();
    let file_name = vm.eval_expr(&args[1]).await?.as_string();
    font_load(vm, font, &file_name).await?;
    Ok(())
}

pub(crate) async fn font_load(vm: &mut VirtualMachine<'_>, font: i32, file_name: &str) -> Res<bool> {
    if !(FIRST_FREE_FONT..=LAST_FONT).contains(&font) {
        log::warn!("font upload ignored: font {font} is not a free slot");
        vm.set_error(PplError::new(ERR_KIND_FONT, ERR_INVALID, format!("font {font} is not a free slot")));
        return Ok(false);
    }
    if !supports_vt_sequences(vm) {
        vm.set_error(PplError::new(ERR_KIND_FONT, ERR_UNAVAILABLE, "the terminal has no fonts"));
        return Ok(false);
    }

    let path = vm.resolve_file(&file_name).await;
    let data = match fs::metadata(&path).and_then(|meta| {
        if meta.len() > MAX_FONT_FILE_BYTES {
            Err(std::io::Error::other("file too large"))
        } else {
            fs::read(&path)
        }
    }) {
        Ok(data) => data,
        Err(err) => {
            log::warn!("Can't load font {}: {err}", path.display());
            vm.set_error(PplError::new(ERR_KIND_FONT, ERR_IO, format!("can't read font {file_name}: {err}")));
            return Ok(false);
        }
    };

    let Ok(bit_font) = icy_engine::BitFont::from_bytes(file_name, &data) else {
        log::warn!("Unknown font format in {}", path.display());
        vm.set_error(PplError::new(ERR_KIND_FONT, ERR_FORMAT, format!("unknown font format in {file_name}")));
        return Ok(false);
    };

    let sequence = bit_font.encode_as_ansi(font as usize);
    if !vm.icy_board_state.reserve_media_upload(sequence.len()) {
        log::warn!("Font upload budget exhausted");
        vm.set_error(PplError::new(ERR_KIND_FONT, ERR_LIMIT, "font upload budget exhausted"));
        return Ok(false);
    }
    vm.icy_board_state.connection.send(sequence.as_bytes()).await?;
    vm.icy_board_state.acknowledge_upload(sequence.len()).await?;
    // The upload bypasses the virtual screen, so it has to be told separately or a later
    // font selection finds no such font.
    vm.icy_board_state.display_screen_mut().buffer.buffer.set_font(font as u8, bit_font);
    vm.operation_succeeded();
    Ok(true)
}

pub async fn backup(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let numcols = vm.eval_expr(&args[0]).await?.as_int();
    if vm.icy_board_state.use_ansi() {
        vm.icy_board_state.print(TerminalTarget::Both, &format!("\x1B[{numcols}D")).await
    } else {
        vm.icy_board_state.print(TerminalTarget::Both, &"\x08".repeat(numcols as usize)).await
    }
}

pub async fn forward(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let numcols = vm.eval_expr(&args[0]).await?.as_int();
    if vm.icy_board_state.use_ansi() {
        vm.icy_board_state.print(TerminalTarget::Both, &format!("\x1B[{numcols}C")).await?;
    } else {
        vm.icy_board_state.print(TerminalTarget::Both, &" ".repeat(numcols as usize)).await?;
    }
    Ok(())
}

pub async fn freshline(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.fresh_line().await?;
    Ok(())
}

/// `WRUSYSDOOR doorname`
///
/// The same drop file as `WRUSYS`, but carrying a TPA record naming the door
/// that is about to be run.
pub async fn wrusysdoor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let door_name = vm.eval_expr(&args[0]).await?.as_string();
    write_user_sys(vm, &door_name).await
}

/// `WRUSYS` - writes USER.SYS so an external program can read the caller.
pub async fn wrusys(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    write_user_sys(vm, "").await
}

async fn write_user_sys(vm: &mut VirtualMachine<'_>, tpa_name: &str) -> Res<()> {
    let path = user_sys_dir(vm).await;
    if let Err(err) = crate::icy_board::doors::pcboard::create_user_sys(vm.icy_board_state, &path, tpa_name).await {
        log::error!("Can't write USER.SYS to {}: {}", path.display(), err);
    }
    Ok(())
}

/// `RDUSYS` - takes back whatever the external program changed in USER.SYS.
pub async fn rdusys(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let path = user_sys_dir(vm).await;
    if let Err(err) = crate::icy_board::doors::pcboard::read_user_sys(&mut vm.user, &path) {
        log::error!("Can't read USER.SYS from {}: {}", path.display(), err);
        return Ok(());
    }
    vm.icy_board_state.session.cur_security = vm.user.security_level;
    vm.icy_board_state.session.page_len = vm.user.page_len;
    Ok(())
}

/// USER.SYS lives beside the board, the way a node work directory used to hold it.
async fn user_sys_dir(vm: &mut VirtualMachine<'_>) -> std::path::PathBuf {
    vm.icy_board_state.get_board().await.root_path.clone()
}
pub async fn newpwd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let new_pwd = vm.eval_expr(&args[0]).await?.as_string();
    let was_changed = vm.icy_board_state.change_password(&new_pwd).await?;
    vm.set_variable(&args[1], VariableValue::new_bool(was_changed)).await?;
    Ok(())
}
/// `OPENCAP file, ocFlag`
///
/// Starts teeing everything the caller would see into `file`, and reports
/// through `ocFlag` whether that worked.
pub async fn opencap(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    let path = vm.resolve_file(&file_name).await;
    let opened = vm.icy_board_state.open_capture(&path);
    vm.set_variable(&args[1], VariableValue::new_bool(opened)).await?;
    Ok(())
}

/// `CLOSECAP` - stops the tee started by `OPENCAP`.
pub async fn closecap(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.close_capture();
    Ok(())
}

pub async fn message(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let (conf, area) = vm.eval_expr(&args[0]).await?.as_msg_id();
    let to = vm.eval_expr(&args[1]).await?.as_string();
    let from = vm.eval_expr(&args[2]).await?.as_string();
    let subject = vm.eval_expr(&args[3]).await?.as_string();
    let sec = vm.eval_expr(&args[4]).await?.as_string();
    let pack_out_date = vm.eval_expr(&args[5]).await?.as_int() as u32;
    let retreceipt = vm.eval_expr(&args[6]).await?.as_bool();
    let echo = vm.eval_expr(&args[7]).await?.as_bool();
    let file = vm.eval_expr(&args[8]).await?.as_string();
    let file = vm.resolve_file(&file).await;
    // PCBoard's entermessagefromfile() returns quietly when the body file is missing.
    if !file.exists() {
        log::error!("PPE function 'message': message text file not found {}", file.display());
        return Ok(());
    }

    // An empty name stands for the caller, and "R" asks for a message only its
    // receiver may read - anything else is the public "N".
    let caller = vm.icy_board_state.session.get_username_or_alias();
    let to = if to.is_empty() { caller.clone() } else { to };
    let from = if from.is_empty() { caller } else { from };

    let mut attributes = 0;
    if sec.trim().to_ascii_uppercase().starts_with('R') {
        attributes |= jam_attributes::MSG_PRIVATE;
    }
    if retreceipt {
        attributes |= jam_attributes::MSG_RECEIPTREQ;
    }
    if echo {
        attributes |= jam_attributes::MSG_TYPEECHO;
    }

    let mut message = JamMessage::default()
        .with_from(BString::from(from))
        .with_to(BString::from(to))
        .with_subject(BString::from(subject))
        .with_date_time(Utc::now())
        .with_attributes(attributes)
        .with_text(BString::from(fs::read_to_string(file)?));
    if pack_out_date > 0 {
        message = message.with_packout_date(IcbDate::from_pcboard(pack_out_date).to_utc_date_time());
    }

    if conf >= 0 {
        vm.icy_board_state.send_message(conf, area, message, IceText::SavingMessage).await?;
    } else {
        vm.icy_board_state.send_message(-1, 0, message, IceText::SavingMessage).await?;
    }
    vm.invalidate_message_base();
    Ok(())
}

pub async fn savescrn(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut buf = vm.icy_board_state.display_screen().buffer.buffer.clone();
    buf.buffer_type = BufferType::Unicode;
    vm.stored_screen = Some(buf);
    Ok(())
}

pub async fn restscrn(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    if let Some(screen) = &mut vm.stored_screen {
        let res = screen_to_pcboard_text(screen)?;
        vm.icy_board_state.print(TerminalTarget::Both, &res).await?;
    }
    Ok(())
}
pub async fn sound(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::warn!("SOUND is not supported");
    Ok(())
}

pub async fn chat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.page_sysop().await?;
    Ok(())
}

pub async fn sprint(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value).await?.as_string();
        vm.icy_board_state.print(TerminalTarget::Sysop, txt).await?;
    }
    Ok(())
}

pub async fn sprintln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value).await?.as_string();
        vm.icy_board_state.print(TerminalTarget::Sysop, txt).await?;
    }
    vm.icy_board_state.print(TerminalTarget::Sysop, "\n").await?;
    Ok(())
}

pub async fn print(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value).await?.as_string();
        vm.icy_board_state.print(TerminalTarget::Both, txt).await?;
    }

    Ok(())
}

pub async fn println(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value).await?.as_string();
        vm.icy_board_state.print(TerminalTarget::Both, txt).await?;
    }
    vm.icy_board_state.new_line().await?;
    Ok(())
}

pub async fn mprint(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value).await?.as_string();
        vm.icy_board_state.print(TerminalTarget::User, txt).await?;
    }
    Ok(())
}

pub async fn mprintln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value).await?.as_string();
        vm.icy_board_state.print(TerminalTarget::User, txt).await?;
    }
    vm.icy_board_state.print(TerminalTarget::User, "\n").await?;
    Ok(())
}

pub async fn rename(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let old = &vm.eval_expr(&args[0]).await?.as_string();
    let new = &vm.eval_expr(&args[1]).await?.as_string();
    let old = vm.resolve_file(&old).await.to_string_lossy().to_string();
    let new = vm.resolve_file(&new).await.to_string_lossy().to_string();

    if let Err(err) = vm.io.rename(&old, &new) {
        log::error!("Error renaming file: {err}");
    }
    Ok(())
}
pub async fn frewind(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    vm.io.frewind(channel)?;
    Ok(())
}
pub async fn pokedw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("POKEDW");
}
pub async fn dbglevel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.debug_level = vm.eval_expr(&args[0]).await?.as_int();
    Ok(())
}
pub async fn showon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.disp_options.display_text = true;
    Ok(())
}
pub async fn showoff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.disp_options.display_text = false;
    Ok(())
}

pub async fn pageon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.paged_sysop = true;
    Ok(())
}

pub async fn pageoff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.paged_sysop = false;
    Ok(())
}

pub async fn fseek(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let pos = vm.eval_expr(&args[1]).await?.as_int();
    let position = vm.eval_expr(&args[2]).await?.as_int();
    vm.io.fseek(channel, pos, position)?;
    Ok(())
}

pub async fn fflush(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    vm.io.fflush(channel)?;
    Ok(())
}
pub async fn fread(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let size = vm.eval_expr(&args[2]).await?.as_int() as usize;
    internal_fread(vm, channel, size, &args[1]).await
}

/// A read that hit the end of the file comes back short, so the bytes that are
/// missing count as zero instead of taking the node down.
fn read_bytes<const N: usize>(data: &[u8]) -> [u8; N] {
    let mut bytes = [0u8; N];
    let len = data.len().min(N);
    bytes[..len].copy_from_slice(&data[..len]);
    bytes
}

async fn internal_fread(vm: &mut VirtualMachine<'_>, channel: i32, size: usize, arg: &PPEExpr) -> Res<()> {
    let val = vm.eval_expr(arg).await?;

    let result = vm.io.fread(channel, size)?;

    match val.get_type() {
        VariableType::String | VariableType::BigStr => {
            let mut vs = String::new();
            for c in result {
                if c == 0 {
                    break;
                }
                vs.push(CP437_TO_UNICODE[c as usize]);
            }
            vm.set_variable(arg, VariableValue::new_string(vs)).await?;
        }
        VariableType::Boolean => {
            vm.set_variable(arg, VariableValue::new_bool(read_bytes::<1>(&result)[0] != 0)).await?;
        }
        VariableType::Byte | VariableType::SByte => {
            vm.set_variable(arg, VariableValue::new_byte(read_bytes::<1>(&result)[0])).await?;
        }
        VariableType::Word | VariableType::SWord => {
            vm.set_variable(arg, VariableValue::new_word(u16::from_le_bytes(read_bytes(&result)))).await?;
        }
        VariableType::Double => {
            vm.set_variable(arg, VariableValue::new_double(f64::from_le_bytes(read_bytes(&result)))).await?;
        }
        _ => match result.len() {
            0 => {
                vm.set_variable(arg, VariableValue::new_int(0)).await?;
            }
            1 => {
                vm.set_variable(arg, VariableValue::new_int(result[0] as i32)).await?;
            }
            2 => {
                vm.set_variable(arg, VariableValue::new_int(i16::from_le_bytes(result[..2].try_into().unwrap()) as i32))
                    .await?;
            }
            4 => {
                vm.set_variable(arg, VariableValue::new_int(i32::from_le_bytes(result[..4].try_into().unwrap())))
                    .await?;
            }
            _ => {
                log::error!("fread: invalid size: {}", result.len());
            }
        },
    }
    Ok(())
}

pub async fn fwrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let val = vm.eval_expr(&args[1]).await?;
    let size = vm.eval_expr(&args[2]).await?.as_int() as usize;
    internal_fwrite(vm, channel, val, size).await
}

fn record_io_error(vm: &mut VirtualMachine<'_>, channel: i32, code: i32, message: String) {
    vm.io.record_error(channel, message.clone());
    vm.set_error(PplError::new(ERR_KIND_FILE, code, message).on_channel(channel));
}

pub async fn fgetrec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let template = vm.eval_expr(&args[1]).await?;
    let count = match crate::vm::record_io::line_count(&template) {
        Ok(count) => count,
        Err(message) => {
            record_io_error(vm, channel, ERR_FORMAT, message);
            return Ok(());
        }
    };
    let mut lines = Vec::with_capacity(count);
    for _ in 0..count {
        let line = vm.io.fget(channel)?;
        if vm.io.ferr(channel) {
            record_io_error(vm, channel, ERR_FORMAT, "text record is truncated".to_string());
            return Ok(());
        }
        lines.push(line);
    }
    match crate::vm::record_io::decode_lines(&template, &lines) {
        Ok(value) => vm.set_variable(&args[1], value).await?,
        Err(message) => record_io_error(vm, channel, ERR_FORMAT, message),
    }
    Ok(())
}

pub async fn fputrec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let value = vm.eval_expr(&args[1]).await?;
    match crate::vm::record_io::encode_lines(&value) {
        Ok(lines) => {
            for line in lines {
                vm.io.fput(channel, format!("{line}\n"))?;
            }
        }
        Err(message) => record_io_error(vm, channel, ERR_FORMAT, message),
    }
    Ok(())
}

pub async fn freadrec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let template = vm.eval_expr(&args[1]).await?;
    if !crate::vm::record_io::is_record(&template) {
        record_io_error(vm, channel, ERR_FORMAT, "a user-defined record is required".to_string());
        return Ok(());
    }
    let header = vm.io.fread(channel, 4)?;
    if header.len() != 4 {
        record_io_error(vm, channel, ERR_FORMAT, "binary record length is truncated".to_string());
        return Ok(());
    }
    let length = u32::from_le_bytes(header.try_into().unwrap()) as usize;
    if length > crate::vm::record_io::MAX_RECORD_FRAME {
        record_io_error(vm, channel, ERR_LIMIT, "record exceeds the 16 MiB frame limit".to_string());
        return Ok(());
    }
    let payload = vm.io.fread(channel, length)?;
    if payload.len() != length {
        record_io_error(vm, channel, ERR_FORMAT, "binary record is truncated".to_string());
        return Ok(());
    }
    match crate::vm::record_io::decode_binary(&template, &payload) {
        Ok(value) => vm.set_variable(&args[1], value).await?,
        Err(message) => record_io_error(vm, channel, ERR_FORMAT, message),
    }
    Ok(())
}

pub async fn fwriterec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let value = vm.eval_expr(&args[1]).await?;
    match crate::vm::record_io::encode_binary(&value) {
        Ok(frame) => vm.io.fwrite(channel, &frame)?,
        Err(message) => {
            let code = if message.contains("16 MiB") { ERR_LIMIT } else { ERR_FORMAT };
            record_io_error(vm, channel, code, message);
        }
    }
    Ok(())
}

pub async fn string_split(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let text = vm.eval_expr(&args[0]).await?.as_string();
    let separator = vm.eval_expr(&args[1]).await?.as_string();
    let target = vm.eval_array_operand(&args[2]).await?;
    let limit = vm.eval_expr(&args[3]).await?.as_int();
    if separator.is_empty() {
        vm.set_error(PplError::new(ERR_KIND_STRING, ERR_INVALID, "STRING.Split separator cannot be empty"));
        return Ok(());
    }
    if limit < 0 {
        vm.set_error(PplError::new(ERR_KIND_STRING, ERR_INVALID, "STRING.Split limit cannot be negative"));
        return Ok(());
    }
    if !matches!(target.vtype, VariableType::String | VariableType::BigStr) || !matches!(target.generic_data, GenericVariableData::Dim1(_)) {
        vm.set_error(PplError::new(
            ERR_KIND_STRING,
            ERR_INVALID,
            "STRING.Split needs a dynamic one-dimensional string array",
        ));
        return Ok(());
    }
    let parts: Vec<_> = if limit == 0 {
        text.split(&separator).map(str::to_string).collect()
    } else {
        text.splitn(limit as usize, &separator).map(str::to_string).collect()
    };
    let values = parts.into_iter().map(|part| VariableValue::new_string(part).convert_to(target.vtype)).collect();
    vm.set_variable(&args[2], VariableValue::new_vector(target.vtype, values)).await?;
    vm.operation_succeeded();
    Ok(())
}

async fn internal_fwrite(vm: &mut VirtualMachine<'_>, channel: i32, val: VariableValue, size: usize) -> Res<()> {
    let mut v = match val.get_type() {
        VariableType::String | VariableType::BigStr => val.as_string().as_bytes().to_vec(),
        VariableType::Boolean => {
            if val.as_bool() {
                vec![1]
            } else {
                vec![0]
            }
        }
        VariableType::Byte | VariableType::SByte => unsafe { vec![val.data.byte_value] },
        VariableType::Word | VariableType::SWord => unsafe { val.data.word_value.to_le_bytes().to_vec() },
        VariableType::Double => unsafe { val.data.double_value.to_le_bytes().to_vec() },
        _ => unsafe { val.data.int_value.to_le_bytes().to_vec() },
    };

    while v.len() < size {
        v.push(0);
    }
    vm.io.fwrite(channel, &v).map_err(|e| {
        log::error!("fwrite error: {e} ({channel})");
        e
    })?;
    Ok(())
}

pub async fn fdefin(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    vm.fd_default_in = channel;
    Ok(())
}
pub async fn fdefout(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    vm.fd_default_out = channel;
    Ok(())
}
pub async fn fdget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = VariableValue::new_string(vm.io.fget(vm.fd_default_in)?);
    vm.set_variable(&args[0], value).await?;
    Ok(())
}

pub async fn fdput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let text = vm.eval_expr(value).await?.as_string();
        vm.io.fput(vm.fd_default_out, text)?;
    }
    Ok(())
}
pub async fn fdputln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let text = vm.eval_expr(value).await?.as_string();
        vm.io.fput(vm.fd_default_out, text)?;
    }
    vm.io.fput(vm.fd_default_out, "\n".to_string())?;
    Ok(())
}

pub async fn fdputpad(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let text = vm.eval_expr(&args[0]).await?.as_string();
    let width = vm.eval_expr(&args[1]).await?.as_int();
    fputpad_internal(vm, vm.fd_default_out, text, width)
}

pub async fn fdread(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let size = vm.eval_expr(&args[1]).await?.as_int() as usize;
    internal_fread(vm, vm.fd_default_in, size, &args[0]).await
}

pub async fn fdwrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let val = vm.eval_expr(&args[0]).await?;
    let size = vm.eval_expr(&args[1]).await?.as_int() as usize;
    internal_fwrite(vm, vm.fd_default_out, val, size).await
}

pub async fn adjbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let bytes = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if bytes > 0 {
            user.stats.total_dnld_bytes = user.stats.total_dnld_bytes.saturating_add(bytes as u64);
        } else {
            user.stats.total_dnld_bytes = user.stats.total_dnld_bytes.saturating_sub(bytes.unsigned_abs() as u64);
        }
        user.stats.today_dnld_bytes = user.stats.today_dnld_bytes.saturating_add(bytes as i64);
        crate::icy_board::limits::adjust_bytes_remaining(&mut vm.icy_board_state.session.bytes_remaining, bytes as i64);
    }
    Ok(())
}

pub async fn alias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.use_alias = vm.eval_expr(&args[0]).await?.as_bool();
    Ok(())
}
pub async fn redim(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let var = vm.eval_expr(&args[0]).await?;
    let dim1 = vm.eval_expr(&args[1]).await?.as_int() as usize;
    let dim2 = if args.len() > 2 { vm.eval_expr(&args[2]).await?.as_int() as usize } else { 0 };
    let dim3 = if args.len() > 3 { vm.eval_expr(&args[3]).await?.as_int() as usize } else { 0 };

    if let PPEExpr::Value(id) = args[0] {
        vm.variable_table.get_value_mut(id).redim((args.len() - 1) as u8, dim1, dim2, dim3);
    } else {
        log::error!("redim arg[0] != variable");
    }
    Ok(())
}
pub async fn append(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = get_file_channel(vm, args).await?;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    vm.io.fappend(channel, &file);
    Ok(())
}

pub async fn copy(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let old = &vm.eval_expr(&args[0]).await?.as_string();
    let new = &vm.eval_expr(&args[1]).await?.as_string();
    if let Err(err) = vm.io.copy(old, new) {
        log::error!("Error renaming file: {err}");
    }
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn kbdflush(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // TODO?
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn mdmflush(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // TODO?
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn keyflush(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.flush_keyboard_input().await
}
pub async fn lastin(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conf = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        user.last_conference = conf as u16;
    }
    Ok(())
}
pub async fn flag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let files = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.session.push_tokens(&files);
    vm.icy_board_state.flag_files_cmd(false).await?;
    Ok(())
}

pub async fn download(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let files = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.session.push_tokens(&files);
    vm.icy_board_state.download(true).await?;
    Ok(())
}

pub async fn getaltuser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let user_record = vm.eval_expr(&args[0]).await?.as_int();
    if user_record <= 0 || user_record as usize > vm.icy_board_state.get_board().await.users.len() {
        // it's expected behavior, user record is unchanged.
        log::warn!("PPE getaltuser: invalid user record #{user_record}");
        return Ok(());
    }
    vm.user = vm.icy_board_state.get_board().await.users[user_record as usize - 1].clone();
    log::info!("PPE getaltuser: switched to user #{} ({})", user_record, vm.user.name);
    vm.set_user_variables()?;
    Ok(())
}

pub async fn adjdbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let bytes = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        user.stats.today_dnld_bytes = user.stats.today_dnld_bytes.saturating_add(bytes as i64);
        crate::icy_board::limits::adjust_bytes_remaining(&mut vm.icy_board_state.session.bytes_remaining, bytes as i64);
    }
    Ok(())
}
pub async fn adjtbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let bytes: i32 = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if bytes > 0 {
            user.stats.total_dnld_bytes = user.stats.total_dnld_bytes.saturating_add(bytes as u64);
        } else {
            user.stats.total_dnld_bytes = user.stats.total_dnld_bytes.saturating_sub(bytes.unsigned_abs() as u64);
        }
    }
    Ok(())
}
pub async fn ayjtfiles(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let files = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if files > 0 {
            user.stats.num_downloads = user.stats.num_downloads.saturating_add(files as u64);
        } else {
            user.stats.num_downloads = user.stats.num_downloads.saturating_sub(files.unsigned_abs() as u64);
        }
    }
    Ok(())
}

pub async fn lang(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let language = vm.eval_expr(&args[0]).await?.as_int();
    let lang = if let Some(lang) = vm.icy_board_state.board.lock().await.languages.get(language as usize) {
        lang.extension.clone()
    } else {
        log::error!("PPE: lang(): Language not found: {language}");
        return Ok(());
    };
    vm.icy_board_state.session.language.clone_from(&lang);
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        user.language = lang;
    }
    Ok(())
}

pub async fn sort(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let PPEExpr::Value(array_idx) = args[0] else {
        return Err(Box::new(VMError::InternalVMError));
    };
    let PPEExpr::Value(indices_idx) = args[1] else {
        return Err(Box::new(VMError::InternalVMError));
    };

    let array = vm.variable_table.get_value(array_idx);
    {
        let indices = vm.variable_table.get_value(indices_idx);
        if indices.vtype != VariableType::Integer {
            return Err(Box::new(IcyError::SortDestinationArrayIntRequired(indices.vtype)));
        }
    }

    let vs = array.get_vector_size() + 1;
    let dim = array.get_dimensions();
    let mut target_indices = (0..vs).collect::<Vec<usize>>();
    for i in 0..vs {
        for j in i + 1..vs {
            let left = array.get_array_value(target_indices[i], 0, 0);
            let right = array.get_array_value(target_indices[j], 0, 0);
            if left > right {
                target_indices.swap(i, j);
            }
        }
    }
    let indices = vm.variable_table.get_value_mut(indices_idx);
    indices.redim(dim, vs, 0, 0);
    for (i, target_index) in target_indices.iter().enumerate() {
        indices.set_array_value(i, 0, 0, VariableValue::new_int(*target_index as i32))?;
    }
    Ok(())
}

pub async fn mousereg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // RIP ONLY
    // num    = Is the RIP region number
    // x1,y1  = The (X,Y) coordinates of the upper-left of the region
    // x2,y2  = The (X,Y) coordinates of the lower-right of the region
    // fontX  = The width of each character in pixels
    // fontY  = The height of each character in pixels
    // invert = A boolean flag (TRUE to invert the region when clicked)
    // clear  = A boolean flag (TRUE to clear and full screen the text window)
    // text   = Text that the remote terminal should transmit when the region is clicked

    let num = vm.eval_expr(&args[0]).await?.as_int();
    let x1 = vm.eval_expr(&args[1]).await?.as_int();
    let y1 = vm.eval_expr(&args[2]).await?.as_int();
    let x2 = vm.eval_expr(&args[3]).await?.as_int();
    let y2 = vm.eval_expr(&args[4]).await?.as_int();
    let font_x = vm.eval_expr(&args[5]).await?.as_int();
    let font_y = vm.eval_expr(&args[6]).await?.as_int();
    let invert = vm.eval_expr(&args[7]).await?.as_bool();
    let clear = vm.eval_expr(&args[8]).await?.as_bool();
    let text = vm.eval_expr(&args[9]).await?.as_string();

    vm.set_rip_mouseregion(num, x1, y1, x2, y2, font_x, font_y, invert, clear, text).await
}

pub async fn scrfile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let line = vm.eval_expr(&args[0]).await?.as_int() - 1;
    if let Some((line, name)) = vm.icy_board_state.scan_filename(line) {
        log::info!("found name {line}:{name}");
        vm.set_variable(&args[0], VariableValue::new_int(line + 1)).await?;
        vm.set_variable(&args[1], VariableValue::new_string(name)).await?;
    } else {
        vm.set_variable(&args[0], VariableValue::new_int(0)).await?;
    }
    Ok(())
}

pub async fn searchinit(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let pattern = vm.eval_expr(&args[0]).await?.as_string();
    let case_sensitive = vm.eval_expr(&args[1]).await?.as_bool();
    vm.icy_board_state.search_init(pattern, case_sensitive);
    Ok(())
}

pub async fn searchfind(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let text = vm.eval_expr(&args[0]).await?.as_string();

    let res = if let Some(regex) = &vm.icy_board_state.session.search_pattern {
        regex.is_match(&text)
    } else {
        false
    };

    vm.set_variable(&args[1], VariableValue::new_bool(res)).await?;
    Ok(())
}

pub async fn searchstop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.stop_search();
    Ok(())
}

pub async fn prfound(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value).await?.as_string();
        vm.icy_board_state.print_found_text(TerminalTarget::Both, txt).await?;
    }
    Ok(())
}
pub async fn prfoundln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value).await?.as_string();
        vm.icy_board_state.print_found_text(TerminalTarget::Both, txt).await?;
    }
    vm.icy_board_state.new_line().await?;
    Ok(())
}

/// `TPAGET keyword, var` - static third party application storage.
pub async fn tpaget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    read_tpa(vm, args, None).await
}
/// `TPAPUT keyword, expr`
pub async fn tpaput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    write_tpa(vm, args, None).await
}
/// `TPACGET keyword, var, conf` - the same storage, kept per conference.
pub async fn tpacgea(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conference = vm.eval_expr(&args[2]).await?.as_int();
    read_tpa(vm, args, Some(conference)).await
}
/// `TPACPUT keyword, expr, conf`
pub async fn tpacput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conference = vm.eval_expr(&args[2]).await?.as_int();
    write_tpa(vm, args, Some(conference)).await
}
/// `TPAREAD keyword, var` - the same record as `TPAGET`, but the value comes
/// back as whatever type the receiving variable was declared with.
pub async fn tparead(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    read_tpa(vm, args, None).await
}
/// `TPAWRITE keyword, expr`
pub async fn tpawrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    write_tpa(vm, args, None).await
}
/// `TPACREAD keyword, var, conf`
pub async fn tpacread(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conference = vm.eval_expr(&args[2]).await?.as_int();
    read_tpa(vm, args, Some(conference)).await
}
/// `TPACWRITE keyword, expr, conf`
pub async fn tpacwrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conference = vm.eval_expr(&args[2]).await?.as_int();
    write_tpa(vm, args, Some(conference)).await
}

async fn read_tpa(vm: &mut VirtualMachine<'_>, args: &[PPEExpr], conference: Option<i32>) -> Res<()> {
    let keyword = vm.eval_expr(&args[0]).await?.as_string();
    let Some(user) = &vm.icy_board_state.session.current_user else {
        return Ok(());
    };
    let data = match conference {
        Some(conference) => user.get_conference_tpa(&keyword, conference.max(0) as usize),
        None => user.get_tpa(&keyword),
    };
    // The variable table converts to the declared type, so a numeric record
    // read into an INTEGER comes back as one.
    let value = VariableValue::new_string(data.to_string());
    vm.set_variable(&args[1], value).await
}

async fn write_tpa(vm: &mut VirtualMachine<'_>, args: &[PPEExpr], conference: Option<i32>) -> Res<()> {
    let keyword = vm.eval_expr(&args[0]).await?.as_string();
    let data = vm.eval_expr(&args[1]).await?.as_string();
    let Some(user) = &mut vm.icy_board_state.session.current_user else {
        return Ok(());
    };
    match conference {
        Some(conference) => user.set_conference_tpa(&keyword, conference.max(0) as usize, &data),
        None => user.set_tpa(&keyword, &data),
    }
    Ok(())
}

pub async fn bitset(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let num_xp = vm.eval_expr(&args[0]).await?;
    if num_xp.get_type() == VariableType::String || num_xp.get_type() == VariableType::BigStr || num_xp.get_dimensions() > 0 {
        log::error!("bitset not supported on data type {}", num_xp.vtype);
        return Ok(());
    }
    let num = num_xp.as_unsigned();
    let bit = vm.eval_expr(&args[1]).await?.as_int();
    let num = num | (1 << bit);
    vm.set_variable(&args[0], VariableValue::new_unsigned(num).convert_to(num_xp.get_type()))
        .await?;
    Ok(())
}

pub async fn bitclear(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let num_xp = vm.eval_expr(&args[0]).await?;
    if num_xp.get_type() == VariableType::String || num_xp.get_type() == VariableType::BigStr || num_xp.get_dimensions() > 0 {
        log::error!("bitclear not supported on data type {}", num_xp.vtype);
        return Ok(());
    }
    let num = num_xp.as_unsigned();
    let bit = vm.eval_expr(&args[1]).await?.as_int();
    let num = num & !(1 << bit);
    vm.set_variable(&args[0], VariableValue::new_unsigned(num).convert_to(num_xp.get_type()))
        .await?;
    Ok(())
}

pub async fn brag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // According to PCB 15.3: No longer supported.
    Ok(())
}

pub async fn frealtuser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    if let Some(user) = vm.icy_board_state.session.current_user.clone() {
        vm.user = user;
        vm.set_user_variables()?;
    }
    Ok(())
}
/// `SETLMR conf, msg`
///
/// Moves the caller's last-message-read pointer. A conference or message number
/// past the end clamps to the highest one there is, so a PPE can ask for
/// "everything" without knowing the numbers.
pub async fn setlmr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let (conference, area) = vm.eval_expr(&args[0]).await?.as_msg_id();
    let requested = vm.eval_expr(&args[1]).await?.as_int().max(0) as u32;

    let conference = {
        let highest = vm.icy_board_state.get_board().await.conferences.len() as i32 - 1;
        conference.min(highest).max(0)
    };
    let Some(msg_base) = vm.message_base_path(conference, area).await else {
        log::error!("SETLMR: no message base {conference}:{area}");
        return Ok(());
    };
    let mut base = match JamMessageBase::open(&msg_base) {
        Ok(base) => base,
        Err(err) => {
            log::error!("SETLMR can't open message base {conference}:{area}: {err}");
            return Ok(());
        }
    };

    let highest = base.highest_message_number();
    let number = requested.min(highest);

    let crc = JamMessageBase::crc(&BString::from(vm.icy_board_state.session.user_name.to_lowercase()));
    let user_id = vm.icy_board_state.session.cur_user_id as u32;
    let mut last_read = match base.find_last_read(crc, user_id)? {
        Some(last_read) => last_read,
        None => base.create_last_read(crc, user_id)?,
    };
    last_read.last_read_msg = number;
    last_read.high_read_msg = last_read.high_read_msg.max(number);
    base.write_last_read(&last_read)?;
    Ok(())
}

pub async fn setenv(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let env = vm.eval_expr(&args[0]).await?.as_string();
    let v: Vec<&str> = env.split('=').collect();
    if v.len() == 2 {
        vm.icy_board_state.set_env(v[0], v[1]);
    } else {
        vm.icy_board_state.remove_env(&env);
    }
    Ok(())
}

pub async fn fcloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    for i in 0..MAX_FILE_CHANNELS {
        let _ = vm.io.fclose(i);
    }
    Ok(())
}
/// `STACKABORT abort`
///
/// Chooses whether a stack error ends the PPE or lets it keep going.
pub async fn stackabort(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.abort_on_stack_error = vm.eval_expr(&args[0]).await?.as_bool();
    Ok(())
}
pub async fn dcreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dcreate(vm, args).await?;
    Ok(())
}
pub async fn dopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dopen(vm, args).await?;
    Ok(())
}
pub async fn dclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dclose(vm, args).await?;
    Ok(())
}
pub async fn dsetalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dsetalias(vm, args).await?;
    Ok(())
}
pub async fn dpack(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dpack(vm, args).await?;
    Ok(())
}
pub async fn dcloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dcloseall(vm, args).await?;
    Ok(())
}
pub async fn dlock(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dlock(vm, args).await?;
    Ok(())
}
pub async fn dlockr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dlock(vm, args).await?;
    Ok(())
}
pub async fn dlockg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dlock(vm, args).await?;
    Ok(())
}
pub async fn dunlock(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dlock(vm, args).await?;
    Ok(())
}
pub async fn dncreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dncreate(vm, args).await?;
    Ok(())
}
pub async fn dnopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dnopen(vm, args).await?;
    Ok(())
}
pub async fn dnclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dnclose(vm, args).await?;
    Ok(())
}
pub async fn dncloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dncloseall(vm, args).await?;
    Ok(())
}
pub async fn dnew(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dnew(vm, args).await?;
    Ok(())
}
pub async fn dadd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dadd(vm, args).await?;
    Ok(())
}
pub async fn dappend(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dappend(vm, args).await?;
    Ok(())
}
pub async fn dtop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dtop(vm, args).await?;
    Ok(())
}
pub async fn dgo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dgo(vm, args).await?;
    Ok(())
}
pub async fn dbottom(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dbottom(vm, args).await?;
    Ok(())
}
pub async fn dskip(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dskip(vm, args).await?;
    Ok(())
}
pub async fn dblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dblank(vm, args).await?;
    Ok(())
}
pub async fn ddelete(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::ddelete(vm, args).await?;
    Ok(())
}
pub async fn drecall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::drecall(vm, args).await?;
    Ok(())
}
pub async fn dtag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dtag(vm, args).await?;
    Ok(())
}
pub async fn dseek(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dseek(vm, args).await?;
    Ok(())
}
pub async fn dfblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dfblank(vm, args).await?;
    Ok(())
}
pub async fn dget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dget_stmt(vm, args).await
}
pub async fn dput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dput(vm, args).await?;
    Ok(())
}
pub async fn dfcopy(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    dbase::ops::dfcopy(vm, args).await?;
    Ok(())
}

pub async fn account(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    use crate::icy_board::pcb::user_inf::AccountUserInf;

    // ACCOUNT(INTEGER field, INTEGER value)
    let field = vm.eval_expr(&args[0]).await?.as_int();
    let value = vm.eval_expr(&args[1]).await?.as_double();

    // Initialize accounting if not present
    if vm.user.account.is_none() {
        vm.user.account = Some(AccountUserInf::default());
    }

    let Some(account) = &mut vm.user.account else {
        return Ok(());
    };

    // Update the specified accounting field
    match field {
        0 => account.starting_balance += value,        // START_BAL
        1 => account.start_this_session += value,      // START_SESSION
        2 => account.debit_call += value,              // DEB_CALL
        3 => account.debit_time += value,              // DEB_TIME
        4 => account.debit_msg_read += value,          // DEB_MSGREAD
        5 => account.debit_msg_read_capture += value,  // DEB_MSGCAP
        6 => account.debit_msg_write += value,         // DEB_MSGWRITE
        7 => account.debit_msg_write_echoed += value,  // DEB_MSGECHOED
        8 => account.debit_msg_write_private += value, // DEB_MSGPRIVATE
        9 => account.debit_download_file += value,     // DEB_DOWNFILE
        10 => account.debit_download_bytes += value,   // DEB_DOWNBYTES
        11 => account.debit_group_chat += value,       // DEB_CHAT
        12 => account.debit_tpu += value,              // DEB_TPU
        13 => account.debit_special += value,          // DEB_SPECIAL
        14 => account.credit_upload_file += value,     // CRED_UPFILE
        15 => account.credit_upload_bytes += value,    // CRED_UPBYTES
        16 => account.credit_special += value,         // CRED_SPECIAL
        17 => {
            // SEC_DROP - Security level to drop to (stored as u8)
            account.drop_sec_level = value.clamp(0.0, 255.0) as u8;
        }
        _ => {
            log::error!("ACCOUNT statement: Invalid field number: {field}");
        }
    }

    // Update session user if this is the current user
    if let Some(session_user) = &mut vm.icy_board_state.session.current_user
        && session_user.get_name() == vm.user.get_name()
    {
        session_user.account.clone_from(&vm.user.account);
    }

    Ok(())
}

pub async fn recordusage(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    use crate::icy_board::pcb::user_inf::AccountUserInf;
    use chrono::Utc;
    use std::fs::OpenOptions;
    use std::io::Write;

    // RECORDUSAGE(INTEGER field, STRING desc1, STRING desc2, DWORD unitcost, INTEGER value)
    let field = vm.eval_expr(&args[0]).await?.as_int();
    let desc1 = vm.eval_expr(&args[1]).await?.as_string();
    let desc2 = vm.eval_expr(&args[2]).await?.as_string();
    let unitcost = vm.eval_expr(&args[3]).await?.as_double();
    let value = vm.eval_expr(&args[4]).await?.as_int();

    // Validate field is in debit/credit range (2-16)
    if !(2..=16).contains(&field) {
        log::error!("RECORDUSAGE: Invalid field number: {field} (must be 2-16)");
        return Ok(());
    }

    // Calculate total charge
    let total_charge = unitcost * value as f64;

    // Initialize accounting if not present
    if vm.user.account.is_none() {
        vm.user.account = Some(AccountUserInf::default());
    }

    let Some(account) = &mut vm.user.account else {
        return Ok(());
    };

    // Update the accounting field (same as ACCOUNT statement)
    match field {
        2 => account.debit_call += total_charge,              // DEB_CALL
        3 => account.debit_time += total_charge,              // DEB_TIME
        4 => account.debit_msg_read += total_charge,          // DEB_MSGREAD
        5 => account.debit_msg_read_capture += total_charge,  // DEB_MSGCAP
        6 => account.debit_msg_write += total_charge,         // DEB_MSGWRITE
        7 => account.debit_msg_write_echoed += total_charge,  // DEB_MSGECHOED
        8 => account.debit_msg_write_private += total_charge, // DEB_MSGPRIVATE
        9 => account.debit_download_file += total_charge,     // DEB_DOWNFILE
        10 => account.debit_download_bytes += total_charge,   // DEB_DOWNBYTES
        11 => account.debit_group_chat += total_charge,       // DEB_CHAT
        12 => account.debit_tpu += total_charge,              // DEB_TPU
        13 => account.debit_special += total_charge,          // DEB_SPECIAL
        14 => account.credit_upload_file += total_charge,     // CRED_UPFILE
        15 => account.credit_upload_bytes += total_charge,    // CRED_UPBYTES
        16 => account.credit_special += total_charge,         // CRED_SPECIAL
        _ => {
            log::error!("RECORDUSAGE: Invalid field number: {field}");
            return Ok(());
        }
    }

    // Update session user if this is the current user
    if let Some(session_user) = &mut vm.icy_board_state.session.current_user
        && session_user.get_name() == vm.user.get_name()
    {
        session_user.account.clone_from(&vm.user.account);
    }

    // Write to accounting tracking file if configured
    let board = vm.icy_board_state.get_board().await;
    if board.config.accounting.enabled && !board.config.accounting.tracking_file.as_os_str().is_empty() {
        let tracking_file = board.resolve_file(&board.config.accounting.tracking_file);
        drop(board); // Release lock before file I/O

        // Format: timestamp, username, field, desc1, desc2, unitcost, quantity, total
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let username = vm.user.get_name();

        let field_name = match field {
            2 => "DEB_CALL",
            3 => "DEB_TIME",
            4 => "DEB_MSGREAD",
            5 => "DEB_MSGCAP",
            6 => "DEB_MSGWRITE",
            7 => "DEB_MSGECHOED",
            8 => "DEB_MSGPRIVATE",
            9 => "DEB_DOWNFILE",
            10 => "DEB_DOWNBYTES",
            11 => "DEB_CHAT",
            12 => "DEB_TPU",
            13 => "DEB_SPECIAL",
            14 => "CRED_UPFILE",
            15 => "CRED_UPBYTES",
            16 => "CRED_SPECIAL",
            _ => "UNKNOWN",
        };

        let log_line = format!("{timestamp}\t{username}\t{field_name}\t{desc1}\t{desc2}\t{unitcost:.2}\t{value}\t{total_charge:.2}\n");

        // Append to tracking file
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(tracking_file) {
            if let Err(e) = file.write_all(log_line.as_bytes()) {
                log::error!("RECORDUSAGE: Failed to write to tracking file: {e}");
            }
        } else {
            log::error!("RECORDUSAGE: Failed to open tracking file");
        }
    }

    Ok(())
}

pub async fn msgtofile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let (conference, area) = vm.eval_expr(&args[0]).await?.as_msg_id();
    let msg_number: i32 = vm.eval_expr(&args[1]).await?.as_int();
    let file_name = vm.eval_expr(&args[2]).await?.as_string();

    let Some(msg_base) = vm.message_base_path(conference, area).await else {
        log::error!("MSGTOFILE: no message base {conference}:{area}");
        return Ok(());
    };
    let base = match JamMessageBase::open(&msg_base) {
        Ok(base) => base,
        Err(err) => {
            log::error!("MSGTOFILE can't open message base in area {area}: {err}");
            return Ok(());
        }
    };
    let header = match base.read_header(msg_number as u32) {
        Ok(header) => header,
        Err(err) => {
            log::error!("MSGTOFILE can't read message header {msg_number} in area {area}: {err}");
            return Ok(());
        }
    };
    let msg_text = match base.read_message_text(&header) {
        Ok(text) => text,
        Err(err) => {
            log::error!("MSGTOFILE can't read message text {msg_number} in area {area}: {err}");
            return Ok(());
        }
    };

    let date_time = DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or_else(Utc::now);
    let date = IcbDate::from_utc(&date_time);
    let time = IcbTime::from_naive(date_time.naive_local());

    // PCBoard keeps at most 25 characters in the fixed To/From/Subject fields and
    // spills anything longer into extended headers (MSGENTER.C).
    let mut ext_headers: Vec<(&str, String)> = Vec::new();
    let to = split_fixed_field(header.to().map(ToString::to_string).unwrap_or_default(), "TO", "TO2", true, &mut ext_headers);
    let from = split_fixed_field(
        header.from().map(ToString::to_string).unwrap_or_default(),
        "FROM",
        "FROM2",
        true,
        &mut ext_headers,
    );
    let subject = split_fixed_field(
        header.subject().map(ToString::to_string).unwrap_or_default(),
        "SUBJECT",
        "SUBJ2",
        false,
        &mut ext_headers,
    );
    if header.attributes & jam_attributes::MSG_RECEIPTREQ != 0 {
        push_ext_header(&mut ext_headers, "REQRR", "Caller has requested a Return Receipt");
    }
    for field in &header.sub_fields {
        let value = field.content().to_string();
        match field.field_type() {
            SubfieldType::EnclFile => push_ext_header(&mut ext_headers, "ATTACH", &value),
            SubfieldType::AddressD => push_ext_header(&mut ext_headers, "ROUTE", &value),
            SubfieldType::PackoutDate => {
                if let Ok(date) = DateTime::parse_from_rfc3339(&value) {
                    push_ext_header(&mut ext_headers, "PACKOUT", &date.format("%m-%d-%y").to_string());
                }
            }
            SubfieldType::FTSKludge => {
                if let Some(value) = value.strip_prefix("NEWSGROUPS: ") {
                    push_ext_header(&mut ext_headers, "UNEWSGR", value);
                } else if let Some(value) = value.strip_prefix("FOLLOWUP-TO: ") {
                    push_ext_header(&mut ext_headers, "UFOLLOW", value);
                }
            }
            _ => {}
        }
    }
    ext_headers.sort_unstable_by(|left, right| left.0.cmp(right.0).then_with(|| left.1.cmp(&right.1)));

    let echo = if header.attributes & jam_attributes::MSG_TYPEECHO != 0 { 'E' } else { ' ' };
    let active = if header.is_deleted() { 226 } else { 225 };
    let body = pcboard_message_body(&msg_text.to_string());
    let stored_len = body.len().saturating_add(ext_headers.len() * 72);
    let blocks = (stored_len.saturating_add(127) / 128).saturating_add(1).min(255);

    let mut msg = String::new();
    let _ = writeln!(msg, "          Status: {}", message_status(&header));
    let _ = writeln!(msg, "  Message Number: {}", header.message_number);
    let _ = writeln!(msg, "Reference Number: {}", header.reply_to);
    let _ = writeln!(msg, "Number of blocks: {blocks}");
    let _ = writeln!(msg, "            Date: {:02}-{:02}-{:02}", date.month(), date.day(), date.year() % 100);
    let _ = writeln!(msg, "            Time: {:02}:{:02}", time.get_hour(), time.get_minute());
    let _ = writeln!(msg, "              To: {to}");
    // PCBoard builds a "Reply" line here but overwrites it before writing, so
    // only "Time of reply" ever reaches the file (SCREXEC.CPP).
    msg.push_str("   Time of reply: \n");
    let _ = writeln!(msg, "            From: {from}");
    let _ = writeln!(msg, "         Subject: {subject}");
    // JAM keeps only a password CRC, so the plaintext PCBoard printed is gone.
    msg.push_str("        Password: \n");
    let _ = writeln!(msg, "          Active: {active}");
    let _ = writeln!(msg, "            Echo:{echo}");
    if !ext_headers.is_empty() {
        let _ = writeln!(msg, "Extended headers: {}", ext_headers.len());
        for (func, value) in &ext_headers {
            let _ = writeln!(msg, "{func:<7}:{value:<60}N");
        }
    }
    msg.push_str("Message Body:\n");
    msg.push_str(&body);

    let file_name = vm.resolve_file(&file_name).await;
    if let Err(err) = append_utf8_with_bom(&file_name, &msg) {
        log::error!("MSGTOFILE can't write message text {msg_number} in area {area}: {err}");
    }
    Ok(())
}

fn append_utf8_with_bom(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    if file.metadata()?.len() == 0 {
        file.write_all(&[0xEF, 0xBB, 0xBF])?;
    }
    file.write_all(text.as_bytes())
}

fn pcboard_message_body(text: &str) -> String {
    let mut body = String::new();
    for line in text.lines() {
        body.push_str(line.trim_end_matches('\r'));
        body.push('\n');
    }
    if body.is_empty() {
        body.push('\n');
    }
    body
}

/// Splits a header field the way `PCBoard` does: up to 25 characters stay in the
/// fixed field, the rest moves into an extended header (and a second one past 60
/// characters). To and From blank their fixed field, Subject keeps its first 25.
fn split_fixed_field(value: String, func: &'static str, func2: &'static str, blank_fixed: bool, ext: &mut Vec<(&'static str, String)>) -> String {
    const FIXED: usize = 25;
    const EXT: usize = 60;
    if value.chars().count() <= FIXED {
        return value;
    }
    let chars: Vec<char> = value.chars().collect();
    ext.push((func, chars.iter().take(EXT).collect()));
    if chars.len() > EXT {
        ext.push((func2, chars.iter().skip(EXT).take(EXT).collect()));
    }
    if blank_fixed { String::new() } else { chars.iter().take(FIXED).collect() }
}

fn push_ext_header(ext: &mut Vec<(&'static str, String)>, function: &'static str, value: &str) {
    ext.push((function, value.chars().take(60).collect()));
}

pub async fn qwklimits(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let field = vm.eval_expr(&args[0]).await?.as_int();
    let limit = vm.eval_expr(&args[1]).await?.as_int();

    // Ensure QWK config exists for the user
    if vm.user.qwk_config.is_none() {
        vm.user.qwk_config = Some(QwkConfigUserInf::default());
    }

    let Some(qwk_config) = &mut vm.user.qwk_config else {
        log::error!("QWKLIMITS: Failed to initialize QWK configuration");
        return Ok(());
    };

    // Get board-wide QWK limits for validation
    let board = vm.icy_board_state.get_board().await;

    let settings: &crate::icy_board::icb_config::QwkSettings = &board.config.qwk_settings;
    match field {
        0 => {
            qwk_config.max_msgs = limit as u16;
        }
        1 => {
            qwk_config.max_msgs_per_conf = limit as u16;
        }
        2 => {
            // ATTACH_LIM_U - Personal attachment size limit (bytes)
            qwk_config.personal_attach_limit = limit as i32;
        }
        3 => {
            // ATTACH_LIM_P - Public attachment size limit (bytes)
            qwk_config.public_attach_limit = limit as i32;
        }
        _ => {
            log::error!("QWKLIMITS: Invalid field number: {field}");
        }
    }
    Ok(())
}

pub async fn command(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let via_cmd_list = vm.eval_expr(&args[0]).await?.as_bool();
    let command_line = vm.eval_expr(&args[1]).await?.as_string();
    vm.icy_board_state.session.push_tokens(&command_line);
    vm.icy_board_state.run_single_command(via_cmd_list).await?;
    Ok(())
}

pub async fn uselmrs(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let use_lmrs = vm.eval_expr(&args[0]).await?.as_bool();
    vm.use_lmrs = use_lmrs;
    Ok(())
}

pub async fn confinfo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conf_num = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let conf_field = vm.eval_expr(&args[1]).await?.as_int();
    let value = vm.eval_expr(&args[2]).await?;
    crate::vm::expressions::set_confinfo(vm, conf_num, conf_field, value).await?;
    Ok(())
}

pub async fn adjtubytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let bytes: i32 = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if bytes > 0 {
            user.stats.total_upld_bytes += bytes as u64;
        } else {
            user.stats.total_upld_bytes = user.stats.total_upld_bytes.saturating_sub(bytes as u64);
        }
    }
    Ok(())
}

pub async fn grafmode(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mode = vm.eval_expr(&args[0]).await?.as_int();
    match mode {
        1 | 2 => {
            // In PCBoard 1) turns graphics on but checks for ANSI
            vm.icy_board_state.session.disp_options.grapics_mode = GraphicsMode::Graphics;
        }
        3 => {
            vm.icy_board_state.session.disp_options.grapics_mode = GraphicsMode::Ansi;
        }
        4 => {
            vm.icy_board_state.session.disp_options.grapics_mode = GraphicsMode::Ctty;
        }
        5 => {
            vm.icy_board_state.session.disp_options.grapics_mode = GraphicsMode::Rip;
        }
        6 => {
            // 6 is new for IcyBoard
            vm.icy_board_state.session.disp_options.grapics_mode = GraphicsMode::Avatar;
        }
        _ => {
            log::error!("PPE unsupported graphics mode: {mode}");
        }
    }
    Ok(())
}

pub async fn adduser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    use crate::icy_board::pcb::user_inf::AccountUserInf;
    use crate::icy_board::user_base::User;
    use chrono::Utc;

    // ADDUSER(STRING username, BOOLEAN keepAltVars)
    let username = vm.eval_expr(&args[0]).await?.as_string();
    let keep_alt_vars = vm.eval_expr(&args[1]).await?.as_bool();

    let trimmed = username.trim();
    if trimmed.is_empty() {
        log::warn!("ADDUSER: empty username ignored");
        return Ok(());
    }

    // Save current user context before we potentially switch
    let original_user = if keep_alt_vars { None } else { Some(vm.user.clone()) };

    // Acquire board lock to check for duplicates and add user
    let mut board_guard = vm.icy_board_state.board.lock().await;

    // Validate for duplicates (case-insensitive name/alias check)
    let duplicate = board_guard
        .users
        .iter()
        .any(|u| u.get_name().eq_ignore_ascii_case(trimmed) || (!u.alias.is_empty() && u.alias.eq_ignore_ascii_case(trimmed)));

    if duplicate {
        log::warn!("ADDUSER: duplicate username '{trimmed}', no user created");
        return Ok(());
    }

    // Create new user with system defaults
    let mut new_user = User::default();
    new_user.set_name(trimmed.to_string());
    new_user.stats.first_date_on = Utc::now();
    new_user.stats.last_on = Utc::now();
    new_user.security_level = board_guard.config.new_user_settings.sec_level;

    // Initialize accounting if enabled
    if board_guard.config.accounting.enabled
        && let Some(acc_cfg) = &board_guard.config.accounting.accounting_config
    {
        new_user.account = Some(AccountUserInf {
            starting_balance: acc_cfg.new_user_balance,
            start_this_session: acc_cfg.new_user_balance,
            ..Default::default()
        });
    }

    // Add user to user base
    let record_index = board_guard.users.new_user(new_user.clone());
    log::info!("ADDUSER: created user '{}' as record #{}", trimmed, record_index + 1);

    // Release board lock before modifying VM state
    drop(board_guard);

    // Handle variable context switching
    if keep_alt_vars {
        // Like GETALTUSER: switch context to new user
        vm.user = new_user;
        vm.set_user_variables()?;
        log::info!("ADDUSER: context switched to new user '{trimmed}'");
    } else {
        // Restore original user context
        if let Some(original) = original_user {
            vm.user = original;
            vm.set_user_variables()?;
        }
    }

    Ok(())
}

/// `KILLMSG conf, msgnum`
///
/// Deletes a message. A message nobody can delete is not an error the PPE gets
/// to see; `PCBoard` simply carries on.
pub async fn killmsg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let (conference, area) = vm.eval_expr(&args[0]).await?.as_msg_id();
    let number = vm.eval_expr(&args[1]).await?.as_int() as u32;

    let Some(msg_base) = vm.message_base_path(conference, area).await else {
        log::error!("KILLMSG: no message base {conference}:{area}");
        return Ok(());
    };
    match JamMessageBase::open(&msg_base) {
        Ok(mut base) => {
            if let Err(err) = base.delete_message(number) {
                log::error!("KILLMSG can't delete message {number} in {conference}:{area}: {err}");
            }
        }
        Err(err) => log::error!("KILLMSG can't open message base {conference}:{area}: {err}"),
    }
    vm.invalidate_message_base();
    Ok(())
}

pub async fn chdir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let dir = vm.eval_expr(&args[0]).await?.as_string();
    let path = vm.resolve_file(&dir).await;
    if path.is_dir() {
        if let Err(err) = env::set_current_dir(&path) {
            log::error!("CHDIR {} error: {}", path.display(), err);
        }
    } else {
        log::error!("CHDIR Can't find directory {}", path.display());
    }
    Ok(())
}

pub async fn mkdir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let dir = vm.eval_expr(&args[0]).await?.as_string();
    let path = vm.resolve_file(&dir).await;
    if let Err(err) = fs::create_dir_all(&path) {
        log::error!("MKDIR  {} error : {}", path.display(), err);
    }
    Ok(())
}
pub async fn rmdir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let dir = vm.eval_expr(&args[0]).await?.as_string();
    let path = vm.resolve_file(&dir).await;
    if path.is_dir() {
        if let Err(err) = fs::remove_dir(&path) {
            log::error!("RMDIR {} error: {}", path.display(), err);
        }
    } else {
        log::error!("RMDIR Can't find directory {}", path.display());
    }
    Ok(())
}
pub async fn fdowraka(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("FDOWRAKA");
}
pub async fn fdoaddaka(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("FDOADDAKA");
}
pub async fn fdowrorg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("FDOWRORG");
}
pub async fn fdoaddorg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    unimplemented_stmt!("FDOADDOR");
}
pub async fn fdoqmod(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let record = vm.eval_expr(&args[0]).await?.as_int();
    let address = vm.eval_expr(&args[1]).await?.as_string();
    let file = vm.eval_expr(&args[2]).await?.as_string();
    let _flags = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).await;
    let ftn = vm.icy_board_state.get_board().await.ftn.clone();

    if let Err(err) = queue::remove(&ftn, record.max(0) as usize) {
        log::error!("FDOQMOD {record}: {err}");
        return Ok(());
    }
    if let Err(err) = queue::add(&ftn, &address, &file) {
        log::error!("FDOQMOD {address}: {err}");
    }
    Ok(())
}
pub async fn fdoqadd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let address = vm.eval_expr(&args[0]).await?.as_string();
    let file = vm.eval_expr(&args[1]).await?.as_string();
    // Pcboard knew a normal and a crash entry. Nothing here calls a link out of
    // turn, so what is queued waits for the next call either way.
    let _flags = vm.eval_expr(&args[2]).await?.as_int();
    let file = vm.resolve_file(&file).await;
    let ftn = vm.icy_board_state.get_board().await.ftn.clone();

    if let Err(err) = queue::add(&ftn, &address, &file) {
        log::error!("FDOQADD {address}: {err}");
    }
    Ok(())
}
pub async fn fdoqdel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let record = vm.eval_expr(&args[0]).await?.as_int();
    let ftn = vm.icy_board_state.get_board().await.ftn.clone();
    match queue::remove(&ftn, record.max(0) as usize) {
        Ok(false) => log::error!("FDOQDEL: nothing is waiting under number {record}"),
        Err(err) => log::error!("FDOQDEL {record}: {err}"),
        Ok(true) => {}
    }
    Ok(())
}
pub async fn sounddelay(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::warn!("SOUNDDELAY is not supported");
    Ok(())
}

pub async fn shortdesc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let use_short_desc = vm.eval_expr(&args[0]).await?.as_bool();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        user.flags.use_short_filedescr = use_short_desc;
    }
    Ok(())
}

/// `MOVE_MSG conf, message, movetype`
///
/// Copies a message out of the conference the caller is in and into `conf`, and
/// deletes the original when `movetype` asks for a move rather than a copy.
pub async fn move_msg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let (to_conf, to_area) = vm.eval_expr(&args[0]).await?.as_msg_id();
    let number = vm.eval_expr(&args[1]).await?.as_int() as u32;
    let moving = vm.eval_expr(&args[2]).await?.as_bool();

    let from_conf = vm.icy_board_state.session.current_conference_number as i32;
    let from_area = vm.icy_board_state.session.current_message_area as i32;

    let (Some(source_path), Some(target_path)) = (vm.message_base_path(from_conf, from_area).await, vm.message_base_path(to_conf, to_area).await) else {
        log::error!("MOVE_MSG: no message base {from_conf}:{from_area} or {to_conf}:{to_area}");
        return Ok(());
    };

    let mut source = match JamMessageBase::open(&source_path) {
        Ok(base) => base,
        Err(err) => {
            log::error!("MOVE_MSG can't open message base {from_conf}:{from_area}: {err}");
            return Ok(());
        }
    };
    let Ok(header) = source.read_header(number) else {
        log::error!("MOVE_MSG: no message {number} in {from_conf}:{from_area}");
        return Ok(());
    };
    let text = source.read_message_text(&header)?;

    let mut message = JamMessage::default()
        .with_from(header.from().cloned().unwrap_or_default())
        .with_to(header.to().cloned().unwrap_or_default())
        .with_subject(header.subject().cloned().unwrap_or_default())
        .with_date_time(Utc::now())
        .with_attributes(header.attributes)
        .with_text(text);
    if header.reply_to != 0 {
        message = message.with_reply_to(header.reply_to);
    }

    let mut target = match JamMessageBase::open(&target_path) {
        Ok(base) => base,
        Err(_) => match JamMessageBase::create(&target_path) {
            Ok(base) => base,
            Err(err) => {
                log::error!("MOVE_MSG can't open message base {to_conf}:{to_area}: {err}");
                return Ok(());
            }
        },
    };
    if let Err(err) = target.write_message(&message) {
        log::error!("MOVE_MSG can't write message into {to_conf}:{to_area}: {err}");
        return Ok(());
    }
    target.write_jhr_header()?;

    // The original only goes away once the copy is safely written.
    if moving {
        let _ = source.delete_message(number);
    }
    vm.invalidate_message_base();
    Ok(())
}

pub async fn set_bank_bal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let field = vm.eval_expr(&args[0]).await?.as_int();
    let value = vm.eval_expr(&args[1]).await?;

    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if user.bank.is_none() {
            user.bank = Some(BankUserInf::default());
        }
        if let Some(bank) = &mut user.bank {
            match field {
                0 => {
                    bank.time_info.last_deposite_date = value.as_date();
                }
                1 => {
                    bank.time_info.last_withdraw_date = value.as_date();
                }
                2 => {
                    bank.time_info.last_transaction_amount = value.as_int() as u32;
                }
                3 => {
                    bank.time_info.amount_saved = value.as_int() as u32;
                }
                4 => {
                    bank.time_info.max_withdrawl_per_day = value.as_int() as u32;
                }
                5 => {
                    bank.time_info.max_stored_amount = value.as_int() as u32;
                }

                6 => {
                    bank.byte_info.last_deposite_date = value.as_date();
                }
                7 => {
                    bank.byte_info.last_withdraw_date = value.as_date();
                }
                8 => {
                    bank.byte_info.last_transaction_amount = value.as_int() as u32;
                }
                9 => {
                    bank.byte_info.amount_saved = value.as_int() as u32;
                }
                10 => {
                    bank.byte_info.max_withdrawl_per_day = value.as_int() as u32;
                }
                11 => {
                    bank.byte_info.max_stored_amount = value.as_int() as u32;
                }

                _ => {
                    log::error!("SET_BANK_BAL: Invalid field {field}");
                }
            }
        }
    }
    Ok(())
}

// Digitized sound (WAV/OGG) via the SyncTERM audio APC extension
// (`ESC _ SyncTERM:A;<verb>;... ESC \`). Unsupported terminals ignore the
// sequence per the ANSI APC spec, so no capability check is required.
//
// Callers pick their own channel (0-15); slot == channel, since each channel
// only ever plays one file at a time and 16 resident slots is plenty for
// overlapping music/fx.

/// Base64-inflated APC payloads are capped client-side at 32 MB; stay well under that.
const MAX_SOUND_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_GFX_SOURCE_BYTES: u64 = 32 * 1024 * 1024;

/// Cancels the client's fixed per-channel headroom (`AUDIO_APC_BASE_DB` = -12dB
/// in `icy_engine_gui`'s `audio_apc.rs`), so a PPL volume of 100 means "as loud
/// as the source allows" rather than a permanently quartered -12dB ceiling.
const SND_HEADROOM_COMPENSATION_DB: f32 = 12.0;
/// Matches the client's floor for a linear volume of 0%.
const SND_MIN_DB: f32 = -60.0;

/// Converts a 0-100 PPL volume percentage to the dB argument sent to the
/// client, compensating for its fixed headroom (see `SND_HEADROOM_COMPENSATION_DB`).
#[allow(clippy::cast_precision_loss)] // percent is clamped to 0..=100, so the cast is exact
fn snd_volume_db(percent: i32) -> f32 {
    let percent = percent.clamp(0, 100);
    if percent == 0 {
        return SND_MIN_DB;
    }
    SND_HEADROOM_COMPENSATION_DB + 20.0 * (percent as f32 / 100.0).log10()
}

async fn send_apc(vm: &mut VirtualMachine<'_>, body: &str) -> Res<()> {
    let mut seq = Vec::with_capacity(body.len() + 16);
    seq.extend_from_slice(b"\x1b_");
    seq.extend_from_slice(body.as_bytes());
    seq.extend_from_slice(b"\x1b\\");
    vm.icy_board_state.connection.send(&seq).await
}

async fn finish_gfx_frame(vm: &mut VirtualMachine<'_>) -> Res<()> {
    let pacing = vm.icy_board_state.ppl_graphics.as_ref().is_some_and(|graphics| graphics.pacing);
    if pacing {
        vm.icy_board_state.await_terminal_ack().await?;
    }
    Ok(())
}

async fn send_gfx_apc(vm: &mut VirtualMachine<'_>, body: &str) -> Res<()> {
    send_apc(vm, body).await?;
    finish_gfx_frame(vm).await
}

async fn send_audio_apc(vm: &mut VirtualMachine<'_>, body: &str) -> Res<()> {
    send_apc(vm, &format!("SyncTERM:A;{body}")).await
}

pub(crate) const SOUND_CHANNELS: usize = 14;

/// Logical channels 0-13 map to APC channels 2-15, because `CTerm` reserves 0 and 1.
pub(crate) fn resolve_sound_channel(channel: i32) -> Option<(usize, u8)> {
    (0..SOUND_CHANNELS as i32).contains(&channel).then(|| (channel as usize, channel as u8 + 2))
}

/// Uploads a file into the client's on-disk media cache (shared with the
/// image APC extension) under a content hash, unless this connection has
/// already pushed that exact content - so replaying a music/fx file only
/// resends the bytes once instead of on every trigger. Returns the cache
/// name the file is stored under, or the error that stopped it.
async fn sndcache_store(vm: &mut VirtualMachine<'_>, file_name: &str) -> Res<Result<String, PplError>> {
    use base64::{Engine as _, engine::general_purpose};
    use sha2::{Digest, Sha256};

    let path = vm.resolve_file(&file_name).await;

    let data = match fs::metadata(&path).and_then(|meta| {
        if meta.len() > MAX_SOUND_FILE_BYTES {
            Err(std::io::Error::other("file too large"))
        } else {
            fs::read(&path)
        }
    }) {
        Ok(data) => data,
        Err(err) => {
            log::warn!("Can't load sound file {}: {err}", path.display());
            return Ok(Err(PplError::new(ERR_KIND_SOUND, ERR_IO, format!("can't read sound {file_name}: {err}"))));
        }
    };

    let hash = format!("{:x}", Sha256::digest(&data));
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("bin");
    let cache_name = format!("{}{}.{extension}", crate::icy_board::state::ppl_graphics::SOUND_CACHE_PREFIX, &hash[..32]);

    if vm.icy_board_state.sound_cache.insert(cache_name.clone()) {
        if !vm.icy_board_state.reserve_media_upload(data.len()) {
            vm.icy_board_state.sound_cache.remove(&cache_name);
            log::warn!("Sound media upload budget exhausted");
            return Ok(Err(PplError::new(ERR_KIND_SOUND, ERR_LIMIT, "sound upload budget exhausted")));
        }
        let encoded = general_purpose::STANDARD.encode(&data);
        send_apc(vm, &format!("SyncTERM:C;S;{cache_name};{encoded}")).await?;
        vm.icy_board_state.acknowledge_upload(encoded.len()).await?;
    }
    Ok(Ok(cache_name))
}

async fn sound_file_supported(vm: &mut VirtualMachine<'_>, file_name: &str) -> Res<bool> {
    let path = vm.resolve_file(&file_name).await;
    let mut head = [0u8; 4096];
    let head_len = std::fs::File::open(&path)
        .and_then(|mut file| std::io::Read::read(&mut file, &mut head))
        .unwrap_or(0);
    let head = &head[..head_len];
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default();
    let format = if extension.eq_ignore_ascii_case("wav") {
        1
    } else if extension.eq_ignore_ascii_case("aiff") || extension.eq_ignore_ascii_case("aif") {
        2
    } else if extension.eq_ignore_ascii_case("flac") {
        3
    } else if extension.eq_ignore_ascii_case("ogg") || extension.eq_ignore_ascii_case("opus") {
        if head.windows(8).any(|window| window == b"OpusHead") { 5 } else { 4 }
    } else {
        return Ok(false);
    };
    let Some((_, major, subtype)) = crate::icy_board::state::SOUND_FORMATS.iter().find(|(known, _, _)| *known == format) else {
        return Ok(false);
    };
    vm.icy_board_state.query_sound_format(format, *major, *subtype).await
}

/// Plays what the caller's cache already holds, which is all an `AUDIO` has to do.
async fn queue_cached_sound(vm: &mut VirtualMachine<'_>, cache_name: &str, channel: u8, logical_channel: usize, looping: bool) -> Res<()> {
    // slot == channel: each channel plays one file at a time, so this keeps
    // overlapping music/fx on distinct channels simple with no slot bookkeeping.
    let slot = channel;
    send_audio_apc(vm, &format!("Load;S={slot};{cache_name}")).await?;
    // Reassert the channel's saved volume (100 by default) every play, since
    // it otherwise sits at the client's quiet default until SNDVOLUME is called.
    let volume = vm.icy_board_state.sound_volume[logical_channel];
    send_audio_apc(vm, &format!("Volume;C={channel};V={:.2}dB", snd_volume_db(volume))).await?;
    if looping {
        send_audio_apc(vm, &format!("Queue;C={channel};S={slot};L")).await?;
    } else {
        send_audio_apc(vm, &format!("Queue;C={channel};S={slot}")).await?;
        // A track that ends reports it, so a PPE can answer without polling for it.
        send_audio_apc(vm, &format!("Update;C={channel}")).await?;
    }
    Ok(())
}

/// `LOADAUDIO file$` - takes a channel for a file and answers the `AUDIO` holding it.
pub async fn load_audio(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    audio_load(vm, &file_name).await
}

pub(crate) async fn audio_load(vm: &mut VirtualMachine<'_>, file_name: &str) -> Res<VariableValue> {
    use crate::icy_board::state::ppl_audio::PplAudio;

    fn failed(vm: &mut VirtualMachine<'_>, error: PplError) -> VariableValue {
        vm.set_error(error);
        PplAudio::invalid()
    }

    let file_name = file_name.to_string();
    if !vm.icy_board_state.query_sound_available().await? {
        return Ok(failed(vm, PplError::new(ERR_KIND_SOUND, ERR_UNAVAILABLE, "the terminal has no sound")));
    }
    if !sound_file_supported(vm, &file_name).await? {
        return Ok(failed(
            vm,
            PplError::new(ERR_KIND_SOUND, ERR_FORMAT, format!("unsupported sound format in {file_name}")),
        ));
    }
    let cache_name = match sndcache_store(vm, &file_name).await? {
        Ok(cache_name) => cache_name,
        Err(error) => return Ok(failed(vm, error)),
    };
    let Some(channel) = vm.icy_board_state.take_ppl_audio(cache_name) else {
        return Ok(failed(vm, PplError::new(ERR_KIND_SOUND, ERR_INVALID, "no sound channel is free")));
    };
    vm.operation_succeeded();
    Ok(PplAudio::value(channel))
}

/// Runs one `AUDIO` member. Answers whether it could be carried out.
/// Sets a channel's volume, over `milliseconds` when that is not zero.
pub(crate) async fn sound_set_volume(vm: &mut VirtualMachine<'_>, logical_channel: i32, volume: i32, milliseconds: i32) -> Res<bool> {
    if vm.icy_board_state.ppl_audio_file(logical_channel).is_none() {
        vm.set_error(PplError::new(ERR_KIND_SOUND, ERR_INVALID, "no sound is loaded").on_channel(logical_channel));
        return Ok(false);
    }
    let Some((logical, channel)) = resolve_sound_channel(logical_channel) else {
        vm.set_error(PplError::new(ERR_KIND_SOUND, ERR_INVALID, "invalid sound channel").on_channel(logical_channel));
        return Ok(false);
    };
    let volume = volume.clamp(0, 100);
    let milliseconds = milliseconds.max(0);
    vm.icy_board_state.sound_volume[logical] = volume;
    let body = if milliseconds > 0 {
        format!("Volume;C={channel};V={:.2}dB;T={milliseconds}", snd_volume_db(volume))
    } else {
        format!("Volume;C={channel};V={:.2}dB", snd_volume_db(volume))
    };
    send_audio_apc(vm, &body).await?;
    vm.operation_succeeded();
    Ok(true)
}

pub(crate) async fn sound_member(vm: &mut VirtualMachine<'_>, logical_channel: i32, name: &unicase::Ascii<String>, arguments: &[VariableValue]) -> Res<bool> {
    use crate::icy_board::state::ppl_audio::{FADE, FREE, PLAY, STOP};

    let Some(file) = vm.icy_board_state.ppl_audio_file(logical_channel).cloned() else {
        vm.set_error(PplError::new(ERR_KIND_SOUND, ERR_INVALID, "no sound is loaded").on_channel(logical_channel));
        return Ok(false);
    };
    let Some((logical, channel)) = resolve_sound_channel(logical_channel) else {
        vm.set_error(PplError::new(ERR_KIND_SOUND, ERR_INVALID, "invalid sound channel").on_channel(logical_channel));
        return Ok(false);
    };
    let integer = |index: usize| arguments.get(index).map_or(0, VariableValue::as_int);

    if *name == *PLAY {
        let looping = arguments.first().is_some_and(VariableValue::as_bool);
        queue_cached_sound(vm, &file, channel, logical, looping).await?;
        vm.icy_board_state.sound_active[logical] = true;
        vm.operation_succeeded();
        return Ok(true);
    }
    if *name == *STOP {
        vm.icy_board_state.sound_active[logical] = false;
        send_audio_apc(vm, &format!("Flush;C={channel};O=0")).await?;
        vm.operation_succeeded();
        return Ok(true);
    }
    if *name == *FADE {
        return sound_set_volume(vm, logical_channel, integer(0), integer(1)).await;
    }
    if *name == *FREE {
        vm.icy_board_state.sound_active[logical] = false;
        vm.icy_board_state.release_ppl_audio(logical_channel);
        send_audio_apc(vm, &format!("Flush;C={channel};O=0")).await?;
        vm.operation_succeeded();
        return Ok(true);
    }
    log::error!("Unknown Audio member {name}");
    Ok(false)
}

/// Reads one pixel back out of a `SURFACE`. Out of bounds and an invalid handle both
/// answer transparent black, the same as `SetPixel` clips a write silently.
pub(crate) async fn surface_get_pixel(vm: &mut VirtualMachine<'_>, handle: i32, arguments: &[VariableValue]) -> Res<VariableValue> {
    let Some(graphics) = vm.icy_board_state.ppl_graphics.as_ref() else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(VariableValue::new_unsigned(0));
    };
    let Some(surface) = graphics.surfaces.get(&handle) else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(VariableValue::new_unsigned(0));
    };
    let x = arguments.first().map_or(0, VariableValue::as_int);
    let y = arguments.get(1).map_or(0, VariableValue::as_int);
    let color = surface.get_pixel(x, y);
    vm.icy_board_state.gfx_error = 0;
    Ok(VariableValue::new_unsigned(color as u64))
}

/// Runs one `SURFACE` member. Answers whether it could be carried out.
pub(crate) async fn surface_member(vm: &mut VirtualMachine<'_>, handle: i32, name: &unicase::Ascii<String>, arguments: &[VariableValue]) -> Res<bool> {
    use crate::icy_board::state::ppl_surface::{
        BLIT, BLIT_RECT, CLEAR, FILL_RECT, FREE, PIN, PRESENT, PRESENT_AT, PRESENT_RECT, RECT, SET_PIXEL, UNPIN, surface_handle,
    };

    let integer = |index: usize| arguments.get(index).map_or(0, VariableValue::as_int);
    let color = |index: usize| arguments.get(index).map_or(0, |value| value.as_unsigned() as u32);

    if *name == *CLEAR || *name == *SET_PIXEL || *name == *FILL_RECT || *name == *RECT {
        let Some(graphics) = vm.icy_board_state.ppl_graphics.as_mut() else {
            vm.icy_board_state.gfx_error = 1;
            return Ok(false);
        };
        let Some(surface) = graphics.surfaces.get_mut(&handle) else {
            vm.icy_board_state.gfx_error = 2;
            return Ok(false);
        };
        graphics.pinned.remove(&handle);
        if *name == *CLEAR {
            surface.clear(color(0));
        } else if *name == *SET_PIXEL {
            surface.set_pixel(integer(0), integer(1), color(2));
        } else if *name == *FILL_RECT {
            surface.fill_rect(integer(0), integer(1), integer(2), integer(3), color(4));
        } else {
            surface.rect(integer(0), integer(1), integer(2), integer(3), color(4));
        }
        vm.icy_board_state.gfx_error = 0;
        return Ok(true);
    }

    if *name == *BLIT || *name == *BLIT_RECT {
        let Some(source_handle) = arguments.first().and_then(surface_handle) else {
            vm.icy_board_state.gfx_error = 2;
            return Ok(false);
        };
        let Some(graphics) = vm.icy_board_state.ppl_graphics.as_mut() else {
            vm.icy_board_state.gfx_error = 1;
            return Ok(false);
        };
        let Some(source) = graphics.surfaces.get(&source_handle).cloned() else {
            vm.icy_board_state.gfx_error = 2;
            return Ok(false);
        };
        let source_rect = if *name == *BLIT {
            (0, 0, source.width as i32, source.height as i32)
        } else {
            (integer(1), integer(2), integer(3), integer(4))
        };
        let destination = if *name == *BLIT { (integer(1), integer(2)) } else { (integer(5), integer(6)) };
        let Some(target) = graphics.surfaces.get_mut(&handle) else {
            vm.icy_board_state.gfx_error = 2;
            return Ok(false);
        };
        graphics.pinned.remove(&handle);
        target.blit(&source, source_rect, destination);
        vm.icy_board_state.gfx_error = 0;
        return Ok(true);
    }

    if *name == *FREE {
        vm.icy_board_state.gfx_error = match vm.icy_board_state.ppl_graphics.as_mut() {
            Some(graphics) => {
                if graphics.remove_surface(handle) {
                    0
                } else {
                    2
                }
            }
            None => 1,
        };
        return Ok(vm.icy_board_state.gfx_error == 0);
    }

    if *name == *PIN || *name == *UNPIN {
        return gfx_pin_handle(vm, handle, *name == *PIN).await;
    }

    if *name == *PRESENT {
        return gfx_present_handle(vm, handle, PresentTarget::Origin, GfxTransform::default()).await;
    }

    if *name == *PRESENT_AT {
        return gfx_present_handle(vm, handle, PresentTarget::Cell(integer(0), integer(1)), GfxTransform::default()).await;
    }

    if *name == *PRESENT_RECT {
        let source = (integer(0), integer(1), integer(2), integer(3));
        let destination = if arguments.len() >= 6 {
            (integer(4), integer(5))
        } else {
            (source.0, source.1)
        };
        let zoom = if arguments.len() >= 8 {
            let (dest_width, dest_height) = (integer(6), integer(7));
            if dest_width <= 0 || dest_height <= 0 || u64::from(dest_width.unsigned_abs()) * u64::from(dest_height.unsigned_abs()) > MAX_GFX_SCALED_PIXELS {
                vm.icy_board_state.gfx_error = 5;
                return Ok(false);
            }
            if source.2 <= 0 || source.3 <= 0 || dest_width % source.2 != 0 || dest_height % source.3 != 0 {
                vm.icy_board_state.gfx_error = 6;
                return Ok(false);
            }
            Some(((dest_width / source.2).unsigned_abs(), (dest_height / source.3).unsigned_abs()))
        } else {
            None
        };
        let flip = if arguments.len() >= 9 { integer(8) } else { 0 };
        let transform = GfxTransform {
            zoom,
            flip_x: flip & 1 != 0,
            flip_y: flip & 2 != 0,
        };
        return gfx_present_handle(vm, handle, PresentTarget::Rect(source, destination), transform).await;
    }

    log::error!("Invalid function call on Surface ({name})");
    Ok(false)
}

pub(crate) enum PresentTarget {
    Origin,
    Cell(i32, i32),
    Rect((i32, i32, i32, i32), (i32, i32)),
}

async fn gfx_pin_handle(vm: &mut VirtualMachine<'_>, handle: i32, enabled: bool) -> Res<bool> {
    use base64::{Engine as _, engine::general_purpose};

    let Some(graphics) = vm.icy_board_state.ppl_graphics.as_mut() else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(false);
    };
    if !enabled {
        vm.icy_board_state.gfx_error = if graphics.pinned.remove(&handle).is_some() { 0 } else { 2 };
        return Ok(vm.icy_board_state.gfx_error == 0);
    }
    if graphics.backend != crate::icy_board::state::ppl_graphics::GFX_BACKEND_JXL {
        vm.icy_board_state.gfx_error = 6;
        return Ok(false);
    }
    let Some(surface) = graphics.surfaces.get(&handle).cloned() else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(false);
    };
    let Some(buffer) = graphics.pin(handle) else {
        vm.icy_board_state.gfx_error = 5;
        return Ok(false);
    };
    let Some(encoded) = gfx_jxl_encode(&surface) else {
        vm.icy_board_state.gfx_error = 4;
        return Ok(false);
    };
    let payload = general_purpose::STANDARD.encode(encoded);
    send_apc(vm, &format!("SyncTERM:C;LoadJXLBlob;B={buffer};{payload}")).await?;
    vm.icy_board_state.gfx_error = 0;
    Ok(true)
}

async fn gfx_present_handle(vm: &mut VirtualMachine<'_>, handle: i32, target: PresentTarget, transform: GfxTransform) -> Res<bool> {
    use crate::icy_board::state::ppl_graphics::GFX_BACKEND_JXL;

    let Some(graphics) = vm.icy_board_state.ppl_graphics.as_ref() else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(false);
    };
    if !transform.is_identity() && graphics.backend != GFX_BACKEND_JXL {
        vm.icy_board_state.gfx_error = 6;
        return Ok(false);
    }
    let Some(surface) = graphics.surfaces.get(&handle) else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(false);
    };
    let backend = graphics.backend;
    let capabilities = graphics.capabilities;

    let (prepared, destination) = match target {
        PresentTarget::Origin => (surface.clone(), (0, 0)),
        PresentTarget::Cell(column, row) => {
            let (column, row) = (column.max(1), row.max(1));
            if backend == GFX_BACKEND_JXL {
                (surface.clone(), ((column - 1) * capabilities.cell_width, (row - 1) * capabilities.cell_height))
            } else {
                let Some(encoded) = icy_sixel::sixel_encode(&surface.pixels, surface.width, surface.height, &icy_sixel::EncodeOptions::default()).ok() else {
                    vm.icy_board_state.gfx_error = 4;
                    return Ok(false);
                };
                let mut output = Vec::with_capacity(encoded.len() + 24);
                output.extend_from_slice(b"\x1b[?1070h");
                output.extend_from_slice(b"\x1b7");
                output.extend_from_slice(format!("\x1b[{row};{column}H").as_bytes());
                output.extend_from_slice(encoded.as_bytes());
                output.extend_from_slice(b"\x1b8");
                output.extend_from_slice(b"\x1b[?1070l");
                vm.icy_board_state.connection.send(&output).await?;
                vm.icy_board_state.ppl_graphics.as_mut().expect("graphics state was checked").sixel_presented = true;
                finish_gfx_frame(vm).await?;
                vm.icy_board_state.gfx_error = 0;
                return Ok(true);
            }
        }
        PresentTarget::Rect((x, y, width, height), destination) => {
            if backend == GFX_BACKEND_JXL {
                let Some((region, _, _)) = surface.region(x, y, width, height) else {
                    vm.icy_board_state.gfx_error = 5;
                    return Ok(false);
                };
                (region, destination)
            } else {
                let Some(output) = surface.region_at((x, y, width, height), destination).as_ref().and_then(gfx_sixel_output) else {
                    vm.icy_board_state.gfx_error = 5;
                    return Ok(false);
                };
                vm.icy_board_state.connection.send(&output).await?;
                vm.icy_board_state.ppl_graphics.as_mut().expect("graphics state was checked").sixel_presented = true;
                finish_gfx_frame(vm).await?;
                vm.icy_board_state.gfx_error = 0;
                return Ok(true);
            }
        }
    };

    if let Some(buffer) = vm
        .icy_board_state
        .ppl_graphics
        .as_ref()
        .and_then(|graphics| graphics.pinned.get(&handle))
        .copied()
    {
        let placement = match target {
            PresentTarget::Rect((x, y, width, height), _) => format!("SX={x};SY={y};SW={width};SH={height};"),
            _ => String::new(),
        };
        send_gfx_apc(
            vm,
            &format!(
                "SyncTERM:P;Paste;B={buffer};{placement}DX={};DY={}{}",
                destination.0,
                destination.1,
                transform.options()
            ),
        )
        .await?;
        vm.icy_board_state.gfx_error = 0;
        return Ok(true);
    }

    gfx_present_surface(vm, prepared, handle, destination, transform).await?;
    Ok(vm.icy_board_state.gfx_error == 0)
}

/// `NewSurface(width, height)` - a blank surface the caller owns until `Free`.
pub async fn new_surface(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let width = vm.eval_expr(&args[0]).await?.as_int();
    let height = vm.eval_expr(&args[1]).await?.as_int();
    gfx_new_surface(vm, width, height)
}

pub(crate) fn gfx_new_surface(vm: &mut VirtualMachine<'_>, width: i32, height: i32) -> Res<VariableValue> {
    use crate::icy_board::state::ppl_surface::PplSurface;

    let (Ok(width), Ok(height)) = (usize::try_from(width), usize::try_from(height)) else {
        vm.icy_board_state.gfx_error = 5;
        return Ok(PplSurface::invalid());
    };
    let Some(surface) = crate::icy_board::state::ppl_graphics::GfxSurface::new(width, height) else {
        vm.icy_board_state.gfx_error = 5;
        return Ok(PplSurface::invalid());
    };
    let Some(graphics) = vm.icy_board_state.ppl_graphics.as_mut() else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(PplSurface::invalid());
    };
    let handle = graphics.allocate_handle();
    if !graphics.insert_surface(handle, surface) {
        vm.icy_board_state.gfx_error = 5;
        return Ok(PplSurface::invalid());
    }
    vm.icy_board_state.gfx_error = 0;
    Ok(PplSurface::value(handle))
}

/// `LoadSurface(file)` - a surface holding what the image file decoded to.
pub async fn load_surface(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    gfx_load_surface(vm, &file_name).await
}

pub(crate) async fn gfx_load_surface(vm: &mut VirtualMachine<'_>, file_name: &str) -> Res<VariableValue> {
    use crate::icy_board::state::ppl_surface::PplSurface;

    let Some(mut surface) = gfx_decode_image(vm, file_name).await else {
        return Ok(PplSurface::invalid());
    };
    surface.cacheable = true;
    let Some(graphics) = vm.icy_board_state.ppl_graphics.as_mut() else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(PplSurface::invalid());
    };
    let handle = graphics.allocate_handle();
    if !graphics.insert_surface(handle, surface) {
        vm.icy_board_state.gfx_error = 5;
        return Ok(PplSurface::invalid());
    }
    vm.icy_board_state.gfx_error = 0;
    Ok(PplSurface::value(handle))
}

pub async fn gfxinit(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    use crate::icy_board::state::ppl_graphics::GFX_BACKEND_AUTO;

    let requested = match args.first() {
        Some(expr) => vm.eval_expr(expr).await?.as_int(),
        None => GFX_BACKEND_AUTO,
    };
    let fullscreen = match args.get(1) {
        Some(expr) => vm.eval_expr(expr).await?.as_bool(),
        None => true,
    };
    gfx_init(vm, requested, fullscreen).await
}

pub(crate) async fn gfx_init(vm: &mut VirtualMachine<'_>, requested: i32, fullscreen: bool) -> Res<()> {
    use crate::icy_board::state::ppl_graphics::{GFX_BACKEND_AUTO, GFX_BACKEND_JXL, GFX_BACKEND_NONE, GFX_BACKEND_SIXEL, PplGraphicsState};

    if !matches!(requested, GFX_BACKEND_AUTO | GFX_BACKEND_SIXEL | GFX_BACKEND_JXL) {
        vm.icy_board_state.gfx_error = 6;
        log::warn!("GFXINIT rejected unknown backend {requested}");
        vm.icy_board_state.ppl_graphics = None;
        return Ok(());
    }

    // Sixel is the one backend with no query of its own, so a plain sixel request
    // does not make the caller wait for answers nobody needs.
    let capabilities = if requested == GFX_BACKEND_SIXEL {
        vm.icy_board_state.session.term_caps.gfx
    } else {
        vm.icy_board_state.query_gfx_capabilities().await?
    };

    let backend = crate::icy_board::state::ppl_graphics::resolve_backend(&capabilities, requested);
    if backend == GFX_BACKEND_NONE {
        vm.icy_board_state.gfx_error = 6;
        log::warn!("GFXINIT cannot serve backend {requested} to this terminal");
        vm.icy_board_state.ppl_graphics = None;
        return Ok(());
    }
    let Some(graphics) = PplGraphicsState::new(backend, fullscreen, capabilities) else {
        vm.icy_board_state.ppl_graphics = None;
        return Ok(());
    };

    vm.icy_board_state.ppl_graphics = Some(graphics);
    vm.icy_board_state.gfx_error = 0;
    if fullscreen {
        vm.icy_board_state.connection.send(b"\x1b[2J\x1b[H\x1b[?25l\x1b[?7l\x1b[?80l\x1b[?1070l").await
    } else {
        Ok(())
    }
}
pub async fn gfxcreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let width = vm.eval_expr(&args[1]).await?.as_int();
    let height = vm.eval_expr(&args[2]).await?.as_int();
    let (Ok(width), Ok(height)) = (usize::try_from(width), usize::try_from(height)) else {
        vm.icy_board_state.gfx_error = 5;
        log::warn!("GFXCREATE requires positive dimensions");
        return Ok(());
    };
    let Some(surface) = crate::icy_board::state::ppl_graphics::GfxSurface::new(width, height) else {
        vm.icy_board_state.gfx_error = 5;
        log::warn!("GFXCREATE rejected surface {slot}: {width}x{height}");
        return Ok(());
    };
    if let Some(graphics) = &mut vm.icy_board_state.ppl_graphics {
        if graphics.insert_surface(slot, surface) {
            vm.icy_board_state.gfx_error = 0;
        } else {
            vm.icy_board_state.gfx_error = 5;
            log::warn!("GFXCREATE graphics memory budget exhausted");
        }
    } else {
        vm.icy_board_state.gfx_error = 1;
    }
    Ok(())
}

/// Reads an image file into a surface, reporting through `Error.Last()` on the way.
pub(crate) async fn gfx_decode_image(vm: &mut VirtualMachine<'_>, file_name: &str) -> Option<crate::icy_board::state::ppl_graphics::GfxSurface> {
    let path = vm.resolve_file(&file_name).await;
    let bytes = match fs::metadata(&path).and_then(|metadata| {
        if metadata.len() > MAX_GFX_SOURCE_BYTES {
            Err(std::io::Error::other("file too large"))
        } else {
            fs::read(&path)
        }
    }) {
        Ok(bytes) => bytes,
        Err(err) => {
            vm.icy_board_state.gfx_error = 3;
            log::warn!("Can't read image {}: {err}", path.display());
            return None;
        }
    };
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(64 * 1024 * 1024);
    let is_jxl = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jxl"));
    let image = if is_jxl {
        jxl_oxide::integration::JxlDecoder::new(std::io::Cursor::new(bytes))
            .ok()
            .and_then(|mut decoder| {
                use image::ImageDecoder as _;
                decoder.set_limits(limits).ok()?;
                image::DynamicImage::from_decoder(decoder).ok()
            })
    } else {
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes));
        reader.limits(limits);
        reader.with_guessed_format().ok().and_then(|reader| reader.decode().ok())
    };
    let Some(image) = image else {
        vm.icy_board_state.gfx_error = 4;
        log::warn!("Can't decode image {}", path.display());
        return None;
    };
    let image = image.to_rgba8();
    let surface = crate::icy_board::state::ppl_graphics::GfxSurface::from_rgba(image.width() as usize, image.height() as usize, image.into_raw());
    if surface.is_none() {
        vm.icy_board_state.gfx_error = 5;
        log::warn!("Rejected image dimensions for {}", path.display());
    }
    surface
}

pub async fn gfxload(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let file_name = vm.eval_expr(&args[1]).await?.as_string();
    let Some(mut surface) = gfx_decode_image(vm, &file_name).await else {
        return Ok(());
    };
    surface.cacheable = true;
    if let Some(graphics) = &mut vm.icy_board_state.ppl_graphics {
        if graphics.insert_surface(slot, surface) {
            vm.icy_board_state.gfx_error = 0;
        } else {
            vm.icy_board_state.gfx_error = 5;
            log::warn!("GFXLOAD graphics memory budget exhausted");
        }
    } else {
        vm.icy_board_state.gfx_error = 1;
    }
    Ok(())
}

pub async fn gfxclear(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let color = vm.eval_expr(&args[1]).await?.as_unsigned() as u32;
    match &mut vm.icy_board_state.ppl_graphics {
        Some(graphics) => match graphics.surfaces.get_mut(&slot) {
            Some(surface) => {
                graphics.pinned.remove(&slot);
                surface.clear(color);
                vm.icy_board_state.gfx_error = 0;
            }
            None => vm.icy_board_state.gfx_error = 2,
        },
        None => vm.icy_board_state.gfx_error = 1,
    }
    Ok(())
}

pub async fn gfxfillrect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let x = vm.eval_expr(&args[1]).await?.as_int();
    let y = vm.eval_expr(&args[2]).await?.as_int();
    let width = vm.eval_expr(&args[3]).await?.as_int();
    let height = vm.eval_expr(&args[4]).await?.as_int();
    let color = vm.eval_expr(&args[5]).await?.as_unsigned() as u32;
    match &mut vm.icy_board_state.ppl_graphics {
        Some(graphics) => match graphics.surfaces.get_mut(&slot) {
            Some(surface) => {
                graphics.pinned.remove(&slot);
                surface.fill_rect(x, y, width, height, color);
                vm.icy_board_state.gfx_error = 0;
            }
            None => vm.icy_board_state.gfx_error = 2,
        },
        None => vm.icy_board_state.gfx_error = 1,
    }
    Ok(())
}

pub async fn gfxrect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let x = vm.eval_expr(&args[1]).await?.as_int();
    let y = vm.eval_expr(&args[2]).await?.as_int();
    let width = vm.eval_expr(&args[3]).await?.as_int();
    let height = vm.eval_expr(&args[4]).await?.as_int();
    let color = vm.eval_expr(&args[5]).await?.as_unsigned() as u32;
    match &mut vm.icy_board_state.ppl_graphics {
        Some(graphics) => match graphics.surfaces.get_mut(&slot) {
            Some(surface) => {
                graphics.pinned.remove(&slot);
                surface.rect(x, y, width, height, color);
                vm.icy_board_state.gfx_error = 0;
            }
            None => vm.icy_board_state.gfx_error = 2,
        },
        None => vm.icy_board_state.gfx_error = 1,
    }
    Ok(())
}

pub async fn gfxblit(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let destination = vm.eval_expr(&args[0]).await?.as_int();
    let source = vm.eval_expr(&args[1]).await?.as_int();
    let x = vm.eval_expr(&args[2]).await?.as_int();
    let y = vm.eval_expr(&args[3]).await?.as_int();
    let Some(graphics) = &mut vm.icy_board_state.ppl_graphics else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(());
    };
    let Some(source_surface) = graphics.surfaces.get(&source).cloned() else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(());
    };
    let Some(destination_surface) = graphics.surfaces.get_mut(&destination) else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(());
    };
    graphics.pinned.remove(&destination);
    destination_surface.blit(&source_surface, (0, 0, source_surface.width as i32, source_surface.height as i32), (x, y));
    vm.icy_board_state.gfx_error = 0;
    Ok(())
}

pub async fn gfxblitrect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let destination = vm.eval_expr(&args[0]).await?.as_int();
    let source = vm.eval_expr(&args[1]).await?.as_int();
    let source_x = vm.eval_expr(&args[2]).await?.as_int();
    let source_y = vm.eval_expr(&args[3]).await?.as_int();
    let source_width = vm.eval_expr(&args[4]).await?.as_int();
    let source_height = vm.eval_expr(&args[5]).await?.as_int();
    let x = vm.eval_expr(&args[6]).await?.as_int();
    let y = vm.eval_expr(&args[7]).await?.as_int();
    let Some(graphics) = &mut vm.icy_board_state.ppl_graphics else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(());
    };
    let Some(source_surface) = graphics.surfaces.get(&source).cloned() else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(());
    };
    let Some(destination_surface) = graphics.surfaces.get_mut(&destination) else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(());
    };
    graphics.pinned.remove(&destination);
    destination_surface.blit(&source_surface, (source_x, source_y, source_width, source_height), (x, y));
    vm.icy_board_state.gfx_error = 0;
    Ok(())
}

fn gfx_sixel_output(surface: &crate::icy_board::state::ppl_graphics::GfxSurface) -> Option<Vec<u8>> {
    let options = icy_sixel::EncodeOptions::default();
    let encoded = match icy_sixel::sixel_encode(&surface.pixels, surface.width, surface.height, &options) {
        Ok(encoded) => encoded,
        Err(err) => {
            log::warn!("GFXPRESENT sixel encode failed: {err}");
            return None;
        }
    };
    let mut output = Vec::with_capacity(encoded.len() + 19);
    output.extend_from_slice(b"\x1b[?1070h");
    output.extend_from_slice(b"\x1b[H");
    output.extend_from_slice(encoded.as_bytes());
    output.extend_from_slice(b"\x1b[?1070l");
    Some(output)
}

/// A composed image is opaque, so the alpha channel is dropped rather than encoded.
fn gfx_jxl_encode(surface: &crate::icy_board::state::ppl_graphics::GfxSurface) -> Option<Vec<u8>> {
    use zune_core::{bit_depth::BitDepth, colorspace::ColorSpace, options::EncoderOptions};
    use zune_jpegxl::JxlSimpleEncoder;

    let rgb: Vec<u8> = surface.pixels.chunks_exact(4).flat_map(|pixel| [pixel[0], pixel[1], pixel[2]]).collect();
    let options = EncoderOptions::new(surface.width, surface.height, ColorSpace::RGB, BitDepth::Eight);
    let mut encoded = Vec::new();
    match JxlSimpleEncoder::new(&rgb, options).encode(&mut encoded) {
        Ok(_) => Some(encoded),
        Err(err) => {
            log::warn!("GFXPRESENT JPEG XL encode failed: {err:?}");
            None
        }
    }
}

/// The client side transforms the image APC applies while drawing: integer zoom
/// (`ZX`/`ZY`) and the mirror flags (`FX`/`FY`).
#[derive(Default, Clone, Copy)]
struct GfxTransform {
    zoom: Option<(u32, u32)>,
    flip_x: bool,
    flip_y: bool,
}

impl GfxTransform {
    fn is_identity(&self) -> bool {
        self.zoom.is_none() && !self.flip_x && !self.flip_y
    }

    fn options(&self) -> String {
        use std::fmt::Write as _;

        let mut options = String::new();
        if let Some((zoom_x, zoom_y)) = self.zoom {
            let _ = write!(options, ";ZX={zoom_x};ZY={zoom_y}");
        }
        if self.flip_x {
            options.push_str(";FX");
        }
        if self.flip_y {
            options.push_str(";FY");
        }
        options
    }
}

/// Hands one surface to the caller at a pixel destination, the way the active backend wants it.
///
/// The JPEG XL path prefers the per board cache: a surface that still holds what `GFXLOAD`
/// read is stored under its content hash and drawn by name, so it survives into later
/// sessions and is uploaded at most once. A composed frame changes every time and would only
/// fill that cache up, so it goes inline where the terminal is new enough to take it.
async fn gfx_present_surface(
    vm: &mut VirtualMachine<'_>,
    surface: crate::icy_board::state::ppl_graphics::GfxSurface,
    slot: i32,
    destination: (i32, i32),
    transform: GfxTransform,
) -> Res<()> {
    use crate::icy_board::state::ppl_graphics::{CACHE_PREFIX, GFX_BACKEND_JXL};
    use base64::{Engine as _, engine::general_purpose};
    use sha2::{Digest, Sha256};

    let Some((backend, capabilities)) = vm
        .icy_board_state
        .ppl_graphics
        .as_ref()
        .map(|graphics| (graphics.backend, graphics.capabilities))
    else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(());
    };
    if backend != GFX_BACKEND_JXL {
        if !transform.is_identity() {
            vm.icy_board_state.gfx_error = 6;
            return Ok(());
        }
        let Some(output) = gfx_sixel_output(&surface) else {
            vm.icy_board_state.gfx_error = 4;
            return Ok(());
        };
        vm.icy_board_state.connection.send(&output).await?;
        vm.icy_board_state.ppl_graphics.as_mut().expect("graphics state was checked").sixel_presented = true;
        vm.icy_board_state.gfx_error = 0;
        return finish_gfx_frame(vm).await;
    }
    let cacheable = surface.cacheable;

    let Some(encoded) = gfx_jxl_encode(&surface) else {
        vm.icy_board_state.gfx_error = 4;
        return Ok(());
    };
    if encoded.len() > MAX_GFX_FRAME_BYTES {
        log::warn!("GFXPRESENT frame of {} bytes is too large to send", encoded.len());
        vm.icy_board_state.gfx_error = 5;
        return Ok(());
    }
    let (x, y) = destination;
    let placement = format!("DX={x};DY={y}{}", transform.options());

    if !cacheable && capabilities.inline_blobs() {
        let payload = general_purpose::STANDARD.encode(&encoded);
        send_gfx_apc(vm, &format!("SyncTERM:C;DrawJXLBlob;{placement};{payload}")).await?;
        vm.icy_board_state.gfx_error = 0;
        return Ok(());
    }

    let name = if cacheable {
        format!("{CACHE_PREFIX}{}.jxl", &format!("{:x}", Sha256::digest(&encoded))[..32])
    } else {
        // A frame that keeps changing reuses one name per node and surface instead of
        // leaving a new file behind for every frame drawn.
        format!("{CACHE_PREFIX}n{}s{}.jxl", vm.icy_board_state.node, slot.unsigned_abs())
    };
    if !cacheable || vm.icy_board_state.gfx_cache.insert(name.clone()) {
        if cacheable && !vm.icy_board_state.reserve_media_upload(encoded.len()) {
            vm.icy_board_state.gfx_cache.remove(&name);
            vm.icy_board_state.gfx_error = 5;
            log::warn!("Graphics media upload budget exhausted");
            return Ok(());
        }
        let payload = general_purpose::STANDARD.encode(&encoded);
        send_apc(vm, &format!("SyncTERM:C;S;{name};{payload}")).await?;
        vm.icy_board_state.acknowledge_upload(payload.len()).await?;
    }
    send_gfx_apc(vm, &format!("SyncTERM:C;DrawJXL;{placement};{name}")).await?;
    vm.icy_board_state.gfx_error = 0;
    Ok(())
}

/// Base64 inflates the payload by a third, so the encoded frame is capped well below
/// what a terminal is willing to buffer.
const MAX_GFX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// What the client refuses to scale beyond, so an impossible request is reported here.
const MAX_GFX_SCALED_PIXELS: u64 = 16_000_000;

pub async fn gfxpresent(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(buffer) = vm
        .icy_board_state
        .ppl_graphics
        .as_ref()
        .and_then(|graphics| graphics.pinned.get(&slot))
        .copied()
    {
        send_gfx_apc(vm, &format!("SyncTERM:P;Paste;B={buffer};DX=0;DY=0")).await?;
        vm.icy_board_state.gfx_error = 0;
        return Ok(());
    }
    let Some(surface) = vm
        .icy_board_state
        .ppl_graphics
        .as_ref()
        .and_then(|graphics| graphics.surfaces.get(&slot))
        .cloned()
    else {
        vm.icy_board_state.gfx_error = if vm.icy_board_state.ppl_graphics.is_some() { 2 } else { 1 };
        return Ok(());
    };
    gfx_present_surface(vm, surface, slot, (0, 0), GfxTransform::default()).await
}

pub async fn gfxpresentrect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    use crate::icy_board::state::ppl_graphics::GFX_BACKEND_JXL;

    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let x = vm.eval_expr(&args[1]).await?.as_int();
    let y = vm.eval_expr(&args[2]).await?.as_int();
    let width = vm.eval_expr(&args[3]).await?.as_int();
    let height = vm.eval_expr(&args[4]).await?.as_int();
    let destination = match (args.get(5), args.get(6)) {
        (Some(destination_x), Some(destination_y)) => (vm.eval_expr(destination_x).await?.as_int(), vm.eval_expr(destination_y).await?.as_int()),
        _ => (x, y),
    };
    let requested_size = match (args.get(7), args.get(8)) {
        (Some(dest_width), Some(dest_height)) => Some((vm.eval_expr(dest_width).await?.as_int(), vm.eval_expr(dest_height).await?.as_int())),
        _ => None,
    };
    let flip = match args.get(9) {
        Some(flip) => vm.eval_expr(flip).await?.as_int(),
        None => 0,
    };
    let zoom = match requested_size {
        Some((dest_width, dest_height)) => {
            if dest_width <= 0 || dest_height <= 0 || u64::from(dest_width.unsigned_abs()) * u64::from(dest_height.unsigned_abs()) > MAX_GFX_SCALED_PIXELS {
                vm.icy_board_state.gfx_error = 5;
                return Ok(());
            }
            if width <= 0 || height <= 0 || dest_width % width != 0 || dest_height % height != 0 {
                vm.icy_board_state.gfx_error = 6;
                return Ok(());
            }
            let zoom_x = (dest_width / width).unsigned_abs();
            let zoom_y = (dest_height / height).unsigned_abs();
            if zoom_x == 0 || zoom_y == 0 {
                vm.icy_board_state.gfx_error = 6;
                return Ok(());
            }
            Some((zoom_x, zoom_y))
        }
        None => None,
    };
    let transform = GfxTransform {
        zoom,
        flip_x: flip & 1 != 0,
        flip_y: flip & 2 != 0,
    };
    // Scaling and mirroring are image APC options, so sixel cannot serve them.
    if !transform.is_identity() {
        match vm.icy_board_state.ppl_graphics.as_ref().map(|graphics| graphics.backend) {
            Some(GFX_BACKEND_JXL) => {}
            Some(_) => {
                vm.icy_board_state.gfx_error = 6;
                return Ok(());
            }
            None => {
                vm.icy_board_state.gfx_error = 1;
                return Ok(());
            }
        }
    }
    if let Some(buffer) = vm
        .icy_board_state
        .ppl_graphics
        .as_ref()
        .and_then(|graphics| graphics.pinned.get(&slot))
        .copied()
    {
        send_gfx_apc(
            vm,
            &format!(
                "SyncTERM:P;Paste;B={buffer};SX={x};SY={y};SW={width};SH={height};DX={};DY={}{}",
                destination.0,
                destination.1,
                transform.options()
            ),
        )
        .await?;
        vm.icy_board_state.gfx_error = 0;
        return Ok(());
    }

    let prepared = {
        let Some(graphics) = vm.icy_board_state.ppl_graphics.as_ref() else {
            vm.icy_board_state.gfx_error = 1;
            return Ok(());
        };
        let Some(surface) = graphics.surfaces.get(&slot) else {
            vm.icy_board_state.gfx_error = 2;
            return Ok(());
        };
        // Sixel has to draw from the screen origin, so its region keeps the pixels in
        // place; the image APC takes the region on its own and is told where it goes.
        if graphics.backend == GFX_BACKEND_JXL {
            surface.region(x, y, width, height).map(|(region, _, _)| (region, destination.0, destination.1))
        } else {
            let Some(output) = surface.region_at((x, y, width, height), destination).as_ref().and_then(gfx_sixel_output) else {
                vm.icy_board_state.gfx_error = 5;
                return Ok(());
            };
            vm.icy_board_state.connection.send(&output).await?;
            vm.icy_board_state.ppl_graphics.as_mut().expect("graphics state was checked").sixel_presented = true;
            finish_gfx_frame(vm).await?;
            vm.icy_board_state.gfx_error = 0;
            return Ok(());
        }
    };
    let Some((region, origin_x, origin_y)) = prepared else {
        vm.icy_board_state.gfx_error = 5;
        return Ok(());
    };
    gfx_present_surface(vm, region, slot, (origin_x, origin_y), transform).await
}

pub async fn gfxpresentat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    use crate::icy_board::state::ppl_graphics::GFX_BACKEND_JXL;

    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let column = vm.eval_expr(&args[1]).await?.as_int().max(1);
    let row = vm.eval_expr(&args[2]).await?.as_int().max(1);

    let prepared = {
        let Some(graphics) = vm.icy_board_state.ppl_graphics.as_ref() else {
            vm.icy_board_state.gfx_error = 1;
            return Ok(());
        };
        if !graphics.surfaces.contains_key(&slot) {
            vm.icy_board_state.gfx_error = 2;
            return Ok(());
        }
        if graphics.backend == GFX_BACKEND_JXL {
            let capabilities = graphics.capabilities;
            graphics
                .surfaces
                .get(&slot)
                .cloned()
                .map(|surface| (surface, ((column - 1) * capabilities.cell_width, (row - 1) * capabilities.cell_height)))
        } else {
            let Some(encoded) = graphics
                .surfaces
                .get(&slot)
                .and_then(|surface| icy_sixel::sixel_encode(&surface.pixels, surface.width, surface.height, &icy_sixel::EncodeOptions::default()).ok())
            else {
                vm.icy_board_state.gfx_error = 4;
                return Ok(());
            };
            let mut output = Vec::with_capacity(encoded.len() + 24);
            output.extend_from_slice(b"\x1b[?1070h");
            output.extend_from_slice(b"\x1b7");
            output.extend_from_slice(format!("\x1b[{row};{column}H").as_bytes());
            output.extend_from_slice(encoded.as_bytes());
            output.extend_from_slice(b"\x1b8");
            output.extend_from_slice(b"\x1b[?1070l");
            vm.icy_board_state.connection.send(&output).await?;
            vm.icy_board_state.ppl_graphics.as_mut().expect("graphics state was checked").sixel_presented = true;
            finish_gfx_frame(vm).await?;
            vm.icy_board_state.gfx_error = 0;
            return Ok(());
        }
    };
    let Some((surface, destination)) = prepared else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(());
    };
    gfx_present_surface(vm, surface, slot, destination, GfxTransform::default()).await
}

pub async fn gfxfree(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let slot = vm.eval_expr(&args[0]).await?.as_int();
    vm.icy_board_state.gfx_error = match &mut vm.icy_board_state.ppl_graphics {
        Some(graphics) => {
            if graphics.remove_surface(slot) {
                0
            } else {
                2
            }
        }
        None => 1,
    };
    Ok(())
}

pub async fn gfxpin(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    use base64::{Engine as _, engine::general_purpose};

    let slot = vm.eval_expr(&args[0]).await?.as_int();
    let enabled = match args.get(1) {
        Some(enabled) => vm.eval_expr(enabled).await?.as_bool(),
        None => true,
    };
    if !enabled {
        if let Some(graphics) = &mut vm.icy_board_state.ppl_graphics {
            vm.icy_board_state.gfx_error = if graphics.pinned.remove(&slot).is_some() { 0 } else { 2 };
        } else {
            vm.icy_board_state.gfx_error = 1;
        }
        return Ok(());
    }
    let Some(graphics) = vm.icy_board_state.ppl_graphics.as_mut() else {
        vm.icy_board_state.gfx_error = 1;
        return Ok(());
    };
    if graphics.backend != crate::icy_board::state::ppl_graphics::GFX_BACKEND_JXL {
        vm.icy_board_state.gfx_error = 6;
        return Ok(());
    }
    let Some(surface) = graphics.surfaces.get(&slot).cloned() else {
        vm.icy_board_state.gfx_error = 2;
        return Ok(());
    };
    let Some(buffer) = graphics.pin(slot) else {
        vm.icy_board_state.gfx_error = 5;
        return Ok(());
    };
    let Some(encoded) = gfx_jxl_encode(&surface) else {
        vm.icy_board_state.gfx_error = 4;
        return Ok(());
    };
    let payload = general_purpose::STANDARD.encode(encoded);
    send_apc(vm, &format!("SyncTERM:C;LoadJXLBlob;B={buffer};{payload}")).await?;
    vm.icy_board_state.gfx_error = 0;
    Ok(())
}

pub async fn gfxsetpacing(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let frames = vm.eval_expr(&args[0]).await?.as_int();
    gfx_set_pacing(vm, frames > 0);
    Ok(())
}

pub(crate) fn gfx_set_pacing(vm: &mut VirtualMachine<'_>, enabled: bool) {
    if let Some(graphics) = &mut vm.icy_board_state.ppl_graphics {
        graphics.pacing = enabled;
        vm.icy_board_state.gfx_error = 0;
    } else {
        vm.icy_board_state.gfx_error = 1;
    }
}

pub async fn gfxshutdown(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    gfx_shutdown(vm).await
}

pub(crate) async fn gfx_shutdown(vm: &mut VirtualMachine<'_>) -> Res<()> {
    let Some(graphics) = vm.icy_board_state.ppl_graphics.take() else {
        vm.icy_board_state.gfx_error = 0;
        return Ok(());
    };
    vm.icy_board_state.gfx_error = 0;
    if graphics.fullscreen {
        vm.icy_board_state.connection.send(b"\x1b[?80h\x1b[?7h\x1b[?25h").await?;
    }
    if graphics.sixel_presented {
        vm.icy_board_state.connection.send(dos_palette_sequence().as_bytes()).await?;
    }
    vm.icy_board_state.print(TerminalTarget::Both, "\x1b[0m").await?;
    vm.icy_board_state.reset_color(TerminalTarget::Both).await
}

/// Runs one `GFX` member.
pub(crate) async fn gfx_member(vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, arguments: &[VariableValue]) -> Res<VariableValue> {
    use crate::icy_board::state::ppl_gfx::{INIT, SHUTDOWN};

    if *name == *INIT {
        let requested = arguments
            .first()
            .map_or(crate::icy_board::state::ppl_graphics::GFX_BACKEND_AUTO, VariableValue::as_int);
        let fullscreen = arguments.get(1).is_none_or(VariableValue::as_bool);
        gfx_init(vm, requested, fullscreen).await?;
        return Ok(VariableValue::new_bool(vm.icy_board_state.gfx_error == 0));
    }
    if *name == *SHUTDOWN {
        gfx_shutdown(vm).await?;
        return Ok(VariableValue::new_bool(true));
    }
    Err(format!("Unknown GFX function {name}").into())
}
