#![allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]

use std::fs;
use std::path::PathBuf;

use crate::ast::constant::STACK_LIMIT;
use crate::datetime::{IcbDate, IcbTime};
use crate::executable::{GenericVariableData, PPEExpr, VariableData, VariableType, VariableValue};
use crate::icy_board::state::functions::{MASK_ALNUM, MASK_ALPHA, MASK_ASCII, MASK_FILE, MASK_NUM, MASK_PATH, MASK_PWD};
use crate::icy_board::state::GraphicsMode;
use crate::parser::CONFERENCE_ID;
use crate::vm::{TerminalTarget, VirtualMachine};
use crate::Res;
use chrono::Local;
use codepages::tables::{CP437_TO_UNICODE, UNICODE_TO_CP437};
use icy_engine::{update_crc32, Position, TextPane};
use radix_fmt::radix;
use rand::Rng; // 0.8.5

/// Should never be called. But some op codes are invalid as function call (like plus or function call)
/// and are handled by it's own `PPEExpressions` and will point to this function.
///
/// # Panics
///
/// Always
pub async fn invalid(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
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
pub async fn len(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    Ok(VariableValue::new_int(str.chars().count() as i32))
}

/// Returns the lowercase equivalent of a string
/// # Arguments
///  * `str` - A string value
/// # Returns
///  `VariableValue::String` - lowercase equivalent of `str`
pub async fn lower(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    Ok(VariableValue::new_string(str.to_lowercase()))
}

/// Returns the uppercase equivalent of a string
/// # Arguments
///  * `str` - A string value
/// # Returns
///  `VariableValue::String` - uppercase equivalent of `str`
pub async fn upper(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    Ok(VariableValue::new_string(str.to_uppercase()))
}

/// Returns a substring
/// # Arguments
///  * `str` - A string value
///  * `pos` - An integer value with a position from str to begin the substring 1 == first char
///  * `chars` - An integer value with the number of chars to take from `str`
/// # Returns
///  the substring of `str`, "" if chars <= 0, Will add padding up to the full length specified
pub async fn mid(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    let mut pos = vm.eval_expr(&args[1]).await?.as_int() - 1; // 1 based
    let mut chars = vm.eval_expr(&args[2]).await?.as_int();
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
        str.chars().skip(pos as usize).take(chars as usize).for_each(|c| res.push(c));
    }
    Ok(VariableValue::new_string(res))
}

pub async fn left(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut chars = vm.eval_expr(&args[1]).await?.as_int();
    if chars <= 0 {
        return Ok(VariableValue::new_string(String::new()));
    }
    let str = vm.eval_expr(&args[0]).await?.as_string().chars().collect::<Vec<_>>();
    let mut res = String::new();
    if chars > 0 {
        if chars < str.len() as i32 {
            str.iter().take(chars as usize).for_each(|c| res.push(*c));
        } else {
            str.iter().for_each(|c| res.push(*c));
            chars -= str.len() as i32;
            while chars > 0 {
                res.push(' ');
                chars -= 1;
            }
        }
    }
    Ok(VariableValue::new_string(res))
}

pub async fn right(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let chars = vm.eval_expr(&args[1]).await?.as_int();
    if chars <= 0 {
        return Ok(VariableValue::new_string(String::new()));
    }
    let mut chars = chars as usize;

    let mut res = String::new();
    let str = vm.eval_expr(&args[0]).await?.as_string().chars().collect::<Vec<_>>();
    if chars > 0 {
        while chars > str.len() {
            res.push(' ');
            chars -= 1;
        }
        str.iter().rev().take(chars).for_each(|c| res.push(*c));
    }
    Ok(VariableValue::new_string(res))
}

pub async fn space(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let chars = vm.eval_expr(&args[0]).await?.as_int();
    if chars <= 0 {
        return Ok(VariableValue::new_string(String::new()));
    }
    let res = " ".repeat(chars as usize);
    Ok(VariableValue::new_string(res))
}

pub async fn ferr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let channel = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_bool(vm.io.ferr(channel as usize)))
}

pub async fn chr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let c = vm.eval_expr(&args[0]).await?.as_int();
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

pub async fn asc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let c = vm.eval_expr(&args[0]).await?.as_string();
    if c.is_empty() {
        return Ok(VariableValue::new_int(0));
    }
    let ch = c.chars().next().unwrap_or('\0');
    if let Some(cp437) = UNICODE_TO_CP437.get(&ch) {
        return Ok(VariableValue::new_int(*cp437 as i32));
    }
    Ok(VariableValue::new_int(ch as i32))
}

/// Returns the position of a substring
/// # Arguments
///  * `str` - A string value
///  * `sub` - A string expression to search for
/// # Returns
///  A 1 based integer of the position of sub or 0 if sub is not found.
pub async fn instr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    let sub = vm.eval_expr(&args[1]).await?.as_string();
    if sub.is_empty() {
        return Ok(VariableValue::new_int(0));
    }
    match str.find(&sub) {
        Some(x) => {
            let x = str[0..x].chars().count();
            Ok(VariableValue::new_int(1 + x as i32))
        }
        _ => Ok(VariableValue::new_int(0)),
    }
}

/// Returns a flag indicating if the user has aborted the display of information.
pub async fn abort(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.disp_options.abort_printout))
}

/// Trim specified characters from the beginning of a string
/// # Arguments
///  * `str` - A string value
///  * `ch` - A string with the character to strip from the beginning of `str`
/// # Returns
///  The trimmed `str`
pub async fn ltrim(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut ch = vm.eval_expr(&args[1]).await?.as_string();
    if ch.is_empty() {
        return Ok(vm.eval_expr(&args[0]).await?.clone());
    }
    let str = vm.eval_expr(&args[0]).await?.as_string();
    let pat = ch.remove(0);
    Ok(VariableValue::new_string(str.trim_start_matches(pat).to_string()))
}

/// Replaces all occurences of a given character to another character in a string.
/// # Arguments
///  * `str` - A string value
///  * `old` - A string with the old character
///  * `new` - A string with the new character
pub async fn replace(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    let old = vm.eval_expr(&args[1]).await?.as_string();
    let new = vm.eval_expr(&args[2]).await?.as_string();

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
pub async fn strip(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    let ch: String = vm.eval_expr(&args[1]).await?.as_string();
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
pub async fn stripatx(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
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

pub async fn replacestr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let input = vm.eval_expr(&args[0]).await?.as_string();
    let search = vm.eval_expr(&args[1]).await?.as_string();
    let replace = vm.eval_expr(&args[2]).await?.as_string();
    Ok(VariableValue::new_string(input.replace(&search, &replace)))
}

pub async fn stripstr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let input = vm.eval_expr(&args[0]).await?.as_string();
    let search = vm.eval_expr(&args[1]).await?.as_string();
    Ok(VariableValue::new_string(input.replace(&search, "")))
}

/// Trim specified characters from the end of a string
/// # Arguments
///  * `str` - A string value
///  * `ch` - A string with the character to strip from the end of `str`
/// # Returns
///  The trimmed `str`
pub async fn rtrim(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut ch = vm.eval_expr(&args[1]).await?.as_string();
    if ch.is_empty() {
        return Ok(vm.eval_expr(&args[0]).await?.clone());
    }
    let str = vm.eval_expr(&args[0]).await?.as_string();

    let pat = ch.remove(0);
    Ok(VariableValue::new_string(str.trim_end_matches(pat).to_string()))
}

/// Trim specified characters from the beginning and end of a string
/// # Arguments
///  * `str` - A string value
///  * `ch` - A string with the character to strip from the beginning and end of `str`
/// # Returns
///  The trimmed `str`
pub async fn trim(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut ch = vm.eval_expr(&args[1]).await?.as_string();
    if ch.is_empty() {
        return Ok(vm.eval_expr(&args[0]).await?.clone());
    }
    let str = vm.eval_expr(&args[0]).await?.as_string();

    let pat = ch.remove(0);
    Ok(VariableValue::new_string(str.trim_matches(pat).to_string()))
}

pub async fn random(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let upper = vm.eval_expr(&args[0]).await?.as_int();
    if upper <= 0 {
        return Ok(VariableValue::new_int(0));
    }

    let mut rng = rand::thread_rng();
    Ok(VariableValue::new_int(rng.gen_range(0..upper)))
}

pub async fn date(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_date(IcbDate::today().to_pcboard_date()))
}

pub async fn time(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_time(IcbTime::now().to_pcboard_time()))
}

pub async fn u_name(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new_string(user.get_name().clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}

pub async fn u_ldate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Date,
            VariableData::from_int(IcbDate::from_utc(user.stats.last_on).to_pcboard_date()),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Date, VariableData::default()))
    }
}

pub async fn u_ltime(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Time,
            VariableData::from_int(IcbTime::from_naive(user.stats.last_on.naive_local()).to_pcboard_time()),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Time, VariableData::default()))
    }
}

pub async fn u_ldir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Time,
            VariableData::from_int(IcbTime::from_naive(user.date_last_dir_read.naive_local()).to_pcboard_time()),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Time, VariableData::default()))
    }
}
pub async fn u_lmr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO") // TODO
}
pub async fn u_logons(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(user.stats.num_times_on as i32),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_ful(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(VariableType::Integer, VariableData::from_int(user.stats.num_uploads as i32)))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_fdl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(user.stats.num_downloads as i32),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_bdlday(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(user.stats.today_dnld_bytes as i32),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_timeon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(0), // TODO: ON TIME COUNTER
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_bdl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(user.stats.total_dnld_bytes as i32),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_bul(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(user.stats.total_upld_bytes as i32),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_msgrd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(user.stats.messages_read as i32),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_msgwr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(user.stats.messages_left as i32),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}

pub async fn year(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbDate::from_pcboard(var as u32).get_year() as i32))
}
pub async fn month(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbDate::from_pcboard(var as u32).get_month() as i32))
}
pub async fn day(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbDate::from_pcboard(var as u32).get_day() as i32))
}
pub async fn dow(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbDate::from_pcboard(var as u32).get_day_of_week() as i32))
}
pub async fn hour(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbTime::from_pcboard(var).get_hour() as i32))
}
pub async fn min(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbTime::from_pcboard(var).get_minute() as i32))
}
pub async fn sec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbTime::from_pcboard(var).get_second() as i32))
}
pub async fn timeap(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_string(IcbTime::from_pcboard(var).to_string()))
}
pub async fn ver(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(1540))
}
pub async fn nochar(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(vm.icy_board_state.session.no_char.to_string()))
}
pub async fn yeschar(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(vm.icy_board_state.session.yes_char.to_string()))
}

pub async fn inkey(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    target_inkey(vm, TerminalTarget::Both).await
}

pub async fn target_inkey(vm: &mut VirtualMachine<'_>, target: TerminalTarget) -> Res<VariableValue> {
    if let Some(key_char) = vm.icy_board_state.get_char(target).await? {
        if key_char.ch as u8 == 127 {
            return Ok(VariableValue::new_string("DEL".to_string()));
        }
        if key_char.ch == '\x1B' {
            if let Some(key_char) = vm.icy_board_state.get_char(target).await? {
                if key_char.ch == '[' {
                    if let Some(key_char) = vm.icy_board_state.get_char(target).await? {
                        match key_char.ch {
                            'A' => return Ok(VariableValue::new_string("UP".to_string())),
                            'B' => return Ok(VariableValue::new_string("DOWN".to_string())),
                            'C' => return Ok(VariableValue::new_string("RIGHT".to_string())),
                            'D' => return Ok(VariableValue::new_string("LEFT".to_string())),

                            'H' => return Ok(VariableValue::new_string("HOME".to_string())),
                            'K' => return Ok(VariableValue::new_string("END".to_string())),

                            'V' => return Ok(VariableValue::new_string("PGUP".to_string())),
                            'U' => return Ok(VariableValue::new_string("PGDN".to_string())),

                            '@' => return Ok(VariableValue::new_string("INS".to_string())),

                            _ => return Ok(VariableValue::new_string(key_char.ch.to_string())),
                        }
                    }
                }
            }
            return Ok(VariableValue::new_string("\x1B".to_string()));
        }
        Ok(VariableValue::new_string(key_char.ch.to_string()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}

pub async fn tostring(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(vm.eval_expr(&args[0]).await?.as_string()))
}
pub async fn mask_pwd(_vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_PWD.to_string()))
}
pub async fn mask_alpha(_vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_ALPHA.to_string()))
}
pub async fn mask_num(_vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_NUM.to_string()))
}
pub async fn mask_alnum(_vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_ALNUM.to_string()))
}
pub async fn mask_file(_vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_FILE.to_string()))
}
pub async fn mask_path(_vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_PATH.to_string()))
}
pub async fn mask_ascii(_vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(MASK_ASCII.to_string()))
}
pub async fn curconf(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.current_conference_number as i32))
}
pub async fn pcbdat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(vm.icy_board_state.get_pcbdat().await?))
}

pub async fn ppepath(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let Some(dir) = vm.file_name.parent() else {
        return Ok(VariableValue::new_string(String::new()));
    };
    let mut res = dir.to_string_lossy().to_string();
    res.push('/');
    Ok(VariableValue::new_string(res))
}

pub async fn valdate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let date = vm.eval_expr(&args[0]).await?.as_string();
    Ok(VariableValue::new_bool(!IcbDate::parse(&date).is_empty()))
}
pub async fn valtime(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let time = vm.eval_expr(&args[0]).await?.as_string();
    Ok(VariableValue::new_bool(!IcbTime::parse(&time).is_empty()))
}

pub async fn pcbnode(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.node as i32 + 1))
}

pub async fn readline(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    let line = vm.eval_expr(&args[1]).await?.as_int();
    let file_name = vm.resolve_file(&file_name).await;

    if let Ok(file) = fs::read(&file_name) {
        let file = file.iter().map(|x| CP437_TO_UNICODE[*x as usize]).collect::<String>();

        let line_text = file.lines().nth(line as usize - 1).unwrap_or_default();
        Ok(VariableValue::new_string(line_text.to_string()))
    } else {
        log::warn!("PPE readline: file not found: {}", file_name.display());
        Ok(VariableValue::new_string(String::new()))
    }
}

pub async fn sysopsec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(
        vm.icy_board_state.get_board().await.config.sysop_security_level.sysop as i32,
    ))
}
pub async fn onlocal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.is_local))
}

pub async fn un_stat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_int(node.status as i32))
    } else {
        Ok(VariableValue::new_int(0))
    }
}

pub async fn un_name(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_string(node.name.clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub async fn un_city(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_string(node.city.clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub async fn un_oper(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_string(node.operation.clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub async fn cursec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.cur_security as i32))
}

pub async fn gettoken(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(tok) = vm.icy_board_state.session.tokens.pop_front() {
        Ok(VariableValue::new_string(tok))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub async fn minleft(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.minutes_left() as i32))
}

pub async fn minon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let min = (Local::now() - vm.icy_board_state.session.login_date).num_minutes();
    Ok(VariableValue::new_int(min as i32))
}

pub async fn getenv(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = &vm.eval_expr(&args[0]).await?.as_string();
    if let Some(var) = vm.icy_board_state.get_env(var) {
        Ok(VariableValue::new_string(var.to_string()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub async fn callid(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regah(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regbl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regbh(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regcl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regch(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regdl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regdh(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regax(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regbx(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regcx(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regdx(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regsi(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regdi(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regcf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn regds(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn reges(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn b2w(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn peekb(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn peekw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn mkaddr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let seg = vm.eval_expr(&args[0]).await?.as_int();
    let off = vm.eval_expr(&args[1]).await?.as_int();
    Ok(VariableValue::new_int(seg.wrapping_mul(0x10000) | off))
}
pub async fn exist(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    log::warn!("1 exist: {}", file_name);
    let file_name = vm.resolve_file(&file_name).await;
    log::warn!("exist: {}", file_name.display());
    Ok(VariableValue::new_bool(file_name.exists()))
}

/// Convert an integer to a string in a specified number base.
/// # Arguments
///  * `int` - Any integer to convert to string format.
///  * `base` - The base to use for the conversion. 2 <= base <= 36
/// # Returns
///  A string representation of `int` in the specified base.
pub async fn i2s(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let int = vm.eval_expr(&args[0]).await?.as_int();
    let base = vm.eval_expr(&args[1]).await?.as_int();
    let s = radix(int, base as u8).to_string();
    Ok(VariableValue::new_string(s))
}

/// Convert a string in a specified number base to an integer.
/// # Arguments
///  * `src` - A string value to convert to an integer.
///  * `base` - The base to use for the conversion. 2 <= base <= 36
/// # Returns
///  An integer representation of `s` in the specified base.
pub async fn s2i(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let src = vm.eval_expr(&args[0]).await?.as_string();
    let base = vm.eval_expr(&args[1]).await?.as_int();
    if src.is_empty() {
        return Ok(VariableValue::new_int(0));
    }
    let i = i32::from_str_radix(&src, base as u32)?;
    Ok(VariableValue::new_int(i))
}
pub async fn carrier(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.get_bps()))
}
pub async fn tokenstr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let mut res = String::new();
    for tok in vm.icy_board_state.session.tokens.drain(..) {
        if !res.is_empty() {
            res.push(';');
        }
        res.push_str(&tok);
    }
    Ok(VariableValue::new_string(res))
}
pub async fn cdon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn langext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(vm.icy_board_state.session.language.clone()))
}
pub async fn ansion(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(
        vm.icy_board_state.session.disp_options.grapics_mode != GraphicsMode::Ctty,
    ))
}
pub async fn valcc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn fmtcc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn cctype(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub async fn getx(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.get_caret_position().0 + 1))
}

pub async fn gety(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let y = vm.icy_board_state.get_caret_position().1;
    Ok(VariableValue::new_int(y + 1))
}

pub async fn band(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let left = vm.eval_expr(&args[0]).await?.as_int();
    let right = vm.eval_expr(&args[1]).await?.as_int();
    Ok(VariableValue::new_int(left & right))
}

pub async fn bor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let left = vm.eval_expr(&args[0]).await?.as_int();
    let right = vm.eval_expr(&args[1]).await?.as_int();
    Ok(VariableValue::new_int(left | right))
}

pub async fn bxor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let left = vm.eval_expr(&args[0]).await?.as_int();
    let right = vm.eval_expr(&args[1]).await?.as_int();
    Ok(VariableValue::new_int(left ^ right))
}

pub async fn bnot(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let val = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(!val))
}

pub async fn u_pwdhist(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let hist = vm.eval_expr(&args[0]).await?.as_int();
    match hist {
        1..3 => {
            if let Some(user) = &vm.icy_board_state.session.current_user {
                if let Some(pwd) = user.password.prev_pwd.get(hist as usize - 1) {
                    return Ok(VariableValue::new_string(format!("{}", pwd)));
                }
                Ok(VariableValue::new_string(String::new()))
            } else {
                Ok(VariableValue::new_string(String::new()))
            }
        }
        _ => Ok(VariableValue::new_string(String::new())),
    }
}

pub async fn u_pwdlc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Date,
            VariableData::from_int(IcbDate::from_utc(user.password.last_change).to_pcboard_date()),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Date, VariableData::default()))
    }
}

pub async fn u_pwdtc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new(
            VariableType::Integer,
            VariableData::from_int(user.password.times_changed as i32),
        ))
    } else {
        Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
    }
}
pub async fn u_stat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let option = vm.eval_expr(&args[0]).await?.as_int();
    match option {
        1 => {
            //  first date the user called the system
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Date,
                    VariableData::from_int(IcbDate::from_utc(user.stats.first_date_on).to_pcboard_date()),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Date, VariableData::default()))
            }
        }
        2 => {
            //  number of SysOp pages the user has requested
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_sysop_pages as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        3 => {
            //  number of group chats the user has participated in
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_group_chats as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        4 => {
            //  number of comments the user has left
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_comments as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        5..=9 => {
            // Number of x bps connects
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_times_on as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        10 => {
            // number of security violations
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_sec_viol as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        11 => {
            // number of “not registered in conference” warnings
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(VariableType::Integer, VariableData::from_int(user.stats.num_not_reg as i32)))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        12 => {
            // number of times the users download limit has been reached
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_reach_dnld_lim as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        13 => {
            // number of “file not found” warnings
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_file_not_found as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        14 => {
            // number of password errors the user has had
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_password_failures as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        15 => {
            //  number of verify errors the user has had
            if let Some(user) = &vm.icy_board_state.session.current_user {
                Ok(VariableValue::new(
                    VariableType::Integer,
                    VariableData::from_int(user.stats.num_verify_errors as i32),
                ))
            } else {
                Ok(VariableValue::new(VariableType::Integer, VariableData::default()))
            }
        }
        _ => Ok(VariableValue::new_int(0)),
    }
}
pub async fn defcolor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let color = vm.icy_board_state.get_board().await.config.color_configuration.default.clone();
    match color {
        crate::icy_board::icb_config::IcbColor::None => Ok(VariableValue::new_int(7)),
        crate::icy_board::icb_config::IcbColor::Dos(col) => Ok(VariableValue::new_int(col as i32)),
        crate::icy_board::icb_config::IcbColor::IcyEngine(_) => Ok(VariableValue::new_int(7)),
    }
}
pub async fn abs(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let val = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(val.abs()))
}

pub async fn grafmode(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    match vm.icy_board_state.session.disp_options.grapics_mode {
        crate::icy_board::state::GraphicsMode::Ctty => Ok(VariableValue::new_string("N".to_string())),
        crate::icy_board::state::GraphicsMode::Ansi => Ok(VariableValue::new_string("A".to_string())),
        crate::icy_board::state::GraphicsMode::Graphics => Ok(VariableValue::new_string("G".to_string())),
        crate::icy_board::state::GraphicsMode::Avatar => {
            // Avatar is new!
            Ok(VariableValue::new_string("V".to_string()))
        }
        crate::icy_board::state::GraphicsMode::Rip => Ok(VariableValue::new_string("R".to_string())),
    }
}

// psa stands for "pcboard supported allocations"
// pcboard supported allocations are:
// 1 - Alias support (PCBALIAS)
// 2 - Verification support (PCBVERIFY)
// 3 - Address support (PCBADDRESS)
// 4 - Password-Changing support (PCBPASSWORD)
// 5 - Caller Statistics support (PCBSTATS)
// 6 - Caller Notes support (PCBNOTES)
// 7 - Accounting Support (PCBACCOUNT)
// 8 - QWK/Net Support (PCBQWKNET)
// 9 - Personal Info Support (PCBPERSONAL)
// 10 - Time/Byte Bank Support (PCBBANK)
//
// IcyBoard supports most of these, however I pretend it's not if the feature isn't used.
pub async fn psa(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    let res = match var {
        1 => vm.icy_board_state.session.use_alias,
        2 => vm.icy_board_state.board.lock().await.config.new_user_settings.ask_verification,
        3 => vm.icy_board_state.board.lock().await.config.new_user_settings.ask_address,
        4 => {
            // Password support
            true
        }
        5 => {
            // Statistics support
            true
        }
        6 => {
            // Notes support
            vm.icy_board_state.board.lock().await.config.new_user_settings.ask_comment
        }
        _ => false,
    };

    Ok(VariableValue::new_bool(res))
}

#[allow(clippy::unnecessary_wraps)]
pub async fn fileinf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let file = vm.eval_expr(&args[0]).await?.as_string();
    let item = vm.eval_expr(&args[1]).await?.as_int();

    let file = vm.resolve_file(&file).await;
    let path = PathBuf::from(&file);
    match item {
        1 => Ok(VariableValue::new_bool(file.exists())),
        2 => Ok(VariableValue::new(VariableType::Date, VariableData::default())), // TODO: File date
        3 => Ok(VariableValue::new(VariableType::Time, VariableData::default())), // TODO: File time
        4 => Ok(VariableValue::new_int(file.metadata()?.len() as i32)),
        5 => Ok(VariableValue::new_int(0)),                   // TODO: File attributes
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

pub async fn ppename(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let p = vm.file_name.with_extension("");
    let Some(dir) = p.file_name() else {
        return Ok(VariableValue::new_string(String::new()));
    };
    let res = dir.to_string_lossy().to_string();
    Ok(VariableValue::new_string(res))
}

pub async fn mkdate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub async fn curcolor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let attr = vm.icy_board_state.display_screen().caret.get_attribute().as_u8(icy_engine::IceMode::Blink);
    Ok(VariableValue::new_int(attr as i32))
}

pub async fn kinkey(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    target_inkey(vm, TerminalTarget::Sysop).await
}
pub async fn minkey(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    target_inkey(vm, TerminalTarget::User).await
}
pub async fn maxnode(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.nodes.len() as i32))
}
pub async fn slpath(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.icy_board_state
            .get_board()
            .await
            .config
            .paths
            .security_file_path
            .to_string_lossy()
            .to_string(),
    ))
}
pub async fn helppath(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.icy_board_state.get_board().await.config.paths.help_path.to_string_lossy().to_string(),
    ))
}
pub async fn temppath(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(
        vm.icy_board_state.get_board().await.config.paths.tmp_work_path.to_string_lossy().to_string(),
    ))
}
pub async fn modem(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn loggedon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(!vm.icy_board_state.session.user_name.is_empty()))
}
pub async fn callnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn mgetbyte(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn tokcount(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.tokens.len() as i32))
}

pub async fn u_recnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let user_name = vm.eval_expr(&args[0]).await?.as_string().to_uppercase();
    for (i, user) in vm.icy_board_state.get_board().await.users.iter().enumerate() {
        if user.get_name().to_uppercase() == user_name {
            return Ok(VariableValue::new_int(i as i32));
        }
    }
    Ok(VariableValue::new_int(-1))
}

pub async fn u_inconf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn peekdw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function 'peekdw' !");
    let mut rng = rand::thread_rng();

    Ok(VariableValue::new_int(rng.gen()))
}
pub async fn dbglevel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.debug_level))
}
pub async fn scrtext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let col = vm.eval_expr(&args[0]).await?.as_int() - 1;
    let row = vm.eval_expr(&args[1]).await?.as_int() - 1;
    let len = vm.eval_expr(&args[2]).await?.as_int();
    let code = vm.eval_expr(&args[3]).await?.as_bool();
    let mut res = String::new();

    let mut cur_color = -1;
    for i in 0..len {
        let ch = vm.icy_board_state.display_screen().buffer.get_char(Position::new(col + i, row));
        if code {
            let col = ch.attribute.as_u8(icy_engine::IceMode::Blink) as i32;
            if cur_color != col && (!ch.is_transparent() || cur_color & 0b0111_0000 != col & 0b0111_0000) {
                res.push_str(&format!("@X{:02X}", col));
                cur_color = col;
            }
        }
        res.push(ch.ch);
    }
    Ok(VariableValue::new_string(res))
}
pub async fn showstat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.disp_options.show_on_screen))
}
pub async fn pagestat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.paged_sysop))
}
pub async fn tobigstr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::BigStr))
}
pub async fn toboolean(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Boolean))
}
pub async fn tobyte(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Byte))
}
pub async fn todate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Date))
}
pub async fn todreal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Double))
}
pub async fn toedate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::EDate))
}
pub async fn tointeger(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Integer))
}
pub async fn tomoney(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Money))
}
pub async fn toreal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Float))
}
pub async fn tosbyte(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::SByte))
}
pub async fn tosword(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::SWord))
}
pub async fn totime(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Time))
}
pub async fn tounsigned(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Unsigned))
}
pub async fn toword(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(vm.eval_expr(&args[0]).await?.clone().convert_to(VariableType::Word))
}
pub async fn mixed(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let param = vm.eval_expr(&args[0]).await?.as_string();
    Ok(VariableValue::new_string(fix_casing(param)))
}

pub async fn alias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.use_alias))
}
pub async fn confreg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let conf_num = vm.eval_expr(&args[0]).await?.as_int() as usize;

    // TODO: What is that ?
    // vm.icy_board_state.get_board().await.conferences[conf_num].
    Ok(VariableValue::new_bool(true))
}
pub async fn confexp(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn confsel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn confsys(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn confmw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn lprinted(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn isnonstop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.is_non_stop))
}
pub async fn errcorrect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    // No longer an issue:
    Ok(VariableValue::new_bool(true))
}
pub async fn confalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn useralias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(
        vm.icy_board_state.session.use_alias && vm.icy_board_state.session.current_conference.allow_aliases,
    ))
}
pub async fn curuser(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.cur_user_id as i32))
}
pub async fn chatstat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn defans(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn lastans(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn meganum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn evttimeadj(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn isbitset(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    let bit = vm.eval_expr(&args[1]).await?.as_int();

    Ok(VariableValue::new_bool(var & (1 << bit) != 0))
}
pub async fn fmtreal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn flagcnt(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn kbdbufsize(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn pplbufsize(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn kbdfilusued(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn lomsgnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("lomsgnum is deprecated!");
    Ok(VariableValue::new_int(1))
}
pub async fn himsgnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("himsgnum is deprecated!");
    Ok(VariableValue::new_int(1))
}

pub async fn drivespace(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn outbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(0))
}
pub async fn hiconfnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.get_board().await.conferences.len() as i32 - 1))
}

pub async fn inbytes(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.inbytes()))
}

pub async fn crc32(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let use_file = vm.eval_expr(&args[0]).await?.as_bool();
    let param = vm.eval_expr(&args[1]).await?.as_string();

    if use_file {
        let file = vm.resolve_file(&param).await;
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

pub async fn pcbmac(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_string();

    if let Some(expanded) = vm
        .icy_board_state
        .translate_variable(crate::vm::TerminalTarget::Sysop, var.trim_matches('@'))
        .await
    {
        Ok(VariableValue::new_string(expanded))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}
pub async fn actmsgnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

/// Usage: `STACKLEFT()`
//  Val: Returns the number of bytes left on the *system* stack.
pub async fn stackleft(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(STACK_LIMIT - vm.return_addresses.len() as i32))
}

/// `STACKERR()`
/// Returns a boolean value which indicates a stack error has occured
pub async fn stackerr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(STACK_LIMIT > vm.return_addresses.len() as i32))
}

pub async fn dgetalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dbof(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dchanged(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn ddecimals(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn ddeleted(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn deof(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn derr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dfields(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dlength(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dname(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dreccount(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn drecno(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dtype(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn fnext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dnext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn toddate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dcloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dsetalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dpack(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dlockf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dlock(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dlockr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dunlock(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dnopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dnclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dncloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dnew(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dadd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dappend(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dtop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dgo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dbottom(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dskip(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn ddelete(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn drecall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dtag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dseek(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dfblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dfcopy(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dselect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn dchkstat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub async fn pcbaccount(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn pcbaccstat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn derrmsg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn account(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn scanmsghdr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn checkrip(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn ripver(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn qwklimits(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn findfirst(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn findnext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn uselmrs(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

pub async fn new_confinfo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let conf_num = vm.eval_expr(&args[0]).await?.as_int() as usize;
    if let Some(conference) = &vm.icy_board_state.get_board().await.conferences.get(conf_num) {
        vm.user_data.push(Box::new((*conference).clone()));
        Ok(VariableValue {
            data: VariableData::from_int(0),
            generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
            vtype: VariableType::UserData(CONFERENCE_ID as u8),
        })
    } else {
        log::error!("PPL: Can't get conference {} (CONFINFO)", conf_num);
        Ok(VariableValue::new_string(String::new()))
    }
}

pub async fn confinfo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let conf_num = vm.eval_expr(&args[0]).await?.as_int() as usize;
    let conf_field = vm.eval_expr(&args[1]).await?.as_int();

    get_confinfo(vm, conf_num, conf_field).await
}
pub async fn get_confinfo(vm: &mut VirtualMachine<'_>, conf_num: usize, conf_field: i32) -> Res<VariableValue> {
    if let Some(conference) = &vm.icy_board_state.get_board().await.conferences.get(conf_num) {
        match conf_field {
            1 => Ok(VariableValue::new_string(conference.name.clone())),
            2 => Ok(VariableValue::new_bool(conference.is_public)),
            3 => Ok(VariableValue::new_bool(conference.auto_rejoin)),
            4 => Ok(VariableValue::new_bool(conference.view_members)),
            5 => Ok(VariableValue::new_bool(conference.private_uploads)),
            6 => Ok(VariableValue::new_bool(conference.private_msgs)),
            7 => Ok(VariableValue::new_bool(false)), // conference.echo_mail
            8 => Ok(VariableValue::new_int(conference.required_security.level() as i32)),
            9 => Ok(VariableValue::new_int(conference.add_conference_security)),
            10 => Ok(VariableValue::new_int(conference.add_conference_time as i32)),
            11 => Ok(VariableValue::new_int(0)),                // message blocks
            12 => Ok(VariableValue::new_string(String::new())), // message file
            13 => Ok(VariableValue::new_string(conference.users_menu.to_string_lossy().to_string())),
            14 => Ok(VariableValue::new_string(conference.sysop_menu.to_string_lossy().to_string())),
            15 => Ok(VariableValue::new_string(conference.news_file.to_string_lossy().to_string())),
            16 => Ok(VariableValue::new_int(conference.pub_upload_sort as i32)),
            17 => Ok(VariableValue::new_string(conference.pub_upload_dir_file.to_string_lossy().to_string())),
            18 => Ok(VariableValue::new_string(conference.pub_upload_location.to_string_lossy().to_string())),
            19 => Ok(VariableValue::new_int(conference.private_upload_sort as i32)),
            20 => Ok(VariableValue::new_string(conference.private_upload_dir_file.to_string_lossy().to_string())),
            21 => Ok(VariableValue::new_string(conference.private_upload_location.to_string_lossy().to_string())),
            22 => Ok(VariableValue::new_string(conference.doors_menu.to_string_lossy().to_string())),
            23 => Ok(VariableValue::new_string(conference.doors_file.to_string_lossy().to_string())),
            24 => Ok(VariableValue::new_string(conference.blt_menu.to_string_lossy().to_string())),
            25 => Ok(VariableValue::new_string(conference.blt_file.to_string_lossy().to_string())),
            26 => Ok(VariableValue::new_string(conference.survey_menu.to_string_lossy().to_string())),
            27 => Ok(VariableValue::new_string(conference.survey_file.to_string_lossy().to_string())),
            28 => Ok(VariableValue::new_string(conference.dir_menu.to_string_lossy().to_string())),
            29 => Ok(VariableValue::new_string(conference.dir_file.to_string_lossy().to_string())),
            30 => Ok(VariableValue::new_string(conference.attachment_location.to_string_lossy().to_string())), // PthNameLoc ???
            31 => Ok(VariableValue::new_bool(false)),                                                          // force echo
            32 => Ok(VariableValue::new_bool(false)),                                                          // read only
            33 => Ok(VariableValue::new_bool(conference.private_msgs)),
            34 => Ok(VariableValue::new_int(0)),      // ret receipt level
            35 => Ok(VariableValue::new_bool(false)), // record origin
            36 => Ok(VariableValue::new_bool(false)), // prompt for routing
            37 => Ok(VariableValue::new_bool(conference.allow_aliases)),
            38 => Ok(VariableValue::new_bool(false)),                                      // show intro  on ra
            39 => Ok(VariableValue::new_int(conference.required_security.level() as i32)), // req level to enter mail
            40 => Ok(VariableValue::new_string(conference.password.to_string())),
            41 => Ok(VariableValue::new_string(conference.intro_file.to_string_lossy().to_string())),
            42 => Ok(VariableValue::new_string(conference.attachment_location.to_string_lossy().to_string())),
            43 => Ok(VariableValue::new_string(String::new())),                      // reg flags
            44 => Ok(VariableValue::new_byte(conference.required_security.level())), // attach level
            45 => Ok(VariableValue::new_byte(0)),                                    // carbon limit
            46 => Ok(VariableValue::new_string(conference.command_file.to_string_lossy().to_string())),
            47 => Ok(VariableValue::new_bool(true)),  // old index
            48 => Ok(VariableValue::new_bool(true)),  // long to names
            49 => Ok(VariableValue::new_byte(0)),     // carbon level
            50 => Ok(VariableValue::new_byte(0)),     // conf type
            51 => Ok(VariableValue::new_int(0)),      // export ptr
            52 => Ok(VariableValue::new_double(0.0)), // charge time
            53 => Ok(VariableValue::new_double(0.0)), // charge msg read
            54 => Ok(VariableValue::new_double(0.0)), // charge msg write
            _ => Ok(VariableValue::new_int(-1)),
        }
    } else {
        Ok(VariableValue::new_int(-1))
    }
}

pub async fn tinkey(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    inkey(vm, args).await
}
pub async fn cwd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn instrr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn fdordaka(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn fdordorg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn fdordarea(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn fdoqrd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn getdrive(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn setdrive(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn bs2i(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn bd2i(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn i2bs(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn i2bd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn ftell(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn os(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn shortdesc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn getbankbal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn getmsghdr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}
pub async fn setmsghdr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("not implemented function!");
    panic!("TODO")
}

/// Should be the same logic than the one in pcboard.
fn fix_casing(param: String) -> String {
    let mut res = String::new();
    let mut first_char = true;
    let mut param = param.to_ascii_lowercase().chars().collect::<Vec<char>>();
    param.push(' ');
    let mut i = 0;
    while i < param.len() - 1 {
        let mut ch = param[i];
        if first_char {
            if param[i..].starts_with(&['i', 'i', 'i', ' ']) {
                res.push_str("III");
                i += 3;
                continue;
            }

            if param[i..].starts_with(&['i', 'i', ' ']) {
                res.push_str("III");
                i += 2;
                continue;
            }

            if param[i..].starts_with(&['m', 'c']) {
                res.push_str("Mc");
                i += 2;
                continue;
            }

            if param[i..].starts_with(&['v', 'o', 'n', ' ']) {
                res.push_str("von");
                i += 3;
                continue;
            }

            if param[i..].starts_with(&['d', 'e', ' ']) {
                res.push_str("de");
                i += 2;
                continue;
            }
            ch = ch.to_ascii_uppercase();
            first_char = false;
        } else {
            ch = ch.to_ascii_lowercase();
        }
        if ch == ' ' {
            first_char = true;
        }
        res.push(ch);
        i += 1;
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_casing() {
        assert_eq!(fix_casing("hello world".to_string()), "Hello World");
        assert_eq!(fix_casing("HELLO WORLD".to_string()), "Hello World");
        assert_eq!(fix_casing("cul de sac".to_string()), "Cul de Sac");
        assert_eq!(fix_casing("freiherr von schaffhausen".to_string()), "Freiherr von Schaffhausen");
        assert_eq!(fix_casing("henry iii".to_string()), "Henry III");
        assert_eq!(fix_casing("mcdonald".to_string()), "McDonald");
    }
}
