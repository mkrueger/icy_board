#![allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]

use std::fs;
use std::path::PathBuf;

use crate::icy_board::state::functions::{
    MASK_ALNUM, MASK_ALPHA, MASK_ASCII, MASK_FILE, MASK_NUM, MASK_PATH, MASK_PWD,
};
use crate::vm::VirtualMachine;
use icy_engine::update_crc32;
use icy_ppe::ast::constant::STACK_LIMIT;
use icy_ppe::datetime::{IcbDate, IcbTime};
use icy_ppe::executable::{PPEExpr, VariableData, VariableType, VariableValue};
use icy_ppe::tables::CP437_TO_UNICODE;
use icy_ppe::Res;
use radix_fmt::radix;
use rand::Rng; // 0.8.5
use substring::Substring;

/// Should never be called. But some op codes are invalid as function call (like plus or function call)
/// and are handled by it's own `PPEExpressions` and will point to this function.
///
/// # Panics
///
/// Always
pub fn invalid(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("Invalid function call")
}

/// Returns the length of a string
/// # Arguments
///  * `str` - A string value
/// # Returns
///  `VariableValue::Integer` - the length of `str`
/// # Remarks
/// 0 means empty string
/// According to specs 256 is the maximum returned
pub fn len(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0])?.as_string();
    Ok(VariableValue::new_int(str.len() as i32))
}

/// Returns the lowercase equivalent of a string
/// # Arguments
///  * `str` - A string value
/// # Returns
///  `VariableValue::String` - lowercase equivalent of `str`
pub fn lower(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0])?.as_string();
    Ok(VariableValue::new_string(str.to_lowercase()))
}

/// Returns the uppercase equivalent of a string
/// # Arguments
///  * `str` - A string value
/// # Returns
///  `VariableValue::String` - uppercase equivalent of `str`
pub fn upper(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0])?.as_string();
    Ok(VariableValue::new_string(str.to_uppercase()))
}

/// Returns a substring
/// # Arguments
///  * `str` - A string value
///  * `pos` - An integer value with a position from str to begin the substring 1 == first char
///  * `chars` - An integer value with the number of chars to take from `str`
/// # Returns
///  the substring of `str`, "" if chars <= 0, Will add padding up to the full length specified
pub fn mid(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0])?.as_string();
    let mut pos = vm.eval_expr(&args[1])?.as_int() - 1; // 1 based
    let mut chars = vm.eval_expr(&args[2])?.as_int();
    if chars <= 0 {
        return Ok(VariableValue::new_string(String::new()));
    }

    let mut res = String::new();
    while pos < 0 {
        res.push(' ');
        pos += 1;
        chars -= 1;
    }

    if chars > 0 {
        res.push_str(str.substring(pos as usize, (pos + chars) as usize));
    }
    Ok(VariableValue::new_string(res))
}

pub fn left(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut chars = vm.eval_expr(&args[1])?.as_int();
    if chars <= 0 {
        return Ok(VariableValue::new_string(String::new()));
    }
    let str = vm.eval_expr(&args[0])?.as_string();
    let mut res = String::new();
    if chars > 0 {
        if chars < str.len() as i32 {
            res.push_str(str.substring(0, chars as usize));
        } else {
            res.push_str(str.as_str());
            chars -= str.len() as i32;
            while chars > 0 {
                res.push(' ');
                chars -= 1;
            }
        }
    }
    Ok(VariableValue::new_string(res))
}

pub fn right(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let chars = vm.eval_expr(&args[1])?.as_int();
    if chars <= 0 {
        return Ok(VariableValue::new_string(String::new()));
    }
    let mut chars = chars as usize;

    let mut res = String::new();
    let str: String = vm.eval_expr(&args[0])?.as_string();
    if chars > 0 {
        while chars > str.len() {
            res.push(' ');
            chars -= 1;
        }
        res.push_str(str.substring(str.len() - chars, str.len()));
    }
    Ok(VariableValue::new_string(res))
}

pub fn space(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut chars = vm.eval_expr(&args[0])?.as_int();
    if chars <= 0 {
        return Ok(VariableValue::new_string(String::new()));
    }
    let mut res = String::new();
    while chars > 0 {
        res.push(' ');
        chars -= 1;
    }
    Ok(VariableValue::new_string(res))
}

pub fn ferr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let channel = vm.eval_expr(&args[0])?.as_int();
    Ok(VariableValue::new_bool(vm.io.ferr(channel as usize)))
}

pub fn chr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let c = vm.eval_expr(&args[0])?.as_int();
    if c <= 0 {
        return Ok(VariableValue::new_string(String::new()));
    }
    // undocumented: returns a space for c > 255
    if c > 255 {
        return Ok(VariableValue::new_string(" ".to_string()));
    }
    let mut res = String::new();
    unsafe {
        res.push(char::from_u32_unchecked(c as u32));
    }
    Ok(VariableValue::new_string(res))
}

pub fn asc(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let c = vm.eval_expr(&args[0])?.as_string();
    if c.is_empty() {
        return Ok(VariableValue::new_int(0));
    }
    Ok(VariableValue::new_int(c.as_bytes()[0] as i32))
}

/// Returns the position of a substring
/// # Arguments
///  * `str` - A string value
///  * `sub` - A string expression to search for
/// # Returns
///  A 1 based integer of the position of sub or 0 if sub is not found.
pub fn instr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0])?.as_string();
    let sub = vm.eval_expr(&args[1])?.as_string();
    if sub.is_empty() {
        return Ok(VariableValue::new_int(0));
    }

    match str.find(&sub) {
        Some(x) => Ok(VariableValue::new_int(1 + x as i32)),
        _ => Ok(VariableValue::new_int(0)),
    }
}

/// Returns a flag indicating if the user has aborted the display of information.
pub fn abort(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(
        vm.icy_board_state.session.disp_options.abort_printout,
    ))
}

/// Trim specified characters from the beginning of a string
/// # Arguments
///  * `str` - A string value
///  * `ch` - A string with the character to strip from the beginning of `str`
/// # Returns
///  The trimmed `str`
pub fn ltrim(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut ch = vm.eval_expr(&args[1])?.as_string();
    if ch.is_empty() {
        return Ok(vm.eval_expr(&args[0])?.clone());
    }
    let str = vm.eval_expr(&args[0])?.as_string();
    let pat = ch.remove(0);
    Ok(VariableValue::new_string(
        str.trim_start_matches(pat).to_string(),
    ))
}

/// Replaces all occurences of a given character to another character in a string.
/// # Arguments
///  * `str` - A string value
///  * `old` - A string with the old character
///  * `new` - A string with the new character
pub fn replace(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0])?.as_string();
    let old = vm.eval_expr(&args[1])?.as_string();
    let new = vm.eval_expr(&args[2])?.as_string();

    let mut res = String::new();
    let Some(old) = old.chars().next() else {
        return Ok(VariableValue::new_string(str));
    };

    if let Some(new) = new.chars().next() {
        for c in str.chars() {
            if c == old {
                res.push(new);
            } else {
                res.push(c);
            }
        }
    }
    Ok(VariableValue::new_string(res))
}

/// Remove all occurences of a given character in a string
/// # Arguments
///  * `str` - A string value
///  * `ch` - A string with the character to remove
pub fn strip(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0])?.as_string();
    let ch: String = vm.eval_expr(&args[1])?.as_string();
    let mut res = String::new();
    if let Some(remove_char) = ch.chars().next() {
        for c in str.chars() {
            if c != remove_char {
                res.push(c);
            }
        }
    }
    Ok(VariableValue::new_string(res))
}

/// Remove @X codes from a string
/// # Arguments
///  * `str` - A string value
/// # Returns
/// A string without any @X codes
pub fn stripatx(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0])?.as_string();
    let mut res = String::new();
    let mut state = 0;
    let mut ch1 = 'A';
    for c in str.chars() {
        match state {
            0 => {
                if c == '@' {
                    state = 1;
                } else {
                    res.push(c);
                }
            }
            1 => {
                if c == 'X' {
                    state = 2;
                } else {
                    res.push('@');
                    res.push(c);
                    state = 0;
                }
            }
            2 => {
                if c.is_ascii_hexdigit() {
                    state = 3;
                } else {
                    res.push('@');
                    res.push(c);
                    ch1 = c;
                    state = 0;
                }
            }
            3 => {
                state = 0;
                if !c.is_ascii_hexdigit() {
                    res.push('@');
                    res.push(ch1);
                    res.push(c);
                }
            }
            _ => {
                state = 0;
            }
        }
    }
    Ok(VariableValue::new_string(res))
}

pub fn replacestr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub fn stripstr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

/// Trim specified characters from the end of a string
/// # Arguments
///  * `str` - A string value
///  * `ch` - A string with the character to strip from the end of `str`
/// # Returns
///  The trimmed `str`
pub fn rtrim(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut ch = vm.eval_expr(&args[1])?.as_string();
    if ch.is_empty() {
        return Ok(vm.eval_expr(&args[0])?.clone());
    }
    let str = vm.eval_expr(&args[0])?.as_string();

    let pat = ch.remove(0);
    Ok(VariableValue::new_string(
        str.trim_end_matches(pat).to_string(),
    ))
}

/// Trim specified characters from the beginning and end of a string
/// # Arguments
///  * `str` - A string value
///  * `ch` - A string with the character to strip from the beginning and end of `str`
/// # Returns
///  The trimmed `str`
pub fn trim(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut ch = vm.eval_expr(&args[1])?.as_string();
    if ch.is_empty() {
        return Ok(vm.eval_expr(&args[0])?.clone());
    }
    let str = vm.eval_expr(&args[0])?.as_string();

    let pat = ch.remove(0);
    Ok(VariableValue::new_string(str.trim_matches(pat).to_string()))
}

pub fn random(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let upper = vm.eval_expr(&args[0])?.as_int();
    if upper <= 0 {
        return Ok(VariableValue::new_int(0));
    }

    let mut rng = rand::thread_rng();
    Ok(VariableValue::new_int(rng.gen_range(0..upper)))
}

pub fn date(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_date(IcbDate::today().to_pcboard_date()))
}

pub fn time(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_time(IcbTime::now().to_pcboard_time()))
}

pub fn u_name(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.current_user {
        Ok(VariableValue::new_string(user.get_name().clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}

pub fn u_ldate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    // vm.pcb_data.users[vm.cur_user].last_date_on
    // TODO
    Ok(VariableValue::new(
        VariableType::Date,
        VariableData::default(),
    ))
}

pub fn u_ltime(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    // TODO
    Ok(VariableValue::new(
        VariableType::Time,
        VariableData::default(),
    ))
}

pub fn u_ldir(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_lmr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_logons(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_ful(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_fdl(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_bdlday(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_timeon(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_bdl(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_bul(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_msgrd(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub fn u_msgwr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}

pub fn year(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn month(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn day(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dow(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn hour(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn min(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn sec(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn timeap(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn ver(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(1540))
}
pub fn nochar(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.icy_board_state.no_char.to_string(),
    ))
}
pub fn yeschar(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.icy_board_state.yes_char.to_string(),
    ))
}

pub fn inkey(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some((echo, ch)) = vm.icy_board_state.get_char()? {
        if ch as u8 == 127 {
            return Ok(VariableValue::new_string("DEL".to_string()));
        }
        if ch == '\x1B' {
            if let Some((echo, ch)) = vm.icy_board_state.get_char()? {
                if ch == '[' {
                    if let Some((echo, ch)) = vm.icy_board_state.get_char()? {
                        match ch {
                            'A' => return Ok(VariableValue::new_string("UP".to_string())),
                            'B' => return Ok(VariableValue::new_string("DOWN".to_string())),
                            'C' => return Ok(VariableValue::new_string("RIGHT".to_string())),
                            'D' => return Ok(VariableValue::new_string("LEFT".to_string())),

                            'H' => return Ok(VariableValue::new_string("HOME".to_string())),
                            'K' => return Ok(VariableValue::new_string("END".to_string())),

                            'V' => return Ok(VariableValue::new_string("PGUP".to_string())),
                            'U' => return Ok(VariableValue::new_string("PGDN".to_string())),

                            '@' => return Ok(VariableValue::new_string("INS".to_string())),

                            _ => return Ok(VariableValue::new_string(ch.to_string())),
                        }
                    }
                }
            }
            return Ok(VariableValue::new_string("\x1B".to_string()));
        }
        Ok(VariableValue::new_string(ch.to_string()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}

pub fn tostring(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.eval_expr(&args[0])?.as_string(),
    ))
}
pub fn mask_pwd(_vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_PWD.to_string()))
}
pub fn mask_alpha(_vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_ALPHA.to_string()))
}
pub fn mask_num(_vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_NUM.to_string()))
}
pub fn mask_alnum(_vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_ALNUM.to_string()))
}
pub fn mask_file(_vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_FILE.to_string()))
}
pub fn mask_path(_vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_PATH.to_string()))
}
pub fn mask_ascii(_vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_ASCII.to_string()))
}

pub fn curconf(vm: &mut VirtualMachine, _args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(
        vm.icy_board_state.session.current_conference_number,
    ))
}

pub fn pcbdat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(vm.icy_board_state.get_pcbdat()?))
}

pub fn ppepath(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let Some(dir) = vm.file_name.parent() else {
        return Ok(VariableValue::new_string(String::new()));
    };
    let mut res = dir.to_string_lossy().to_string();
    res.push('/');
    Ok(VariableValue::new_string(res))
}

pub fn valdate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn valtime(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn pcbnode(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.node_num))
}

pub fn readline(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let file_name = vm.eval_expr(&args[0])?.as_string();
    let line = vm.eval_expr(&args[1])?.as_int();
    let file_name = vm.resolve_file(&file_name);

    if let Ok(file) = fs::read(&file_name) {
        let file = file
            .iter()
            .map(|x| CP437_TO_UNICODE[*x as usize])
            .collect::<String>();

        let line_text = file.lines().nth(line as usize - 1).unwrap_or_default();
        Ok(VariableValue::new_string(line_text.to_string()))
    } else {
        log::warn!("PPE readline: file not found: {}", &file_name);
        Ok(VariableValue::new_string(String::new()))
    }
}

pub fn sysopsec(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(
        vm.icy_board_state
            .board
            .lock()
            .unwrap()
            .config
            .sysop_security_level
            .sysop as i32,
    ))
}
pub fn onlocal(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    // TODO: OnLocal should return true if the user is local, false otherwise
    Ok(VariableValue::new_bool(true))
}

pub fn un_stat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_int(node.status as i32))
    } else {
        Ok(VariableValue::new_int(0))
    }
}

pub fn un_name(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_string(node.name.clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub fn un_city(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_string(node.city.clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub fn un_oper(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_string(node.operation.clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub fn cursec(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub fn gettoken(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(tok) = vm.icy_board_state.session.tokens.pop_front() {
        Ok(VariableValue::new_string(tok))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub fn minleft(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn minon(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn getenv(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = &vm.eval_expr(&args[0])?.as_string();
    if let Some(var) = vm.icy_board_state.get_env(var) {
        Ok(VariableValue::new_string(var.to_string()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub fn callid(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regal(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regah(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regbl(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regbh(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regcl(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regch(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regdl(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regdh(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regax(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regbx(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regcx(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regdx(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regsi(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regdi(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regf(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regcf(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn regds(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn reges(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn b2w(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn peekb(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn peekw(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn mkaddr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn exist(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let file_name = vm.eval_expr(&args[0])?.as_string();
    Ok(VariableValue::new_bool(vm.io.file_exists(&file_name)))
}

/// Convert an integer to a string in a specified number base.
/// # Arguments
///  * `int` - Any integer to convert to string format.
///  * `base` - The base to use for the conversion. 2 <= base <= 36
/// # Returns
///  A string representation of `int` in the specified base.
pub fn i2s(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let int = vm.eval_expr(&args[0])?.as_int();
    let base = vm.eval_expr(&args[1])?.as_int();
    let s = radix(int, base as u8).to_string();
    Ok(VariableValue::new_string(s))
}

/// Convert a string in a specified number base to an integer.
/// # Arguments
///  * `src` - A string value to convert to an integer.
///  * `base` - The base to use for the conversion. 2 <= base <= 36
/// # Returns
///  An integer representation of `s` in the specified base.
pub fn s2i(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let src = vm.eval_expr(&args[0])?.as_string();
    let base = vm.eval_expr(&args[1])?.as_int();
    let i = i32::from_str_radix(&src, base as u32)?;
    Ok(VariableValue::new_int(i))
}
pub fn carrier(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.get_bps()))
}
pub fn tokenstr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn cdon(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn langext(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn ansion(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(
        !vm.icy_board_state.session.disp_options.disable_color,
    ))
}
pub fn valcc(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn fmtcc(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn cctype(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub fn getx(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(
        vm.icy_board_state.get_caret_position().0 + 1,
    ))
}

pub fn gety(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let y = vm.icy_board_state.get_caret_position().1;
    Ok(VariableValue::new_int(y + 1))
}

pub fn band(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let left = vm.eval_expr(&args[0])?.as_int();
    let right = vm.eval_expr(&args[1])?.as_int();
    Ok(VariableValue::new_int(left & right))
}

pub fn bor(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let left = vm.eval_expr(&args[0])?.as_int();
    let right = vm.eval_expr(&args[1])?.as_int();
    Ok(VariableValue::new_int(left | right))
}

pub fn bxor(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let left = vm.eval_expr(&args[0])?.as_int();
    let right = vm.eval_expr(&args[1])?.as_int();
    Ok(VariableValue::new_int(left ^ right))
}

pub fn bnot(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let val = vm.eval_expr(&args[0])?.as_int();
    Ok(VariableValue::new_int(!val))
}

pub fn u_pwdhist(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn u_pwdlc(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn u_pwdtc(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn u_stat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn defcolor(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn abs(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let val = vm.eval_expr(&args[0])?.as_int();
    Ok(VariableValue::new_int(val.abs()))
}

pub fn grafmode(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub fn psa(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

#[allow(clippy::unnecessary_wraps)]
pub fn fileinf(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let file = vm.eval_expr(&args[0])?.as_string();
    let item = vm.eval_expr(&args[1])?.as_int();

    let file = vm.resolve_file(&file);
    let path = PathBuf::from(&file);
    match item {
        1 => Ok(VariableValue::new_bool(vm.io.file_exists(&file))),
        2 => Ok(VariableValue::new(
            VariableType::Date,
            VariableData::default(),
        )), // TODO: File date
        3 => Ok(VariableValue::new(
            VariableType::Time,
            VariableData::default(),
        )), // TODO: File time
        4 => Ok(VariableValue::new_int(vm.io.get_file_size(&file) as i32)),
        5 => Ok(VariableValue::new_int(0)), // TODO: File attributes
        6 => Ok(VariableValue::new_string("C:".to_string())), // Drive
        7 => {
            if let Some(dir) = path.parent() {
                Ok(VariableValue::new_string(dir.to_string_lossy().to_string()))
            } else {
                Ok(VariableValue::new_string(String::new()))
            }
        }
        8 => {
            if let Some(dir) = path.file_name() {
                Ok(VariableValue::new_string(dir.to_string_lossy().to_string()))
            } else {
                Ok(VariableValue::new_string(String::new()))
            }
        }
        9 => {
            if let Some(dir) = path.file_stem() {
                Ok(VariableValue::new_string(dir.to_string_lossy().to_string()))
            } else {
                Ok(VariableValue::new_string(String::new()))
            }
        }
        _ => {
            log::error!("Unknown fileinf item: {}", item);
            Ok(VariableValue::new_int(0))
        }
    }
}

pub fn ppename(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let p = vm.file_name.with_extension("");
    let Some(dir) = p.file_name() else {
        return Ok(VariableValue::new_string(String::new()));
    };
    let res = dir.to_string_lossy().to_string();
    Ok(VariableValue::new_string(res))
}

pub fn mkdate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn curcolor(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn kinkey(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    inkey(vm, args)
}
pub fn minkey(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    inkey(vm, args)
}
pub fn maxnode(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.nodes.len() as i32))
}
pub fn slpath(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.icy_board_state
            .board
            .lock()
            .unwrap()
            .config
            .paths
            .security_file_path
            .to_string_lossy()
            .to_string(),
    ))
}
pub fn helppath(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.icy_board_state
            .board
            .lock()
            .unwrap()
            .config
            .paths
            .help_path
            .to_string_lossy()
            .to_string(),
    ))
}
pub fn temppath(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.icy_board_state
            .board
            .lock()
            .unwrap()
            .config
            .paths
            .tmp_path
            .to_string_lossy()
            .to_string(),
    ))
}
pub fn modem(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn loggedon(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn callnum(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn mgetbyte(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn tokcount(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(
        vm.icy_board_state.session.tokens.len() as i32,
    ))
}

pub fn u_recnum(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let user_name = vm.eval_expr(&args[0])?.as_string().to_uppercase();
    for (i, user) in vm
        .icy_board_state
        .board
        .lock()
        .unwrap()
        .users
        .iter()
        .enumerate()
    {
        if user.get_name().to_uppercase() == user_name {
            return Ok(VariableValue::new_int(i as i32));
        }
    }
    Ok(VariableValue::new_int(-1))
}

pub fn u_inconf(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn peekdw(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dbglevel(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.debug_level))
}
pub fn scrtext(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn showstat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn pagestat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn tobigstr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::BigStr))
}
pub fn toboolean(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Boolean))
}
pub fn tobyte(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Byte))
}
pub fn todate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Date))
}
pub fn todreal(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Double))
}
pub fn toedate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::EDate))
}
pub fn tointeger(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Integer))
}
pub fn tomoney(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Money))
}
pub fn toreal(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Float))
}
pub fn tosbyte(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::SByte))
}
pub fn tosword(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::SWord))
}
pub fn totime(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Time))
}
pub fn tounsigned(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Unsigned))
}
pub fn toword(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm
        .eval_expr(&args[0])?
        .clone()
        .convert_to(VariableType::Word))
}
pub fn mixed(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn alias(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(
        vm.icy_board_state.session.use_alias,
    ))
}
pub fn confreg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let conf_num = vm.eval_expr(&args[0])?.as_int() as usize;

    // TODO: What is that ?
    // vm.icy_board_state.board.lock().unwrap().conferences[conf_num].
    Ok(VariableValue::new_bool(true))
}
pub fn confexp(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn confsel(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn confsys(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn confmw(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn lprinted(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn isnonstop(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn errcorrect(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn confalias(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn useralias(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn curuser(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn chatstat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn defans(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn lastans(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn meganum(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn evttimeadj(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn isbitset(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0])?.as_int();
    let bit = vm.eval_expr(&args[1])?.as_int();

    Ok(VariableValue::new_bool(var & (1 << bit) != 0))
}
pub fn fmtreal(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn flagcnt(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn kbdbufsize(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn pplbufsize(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn kbdfilusued(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn lomsgnum(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn himsgnum(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub fn drivespace(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn outbytes(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(0))
}
pub fn hiconfnum(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(
        vm.icy_board_state.board.lock().unwrap().conferences.len() as i32 - 1,
    ))
}

pub fn inbytes(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.inbytes()))
}

pub fn crc32(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    let use_file = vm.eval_expr(&args[0])?.as_bool();
    let param = vm.eval_expr(&args[1])?.as_string();

    if use_file {
        let file = vm.resolve_file(&param);
        let buffer = fs::read(file)?;
        let crc = calc_crc32(&buffer);
        Ok(VariableValue::new_unsigned(crc as u64))
    } else {
        let crc = calc_crc32(&param.bytes().collect::<Vec<u8>>());
        Ok(VariableValue::new_unsigned(crc as u64))
    }
}

fn calc_crc32(buffer: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for c in buffer {
        crc = update_crc32(crc, *c);
    }
    !crc
}

pub fn pcbmac(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn actmsgnum(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

/// Usage: `STACKLEFT()`
//  Val: Returns the number of bytes left on the *system* stack.
pub fn stackleft(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(
        STACK_LIMIT - vm.return_addresses.len() as i32,
    ))
}

/// `STACKERR()`
/// Returns a boolean value which indicates a stack error has occured
pub fn stackerr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(
        STACK_LIMIT > vm.return_addresses.len() as i32,
    ))
}

pub fn dgetalias(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dbof(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dchanged(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn ddecimals(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn ddeleted(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn deof(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn derr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dfields(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dlength(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dname(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dreccount(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn drecno(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dtype(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn fnext(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dnext(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn toddate(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dcloseall(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dopen(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dclose(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dsetalias(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dpack(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dlockf(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dlock(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dlockr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dunlock(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dnopen(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dnclose(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dncloseall(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dnew(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dadd(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dappend(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dtop(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dgo(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dbottom(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dskip(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dblank(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn ddelete(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn drecall(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dtag(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dseek(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dfblank(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dget(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dput(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dfcopy(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dselect(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn dchkstat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub fn pcbaccount(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn pcbaccstat(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn derrmsg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn account(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn scanmsghdr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn checkrip(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn ripver(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn qwklimits(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn findfirst(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn findnext(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn uselmrs(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn confinfo(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn tinkey(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    inkey(vm, args)
}
pub fn cwd(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn instrr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn fdordaka(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn fdordorg(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn fdordarea(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn fdoqrd(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn getdrive(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn setdrive(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn bs2i(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn bd2i(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn i2bs(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn i2bd(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn ftell(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn os(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn shortdesc(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn getbankbal(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn getmsghdr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub fn setmsghdr(vm: &mut VirtualMachine, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
