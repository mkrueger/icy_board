use std::{fs, thread, time::Duration};

use crate::{
    executable::{PPEExpr, VariableType, VariableValue},
    icy_board::{
        icb_config::IcbColor,
        state::functions::{display_flags, MASK_ALNUM},
    },
    Res,
};
use codepages::tables::CP437_TO_UNICODE;

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

pub async fn end(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.is_running = false;
    Ok(())
}

pub async fn cls(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.print(TerminalTarget::Both, "\x1B[2J").await
}

pub async fn clreol(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.print(TerminalTarget::Both, "\x1B[K").await
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

    let file_name = vm.resolve_file(&file_name);
    vm.icy_board_state.display_file(&file_name).await?;
    Ok(())
}

pub async fn fcreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).to_string_lossy().to_string();
    vm.io.fcreate(channel, &file, am, sm);
    Ok(())
}

pub async fn fopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).to_string_lossy().to_string();
    vm.io.fopen(channel, &file, am, sm)?;
    Ok(())
}

pub async fn fappend(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let file = vm.eval_expr(&args[1]).await?.as_string();
    let am = vm.eval_expr(&args[2]).await?.as_int();
    let sm = vm.eval_expr(&args[3]).await?.as_int();
    let file = vm.resolve_file(&file).to_string_lossy().to_string();
    vm.io.fappend(channel, &file, am, sm);
    Ok(())
}

pub async fn fclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int();
    if channel == -1 {
        // READLINE uses -1 as a special value
        return Ok(());
    }
    if !(0..=7).contains(&channel) {
        return Err(Box::new(IcyError::FileChannelOutOfBounds(channel)));
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
    let user = if let Some(user) = &mut vm.icy_board_state.current_user {
        user.clone()
    } else {
        return Err(Box::new(IcyError::UserNotFound(String::new())));
    };
    vm.set_user_variables(&user);

    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub async fn putuser(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<()> {
    if let Some(mut user) = vm.icy_board_state.current_user.take() {
        vm.put_user_variables(&mut user);
        vm.icy_board_state.current_user = Some(user);
    }
    Ok(())
}

pub async fn defcolor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.reset_color().await
}

pub async fn delete(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let file = &vm.eval_expr(&args[0]).await?.as_string();
    let file = vm.resolve_file(&file).to_string_lossy().to_string();
    if let Err(err) = vm.io.delete(&file) {
        log::error!("Error deleting file: {}", err);
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
    let msg = vm.eval_expr(&args[0]).await?.as_string();
    // let left = &vm.eval_expr(&args[0]).await?;
    log::info!("{}", msg);
    Ok(())
}

const TXT_STOPCHAR: char = '_';

pub async fn input(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0]).await?.as_string();
    let color = IcbColor::Dos(14);
    let output = vm
        .icy_board_state
        .input_string(
            color,
            prompt,
            60,
            &MASK_ALNUM,
            "",
            None,
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
    let output = vm.icy_board_state.input_string(color.into(), prompt, len, &valid, "", None, flags).await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}

pub async fn inputyn(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0]).await?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 1;
    let valid = "YyNn";
    let output = vm.icy_board_state.input_string(color.into(), prompt, len, valid, "", None, 0).await?;

    vm.set_variable(&args[1], VariableValue::new_string(output.to_ascii_uppercase())).await?;
    Ok(())
}

pub async fn inputmoney(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0]).await?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 13;
    let valid = "01234567890+-$.";
    let output = vm.icy_board_state.input_string(color.into(), prompt, len, valid, "", None, 0).await?;
    // TODO: Money conversion.
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}

pub async fn inputint(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0]).await?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 11;
    let valid = "01234567890+-";
    let output = vm.icy_board_state.input_string(color.into(), prompt, len, valid, "", None, 0).await?;
    vm.set_variable(&args[1], VariableValue::new_int(output.parse::<i32>()?)).await?;
    Ok(())
}
pub async fn inputcc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0]).await?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 16;
    let valid = "01234567890";
    let output = vm.icy_board_state.input_string(color.into(), prompt, len, valid, "", None, 0).await?;
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}
pub async fn inputdate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0]).await?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 8;
    let valid = "01234567890-/";
    let output = vm.icy_board_state.input_string(color.into(), prompt, len, valid, "", None, 0).await?;
    // TODO: Date conversion
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}

pub async fn inputtime(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0]).await?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2]).await?.as_int() as u8;
    let len = 8;
    let valid = "01234567890:";
    let output = vm.icy_board_state.input_string(color.into(), prompt, len, valid, "", None, 0).await?;
    // TODO: Time conversion
    vm.set_variable(&args[1], VariableValue::new_string(output)).await?;
    Ok(())
}
pub async fn promptstr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn cdchkoff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    log::error!("not implemented statement!");
    panic!("TODO")
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
    let split = str.split(&[' ', ';'][..]).map(std::string::ToString::to_string);
    vm.icy_board_state.session.tokens.extend(split);
    Ok(())
}

pub async fn gettoken(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub async fn shell(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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

pub async fn inputtext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn join(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn quest(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn blt(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn dir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub async fn kbdstuff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.put_keyboard_buffer(&value, false)?;
    Ok(())
}
pub async fn kbdstring(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0]).await?.as_string();
    vm.icy_board_state.print(TerminalTarget::Both, &value).await?;
    vm.icy_board_state.put_keyboard_buffer(&value, false)?;
    Ok(())
}
pub async fn kbdfile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    let fil_name = vm.resolve_file(&file_name);
    let contents = fs::read_to_string(file_name)?;
    vm.icy_board_state.put_keyboard_buffer(&contents, false)?;

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
    let lonode = vm.eval_expr(&args[0]).await?.as_int();
    let hinode = vm.eval_expr(&args[1]).await?.as_int();
    let message = vm.eval_expr(&args[2]).await?.as_string();
    // TODO: Broadcast
    log::error!("Broadcasting message from {lonode} to {hinode}: {message}");
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
    let value = vm.eval_expr(&args[0]).await?.as_int();

    if let Some(node) = vm.icy_board_state.nodes.get(value as usize) {
        vm.pcb_node = Some(node.clone());
    }
    Ok(())
}

pub async fn wrunet(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let node = vm.eval_expr(&args[0]).await?.as_int();
    let stat = vm.eval_expr(&args[1]).await?.as_string();
    let name = vm.eval_expr(&args[2]).await?.as_string();
    let city = vm.eval_expr(&args[3]).await?.as_string();
    let operation = vm.eval_expr(&args[4]).await?.as_string();
    let broadcast = vm.eval_expr(&args[5]).await?.as_string();

    // Todo: Broadcast

    if let Some(node) = vm.icy_board_state.nodes.get_mut(node as usize) {
        if !stat.is_empty() {
            node.status = stat.as_bytes()[0] as char;
        }
        node.name = name;
        node.city = city;
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
pub async fn wrusys(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("USER.SYS is not supported")
}
pub async fn rdusys(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("USER.SYS is not supported")
}
pub async fn newpwd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn savescrn(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn restscrn(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn sound(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // log::error!("not implemented statement!");
    // panic!("TODO")
    log::warn!("Sound is not supported");
    Ok(())
}
pub async fn chat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    vm.icy_board_state.print(TerminalTarget::User, "\n").await?;
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
    let old = vm.resolve_file(&old).to_string_lossy().to_string();
    let new = vm.resolve_file(&new).to_string_lossy().to_string();

    if let Err(err) = vm.io.rename(&old, &new) {
        log::error!("Error renaming file: {}", err);
    }
    Ok(())
}
pub async fn frewind(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn pokedw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub async fn pageoff(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub async fn fseek(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let pos = vm.eval_expr(&args[1]).await?.as_int();
    let position = vm.eval_expr(&args[2]).await?.as_int();
    vm.io.fseek(channel, pos, position)?;

    Ok(())
}

pub async fn fflush(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fread(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let val = vm.eval_expr(&args[1]).await?;
    let size = vm.eval_expr(&args[2]).await?.as_int() as usize;

    let result = vm.io.fread(channel, size)?;

    if val.get_type() == VariableType::String || val.get_type() == VariableType::BigStr {
        let mut vs = String::new();

        for c in result {
            if c == 0 {
                break;
            }
            vs.push(CP437_TO_UNICODE[c as usize]);
        }
        vm.set_variable(&args[1], VariableValue::new_string(vs)).await?;
        return Ok(());
    }

    match result.len() {
        1 => {
            vm.set_variable(&args[1], VariableValue::new_byte(result[0])).await?;
        }
        2 => {
            let i = u16::from_le_bytes([result[0], result[1]]);
            vm.set_variable(&args[1], VariableValue::new_word(i)).await?;
        }
        4 => {
            let i = i32::from_le_bytes([result[0], result[1], result[2], result[3]]);
            vm.set_variable(&args[1], VariableValue::new_int(i)).await?;
        }
        _ => {
            return Err(Box::new(VMError::FReadError(val.get_type(), result.len(), size)));
        }
    }

    Ok(())
}
pub async fn fwrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdefin(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdefout(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdputln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdputpad(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdread(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdwrite(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn adjbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub async fn alias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.use_alias = vm.eval_expr(&args[0]).await?.as_bool();
    Ok(())
}
pub async fn redim(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn append(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn flag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn download(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn wrusysdoor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub async fn getaltuser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let user_record = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let user = if let Ok(board) = vm.icy_board_state.board.lock() {
        if user_record >= board.users.len() {
            return Err(Box::new(VMError::UserRecordOutOfBounds(user_record)));
        }
        board.users[user_record].clone()
    } else {
        return Err(Box::new(VMError::InternalVMError));
    };
    vm.set_user_variables(&user);
    Ok(())
}

pub async fn adjdbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn adjtbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn ayjtfiles(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn lang(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
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
        indices.set_array_value(i, 0, 0, VariableValue::new_int(*target_index as i32));
    }
    Ok(())
}

pub async fn mousereg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn scrfile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn searchinit(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn searchfind(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn searchstop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn prfound(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn prfoundln(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
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
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn bitclear(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn brag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn frealtuser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn setlmr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
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
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn stackabort(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
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

pub async fn eval(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    // nothing, that just avaluates the parameters (for using function calls as statement)
    Ok(())
}
pub async fn account(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn recordusage(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn msgtofile(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn qwklimits(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn command(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn uselmrs(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn confinfo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn adjtubytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn grafmode(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn adduser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn killmsg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn chdir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn mkdir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn redir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdowraka(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdoaddaka(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdowrorg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdoaddorg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdoqmod(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdoqadd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn fdoqdel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub async fn sounddelay(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
