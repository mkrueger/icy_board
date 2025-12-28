#![allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]

use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs};

use crate::Res;
use crate::ast::constant::STACK_LIMIT;
use crate::datetime::{IcbDate, IcbTime};
use crate::executable::{GenericVariableData, PPEExpr, VariableData, VariableType, VariableValue};
use crate::icy_board::conferences::ConferenceType;
use crate::icy_board::macro_parser::Macro;
use crate::icy_board::read_with_encoding_detection;
use crate::icy_board::security_expr::SecurityExpression;
use crate::icy_board::state::GraphicsMode;
use crate::icy_board::state::functions::{MASK_ALNUM, MASK_ALPHA, MASK_ASCII, MASK_FILE, MASK_NUM, MASK_PATH, MASK_PWD};
use crate::icy_board::user_base::{ConferenceFlags, Password};
use crate::icy_board::user_inf::{BankUserInf, QwkConfigUserInf};
use crate::parser::CONFERENCE_ID;
use crate::vm::{MAX_FILE_CHANNELS, TerminalTarget, VirtualMachine, get_file_channel};
use chrono::{DateTime, Utc};
use icy_engine::{Position, TextPane};
use icy_net::crc::update_crc32;
use jamjam::jam::JamMessageBase;
use jamjam::jam::msg_header::JamMessageHeader;
use jamjam::util::basic_real::{BasicDouble, BasicReal};
use radix_fmt::radix;
use rand::Rng; // 0.8.5

const HDR_ACTIVE: i32 = 0x0E;
const HDR_BLOCKS: i32 = 0x04;
const HDR_DATE: i32 = 0x05;
const HDR_ECHO: i32 = 0x0F;
const HDR_FROM: i32 = 0x0B;
const HDR_MSGNUM: i32 = 0x02;
const HDR_MSGREF: i32 = 0x03;
const HDR_PWD: i32 = 0x0D;
const HDR_REPLY: i32 = 0x0A;
const HDR_RPLYDATE: i32 = 0x08;
const HDR_RPLYTIME: i32 = 0x09;
const HDR_STATUS: i32 = 0x01;
const HDR_SUBJ: i32 = 0x0C;
const HDR_TIME: i32 = 0x06;
const HDR_TO: i32 = 0x07;

macro_rules! unimplemented_function {
    ($name:expr) => {{
        log::error!("{} function not implemented", $name);
        panic!("{} function not implemented", $name);
    }};
}

/// Should never be called. But some op codes are invalid as function call (like plus or function call)
/// and are handled by it's own `PPEExpressions` and will point to this function.
///
/// # Panics
///
/// Always
pub async fn invalid(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("invalid function call (should never happen)!");
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
    let str = vm.eval_expr(&args[0]).await?;
    let val = match str.generic_data {
        GenericVariableData::String(str) => str.chars().count(),
        GenericVariableData::Dim1(items) => items.len() - 1,
        GenericVariableData::Dim2(items) => items.len() - 1,
        GenericVariableData::Dim3(items) => items.len() - 1,
        GenericVariableData::Password(p) => {
            match p {
                Password::PlainText(s) => s.chars().count(),
                _ => 6, // always return 6 for passwords
            }
        }
        GenericVariableData::None if str.vtype == VariableType::String || str.vtype == VariableType::BigStr => 0,
        _ => {
            log::warn!("len: called on invalid type: '{}'.", str.vtype);
            0
        }
    };
    Ok(VariableValue::new_int(val as i32))
}

pub async fn len_dim(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let arr = vm.eval_expr(&args[0]).await?;
    let dim = vm.eval_expr(&args[1]).await?.as_int();

    let val = match arr.generic_data {
        GenericVariableData::String(str) => {
            if dim == 0 {
                str.chars().count()
            } else {
                0
            }
        }
        GenericVariableData::Dim1(items) => {
            if dim == 0 {
                items.len()
            } else {
                0
            }
        }
        GenericVariableData::Dim2(items) => match dim {
            0 => items.len() - 1,
            1 => items[0].len() - 1,
            _ => 0,
        },
        GenericVariableData::Dim3(items) => match dim {
            0 => items.len() - 1,
            1 => items[0].len() - 1,
            2 => items[0][0].len() - 1,
            _ => 0,
        },
        GenericVariableData::Password(_) => {
            if dim == 0 {
                6 // always return 6 for passwords
            } else {
                0
            }
        }
        _ => {
            log::warn!("len({dim}): called on invalid type.");
            0
        }
    };

    Ok(VariableValue::new_int(val as i32))
}

/// Returns the lowercase equivalent of a string
/// # Arguments
///  * `str` - A string value
/// # Returns
///  `VariableValue::String` - lowercase equivalent of `str`
pub async fn lower(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let value = vm.eval_expr(&args[0]).await?;

    // Check if it's a non-plaintext password and return unchanged
    if let GenericVariableData::Password(ref pwd) = value.generic_data {
        if !matches!(pwd, Password::PlainText(_)) {
            return Ok(value.clone());
        }
    }

    let str = value.as_string();
    Ok(VariableValue::new_string(str.to_lowercase()))
}

/// Returns the uppercase equivalent of a string
/// # Arguments
///  * `str` - A string value
/// # Returns
///  `VariableValue::String` - uppercase equivalent of `str`
pub async fn upper(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let value = vm.eval_expr(&args[0]).await?;

    // Check if it's a non-plaintext password and return unchanged
    if let GenericVariableData::Password(ref pwd) = value.generic_data {
        if !matches!(pwd, Password::PlainText(_)) {
            return Ok(value.clone());
        }
    }

    let str = value.as_string();
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
        str.iter().rev().take(chars).rev().for_each(|c| res.push(*c));
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
    let channel = get_file_channel(vm, args).await?;
    Ok(VariableValue::new_bool(vm.io.ferr(channel)))
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
    let ch = codepages::tables::CP437_TO_UNICODE[c as usize].to_string();
    Ok(VariableValue::new_string(ch))
}

pub async fn asc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let c = vm.eval_expr(&args[0]).await?.as_string();
    if c.is_empty() {
        return Ok(VariableValue::new_int(0));
    }
    let ch = c.chars().next().unwrap_or('\0');
    if let Some(cp437) = codepages::tables::UNICODE_TO_CP437.get(&ch) {
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
    let s = vm.eval_expr(&args[0]).await?.as_string();
    let old = vm.eval_expr(&args[1]).await?.as_string();
    let new = vm.eval_expr(&args[2]).await?.as_string();
    let Some(old_ch) = old.chars().next() else {
        return Ok(VariableValue::new_string(s));
    };
    if new.is_empty() {
        return Ok(VariableValue::new_string(s.chars().filter(|c| *c != old_ch).collect()));
    }
    let new_ch = new.chars().next().unwrap();
    Ok(VariableValue::new_string(s.chars().map(|c| if c == old_ch { new_ch } else { c }).collect()))
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
                    res.push('X');
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

    let mut rng = rand::rng();
    Ok(VariableValue::new_int(rng.random_range(0..upper)))
}

pub async fn date(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_date(IcbDate::today().to_pcboard_date()))
}

pub async fn time(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_time(IcbTime::now().to_pcboard_time()))
}

pub async fn u_name(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(vm.user.get_name().clone()))
}

pub async fn u_ldate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Date,
        VariableData::from_int(IcbDate::from_utc(&vm.user.stats.last_on).to_pcboard_date()),
    ))
}

pub async fn u_ltime(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Time,
        VariableData::from_int(IcbTime::from_naive(vm.user.stats.last_on.naive_local()).to_pcboard_time()),
    ))
}

pub async fn u_ldir(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Date,
        VariableData::from_int(IcbDate::from_utc(&vm.user.date_last_dir_read).to_pcboard_date()),
    ))
}
pub async fn u_lmr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.last_msg_read as i32))
}
pub async fn u_logons(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.user.stats.num_times_on as i32))
}
pub async fn u_ful(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.user.stats.num_uploads as i32))
}
pub async fn u_fdl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.user.stats.num_downloads as i32))
}
pub async fn u_bdlday(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Integer,
        VariableData::from_int(vm.user.stats.today_dnld_bytes as i32),
    ))
}
pub async fn u_timeon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let elapsed = (chrono::Utc::now() - vm.icy_board_state.session.login_date).num_seconds();
    Ok(VariableValue::new(VariableType::Integer, VariableData::from_int(elapsed as i32)))
}
pub async fn u_bdl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Double,
        VariableData::from_float(vm.user.stats.total_dnld_bytes as f64),
    ))
}

pub async fn u_bul(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Double,
        VariableData::from_float(vm.user.stats.total_upld_bytes as f64),
    ))
}

pub async fn u_msgrd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Unsigned,
        VariableData::from_int(vm.user.stats.messages_read as i32),
    ))
}

pub async fn u_msgwr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Integer,
        VariableData::from_int(vm.user.stats.messages_left as i32),
    ))
}

pub async fn year(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbDate::from_pcboard(var as u32).year() as i32))
}
pub async fn month(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbDate::from_pcboard(var as u32).month() as i32))
}
pub async fn day(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbDate::from_pcboard(var as u32).day() as i32))
}
pub async fn dow(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(IcbDate::from_pcboard(var as u32).day_of_week() as i32))
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
                            'F' => return Ok(VariableValue::new_string("END".to_string())),

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
    res.push(std::path::MAIN_SEPARATOR);
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

    if let Ok(file) = read_with_encoding_detection(&file_name) {
        let file = file.replace("\r\n", "\n");
        let line_text = file.lines().nth(line as usize - 1).unwrap_or_default();
        Ok(VariableValue::new_string(line_text.to_string()))
    } else {
        log::warn!("PPE readline: file not found: {}", file_name.display());
        Ok(VariableValue::new_string(String::new()))
    }
}

pub async fn sysopsec(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(
        vm.icy_board_state.get_board().await.config.sysop_command_level.sysop as i32,
    ))
}
pub async fn onlocal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.is_local))
}

pub async fn un_stat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        Ok(VariableValue::new_string(node.status.to_char().to_string()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}

pub async fn un_name(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        if let Some(user) = vm.icy_board_state.board.lock().await.users.get(node.cur_user as usize) {
            return Ok(VariableValue::new_string(user.get_name().clone()));
        }
    }
    Ok(VariableValue::new_string(String::new()))
}
pub async fn un_city(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(node) = &vm.pcb_node {
        if let Some(user) = vm.icy_board_state.board.lock().await.users.get(node.cur_user as usize) {
            let city = user.city_or_state.clone();
            return Ok(VariableValue::new_string(city));
        }
    }
    Ok(VariableValue::new_string(String::new()))
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
    let min = (Utc::now() - vm.icy_board_state.session.login_date).num_minutes();
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
    Ok(VariableValue::new_string(vm.icy_board_state.session.caller_number.to_string()))
}

pub async fn regal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGAL");
}
pub async fn regah(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGAH");
}

pub async fn regbl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGBL");
}

pub async fn regbh(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGBH");
}
pub async fn regcl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGCL");
}
pub async fn regch(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGCH");
}
pub async fn regdl(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGDL");
}
pub async fn regdh(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGDH");
}
pub async fn regax(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGAX");
}
pub async fn regbx(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGBX");
}
pub async fn regcx(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGCX");
}
pub async fn regdx(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGDX");
}
pub async fn regsi(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGSI");
}
pub async fn regdi(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGDI");
}
pub async fn regf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGF");
}
pub async fn regcf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGCF");
}
pub async fn regds(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGDS");
}
pub async fn reges(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("REGES");
}

pub async fn b2w(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let low = vm.eval_expr(&args[0]).await?.as_int();
    let hi = vm.eval_expr(&args[1]).await?.as_int();
    Ok(VariableValue::new_int((low & 0xFF) | ((hi & 0xFF) << 8)))
}

pub async fn peekb(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("PEEKB");
}
pub async fn peekw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("PEEKW");
}
pub async fn mkaddr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let seg = vm.eval_expr(&args[0]).await?.as_int();
    let off = vm.eval_expr(&args[1]).await?.as_int();
    Ok(VariableValue::new_int(seg.wrapping_mul(0x10000) | off))
}
pub async fn exist(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let file_name = vm.eval_expr(&args[0]).await?.as_string();
    let file_name = vm.resolve_file(&file_name).await;
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
    let base = vm.eval_expr(&args[1]).await?.as_int() as u32;
    if !(2..=36).contains(&base) || src.is_empty() {
        return Ok(VariableValue::new_int(0));
    }
    let mut acc: u32 = 0;
    for ch in src.chars() {
        match ch.to_digit(base) {
            Some(d) => acc = acc.wrapping_mul(base).wrapping_add(d),
            None => break,
        }
    }
    Ok(VariableValue::new_int(acc as i32))
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

/// Returns TRUE if the carrier detect signal is on
/// deprecated - always true
pub async fn cdon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(true))
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
    let ccnum = vm.eval_expr(&args[0]).await?.as_string();
    let is_valid = if let Ok(card) = ccnum.parse::<creditcard::CreditCard>() {
        true
    } else {
        false
    };
    Ok(VariableValue::new_bool(is_valid))
}

pub async fn fmtcc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let ccnum: String = vm.eval_expr(&args[0]).await?.as_string();
    let fmt = match ccnum.len() {
        13 => {
            format!("{} {} {} {}", &ccnum[0..3], &ccnum[3..7], &ccnum[7..11], &ccnum[11..])
        }
        15 => {
            format!("{} {} {}", &ccnum[0..4], &ccnum[4..10], &ccnum[10..])
        }
        16 => {
            format!("{} {} {} {}", &ccnum[0..4], &ccnum[4..8], &ccnum[8..12], &ccnum[12..])
        }
        _ => ccnum,
    };
    Ok(VariableValue::new_string(fmt))
}

pub async fn cctype(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let ccnum = vm.eval_expr(&args[0]).await?.as_string();
    let issuer = if let Ok(card) = ccnum.parse::<creditcard::CreditCard>() {
        card.issuer().name().to_string()
    } else {
        "UNKNOWN".to_string()
    };

    Ok(VariableValue::new_string(issuer))
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
            if let Some(pwd) = vm.user.password.prev_pwd.get(hist as usize - 1) {
                return Ok(VariableValue::new_password(pwd.clone()));
            }
            Ok(VariableValue::new_string(String::new()))
        }
        _ => Ok(VariableValue::new_string(String::new())),
    }
}

pub async fn u_pwdlc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Date,
        VariableData::from_int(IcbDate::from_utc(&vm.user.password.last_change).to_pcboard_date()),
    ))
}

pub async fn u_pwdtc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new(
        VariableType::Integer,
        VariableData::from_int(vm.user.password.times_changed as i32),
    ))
}

pub async fn u_stat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let option = vm.eval_expr(&args[0]).await?.as_int();
    match option {
        1 => {
            //  first date the user called the system
            Ok(VariableValue::new(
                VariableType::Date,
                VariableData::from_int(IcbDate::from_utc(&vm.user.stats.first_date_on).to_pcboard_date()),
            ))
        }
        2 => {
            //  number of SysOp pages the user has requested
            Ok(VariableValue::new(
                VariableType::Integer,
                VariableData::from_int(vm.user.stats.num_sysop_pages as i32),
            ))
        }
        3 => {
            //  number of group chats the user has participated in
            Ok(VariableValue::new(
                VariableType::Integer,
                VariableData::from_int(vm.user.stats.num_group_chats as i32),
            ))
        }
        4 => {
            //  number of comments the user has left
            Ok(VariableValue::new(
                VariableType::Integer,
                VariableData::from_int(vm.user.stats.num_comments as i32),
            ))
        }
        5..=9 => {
            // Number of x bps connects
            Ok(VariableValue::new(
                VariableType::Integer,
                VariableData::from_int(vm.user.stats.num_times_on as i32),
            ))
        }
        10 => {
            // number of security violations
            Ok(VariableValue::new(
                VariableType::Integer,
                VariableData::from_int(vm.user.stats.num_sec_viol as i32),
            ))
        }
        11 => {
            // number of “not registered in conference” warnings
            Ok(VariableValue::new_int(vm.user.stats.num_not_reg as i32))
        }
        12 => {
            // number of times the users download limit has been reached
            Ok(VariableValue::new_int(vm.user.stats.num_reach_dnld_lim as i32))
        }
        13 => {
            // number of “file not found” warnings
            Ok(VariableValue::new_int(vm.user.stats.num_file_not_found as i32))
        }
        14 => {
            // number of password errors the user has had
            Ok(VariableValue::new_int(vm.user.stats.num_password_failures as i32))
        }
        15 => {
            //  number of verify errors the user has had
            Ok(VariableValue::new_int(vm.user.stats.num_verify_errors as i32))
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
pub async fn psa(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    // OFC all are supported rigt now
    Ok(VariableValue::new_bool(true))
}

#[allow(clippy::unnecessary_wraps)]
pub async fn fileinf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let file = vm.eval_expr(&args[0]).await?.as_string();
    if file.is_empty() {
        log::error!("fileinf: empty filename");
        return Ok(VariableValue::new_int(0));
    }
    let item = vm.eval_expr(&args[1]).await?.as_int();

    let path = vm.resolve_file(&file).await;
    match item {
        1 => Ok(VariableValue::new_bool(path.exists())),
        2 => Ok(VariableValue::new(VariableType::Date, VariableData::default())), // TODO: File date
        3 => Ok(VariableValue::new(VariableType::Time, VariableData::default())), // TODO: File time
        4 => Ok(VariableValue::new_int(path.metadata()?.len() as i32)),
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
    let year = vm.eval_expr(&args[0]).await?.as_int();
    let month = vm.eval_expr(&args[1]).await?.as_int();
    let day = vm.eval_expr(&args[2]).await?.as_int();

    let date = IcbDate::new(month as u8, day as u8, year as u16);
    Ok(VariableValue::new(VariableType::Date, VariableData::from_int(date.to_pcboard_date())))
}

pub async fn curcolor(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let attr = vm.icy_board_state.display_screen().buffer.caret.attribute.as_u8(icy_engine::IceMode::Blink);
    Ok(VariableValue::new_int(attr as i32))
}

pub async fn kinkey(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    target_inkey(vm, TerminalTarget::Sysop).await
}
pub async fn minkey(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    target_inkey(vm, TerminalTarget::User).await
}
pub async fn maxnode(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.node_state.lock().await.len() as i32))
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
    Ok(VariableValue::new_string("CONNECT 9600/ARQ/V32".to_string()))
}

pub async fn loggedon(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(!vm.icy_board_state.session.user_name.is_empty()))
}

pub async fn callnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let board = vm.icy_board_state.get_board().await;
    Ok(VariableValue::new_int(board.statistics.cur_caller_number() as i32))
}

pub async fn mgetbyte(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    // Check if there's any raw input available from the user connection
    if let Ok(Some(byte)) = vm.icy_board_state.get_raw_byte().await {
        Ok(VariableValue::new_int(byte as i32))
    } else {
        // Return -1 if buffer is empty
        Ok(VariableValue::new_int(-1))
    }
}

pub async fn tokcount(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.tokens.len() as i32))
}

pub async fn u_recnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let user_name = vm.eval_expr(&args[0]).await?.as_string().to_uppercase();
    for (i, user) in vm.icy_board_state.get_board().await.users.iter().enumerate() {
        if user.get_name().to_uppercase() == user_name {
            return Ok(VariableValue::new_int(1 + i as i32));
        }
    }
    Ok(VariableValue::new_int(-1))
}

pub async fn u_inconf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let record = vm.eval_expr(&args[0]).await?.as_int();
    let (area, conf) = vm.eval_expr(&args[1]).await?.as_msg_id();
    let board = vm.icy_board_state.get_board().await;
    if let Some(user) = board.users.get(record as usize) {
        if let Some(conf) = &board.conferences.get(conf as usize) {
            if conf.required_security.user_can_access(user) {
                return Ok(VariableValue::new_bool(true));
            }
            if let Some(areas) = &conf.areas {
                if let Some(area) = areas.get(area as usize) {
                    if area.req_level_to_enter.user_can_access(user) {
                        return Ok(VariableValue::new_bool(true));
                    }
                }
            }
        }
    }
    Ok(VariableValue::new_bool(false))
}

pub async fn peekdw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    log::error!("simulating not implementable function 'peekdw' (random number)!");
    let mut rng = rand::rng();
    Ok(VariableValue::new_int(rng.random()))
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

    if let Some(session_user) = &vm.icy_board_state.session.current_user {
        if let Some(flags) = session_user.conference_flags.get(&conf_num) {
            return Ok(VariableValue::new_bool(flags.contains(ConferenceFlags::Registered)));
        }
    }

    Ok(VariableValue::new_bool(false))
}

pub async fn confexp(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let conf_num = vm.eval_expr(&args[0]).await?.as_int() as usize;

    if let Some(session_user) = &vm.icy_board_state.session.current_user {
        if let Some(flags) = session_user.conference_flags.get(&conf_num) {
            return Ok(VariableValue::new_bool(flags.contains(ConferenceFlags::Expired)));
        }
    }

    Ok(VariableValue::new_bool(false))
}

pub async fn confsel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let conf_num = vm.eval_expr(&args[0]).await?.as_int() as usize;

    if let Some(session_user) = &vm.icy_board_state.session.current_user {
        if let Some(flags) = session_user.conference_flags.get(&conf_num) {
            return Ok(VariableValue::new_bool(flags.contains(ConferenceFlags::Selected)));
        }
    }

    Ok(VariableValue::new_bool(false))
}

pub async fn confsys(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let conf_num = vm.eval_expr(&args[0]).await?.as_int() as usize;

    if let Some(session_user) = &vm.icy_board_state.session.current_user {
        if let Some(flags) = session_user.conference_flags.get(&conf_num) {
            return Ok(VariableValue::new_bool(flags.contains(ConferenceFlags::Sysop)));
        }
    }

    Ok(VariableValue::new_bool(false))
}

pub async fn confmw(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let conf_num = vm.eval_expr(&args[0]).await?.as_int() as usize;

    if let Some(session_user) = &vm.icy_board_state.session.current_user {
        if let Some(flags) = session_user.conference_flags.get(&conf_num) {
            return Ok(VariableValue::new_bool(flags.contains(ConferenceFlags::MailWaiting)));
        }
    }

    Ok(VariableValue::new_bool(false))
}

pub async fn lprinted(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.disp_options.num_lines_printed as i32))
}

pub async fn isnonstop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(!vm.icy_board_state.session.disp_options.count_lines))
}

pub async fn errcorrect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    // No longer an issue:
    Ok(VariableValue::new_bool(true))
}

pub async fn confalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.current_conference.allow_aliases))
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
    if let Some(state) = &vm.icy_board_state.node_state.lock().await[vm.icy_board_state.node] {
        return Ok(VariableValue::new_bool(state.enabled_chat));
    }
    Ok(VariableValue::new_bool(false))
}

pub async fn defans(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(answer) = &vm.icy_board_state.session.default_answer {
        Ok(VariableValue::new_string(answer.clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}

pub async fn lastans(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(answer) = &vm.icy_board_state.session.last_answer {
        Ok(VariableValue::new_string(answer.clone()))
    } else {
        Ok(VariableValue::new_string(String::new()))
    }
}

pub fn to_base_36(min_len: usize, number: i32) -> String {
    let mut n = number;
    let mut out = Vec::new();
    while out.len() < min_len || n > 0 {
        let d = (n % 36) as u8;
        out.push(if d < 10 { (b'0' + d) as char } else { (b'A' + d - 10) as char });
        n /= 36;
    }
    out.iter().rev().collect()
}

pub async fn meganum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_string(to_base_36(0, var)))
}

pub async fn evttimeadj(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("EVTTIMEADJ");
}

pub async fn isbitset(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let var = vm.eval_expr(&args[0]).await?.as_int();
    let bit = vm.eval_expr(&args[1]).await?.as_int();

    Ok(VariableValue::new_bool(var & (1 << bit) != 0))
}

pub async fn fmtreal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let value = vm.eval_expr(&args[0]).await?.as_double();
    let field_width = vm.eval_expr(&args[1]).await?.as_int() as usize;
    let decimal_places = vm.eval_expr(&args[2]).await?.as_int() as usize;

    // Format the number with the specified decimal places
    let formatted = format!("{:.prec$}", value, prec = decimal_places);

    // Right-justify with spaces if needed
    let result = if formatted.len() < field_width {
        format!("{:>width$}", formatted, width = field_width)
    } else {
        formatted
    };

    Ok(VariableValue::new_string(result))
}

pub async fn flagcnt(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.session.flagged_files.len() as i32))
}

pub async fn kbdbufsize(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(vm.icy_board_state.kbdbufsize()))
}

pub async fn pplbufsize(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    // Seems to always return null in PCBoard
    Ok(VariableValue::new_int(0))
}

pub async fn kbdfilusued(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("KBDFILUSUED");
}

pub async fn lomsgnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let area = 0;
    let msg_base: PathBuf = vm.icy_board_state.session.current_conference.areas.as_ref().unwrap()[area].path.clone();
    match JamMessageBase::open(msg_base) {
        Ok(base) => Ok(VariableValue::new_int(base.base_messagenumber() as i32)),
        Err(err) => {
            log::error!("LOMSGNUM can't open message base in area {area}: {err}");
            Ok(VariableValue::new_int(0))
        }
    }
}

pub async fn himsgnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let area = 0;
    let msg_base = vm.icy_board_state.session.current_conference.areas.as_ref().unwrap()[area].path.clone();
    match JamMessageBase::open(&msg_base) {
        Ok(base) => Ok(VariableValue::new_int((base.base_messagenumber() + base.active_messages() - 1) as i32)),
        Err(err) => {
            log::error!("HIMSGNUM can't open message base in area {area}: {err}");
            Ok(VariableValue::new_int(0))
        }
    }
}

pub async fn drivespace(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DRIVESPACE");
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
    if let Ok(pm) = Macro::from_str(&var.trim_matches('@')) {
        if let Some(expanded) = vm.icy_board_state.run_macro(crate::vm::TerminalTarget::Sysop, pm).await {
            return Ok(VariableValue::new_string(expanded));
        }
    }
    Ok(VariableValue::new_string(String::new()))
}
pub async fn actmsgnum(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let area = vm.icy_board_state.session.current_message_area;
    let msg_base = vm.icy_board_state.session.current_conference.areas.as_ref().unwrap()[area].path.clone();
    match jamjam::jam::JamMessageBase::open(msg_base) {
        Ok(base) => Ok(VariableValue::new_int(base.active_messages() as i32)),
        Err(err) => {
            log::error!("ACTMSGNUM can't open message base: {err}");
            Ok(VariableValue::new_int(0))
        }
    }
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
    unimplemented_function!("DGETALIAS");
}
pub async fn dbof(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DBOF");
}
pub async fn dchanged(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DCHANGED");
}
pub async fn ddecimals(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DDECIMALS");
}
pub async fn ddeleted(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DDELETED");
}
pub async fn deof(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DEOF");
}
pub async fn derr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DERR");
}
pub async fn dfields(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DFIELDS");
}
pub async fn dlength(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DLENGTH");
}
pub async fn dname(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DNAME");
}
pub async fn dreccount(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DRECCOUNT");
}
pub async fn drecno(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DRECNO");
}
pub async fn dtype(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DTYPE");
}
pub async fn fnext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    for i in 1..MAX_FILE_CHANNELS {
        if !vm.io.is_open(i) {
            return Ok(VariableValue::new_int(i as i32));
        }
    }
    Ok(VariableValue::new_int(-1))
}

pub async fn dnext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DNEXT");
}
pub async fn toddate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("TODDATE");
}
pub async fn dcloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DCLOSEALL");
}
pub async fn dopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DOPEN");
}
pub async fn dclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DCLOSE");
}
pub async fn dsetalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DSETALIAS");
}
pub async fn dpack(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DPACK");
}
pub async fn dlockf(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DLOCKF");
}
pub async fn dlock(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DLOCK");
}
pub async fn dlockr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DLOCKR");
}
pub async fn dunlock(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DUNLOCK");
}
pub async fn dnopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DNOPEN");
}
pub async fn dnclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DNCLOSE");
}
pub async fn dncloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DNCLOSEALL");
}
pub async fn dnew(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DNEW");
}
pub async fn dadd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DADD");
}
pub async fn dappend(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DAPPEND");
}
pub async fn dtop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DTOP");
}
pub async fn dgo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DGO");
}
pub async fn dbottom(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DBOTTOM");
}
pub async fn dskip(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DSKIP");
}
pub async fn dblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DBLANK");
}
pub async fn ddelete(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DDELETE");
}
pub async fn drecall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DRECALL");
}
pub async fn dtag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DTAG");
}
pub async fn dseek(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DSEEK");
}
pub async fn dfblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DFBLANK");
}
pub async fn dget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DGET");
}
pub async fn dput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DPUT");
}
pub async fn dfcopy(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DFCOPY");
}
pub async fn dselect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DSELECT");
}
pub async fn dchkstat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DCHKSTAT");
}

pub async fn pcbaccount(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let field = vm.eval_expr(&args[0]).await?.as_int();

    if let Some(accounting) = &vm.icy_board_state.get_board().await.config.accounting.accounting_config {
        match field {
            0 => return Ok(VariableValue::new_double(accounting.new_user_balance)),
            1 => return Ok(VariableValue::new_double(accounting.charge_per_logon)),
            2 => return Ok(VariableValue::new_double(accounting.charge_per_time)),
            3 => return Ok(VariableValue::new_double(accounting.charge_per_peak_time)),
            4 => return Ok(VariableValue::new_double(accounting.charge_per_group_chat_time)),
            5 => return Ok(VariableValue::new_double(accounting.charge_per_msg_read)),
            6 => return Ok(VariableValue::new_double(accounting.charge_per_msg_read_captured)),
            7 => return Ok(VariableValue::new_double(accounting.charge_per_msg_written)),
            8 => return Ok(VariableValue::new_double(accounting.charge_per_msg_write_echoed)),
            9 => return Ok(VariableValue::new_double(accounting.charge_per_msg_write_private)),
            10 => return Ok(VariableValue::new_double(accounting.charge_per_download_file)),
            11 => return Ok(VariableValue::new_double(accounting.charge_per_download_bytes)),
            12 => return Ok(VariableValue::new_double(accounting.pay_back_for_upload_file)),
            13 => return Ok(VariableValue::new_double(accounting.pay_back_for_upload_bytes)),
            14 => return Ok(VariableValue::new_double(accounting.warn_level)),
            _ => {
                log::error!("PCBACCOUNT: Invalid field number: {field}");
            }
        }
    }
    Ok(VariableValue::new_double(-1.0))
}

pub async fn pcbaccstat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let field = vm.eval_expr(&args[0]).await?.as_int();
    match field {
        0 => {
            // ActStatus
            // TODO
            Ok(VariableValue::new_int(b'T' as i32))
        }
        1 => Ok(VariableValue::new_double(vm.icy_board_state.session.current_conference.charge_time)),
        2 => Ok(VariableValue::new_double(vm.icy_board_state.session.current_conference.charge_msg_read)),
        3 => Ok(VariableValue::new_double(vm.icy_board_state.session.current_conference.charge_msg_write)),
        4 => {
            // Balance
            Ok(VariableValue::new_double(vm.icy_board_state.session.calculate_balance()))
        }
        _ => {
            log::error!("PCBACCSTAT: Invalid field number: {field}");
            Ok(VariableValue::new_double(-1.0))
        }
    }
}

pub async fn derrmsg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("DERRMSG");
}

pub async fn account(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let field = vm.eval_expr(&args[0]).await?.as_int();

    // Get or initialize user accounting data
    if vm.user.account.is_none() {
        vm.user.account = Some(crate::icy_board::user_inf::AccountUserInf::default());
    }

    let Some(accounting) = &vm.user.account else {
        return Ok(VariableValue::new_double(0.0));
    };

    let value = match field {
        0 => accounting.starting_balance,        // START_BAL - User's starting balance
        1 => accounting.starting_balance,        // TODO: START_SESSION - Starting balance for this session
        2 => accounting.debit_call,              // DEB_CALL - Debit for this call
        3 => accounting.debit_time,              // DEB_TIME - Debit for time online
        4 => accounting.debit_msg_read,          // DEB_MSGREAD - Debit for reading messages
        5 => accounting.debit_msg_read_capture,  // DEB_MSGCAP - Debit for capturing messages
        6 => accounting.debit_msg_write,         // DEB_MSGWRITE - Debit for writing messages
        7 => accounting.debit_msg_write_echoed,  // DEB_MSGECHOED - Debit for echoed messages
        8 => accounting.debit_msg_write_private, // DEB_MSGPRIVATE - Debit for private messages
        9 => accounting.debit_download_file,     // DEB_DOWNFILE - Debit for downloading files
        10 => accounting.debit_download_bytes,   // DEB_DOWNBYTES - Debit for downloading bytes
        11 => accounting.debit_group_chat,       // DEB_CHAT - Debit for chat time
        12 => accounting.debit_tpu,              // DEB_TPU - Debit for TPU
        13 => accounting.debit_special,          // DEB_SPECIAL - Special debit
        14 => accounting.credit_upload_file,     // CRED_UPFILE - Credit for uploading files
        15 => accounting.credit_upload_bytes,    // CRED_UPBYTES - Credit for uploading bytes
        16 => accounting.credit_special,         // CRED_SPECIAL - Special credit
        17 => {
            // SEC_DROP - Security level to drop to at 0 credits
            let level = if let Some(config) = &vm.icy_board_state.get_board().await.config.accounting.accounting_config {
                accounting.drop_sec_level as i32
            } else {
                0
            };
            return Ok(VariableValue::new_int(level));
        }
        _ => {
            log::error!("ACCOUNT: Invalid field number: {field}");
            0.0
        }
    };

    Ok(VariableValue::new_double(value))
}

pub async fn scanmsghdr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("SCANMSGHDR");
}

pub async fn checkrip(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.icy_board_state.session.term_caps.rip_version.is_some()))
}

pub async fn ripver(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let ver = vm.icy_board_state.session.term_caps.rip_version.clone().unwrap_or("0".to_string());
    Ok(VariableValue::new_string(ver))
}

pub async fn qwklimits(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let field = vm.eval_expr(&args[0]).await?.as_int();

    // Check if QWK limits are initialized for the user
    if vm.user.qwk_config.is_none() {
        vm.user.qwk_config = Some(QwkConfigUserInf::default());
    }

    let Some(qwk_config) = &vm.user.qwk_config else {
        return Ok(VariableValue::new_int(0));
    };

    let value = match field {
        0 => qwk_config.max_msgs as i32,              // MAXMSGS - Max messages per QWK packet
        1 => qwk_config.max_msgs_per_conf as i32,     // CMAXMSGS - Max messages per conference
        2 => qwk_config.personal_attach_limit as i32, // ATTACH_LIM_U - Personal attachment size limit (bytes)
        3 => qwk_config.public_attach_limit as i32,   // ATTACH_LIM_P - Public attachment size limit (bytes)
        _ => {
            log::error!("QWKLIMITS: Invalid field number: {field}");
            0
        }
    };

    Ok(VariableValue::new_int(value))
}

pub async fn findfirst(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let filespec = vm.eval_expr(&args[0]).await?.as_string();
    vm.file_list.clear();

    if let Ok(g) = glob::glob(&filespec) {
        for entry in g {
            match entry {
                Ok(path) => {
                    let path = path.to_string_lossy().to_string();
                    vm.file_list.push_back(path);
                }
                Err(e) => {
                    continue;
                }
            }
        }
    }
    Ok(VariableValue::new_string(vm.file_list.pop_front().unwrap_or(String::new())))
}

pub async fn findnext(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_string(vm.file_list.pop_front().unwrap_or(String::new())))
}

pub async fn uselmrs(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_bool(vm.use_lmrs))
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
            4 => Ok(VariableValue::new_bool(conference.allow_view_conf_members)),
            5 => Ok(VariableValue::new_bool(conference.private_uploads)),
            6 => Ok(VariableValue::new_bool(conference.private_msgs)),
            7 => Ok(VariableValue::new_bool(conference.echo_mail_in_conference)),
            8 => Ok(VariableValue::new_int(conference.required_security.level() as i32)),
            9 => Ok(VariableValue::new_int(conference.add_conference_security)),
            10 => Ok(VariableValue::new_int(conference.add_conference_time as i32)),
            11 => Ok(VariableValue::new_int(0)),                // message blocks
            12 => Ok(VariableValue::new_string(String::new())), // message file
            13 => Ok(VariableValue::new_string(conference.users_menu.to_string_lossy().to_string())),
            14 => Ok(VariableValue::new_string(conference.sysop_menu.to_string_lossy().to_string())),
            15 => Ok(VariableValue::new_string(conference.news_file.to_string_lossy().to_string())),
            16 => Ok(VariableValue::new_int(conference.pub_upload_sort as i32)),
            17 => Ok(VariableValue::new_string(String::new())), // public upload dir file
            18 => Ok(VariableValue::new_string(conference.pub_upload_location.to_string_lossy().to_string())),
            19 => Ok(VariableValue::new_int(conference.private_upload_sort as i32)),
            20 => Ok(VariableValue::new_string(String::new())), // private upload dir file
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
            31 => Ok(VariableValue::new_bool(conference.force_echomail)),                                      // force echo
            32 => Ok(VariableValue::new_bool(conference.is_read_only)),                                        // read only
            33 => Ok(VariableValue::new_bool(conference.private_msgs)),
            34 => Ok(VariableValue::new_int(0)),                              // ret receipt level
            35 => Ok(VariableValue::new_bool(conference.record_origin)),      // record origin
            36 => Ok(VariableValue::new_bool(conference.prompt_for_routing)), // prompt for routing
            37 => Ok(VariableValue::new_bool(conference.allow_aliases)),
            38 => Ok(VariableValue::new_bool(conference.show_intro_in_scan)), //  show intro  on ra
            39 => Ok(VariableValue::new_int(conference.required_security.level() as i32)), // req level to enter mail
            40 => Ok(VariableValue::new_string(conference.password.to_string())),
            41 => Ok(VariableValue::new_string(conference.intro_file.to_string_lossy().to_string())),
            42 => Ok(VariableValue::new_string(conference.attachment_location.to_string_lossy().to_string())),
            43 => Ok(VariableValue::new_string(String::new())),                      // reg flags
            44 => Ok(VariableValue::new_byte(conference.required_security.level())), // attach level
            45 => Ok(VariableValue::new_byte(conference.carbon_list_limit)),         // carbon limit
            46 => Ok(VariableValue::new_string(conference.command_file.to_string_lossy().to_string())),
            47 => Ok(VariableValue::new_bool(false)),                              // old index
            48 => Ok(VariableValue::new_bool(conference.long_to_names)),           // long to names
            49 => Ok(VariableValue::new_byte(0)),                                  // carbon level
            50 => Ok(VariableValue::new_byte(conference.conference_type.to_u8())), // conf type
            51 => Ok(VariableValue::new_int(0)),                                   // export ptr
            52 => Ok(VariableValue::new_double(conference.charge_time)),           // charge time
            53 => Ok(VariableValue::new_double(conference.charge_msg_read)),       // charge msg read
            54 => Ok(VariableValue::new_double(conference.charge_msg_write)),      // charge msg write
            _ => Ok(VariableValue::new_int(-1)),
        }
    } else {
        Ok(VariableValue::new_int(-1))
    }
}

pub async fn set_confinfo(vm: &mut VirtualMachine<'_>, conf_num: usize, conf_field: i32, value: VariableValue) -> Res<()> {
    if let Some(conference) = vm.icy_board_state.get_board().await.conferences.get_mut(conf_num) {
        match conf_field {
            1 => conference.name = value.as_string(),
            2 => conference.is_public = value.as_bool(),
            3 => conference.auto_rejoin = value.as_bool(),
            4 => conference.allow_view_conf_members = value.as_bool(),
            5 => conference.private_uploads = value.as_bool(),
            6 => conference.private_msgs = value.as_bool(),
            7 => conference.echo_mail_in_conference = value.as_bool(),
            8 => conference.required_security = SecurityExpression::Constant(crate::icy_board::security_expr::Value::Integer(value.as_int() as i64)),
            9 => conference.add_conference_security = value.as_int(),
            10 => conference.add_conference_time = value.as_int() as u16,
            11 => (), // message blocks
            12 => (), // message file
            13 => conference.users_menu = PathBuf::from_str(&value.as_string())?,
            14 => conference.sysop_menu = PathBuf::from_str(&value.as_string())?,
            15 => conference.news_file = PathBuf::from_str(&value.as_string())?,
            16 => conference.pub_upload_sort = value.as_int() as u8,
            17 => (), // public upload dir file
            18 => conference.pub_upload_location = PathBuf::from_str(&value.as_string())?,
            19 => conference.private_upload_sort = value.as_int() as u8,
            20 => (), // private upload dir file
            21 => conference.private_upload_location = PathBuf::from_str(&value.as_string())?,
            22 => conference.doors_menu = PathBuf::from_str(&value.as_string())?,
            23 => conference.doors_file = PathBuf::from_str(&value.as_string())?,
            24 => conference.blt_menu = PathBuf::from_str(&value.as_string())?,
            25 => conference.blt_file = PathBuf::from_str(&value.as_string())?,
            26 => conference.survey_menu = PathBuf::from_str(&value.as_string())?,
            27 => conference.survey_file = PathBuf::from_str(&value.as_string())?,
            28 => conference.dir_menu = PathBuf::from_str(&value.as_string())?,
            29 => conference.dir_file = PathBuf::from_str(&value.as_string())?,
            30 => conference.attachment_location = PathBuf::from_str(&value.as_string())?,
            31 => conference.force_echomail = value.as_bool(),
            32 => conference.is_read_only = value.as_bool(),
            33 => conference.private_msgs = value.as_bool(),
            34 => (), // ret receipt level
            35 => conference.record_origin = value.as_bool(),
            36 => conference.prompt_for_routing = value.as_bool(),
            37 => conference.allow_aliases = value.as_bool(),
            38 => conference.show_intro_in_scan = value.as_bool(),
            39 => conference.required_security = SecurityExpression::Constant(crate::icy_board::security_expr::Value::Integer(value.as_int() as i64)),
            40 => conference.password = Password::PlainText(value.as_string()),
            41 => conference.intro_file = PathBuf::from_str(&value.as_string())?,
            42 => conference.attachment_location = PathBuf::from_str(&value.as_string())?,
            43 => (), // reg flags
            44 => conference.sec_attachments = SecurityExpression::Constant(crate::icy_board::security_expr::Value::Integer(value.as_int() as i64)),
            45 => (), // conference.carbon_limit = value.as_byte(),
            46 => conference.command_file = PathBuf::from_str(&value.as_string())?,
            47 => (), // old index
            48 => conference.long_to_names = value.as_bool(),
            49 => (), // conference.carbon_level = value.as_byte(),
            50 => conference.conference_type = ConferenceType::from_u8(value.as_byte()),
            51 => (), // conference.export_ptr = value.as_int(),
            52 => conference.charge_time = value.as_double(),
            53 => conference.charge_msg_read = value.as_double(),
            54 => conference.charge_msg_write = value.as_double(),
            _ => (),
        }
    }

    Ok(())
}

pub async fn tinkey(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    inkey(vm, args).await
}

pub async fn cwd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    match env::current_dir() {
        Ok(cur) => Ok(VariableValue::new_string(cur.to_string_lossy().to_string())),
        Err(err) => {
            log::error!("CWD error: {err}");
            Ok(VariableValue::new_string(String::new()))
        }
    }
}

pub async fn instrr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    let sub = vm.eval_expr(&args[1]).await?.as_string();
    if sub.is_empty() {
        return Ok(VariableValue::new_int(0));
    }
    match str.rfind(&sub) {
        Some(x) => {
            let x = str[0..x].chars().count();
            Ok(VariableValue::new_int(1 + x as i32))
        }
        _ => Ok(VariableValue::new_int(0)),
    }
}

pub async fn fdordaka(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("FDORDAKA");
}
pub async fn fdordorg(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("FDORDORG");
}

pub async fn fdordarea(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("FDORDAREA");
}

pub async fn fdoqrd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    unimplemented_function!("FDOQRD");
}

pub async fn getdrive(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    Ok(VariableValue::new_int(0))
}

pub async fn setdrive(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let drive = vm.eval_expr(&args[0]).await?.as_int();
    Ok(VariableValue::new_int(drive))
}

pub async fn bs2i(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    let val = BasicReal::from(str.chars().take(4).map(|c| c as u8).collect::<Vec<u8>>());
    Ok(VariableValue::new_int(val.into()))
}

pub async fn bd2i(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let str = vm.eval_expr(&args[0]).await?.as_string();
    let val: i64 = BasicDouble::from(str.chars().take(8).map(|c| c as u8).collect::<Vec<u8>>()).into();
    Ok(VariableValue::new_unsigned(val as u64))
}

pub async fn i2bs(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let val = vm.eval_expr(&args[0]).await?.as_int();
    let val = BasicReal::from(val);
    let a = val.bytes().iter().map(|c| *c as char).collect::<String>();
    Ok(VariableValue::new_string(a))
}

pub async fn i2bd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let val = vm.eval_expr(&args[0]).await?.as_unsigned() as i64;
    let val = BasicDouble::from(val);
    let a = val.bytes().iter().map(|c| *c as char).collect::<String>();
    Ok(VariableValue::new_string(a))
}

pub async fn ftell(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let channel = get_file_channel(vm, args).await?;
    Ok(VariableValue::new_int(vm.io.ftell(channel)? as i32))
}

pub async fn os(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let os_code = if cfg!(target_os = "windows") {
        1
    } else if cfg!(target_os = "linux") {
        3
    } else if cfg!(target_os = "macos") {
        4
    } else {
        0 // Unknown/other (BSDs, etc.)
    };
    Ok(VariableValue::new_int(os_code))
}

pub async fn shortdesc(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    if let Some(user) = &vm.icy_board_state.session.current_user {
        Ok(VariableValue::new_bool(user.flags.use_short_filedescr))
    } else {
        Ok(VariableValue::new_bool(false))
    }
}
pub async fn getbankbal(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let field = vm.eval_expr(&args[0]).await?.as_int();
    let value = vm.eval_expr(&args[1]).await?;

    if let Some(user) = &mut vm.icy_board_state.session.current_user {
        if user.bank.is_none() {
            user.bank = Some(BankUserInf::default());
        }
        if let Some(bank) = &mut user.bank {
            let value = match field {
                0 => VariableValue::new_date(bank.time_info.last_deposite_date.to_pcboard_date()),
                1 => VariableValue::new_date(bank.time_info.last_withdraw_date.to_pcboard_date()),
                2 => VariableValue::new_int(bank.time_info.last_transaction_amount as i32),
                3 => VariableValue::new_int(bank.time_info.amount_saved as i32),
                4 => VariableValue::new_int(bank.time_info.max_withdrawl_per_day as i32),
                5 => VariableValue::new_int(bank.time_info.max_stored_amount as i32),

                6 => VariableValue::new_date(bank.byte_info.last_deposite_date.to_pcboard_date()),
                7 => VariableValue::new_date(bank.byte_info.last_withdraw_date.to_pcboard_date()),
                8 => VariableValue::new_int(bank.byte_info.last_transaction_amount as i32),
                9 => VariableValue::new_int(bank.byte_info.amount_saved as i32),
                10 => VariableValue::new_int(bank.byte_info.max_withdrawl_per_day as i32),
                11 => VariableValue::new_int(bank.byte_info.max_stored_amount as i32),
                _ => {
                    log::error!("GET_BANK_BAL: Invalid field {}", field);
                    return Ok(VariableValue::new_int(0));
                }
            };
            return Ok(value);
        }
    }
    Ok(VariableValue::new_int(0))
}

pub async fn getmsghdr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let (conf_num, area_num) = vm.eval_expr(&args[0]).await?.as_msg_id();
    let field_num = vm.eval_expr(&args[2]).await?.as_int();
    let msg_num = vm.eval_expr(&args[1]).await?.as_int() as u32;
    if let Some((cn, an, mn, header)) = &vm.cached_msg_header {
        if conf_num == *cn && area_num == *an && msg_num == *mn {
            return get_field(field_num, header);
        }
    }

    let msg_base = {
        let board = vm.icy_board_state.get_board().await;
        let Some(conf) = board.conferences[conf_num as usize].areas.as_ref() else {
            log::error!("Can't read conference {conf_num}");
            return Ok(VariableValue::new_bool(false));
        };
        let Some(area) = conf.get(area_num as usize) else {
            log::error!("Can't read area {area_num} from {conf_num}");
            return Ok(VariableValue::new_bool(false));
        };
        area.path.clone()
    };

    let base = JamMessageBase::open(msg_base)?;
    match base.read_header(msg_num) {
        Ok(header) => {
            let res = get_field(field_num, &header);
            vm.cached_msg_header = Some((conf_num, area_num, msg_num, header));
            res
        }
        Err(err) => {
            log::error!("Can't read header {msg_num} from {conf_num}:{area_num} ({})", err);
            if field_num == HDR_ACTIVE {
                return Ok(VariableValue::new_int(226));
            }
            return Ok(VariableValue::new_bool(false));
        }
    }
}

fn get_field(field_num: i32, header: &JamMessageHeader) -> Res<VariableValue> {
    match field_num {
        HDR_ACTIVE => Ok(VariableValue::new_int(if header.is_deleted() { 226 } else { 225 })),
        HDR_BLOCKS => Ok(VariableValue::new_int((header.txt_len / 128) as i32)),
        HDR_DATE => {
            let date_time = DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or(Utc::now());
            let date = IcbDate::from_utc(&date_time);
            Ok(VariableValue::new_date(date.to_pcboard_date()))
        }
        HDR_ECHO => {
            // TODO
            Ok(VariableValue::new_bool(false))
        }
        HDR_FROM => {
            if let Some(from) = header.get_from() {
                Ok(VariableValue::new_string(from.to_string()))
            } else {
                Ok(VariableValue::new_string(String::new()))
            }
        }
        HDR_MSGNUM => Ok(VariableValue::new_int(header.message_number as i32)),
        HDR_MSGREF => Ok(VariableValue::new_int(header.reply_to as i32)),
        HDR_PWD => Ok(VariableValue::new_int(header.password_crc as i32)),
        HDR_REPLY => Ok(VariableValue::new_int(header.reply_to as i32)),
        HDR_RPLYDATE => {
            // TODO
            Ok(VariableValue::new_int(0))
        }
        HDR_RPLYTIME => {
            // TODO
            Ok(VariableValue::new_int(0))
        }
        HDR_STATUS => {
            // TODO
            Ok(VariableValue::new_int(0))
        }
        HDR_SUBJ => {
            if let Some(subj) = header.get_subject() {
                Ok(VariableValue::new_string(subj.to_string()))
            } else {
                Ok(VariableValue::new_string(String::new()))
            }
        }
        HDR_TIME => {
            let date_time = DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or(Utc::now());
            let time = IcbTime::from_naive(date_time.naive_local());
            Ok(VariableValue::new_time(time.to_pcboard_time()))
        }
        HDR_TO => {
            if let Some(to) = header.get_to() {
                Ok(VariableValue::new_string(to.to_string()))
            } else {
                Ok(VariableValue::new_string(String::new()))
            }
        }
        _ => {
            log::error!("PPL: Invalid message header field {field_num}");
            if field_num == HDR_ACTIVE {
                return Ok(VariableValue::new_int(226));
            }
            Ok(VariableValue::new_string(String::new()))
        }
    }
}

pub async fn setmsghdr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    vm.cached_msg_header = None;
    unimplemented_function!("SETMSGHDR");
}

pub async fn area_id(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let conference = vm.eval_expr(&args[0]).await?.as_int();
    let area = vm.eval_expr(&args[1]).await?.as_int();
    Ok(VariableValue::new_msg_id(conference, area))
}

/// Should be the same logic than the one in pcboard.
pub fn fix_casing(param: String) -> String {
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

pub async fn web_request(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<VariableValue> {
    let url = vm.eval_expr(&args[0]).await?.as_string();
    let response = reqwest::get(url.clone()).await?.text().await?;
    Ok(VariableValue::new_string(response))
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
