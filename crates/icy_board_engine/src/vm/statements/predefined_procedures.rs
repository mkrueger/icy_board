use std::{env, fs, thread, time::Duration};

use crate::{
    datetime::IcbDate,
    executable::{PPEExpr, VariableType, VariableValue},
    icy_board::{
        icb_config::IcbColor,
        state::{
            functions::{display_flags, MASK_ALNUM},
            GraphicsMode, NodeState, NodeStatus,
        },
    },
    Res,
};
use bstr::BString;
use chrono::{DateTime, Utc};
use codepages::tables::CP437_TO_UNICODE;
use icy_engine::{BufferType, OutputFormat, SaveOptions, ScreenPreperation};
use jamjam::jam::JamMessage;

use crate::{
    icy_board::icb_text::IceText,
    vm::{TerminalTarget, VMError, VirtualMachine},
};

use super::super::errors::IcyError;

/// Should never be called. But some op codes are invalid as statement call (like if or return)
/// and are handled by it's own `PPECommands` and will point to this function.
///
/// # Panics
///
/// Always
pub async fn invalid(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
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

    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn confunflag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    vm.io.fcreate(channel, &file, am, sm);
    Ok(())
}

pub async fn fopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    vm.io.fopen(channel, &file, am, sm)?;
    Ok(())
}

pub async fn fappend(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    vm.io.fappend(channel, &file);
    Ok(())
}

pub async fn fclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int();
    if channel == -1 {
        // READLINE uses -1 as a special value
        return Ok(());
    }
    vm.io.fclose(channel as usize)?;
    Ok(())
}

pub async fn fget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let value = VariableValue::new_string(vm.io.fget(channel)?);
    vm.set_variable(&args[1], value).await?;
    Ok(())
}

pub async fn fput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;

    for value in &args[1..] {
        let text = vm.eval_expr(value).await?.as_string();
        vm.io.fput(channel, text)?;
    }
    Ok(())
}

pub async fn fputln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;

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
    // TODO?: unused
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn fputpad(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
        vm.put_user_variables(&mut user);
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
    if let Err(err) = vm.io.delete(&file) {
        if err.kind() != std::io::ErrorKind::NotFound {
            log::error!("Error deleting file'{}': {}", file, err);
        }
    }
    Ok(())
}

pub async fn deluser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn adjtime(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let min = vm.eval_expr(&args[0]).await?.as_int();
    vm.icy_board_state.session.time_limit += min;
    Ok(())
}

pub async fn log(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut msg = vm.eval_expr(&args[0]).await?.as_string();
    let left_justify = vm.eval_expr(&args[1]).await?.as_bool();
    if left_justify {
        msg = msg.trim_start().to_string();
    }
    log::info!("{}", msg);
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
            &MASK_ALNUM,
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
            &MASK_ALNUM,
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
            &"",
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
    if default.is_empty() {
        None
    } else {
        Some(default)
    }
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
    // TODO: Money conversion.
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
    vm.set_variable(&args[1], VariableValue::new_int(output.parse::<i32>()?)).await?;
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
    // TODO: Date conversion
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
    // TODO: Time conversion
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}
pub async fn promptstr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_int();
    // 1 Output Variable
    let len = vm.eval_expr(&args[2]).await?.as_int();
    let valid = vm.eval_expr(&args[3]).await?.as_string();
    let flags = vm.eval_expr(&args[4]).await?.as_int();
    let output = vm
        .icy_board_state
        .input_field(IceText::from(prompt as usize), len, &valid, "", None, flags)
        .await?;
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
        thread::sleep(Duration::from_millis((ticks as f32 * 1000.0 / 18.2) as u64));
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
    let str = vm.eval_expr(&args[0]).await?.to_string();
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
    let cmd = vm.eval_expr(&args[0]).await?.as_string();

    log::error!("PPE wanted to shell out to '{cmd}'!");
    //Err(VMError::ErrorInFunctionCall("shell".to_string(), "shell out is not supported.".to_string()).into())
    Ok(())
}

pub async fn disptext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let rec = vm.eval_expr(&args[0]).await?.as_int();
    let flags = vm.eval_expr(&args[1]).await?.as_int();

    vm.icy_board_state.display_text(IceText::from(rec as usize), flags).await
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
        vm.icy_board_state.join_conference(conf as u16, true).await;
    }
    Ok(())
}
pub async fn quest(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let nr = vm.eval_expr(&args[0]).await?.as_int();
    let surveys = vm.icy_board_state.load_surveys().await?;
    if let Some(survey) = surveys.get(nr as usize) {
        vm.icy_board_state.start_survey(survey).await?;
    }
    Ok(())
}

pub async fn blt(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let nr = vm.eval_expr(&args[0]).await?.as_int();
    let bulletins = vm.icy_board_state.load_bullettins().await?;
    if let Some(bulletin) = bulletins.get(nr as usize) {
        vm.icy_board_state.display_file(&bulletin.file).await?;
    }
    Ok(())
}

pub async fn dir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("dir not implemented statement!");
    panic!("TODO")
}

pub async fn kbdstuff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.stuff_keyboard_buffer(&value, false)?;
    Ok(())
}
pub async fn kbdstring(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.print(TerminalTarget::Both, &value).await?;
    vm.icy_board_state.stuff_keyboard_buffer(&value, false)?;
    Ok(())
}
pub async fn kbdfile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    let fil_name = vm.resolve_file(&file_name).await;
    let contents = fs::read_to_string(file_name)?;
    vm.icy_board_state.stuff_keyboard_buffer(&contents, false)?;

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
    log::error!("not implemented statement!");
    panic!("TODO")
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
    vm.icy_board_state.print(TerminalTarget::Both, &value).await
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
            status: node.status.clone(),
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
    let broadcast = vm.eval_expr(&args[5]).await?.as_string();

    // Todo: Name/City/Broadcast

    if let Some(Some(node)) = vm.icy_board_state.node_state.lock().await.get_mut(node as usize) {
        if !stat.is_empty() {
            if let Some(stat) = NodeStatus::from_char(stat.chars().next().unwrap()) {
                node.status = stat;
            }
        }
        node.operation = operation;
    } else {
        log::error!("PPE wrunet - node invalid: {}", node);
    }

    Ok(())
}

pub async fn dointr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("System interrupts are not (yet) supported")
}
pub async fn varseg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("System interrupts are not (yet) supported")
}
pub async fn varoff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("System interrupts are not (yet) supported")
}
pub async fn pokeb(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("DOS memory access is not (yet) supported")
}
pub async fn pokew(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("DOS memory access is not (yet) supported")
}
pub async fn varaddr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("System interrupts are not (yet) supported")
}

pub async fn ansipos(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let x = vm.eval_expr(&args[0]).await?.as_int();
    let y = vm.eval_expr(&args[1]).await?.as_int();
    vm.icy_board_state.gotoxy(TerminalTarget::Both, x, y).await
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
    vm.icy_board_state.print(TerminalTarget::Both, "\r\n").await?;
    Ok(())
}
pub async fn wrusysdoor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("wrusysdoor not implemented statement!");
    panic!("TODO")
}
pub async fn wrusys(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("USER.SYS is not supported")
}
pub async fn rdusys(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("USER.SYS is not supported")
}
pub async fn newpwd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let new_pwd = vm.eval_expr(&args[0]).await?.as_string();
    let was_changed = vm.icy_board_state.change_password(&new_pwd).await?;
    vm.set_variable(&args[1], VariableValue::new_bool(was_changed)).await?;
    Ok(())
}
pub async fn opencap(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn closecap(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub async fn message(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let conf = vm.eval_expr(&args[0]).await?.as_int();
    let to = vm.eval_expr(&args[1]).await?.as_string();
    let from = vm.eval_expr(&args[2]).await?.as_string();
    let subject = vm.eval_expr(&args[3]).await?.as_string();
    let sec = vm.eval_expr(&args[4]).await?.as_string(); // Security
    let pack_out_date = vm.eval_expr(&args[5]).await?.as_int() as u32;
    let retreceipt = vm.eval_expr(&args[6]).await?.as_bool();
    let echo = vm.eval_expr(&args[7]).await?.as_bool();
    let file = vm.eval_expr(&args[8]).await?.as_string();
    let file = vm.resolve_file(&file).await;
    if !file.exists() {
        log::error!("PPE function 'message': Message text file not found {}", file.display());
        return Err(Box::new(IcyError::FileNotFound("MESSAGE".to_string(), file.display().to_string())));
    }
    let mut message = JamMessage::default()
        .with_from(BString::from(from))
        .with_to(BString::from(to))
        .with_subject(BString::from(subject))
        .with_date_time(Utc::now())
        .with_text(BString::from(fs::read_to_string(file)?));
    // TODO: Message Security
    if pack_out_date > 0 {
        message = message.with_packout_date(IcbDate::from_pcboard(pack_out_date).to_utc_date_time());
    }

    if conf >= 0 {
        if let Ok(area_opt) = vm.icy_board_state.show_message_areas(conf as u16).await {
            match area_opt {
                Some(area) => {
                    vm.icy_board_state
                        .send_message(conf as i32, area as i32, message, IceText::SavingMessage)
                        .await?;
                }
                None => {
                    vm.icy_board_state
                        .display_text(IceText::MessageAborted, display_flags::LFBEFORE | display_flags::NEWLINE)
                        .await?;
                    log::error!("Message area not found: {}", conf);
                }
            }
        }
    } else {
        vm.icy_board_state.send_message(-1, 0, message, IceText::SavingMessage).await?;
    }
    Ok(())
}

pub async fn savescrn(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut buf = vm.icy_board_state.display_screen().buffer.flat_clone(false);
    buf.buffer_type = BufferType::Unicode;
    vm.stored_screen = Some(buf);
    Ok(())
}

pub async fn restscrn(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    if let Some(screen) = &vm.stored_screen {
        let mut options = SaveOptions::default();
        options.screen_preparation = ScreenPreperation::ClearScreen;
        options.save_sauce = false;
        options.modern_terminal_output = true;
        let res = icy_engine::formats::PCBoard::default().to_bytes(screen, &options)?;
        let res = unsafe { String::from_utf8_unchecked(res) };
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
    vm.icy_board_state.print(TerminalTarget::Both, "\r\n").await?;
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
        log::error!("Error renaming file: {}", err);
    }
    Ok(())
}
pub async fn frewind(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    vm.io.frewind(channel)?;
    Ok(())
}
pub async fn pokedw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO pokedw")
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
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let pos = vm.eval_expr(&args[1]).await?.as_int();
    let position = vm.eval_expr(&args[2]).await?.as_int();
    vm.io.fseek(channel, pos, position)?;
    Ok(())
}

pub async fn fflush(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("fflush not implemented statement!");
    panic!("TODO")
}
pub async fn fread(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let size = vm.eval_expr(&args[2]).await?.as_int() as usize;
    internal_fread(vm, channel, size, &args[1]).await
}

async fn internal_fread(vm: &mut VirtualMachine<'_>, channel: usize, size: usize, arg: &PPEExpr) -> Res<()> {
    let val = vm.eval_expr(&arg).await?;

    let result = vm.io.fread(channel, size).map_err(|e| {
        log::error!("fread error: {} ({})", e, channel);
        e
    })?;

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
            vm.set_variable(arg, VariableValue::new_bool(result[0] != 0)).await?;
        }
        VariableType::Byte | VariableType::SByte => {
            vm.set_variable(arg, VariableValue::new_byte(result[0])).await?;
        }
        VariableType::Word | VariableType::SWord => {
            vm.set_variable(arg, VariableValue::new_word(u16::from_le_bytes([result[0], result[1]])))
                .await?;
        }
        VariableType::Double => {
            vm.set_variable(
                arg,
                VariableValue::new_double(f64::from_le_bytes([
                    result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
                ])),
            )
            .await?;
        }
        _ => {
            match result.len() {
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
            };
        }
    };
    Ok(())
}

pub async fn fwrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let val = vm.eval_expr(&args[1]).await?;
    let size = vm.eval_expr(&args[2]).await?.as_int() as usize;
    internal_fwrite(vm, channel, val, size).await
}

async fn internal_fwrite(vm: &mut VirtualMachine<'_>, channel: usize, val: VariableValue, size: usize) -> Res<()> {
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
        log::error!("fwrite error: {} ({})", e, channel);
        e
    })?;
    Ok(())
}

pub async fn fdefin(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    vm.fd_default_in = channel;
    Ok(())
}
pub async fn fdefout(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
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
    log::error!("fdputpad not implemented statement!");
    panic!("TODO")
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
            user.stats.total_dnld_bytes += bytes as u64;
            user.stats.today_dnld_bytes += bytes as u64;
        } else {
            user.stats.total_dnld_bytes = user.stats.total_dnld_bytes.saturating_sub(bytes as u64);
            user.stats.today_dnld_bytes = user.stats.today_dnld_bytes.saturating_sub(bytes as u64);
        }
        vm.icy_board_state.session.bytes_remaining += bytes as i64;
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
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let file = vm.resolve_file(&file).await.to_string_lossy().to_string();
    vm.io.fappend(channel, &file);
    Ok(())
}

pub async fn copy(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let old = &vm.eval_expr(&args[0]).await?.as_string();
    let new = &vm.eval_expr(&args[1]).await?.as_string();
    if let Err(err) = vm.io.copy(old, new) {
        log::error!("Error renaming file: {}", err);
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
pub async fn keyflush(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // TODO?
    Ok(())
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
    vm.icy_board_state.download().await?;
    Ok(())
}

pub async fn getaltuser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let user_record = vm.eval_expr(&args[0]).await?.as_int() as usize;
    if user_record >= vm.icy_board_state.get_board().await.users.len() {
        return Err(Box::new(VMError::UserRecordOutOfBounds(user_record)));
    }
    vm.user = vm.icy_board_state.get_board().await.users[user_record].clone();
    log::info!("get alt user ({user_record}) {}", vm.user.name);
    vm.set_user_variables()?;
    Ok(())
}

pub async fn adjdbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let bytes = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if bytes > 0 {
            user.stats.today_dnld_bytes += bytes as u64;
        } else {
            user.stats.today_dnld_bytes = user.stats.today_dnld_bytes.saturating_sub(bytes as u64);
        }
        vm.icy_board_state.session.bytes_remaining += bytes as i64;
    }
    Ok(())
}
pub async fn adjtbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let bytes: i32 = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if bytes > 0 {
            user.stats.total_dnld_bytes += bytes as u64;
        } else {
            user.stats.total_dnld_bytes = user.stats.total_dnld_bytes.saturating_sub(bytes as u64);
        }
    }
    Ok(())
}
pub async fn ayjtfiles(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let files = vm.eval_expr(&args[0]).await?.as_int();
    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if files > 0 {
            user.stats.num_downloads += files as u64;
        } else {
            user.stats.num_downloads = user.stats.num_downloads.saturating_sub(files as u64);
        }
    }
    Ok(())
}

pub async fn lang(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let language = vm.eval_expr(&args[0]).await?.as_int();
    let lang = if let Some(lang) = vm.icy_board_state.board.lock().await.languages.languages.get(language as usize) {
        lang.extension.clone()
    } else {
        log::error!("PPE: lang(): Language not found: {}", language);
        return Ok(());
    };
    vm.icy_board_state.session.language = lang.clone();
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
    log::error!("searchinit not implemented statement!");
    panic!("TODO")
}
pub async fn searchfind(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("searchfind not implemented statement!");
    panic!("TODO")
}
pub async fn searchstop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("searchstop not implemented statement!");
    panic!("TODO")
}
pub async fn prfound(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn prfoundln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!(" prfoundlnnot implemented statement!");
    panic!("TODO")
}

pub async fn tpaget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn tpaput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn tpacgea(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn tpacput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn tparead(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn tpawrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn tpacread(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn tpacwrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    log::error!("brag not implemented statement!");
    panic!("TODO")
}
pub async fn frealtuser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    if let Some(user) = vm.icy_board_state.session.current_user.clone() {
        vm.user = user;
        vm.set_user_variables()?;
    }
    Ok(())
}
pub async fn setlmr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!(" setlmr not implemented statement!");
    panic!("TODO")
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
    for i in 0..8 {
        let _ = vm.io.fclose(i);
    }
    Ok(())
}
pub async fn stackabort(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("stackabort not implemented statement!");
    panic!("TODO")
}
pub async fn dcreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dsetalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dpack(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dcloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dlock(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dlockr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dlockg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dunlock(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dncreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dnopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dnclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dncloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dnew(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dadd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dappend(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dtop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dgo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dbottom(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dskip(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn ddelete(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn drecall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dtag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dseek(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dfblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dfcopy(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub async fn account(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("account not implemented statement!");
    panic!("TODO")
}

pub async fn recordusage(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("recordusage not implemented statement!");
    panic!("TODO")
}
pub async fn msgtofile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("msgtofile not implemented statement!");
    panic!("TODO")
}
pub async fn qwklimits(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("qwklimits not implemented statement!");
    panic!("TODO")
}
pub async fn command(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let via_cmd_list = vm.eval_expr(&args[0]).await?.as_bool();
    let command_line = vm.eval_expr(&args[1]).await?.as_string();
    vm.icy_board_state.run_single_command(via_cmd_list, command_line).await
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
            log::error!("PPE unsupported graphics mode: {}", mode);
        }
    }
    Ok(())
}

pub async fn adduser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("adduser not implemented statement!");
    panic!("TODO")
}
pub async fn killmsg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("killmsg not implemented statement!");
    panic!("TODO")
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
    log::error!("fdowraka not implemented statement!");
    panic!("TODO")
}
pub async fn fdoaddaka(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("fdoaddaka not implemented statement!");
    panic!("TODO")
}
pub async fn fdowrorg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("fdowrorg not implemented statement!");
    panic!("TODO")
}
pub async fn fdoaddorg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("fdoaddorg not implemented statement!");
    panic!("TODO")
}
pub async fn fdoqmod(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("fdoqmod not implemented statement!");
    panic!("TODO")
}
pub async fn fdoqadd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("fdoqadd not implemented statement!");
    panic!("TODO")
}
pub async fn fdoqdel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("fdoqdel not implemented statement!");
    panic!("TODO")
}
pub async fn sounddelay(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::warn!("SOUNDDELAY is not supported");
    Ok(())
}
