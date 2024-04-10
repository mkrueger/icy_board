use std::{fs, thread, time::Duration};

use icy_ppe::{
    executable::{PPEExpr, VariableType, VariableValue},
    tables::CP437_TO_UNICODE,
    Res,
};

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
pub fn invalid(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("Invalid statement");
}

pub fn end(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.is_running = false;
    Ok(())
}

pub fn cls(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.print(TerminalTarget::Both, "\x1B[2J")
}

pub fn clreol(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.print(TerminalTarget::Both, "\x1B[K")
}

pub fn more(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.more_promt()?;
    Ok(())
}

pub fn wait(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.press_enter()?;
    Ok(())
}

pub fn color(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let color = vm.eval_expr(&args[0])?.as_int() as u8;
    vm.icy_board_state.set_color(color.into())?;
    Ok(())
}

pub fn confflag(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn confunflag(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

/// # Errors
/// Errors if
pub fn dispfile(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let file_name = &vm.eval_expr(&args[0])?.as_string();

    vm.icy_board_state.display_file(file_name)?;
    Ok(())
}

pub fn input(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn fcreate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int() as usize;
    let file = vm.eval_expr(&args[1])?.as_string();
    let am = vm.eval_expr(&args[2])?.as_int();
    let sm = vm.eval_expr(&args[3])?.as_int();
    let file = vm.resolve_file(&file);
    vm.io.fcreate(channel, &file, am, sm);
    Ok(())
}

pub fn fopen(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int() as usize;
    let file = vm.eval_expr(&args[1])?.as_string();
    let am = vm.eval_expr(&args[2])?.as_int();
    let sm = vm.eval_expr(&args[3])?.as_int();
    let file = vm.resolve_file(&file);
    vm.io.fopen(channel, &file, am, sm)?;
    Ok(())
}

pub fn fappend(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int() as usize;
    let file = vm.eval_expr(&args[1])?.as_string();
    let am = vm.eval_expr(&args[2])?.as_int();
    let sm = vm.eval_expr(&args[3])?.as_int();
    let file = vm.resolve_file(&file);
    vm.io.fappend(channel, &file, am, sm);
    Ok(())
}

pub fn fclose(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int();
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

pub fn fget(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int() as usize;
    let value = VariableValue::new_string(vm.io.fget(channel)?);
    vm.set_variable(&args[1], value)?;
    Ok(())
}

pub fn fput(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int() as usize;

    for value in &args[1..] {
        let text = vm.eval_expr(value)?.as_string();
        vm.io.fput(channel, text)?;
    }
    Ok(())
}

pub fn fputln(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int() as usize;

    for value in &args[1..] {
        let text = vm.eval_expr(value)?.as_string();
        vm.io.fput(channel, text)?;
    }
    vm.io.fput(channel, "\n".to_string())?;
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub fn resetdisp(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    // TODO?: unused
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub fn startdisp(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    // TODO?: unused
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub fn fputpad(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

/// # Errors
/// Errors if the variable is not found.
pub fn hangup(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.hangup(crate::vm::HangupType::Hangup)?;
    vm.is_running = false;
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub fn getuser(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
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
pub fn putuser(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
    if let Some(mut user) = vm.icy_board_state.current_user.take() {
        vm.put_user_variables(&mut user);
        vm.icy_board_state.current_user = Some(user);
    }
    Ok(())
}

pub fn defcolor(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn delete(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let file = &vm.eval_expr(&args[0])?.as_string();
    let file = vm.resolve_file(&file);
    if let Err(err) = vm.io.delete(&file) {
        log::error!("Error deleting file: {}", err);
    }
    Ok(())
}

pub fn deluser(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn adjtime(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn log(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let msg = vm.eval_expr(&args[0])?.as_string();
    // let left = &vm.eval_expr(&args[0])?;
    log::info!("{}", msg);
    Ok(())
}

const TXT_STOPCHAR: char = '_';

pub fn inputstr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let prompt = vm.eval_expr(&args[0])?.as_string();
    // 1 Output Variable
    let color = vm.eval_expr(&args[2])?.as_int() as u8;
    let len = vm.eval_expr(&args[3])?.as_int();
    let valid = vm.eval_expr(&args[4])?.as_string();
    let flags = vm.eval_expr(&args[5])?.as_int();
    let output = vm
        .icy_board_state
        .input_string(color.into(), prompt, len, &valid, "", flags)?;
    vm.set_variable(&args[1], VariableValue::new_string(output))?;
    Ok(())
}

pub fn inputyn(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0])?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2])?.as_int() as u8;
    let len = 1;
    let valid = "YyNn";
    let output = vm
        .icy_board_state
        .input_string(color.into(), prompt, len, valid, "", 0)?;

    vm.set_variable(
        &args[1],
        VariableValue::new_string(output.to_ascii_uppercase()),
    )?;
    Ok(())
}

pub fn inputmoney(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0])?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2])?.as_int() as u8;
    let len = 13;
    let valid = "01234567890+-$.";
    let output = vm
        .icy_board_state
        .input_string(color.into(), prompt, len, valid, "", 0)?;
    // TODO: Money conversion.
    vm.set_variable(&args[1], VariableValue::new_string(output))?;
    Ok(())
}

pub fn inputint(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0])?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2])?.as_int() as u8;
    let len = 11;
    let valid = "01234567890+-";
    let output = vm
        .icy_board_state
        .input_string(color.into(), prompt, len, valid, "", 0)?;
    vm.set_variable(&args[1], VariableValue::new_int(output.parse::<i32>()?))?;
    Ok(())
}
pub fn inputcc(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0])?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2])?.as_int() as u8;
    let len = 16;
    let valid = "01234567890";
    let output = vm
        .icy_board_state
        .input_string(color.into(), prompt, len, valid, "", 0)?;
    vm.set_variable(&args[1], VariableValue::new_string(output))?;
    Ok(())
}
pub fn inputdate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0])?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2])?.as_int() as u8;
    let len = 8;
    let valid = "01234567890-/";
    let output = vm
        .icy_board_state
        .input_string(color.into(), prompt, len, valid, "", 0)?;
    // TODO: Date conversion
    vm.set_variable(&args[1], VariableValue::new_string(output))?;
    Ok(())
}

pub fn inputtime(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let mut prompt = vm.eval_expr(&args[0])?.as_string();
    if prompt.ends_with(TXT_STOPCHAR) {
        prompt.pop();
    }

    let color = vm.eval_expr(&args[2])?.as_int() as u8;
    let len = 8;
    let valid = "01234567890:";
    let output = vm
        .icy_board_state
        .input_string(color.into(), prompt, len, valid, "", 0)?;
    // TODO: Time conversion
    vm.set_variable(&args[1], VariableValue::new_string(output))?;
    Ok(())
}
pub fn promptstr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dtron(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    // IGNORE
    Ok(())
}

pub fn dtroff(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.hangup(crate::vm::HangupType::Hangup)?;
    Ok(())
}

pub fn cdchkon(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn cdchkoff(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn delay(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    // 1 tick is ~1/18.2s
    let ticks = vm.eval_expr(&args[0])?.as_int();
    if ticks > 0 {
        thread::sleep(Duration::from_millis((ticks as f32 * 1000.0 / 18.2) as u64));
    }
    Ok(())
}

pub fn sendmodem(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn inc(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let new_value = vm.eval_expr(&args[0])? + VariableValue::new_int(1);
    vm.set_variable(&args[0], new_value)?;
    Ok(())
}

pub fn dec(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let new_value = vm.eval_expr(&args[0])?.clone() - VariableValue::new_int(1);
    vm.set_variable(&args[0], new_value)?;
    Ok(())
}

pub fn newline(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.write_raw(TerminalTarget::Both, &['\n'])
}

pub fn newlines(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let count = vm.eval_expr(&args[0])?.as_int();
    for _ in 0..count {
        newline(vm, args)?;
    }
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub fn tokenize(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let str = vm.eval_expr(&args[0])?.to_string();
    let split = str
        .split(&[' ', ';'][..])
        .map(std::string::ToString::to_string);
    vm.icy_board_state.session.tokens.extend(split);
    Ok(())
}

pub fn gettoken(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn shell(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn disptext(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let rec = vm.eval_expr(&args[0])?.as_int();
    let flags = vm.eval_expr(&args[1])?.as_int();

    vm.icy_board_state
        .display_text(IceText::from(rec as usize), flags)
}

pub fn stop(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.is_running = false;
    Ok(())
}

pub fn inputtext(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn beep(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.bell()
}

pub fn push(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    for p in args {
        let value = vm.eval_expr(p)?;
        vm.push_pop_stack.push(value);
    }
    Ok(())
}

pub fn pop(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    for arg in args {
        if let Some(val) = vm.push_pop_stack.pop() {
            vm.set_variable(arg, val)?;
        } else {
            return Err(Box::new(VMError::PushPopStackEmpty));
        }
    }
    Ok(())
}

pub fn call(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn join(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn quest(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn blt(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dir(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn kbdstuff(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0])?.as_string();
    vm.icy_board_state.put_keyboard_buffer(&value)?;
    Ok(())
}
pub fn kbdstring(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0])?.as_string();
    vm.icy_board_state.print(TerminalTarget::Both, &value)?;
    vm.icy_board_state.put_keyboard_buffer(&value)?;
    Ok(())
}
pub fn kbdfile(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let file_name = vm.eval_expr(&args[0])?.as_string();
    let fil_name = vm.resolve_file(&file_name);
    let contents = fs::read_to_string(file_name)?;
    vm.icy_board_state.put_keyboard_buffer(&contents)?;

    Ok(())
}

pub fn bye(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.hangup(crate::vm::HangupType::Bye)?;
    vm.is_running = false;
    Ok(())
}

pub fn goodbye(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.hangup(crate::vm::HangupType::Goodbye)?;
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
pub fn broadcast(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let lonode = vm.eval_expr(&args[0])?.as_int();
    let hinode = vm.eval_expr(&args[1])?.as_int();
    let message = vm.eval_expr(&args[2])?.as_string();
    // TODO: Broadcast
    println!("Broadcasting message from {lonode} to {hinode}: {message}");
    Ok(())
}

pub fn waitfor(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn kbdchkon(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.reset_keyboard_check_timer();
    Ok(())
}

pub fn kbdchkoff(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.keyboard_timer_check = false;
    Ok(())
}

pub fn optext(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.op_text = vm.eval_expr(&args[0])?.as_string();
    Ok(())
}
pub fn dispstr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0])?.as_string();
    vm.icy_board_state.print(TerminalTarget::Both, &value)
}

pub fn rdunet(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let value = vm.eval_expr(&args[0])?.as_int();

    if let Some(node) = vm.icy_board_state.nodes.get(value as usize) {
        vm.pcb_node = Some(node.clone());
    }
    Ok(())
}

pub fn wrunet(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let node = vm.eval_expr(&args[0])?.as_int();
    let stat = vm.eval_expr(&args[1])?.as_string();
    let name = vm.eval_expr(&args[2])?.as_string();
    let city = vm.eval_expr(&args[3])?.as_string();
    let operation = vm.eval_expr(&args[4])?.as_string();
    let broadcast = vm.eval_expr(&args[5])?.as_string();

    // Todo: Broadcast

    if !stat.is_empty() {
        vm.icy_board_state.nodes[node as usize].status = stat.as_bytes()[0] as char;
    }
    vm.icy_board_state.nodes[node as usize].name = name;
    vm.icy_board_state.nodes[node as usize].city = city;
    vm.icy_board_state.nodes[node as usize].operation = operation;

    Ok(())
}

pub fn dointr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("System interrupts are not (yet) supported")
}
pub fn varseg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("System interrupts are not (yet) supported")
}
pub fn varoff(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("System interrupts are not (yet) supported")
}
pub fn pokeb(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("DOS memory access is not (yet) supported")
}
pub fn pokew(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("DOS memory access is not (yet) supported")
}
pub fn varaddr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("System interrupts are not (yet) supported")
}

pub fn ansipos(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let x = vm.eval_expr(&args[0])?.as_int();
    let y = vm.eval_expr(&args[1])?.as_int();
    vm.icy_board_state.gotoxy(TerminalTarget::Both, x, y)
}

pub fn backup(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let numcols = vm.eval_expr(&args[0])?.as_int();
    if vm.icy_board_state.use_ansi() {
        vm.icy_board_state
            .print(TerminalTarget::Both, &format!("\x1B[{numcols}D"))
    } else {
        vm.icy_board_state
            .print(TerminalTarget::Both, &"\x08".repeat(numcols as usize))
    }
}

pub fn forward(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let numcols = vm.eval_expr(&args[0])?.as_int();
    if vm.icy_board_state.use_ansi() {
        vm.icy_board_state
            .print(TerminalTarget::Both, &format!("\x1B[{numcols}C"))?;
    } else {
        vm.icy_board_state
            .print(TerminalTarget::Both, &" ".repeat(numcols as usize))?;
    }
    Ok(())
}
pub fn freshline(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.print(TerminalTarget::Both, "\r\n")?;
    Ok(())
}
pub fn wrusys(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("USER.SYS is not supported")
}
pub fn rdusys(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("USER.SYS is not supported")
}
pub fn newpwd(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn opencap(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn closecap(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn message(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn savescrn(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn restscrn(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn sound(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn chat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn sprint(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value)?.as_string();
        vm.icy_board_state.print(TerminalTarget::Sysop, txt)?;
    }
    Ok(())
}

pub fn sprintln(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value)?.as_string();
        vm.icy_board_state.print(TerminalTarget::Sysop, txt)?;
    }
    vm.icy_board_state.print(TerminalTarget::Sysop, "\n")?;
    Ok(())
}

pub fn print(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value)?.as_string();
        vm.icy_board_state.print(TerminalTarget::Both, txt)?;
    }
    Ok(())
}

pub fn println(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value)?.as_string();
        vm.icy_board_state.print(TerminalTarget::Both, txt)?;
    }
    vm.icy_board_state.print(TerminalTarget::User, "\n")?;
    Ok(())
}

pub fn mprint(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value)?.as_string();
        vm.icy_board_state.print(TerminalTarget::User, txt)?;
    }
    Ok(())
}

pub fn mprintln(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    for value in args {
        let txt = &vm.eval_expr(value)?.as_string();
        vm.icy_board_state.print(TerminalTarget::User, txt)?;
    }
    vm.icy_board_state.print(TerminalTarget::User, "\n")?;
    Ok(())
}

pub fn rename(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let old = &vm.eval_expr(&args[0])?.as_string();
    let new = &vm.eval_expr(&args[1])?.as_string();
    let old = vm.resolve_file(&old);
    let new = vm.resolve_file(&new);

    if let Err(err) = vm.io.rename(&old, &new) {
        log::error!("Error renaming file: {}", err);
    }
    Ok(())
}
pub fn frewind(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn pokedw(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dbglevel(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.debug_level = vm.eval_expr(&args[0])?.as_int();
    Ok(())
}
pub fn showon(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.disp_options.display_text = true;
    Ok(())
}
pub fn showoff(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.disp_options.display_text = false;
    Ok(())
}

pub fn pageon(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn pageoff(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn fseek(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int() as usize;
    let pos = vm.eval_expr(&args[1])?.as_int();
    let position = vm.eval_expr(&args[2])?.as_int();
    vm.io.fseek(channel, pos, position)?;

    Ok(())
}

pub fn fflush(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fread(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let channel = vm.eval_expr(&args[0])?.as_int() as usize;
    let val = vm.eval_expr(&args[1])?;
    let size = vm.eval_expr(&args[2])?.as_int() as usize;

    let result = vm.io.fread(channel, size)?;

    if val.get_type() == VariableType::String || val.get_type() == VariableType::BigStr {
        let mut vs = String::new();

        for c in result {
            if c == 0 {
                break;
            }
            vs.push(CP437_TO_UNICODE[c as usize]);
        }
        vm.set_variable(&args[1], VariableValue::new_string(vs))?;
        return Ok(());
    }

    match result.len() {
        1 => {
            vm.set_variable(&args[1], VariableValue::new_byte(result[0]))?;
        }
        2 => {
            let i = u16::from_le_bytes([result[0], result[1]]);
            vm.set_variable(&args[1], VariableValue::new_word(i))?;
        }
        4 => {
            let i = i32::from_le_bytes([result[0], result[1], result[2], result[3]]);
            vm.set_variable(&args[1], VariableValue::new_int(i))?;
        }
        _ => {
            return Err(Box::new(VMError::FReadError(
                val.get_type(),
                result.len(),
                size,
            )));
        }
    }

    Ok(())
}
pub fn fwrite(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdefin(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdefout(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdget(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdput(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdputln(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdputpad(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdread(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdwrite(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn adjbytes(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn alias(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    vm.icy_board_state.session.use_alias = vm.eval_expr(&args[0])?.as_bool();
    Ok(())
}
pub fn redim(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn append(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn copy(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let old = &vm.eval_expr(&args[0])?.as_string();
    let new = &vm.eval_expr(&args[1])?.as_string();
    if let Err(err) = vm.io.copy(old, new) {
        log::error!("Error renaming file: {}", err);
    }
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub fn kbdflush(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    // TODO?
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub fn mdmflush(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    // TODO?
    Ok(())
}

/// # Errors
/// Errors if the variable is not found.
pub fn keyflush(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    // TODO?
    Ok(())
}
pub fn lastin(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn flag(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn download(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn wrusysdoor(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn getaltuser(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let user_record = vm.eval_expr(&args[0])?.as_int() as usize;
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

pub fn adjdbytes(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn adjtbytes(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn ayjtfiles(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn lang(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn sort(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
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
            return Err(Box::new(IcyError::SortDestinationArrayIntRequired(
                indices.vtype,
            )));
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

pub fn mousereg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn scrfile(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn searchinit(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn searchfind(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn searchstop(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn prfound(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn prfoundln(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn tpaget(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn tpaput(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn tpacgea(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn tpacput(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn tparead(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn tpawrite(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn tpacread(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn tpacwrite(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn bitset(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn bitclear(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn brag(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn frealtuser(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn setlmr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn setenv(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    let env = vm.eval_expr(&args[0])?.as_string();
    let v: Vec<&str> = env.split('=').collect();
    if v.len() == 2 {
        vm.icy_board_state.set_env(v[0], v[1]);
    } else {
        vm.icy_board_state.remove_env(&env);
    }
    Ok(())
}

pub fn fcloseall(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn stackabort(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dcreate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dopen(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dclose(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dsetalias(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dpack(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dcloseall(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dlock(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dlockr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dlockg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dunlock(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dncreate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dnopen(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dnclose(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dncloseall(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dnew(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dadd(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dappend(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dtop(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dgo(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dbottom(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dskip(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dblank(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn ddelete(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn drecall(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dtag(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dseek(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dfblank(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dget(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dput(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn dfcopy(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}

pub fn eval(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    // nothing, that just avaluates the parameters (for using function calls as statement)
    Ok(())
}
pub fn account(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn recordusage(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn msgtofile(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn qwklimits(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn command(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn uselmrs(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn confinfo(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn adjtubytes(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn grafmode(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn adduser(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn killmsg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn chdir(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn mkdir(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn redir(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdowraka(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdoaddaka(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdowrorg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdoaddorg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdoqmod(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdoqadd(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn fdoqdel(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
pub fn sounddelay(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<()> {
    log::error!("not implemented statement!");
    panic!("TODO")
}
