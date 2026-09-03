use i18n_embed_fl::fl;
use icy_board_engine::executable::{FuncOpCode, FunctionDefinition, OpCode, Signature, StatementDefinition, VariableType};
use icy_board_engine::parser::{
    AUDIO_ID, BOARD_ID, CHECKSUM_ENUM_ID, CONFERENCE_ID, CONTACT_ID, DOOR_ID, EDITOR_MODE_ENUM_ID, ERR_CODE_ENUM_ID, ERR_KIND_ENUM_ID, ERROR_ID, EVENT_ID,
    EVENT_KIND_ENUM_ID, FILE_DIRECTORY_ID, GFX_BACKEND_ENUM_ID, GFX_ID, HTTP_ID, HTTP_METHOD_ENUM_ID, HTTP_REQUEST_ID, HTTP_RESPONSE_ID, MACROS_ID, MARGINS_ID,
    MESSAGE_AREA_ID, MOUSE_ACTION_ENUM_ID, MOUSE_BUTTON_ENUM_ID, MOUSE_MODE_ENUM_ID, MOUSE_TRACKING_ENUM_ID, MSG_FIELD_ENUM_ID, MSG_ID, PALETTE_ID, REGEX_ID,
    REGEX_MATCH_ID, REGEX_OPTIONS_ENUM_ID, SESSION_ID, STRING_COMPARISON_ENUM_ID, SURFACE_ID, TERM_INFO_ID, TERM_INPUT_ID, TERMINAL_ID, USER_ID,
    UserTypeRegistry,
};
use std::fmt::Write as _;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::LANGUAGE_LOADER;

pub fn get_const_hover(c: &icy_board_engine::ast::constant::BuiltinConst) -> Option<Hover> {
    match c.name {
        "TRUE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-true")),
        "FALSE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-false")),
        "STK_LIMIT" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-stk_limit")),
        "ATTACH_LIM_P" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-attach_lim_p")),
        "ATTACH_LIM_U" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-attach_lim_u")),
        "ACC_CUR_BAL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-acc_cur_bal")),
        "F_NET" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-f_net")),
        "CMAXMSGS" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-cmaxmsgs")),
        "MAXMSGS" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-maxmsgs")),
        "CUR_USER" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-cur_user")),
        "NO_USER" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-no_user")),
        "ACC_STAT" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-acc_stat")),
        "ACC_TIME" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-acc_time")),
        "ACC_MSGREAD" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-acc_msgread")),
        "ACC_MSGWRITE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-acc_msgwrite")),
        "DEFS" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-defs")),
        "BELL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-bell")),
        "LOGIT" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-logit")),
        "LOGITLEFT" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-logitleft")),
        "AUTO" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-auto")),
        "ECHODOTS" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-echodots")),
        "ERASELINE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-eraseline")),
        "FIELDLEN" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-fieldlen")),
        "GUIDE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-guide")),
        "HIGHASCII" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-highascii")),
        "LFAFTER" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-lfafter")),
        "LFBEFORE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-lfbefore")),
        "NEWLINE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-newline")),
        "NOCLEAR" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-noclear")),
        "STACKED" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-stacked")),
        "UPCASE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-upcase")),
        "WORDWRAP" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-wordwrap")),
        "YESNO" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-yesno")),
        "NEWBALANCE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-newbalance")),
        "CHRG_CALL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_call")),
        "CHRG_TIME" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_time")),
        "CHRG_PEAKTIME" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_peaktime")),
        "CHRG_CHAT" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_chat")),
        "CHRG_MSGREAD" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_msgread")),
        "CHRG_MSGCAP" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_msgcap")),
        "CHRG_MSGWRITE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_msgwrite")),
        "CHRG_MSGECHOED" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_msgechoed")),
        "CHRG_MSGPRIVATE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_msgprivate")),
        "CHRG_DOWNFILE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_downfile")),
        "CHRG_DOWNBYTES" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-chrg_downbytes")),
        "PAY_UPFILE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-pay_upfile")),
        "PAY_UPBYTES" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-pay_upbytes")),
        "WARNLEVEL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-warnlevel")),
        "CRC_FILE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-crc_file")),
        "CRC_STR" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-crc_str")),
        "START_BAL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-start_bal")),
        "START_SESSION" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-start_session")),
        "DEB_CALL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_call")),
        "DEB_TIME" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_time")),
        "DEB_MSGREAD" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_msgread")),
        "DEB_MSGCAP" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_msgcap")),
        "DEB_MSGWRITE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_msgwrite")),
        "DEB_MSGECHOED" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_msgechoed")),
        "DEB_MSGPRIVATE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_msgprivate")),
        "DEB_DOWNFILE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_downfile")),
        "DEB_DOWNBYTES" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_downbytes")),
        "DEB_CHAT" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_chat")),
        "DEB_TPU" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_tpu")),
        "DEB_SPECIAL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-deb_special")),
        "CRED_UPFILE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-cred_upfile")),
        "CRED_UPBYTES" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-cred_upbytes")),
        "CRED_SPECIAL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-cred_special")),
        "SEC_DROP" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-sec_drop")),
        "F_EXP" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-f_exp")),
        "F_MW" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-f_mw")),
        "F_REG" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-f_reg")),
        "F_SEL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-f_sel")),
        "F_SYS" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-f_sys")),
        "FCL" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-fcl")),
        "FNS" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-fns")),
        "NC" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-nc")),
        "GRAPH" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-graph")),
        "SEC" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-sec")),
        "LANG" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-lang")),
        "HDR_ACTIVE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_active")),
        "HDR_BLOCKS" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_blocks")),
        "HDR_DATE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_date")),
        "HDR_ECHO" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_echo")),
        "HDR_FROM" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_from")),
        "HDR_MSGNUM" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_msgnum")),
        "HDR_MSGREF" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_msgref")),
        "HDR_PWD" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_pwd")),
        "HDR_REPLY" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_reply")),
        "HDR_RPLYDATE" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_rplydate")),
        "HDR_RPLYTIME" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_rplytime")),
        "HDR_STATUS" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_status")),
        "HDR_SUBJ" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_subj")),
        "HDR_TIME" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_time")),
        "HDR_TO" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-hdr_to")),
        "O_RD" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-o_rd")),
        "O_RW" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-o_rw")),
        "O_WR" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-o_wr")),
        "SEEK_CUR" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-seek_cur")),
        "SEEK_END" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-seek_end")),
        "SEEK_SET" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-seek_set")),
        "S_DB" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-s_db")),
        "S_DN" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-s_dn")),
        "S_DR" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-s_dr")),
        "S_DW" => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-s_dw")),
        _ => get_sig_hint(c.get_signature(), fl!(crate::LANGUAGE_LOADER, "hint-const-builtin")),
    }
}

pub fn get_type_hover(var_type: VariableType) -> Option<Hover> {
    get_type_hover_for_version(var_type, 100)
}

pub fn get_type_hover_for_version(var_type: VariableType, language_version: u16) -> Option<Hover> {
    match var_type {
        VariableType::Boolean => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-boolean")),
        VariableType::Unsigned => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-unsigned")),
        VariableType::Long => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-long")),
        VariableType::ULong => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-ulong")),
        VariableType::UserData(id) if id == REGEX_ID as u8 => get_sig_hint(Signature::new("REGEX".to_string()), fl!(LANGUAGE_LOADER, "hint-type-regex")),
        VariableType::UserData(id) if id == REGEX_MATCH_ID as u8 => {
            get_sig_hint(Signature::new("REGEXMATCH".to_string()), fl!(LANGUAGE_LOADER, "hint-type-regex-match"))
        }
        VariableType::UserData(id) if id == BOARD_ID as u8 => get_sig_hint(Signature::new("BOARD".to_string()), fl!(LANGUAGE_LOADER, "hint-type-board")),
        VariableType::UserData(id) if id == SESSION_ID as u8 => get_sig_hint(Signature::new("SESSION".to_string()), fl!(LANGUAGE_LOADER, "hint-type-session")),
        VariableType::UserData(id) if id == USER_ID as u8 => get_sig_hint(Signature::new("USER".to_string()), fl!(LANGUAGE_LOADER, "hint-type-user")),
        VariableType::UserData(id) if id == HTTP_ID as u8 => get_sig_hint(Signature::new("HTTP".to_string()), fl!(LANGUAGE_LOADER, "hint-type-http")),
        VariableType::UserData(id) if id == HTTP_REQUEST_ID as u8 => {
            get_sig_hint(Signature::new("HTTPREQUEST".to_string()), fl!(LANGUAGE_LOADER, "hint-type-http-request"))
        }
        VariableType::UserData(id) if id == HTTP_RESPONSE_ID as u8 => {
            get_sig_hint(Signature::new("HTTPRESPONSE".to_string()), fl!(LANGUAGE_LOADER, "hint-type-http-response"))
        }
        VariableType::UserData(id) if id == GFX_ID as u8 => get_sig_hint(Signature::new("GFX".to_string()), fl!(LANGUAGE_LOADER, "hint-type-gfx")),
        VariableType::UserData(id) if id == SURFACE_ID as u8 => get_sig_hint(Signature::new("SURFACE".to_string()), fl!(LANGUAGE_LOADER, "hint-type-surface")),
        VariableType::UserData(id) if id == GFX_BACKEND_ENUM_ID => {
            get_sig_hint(Signature::new("GFXBACKEND".to_string()), fl!(LANGUAGE_LOADER, "hint-type-gfx-backend"))
        }
        VariableType::UserData(id) if id == CHECKSUM_ENUM_ID => {
            get_sig_hint(Signature::new("CHECKSUM".to_string()), fl!(LANGUAGE_LOADER, "hint-type-checksum"))
        }
        VariableType::UserData(id) if id == TERMINAL_ID as u8 => {
            get_sig_hint(Signature::new("TERMINAL".to_string()), fl!(LANGUAGE_LOADER, "hint-type-terminal"))
        }
        VariableType::UserData(id) if id == TERM_INFO_ID as u8 => {
            get_sig_hint(Signature::new("TERMINFO".to_string()), fl!(LANGUAGE_LOADER, "hint-type-terminfo"))
        }
        VariableType::UserData(id) if id == TERM_INPUT_ID as u8 => {
            get_sig_hint(Signature::new("TERMINPUT".to_string()), fl!(LANGUAGE_LOADER, "hint-type-terminput"))
        }
        VariableType::UserData(id) if id == MARGINS_ID as u8 => get_sig_hint(Signature::new("MARGINS".to_string()), fl!(LANGUAGE_LOADER, "hint-type-margins")),
        VariableType::UserData(id) if id == PALETTE_ID as u8 => get_sig_hint(Signature::new("PALETTE".to_string()), fl!(LANGUAGE_LOADER, "hint-type-palette")),
        VariableType::UserData(id) if id == MACROS_ID as u8 => get_sig_hint(Signature::new("MACROS".to_string()), fl!(LANGUAGE_LOADER, "hint-type-macros")),
        VariableType::UserData(id) if id == AUDIO_ID as u8 => get_sig_hint(Signature::new("AUDIO".to_string()), fl!(LANGUAGE_LOADER, "hint-type-audio")),
        VariableType::UserData(id) if id == ERROR_ID as u8 => get_sig_hint(Signature::new("ERROR".to_string()), fl!(LANGUAGE_LOADER, "hint-type-error")),
        VariableType::UserData(id) if id == EVENT_ID as u8 => get_sig_hint(Signature::new("EVENT".to_string()), fl!(LANGUAGE_LOADER, "hint-type-event")),
        VariableType::UserData(id) if id == MSG_ID as u8 => get_sig_hint(Signature::new("MSG".to_string()), fl!(LANGUAGE_LOADER, "hint-type-msg")),
        VariableType::UserData(id) if id == CONFERENCE_ID as u8 => {
            get_sig_hint(Signature::new("CONFERENCE".to_string()), fl!(LANGUAGE_LOADER, "hint-type-conference"))
        }
        VariableType::UserData(id) if id == MESSAGE_AREA_ID as u8 => get_sig_hint(Signature::new("AREA".to_string()), fl!(LANGUAGE_LOADER, "hint-type-area")),
        VariableType::UserData(id) if id == FILE_DIRECTORY_ID as u8 => {
            get_sig_hint(Signature::new("DIRECTORY".to_string()), fl!(LANGUAGE_LOADER, "hint-type-directory"))
        }
        VariableType::UserData(id) if id == DOOR_ID as u8 => get_sig_hint(Signature::new("DOOR".to_string()), fl!(LANGUAGE_LOADER, "hint-type-door")),
        VariableType::UserData(id) if id == CONTACT_ID as u8 => get_sig_hint(Signature::new("CONTACT".to_string()), fl!(LANGUAGE_LOADER, "hint-type-contact")),
        VariableType::UserData(id)
            if matches!(
                id,
                EVENT_KIND_ENUM_ID
                    | MOUSE_ACTION_ENUM_ID
                    | MOUSE_BUTTON_ENUM_ID
                    | MOUSE_MODE_ENUM_ID
                    | MOUSE_TRACKING_ENUM_ID
                    | ERR_KIND_ENUM_ID
                    | ERR_CODE_ENUM_ID
                    | EDITOR_MODE_ENUM_ID
                    | MSG_FIELD_ENUM_ID
                    | HTTP_METHOD_ENUM_ID
                    | REGEX_OPTIONS_ENUM_ID
                    | STRING_COMPARISON_ENUM_ID
            ) =>
        {
            get_sig_hint(Signature::new("ENUM".to_string()), fl!(LANGUAGE_LOADER, "hint-type-enum-400"))
        }
        VariableType::Date => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-date")),
        VariableType::EDate => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-edate")),
        VariableType::Integer => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-integer")),
        VariableType::Money => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-money")),
        VariableType::Float => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-float")),
        VariableType::UnboundedString | VariableType::String if language_version >= 400 => get_sig_hint(
            VariableType::UnboundedString.get_signature(),
            fl!(LANGUAGE_LOADER, "hint-type-string-unbounded"),
        ),
        VariableType::String => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-string")),
        VariableType::Time => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-time")),
        VariableType::Byte => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-byte")),
        VariableType::Word => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-word")),
        VariableType::SByte => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-sbyte")),
        VariableType::SWord => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-sword")),
        VariableType::BigStr => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-bigstr")),
        VariableType::Bytes => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-bytes")),
        VariableType::Double => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-double")),
        VariableType::DDate => get_sig_hint(var_type.get_signature(), fl!(LANGUAGE_LOADER, "hint-type-ddate")),
        _ => None,
    }
}

pub fn get_member_documentation(var_type: VariableType, member: &str) -> Option<String> {
    if var_type == VariableType::Bytes {
        return match member.to_ascii_lowercase().as_str() {
            "len" => Some(fl!(LANGUAGE_LOADER, "hint-bytes-len")),
            "tostring" => Some(fl!(LANGUAGE_LOADER, "hint-bytes-to-string")),
            "tobase64" => Some(fl!(LANGUAGE_LOADER, "hint-bytes-to-base64")),
            "tohex" => Some(fl!(LANGUAGE_LOADER, "hint-bytes-to-hex")),
            "getchecksum" => Some(fl!(LANGUAGE_LOADER, "hint-bytes-get-checksum")),
            "frombase64" => Some(fl!(LANGUAGE_LOADER, "hint-bytes-from-base64")),
            _ => None,
        };
    }
    let VariableType::UserData(id) = var_type else {
        return None;
    };
    if id == BOARD_ID as u8 && member.eq_ignore_ascii_case("Users") {
        return Some(fl!(LANGUAGE_LOADER, "hint-member-board-users"));
    }
    if id == USER_ID as u8 && member.eq_ignore_ascii_case("Valid") {
        return Some(fl!(LANGUAGE_LOADER, "hint-member-user-valid"));
    }
    if id == TERMINAL_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "info" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-info")),
            "gfx" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-gfx")),
            "input" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-input")),
            "margins" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-margins")),
            "palette" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-palette")),
            "macros" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-macros")),
            "beginupdate" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-begin-update")),
            "endupdate" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-end-update")),
            "setfont" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-set-font")),
            "loadfont" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminal-load-font")),
            _ => None,
        };
    }
    if id == TERM_INFO_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "program" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-program")),
            "deviceattrs" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-device-attrs")),
            "columns" | "rows" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-cells")),
            "utf8" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-utf8")),
            "ripversion" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-rip")),
            "ctermlevel" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-cterm")),
            "sixel" | "jxl" | "inlinegraphics" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-graphics")),
            "audio" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-audio")),
            "physicalkeys" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-physical-keys")),
            "pixelmouse" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-pixel-mouse")),
            "clientblit" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-client-blit")),
            "synchronizedoutput" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-synchronized-output")),
            "terminalmacros" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-terminal-macros")),
            "cellwidth" | "cellheight" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-cell-pixels")),
            "screenwidth" | "screenheight" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminfo-screen-pixels")),
            _ => None,
        };
    }
    if id == MARGINS_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "setvertical" => Some(fl!(LANGUAGE_LOADER, "hint-member-margins-set-vertical")),
            "sethorizontal" => Some(fl!(LANGUAGE_LOADER, "hint-member-margins-set-horizontal")),
            "resetvertical" => Some(fl!(LANGUAGE_LOADER, "hint-member-margins-reset-vertical")),
            "resethorizontal" => Some(fl!(LANGUAGE_LOADER, "hint-member-margins-reset-horizontal")),
            "resetall" => Some(fl!(LANGUAGE_LOADER, "hint-member-margins-reset-all")),
            "top" | "bottom" | "left" | "right" => Some(fl!(LANGUAGE_LOADER, "hint-member-margins-edge")),
            "hasvertical" | "hashorizontal" => Some(fl!(LANGUAGE_LOADER, "hint-member-margins-active")),
            _ => None,
        };
    }
    if id == TERM_INPUT_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "poll" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminput-poll")),
            "wait" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminput-wait")),
            "mouseon" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminput-mouse-on")),
            "mouseoff" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminput-mouse-off")),
            "keyboardon" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminput-keyboard-on")),
            "keyboardoff" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminput-keyboard-off")),
            "release" => Some(fl!(LANGUAGE_LOADER, "hint-member-terminput-release")),
            _ => None,
        };
    }
    if id == PALETTE_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "set" => Some(fl!(LANGUAGE_LOADER, "hint-member-palette-set")),
            "reset" => Some(fl!(LANGUAGE_LOADER, "hint-member-palette-reset")),
            "resetall" => Some(fl!(LANGUAGE_LOADER, "hint-member-palette-reset-all")),
            _ => None,
        };
    }
    if id == MACROS_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "recording" => Some(fl!(LANGUAGE_LOADER, "hint-member-macros-recording")),
            "beginrecord" => Some(fl!(LANGUAGE_LOADER, "hint-member-macros-begin-record")),
            "endrecord" => Some(fl!(LANGUAGE_LOADER, "hint-member-macros-end-record")),
            "play" => Some(fl!(LANGUAGE_LOADER, "hint-member-macros-play")),
            "delete" => Some(fl!(LANGUAGE_LOADER, "hint-member-macros-delete")),
            "deleteall" => Some(fl!(LANGUAGE_LOADER, "hint-member-macros-delete-all")),
            _ => None,
        };
    }
    if id == AUDIO_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "valid" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-valid")),
            "playing" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-playing")),
            "setvolume" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-set-volume")),
            "channel" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-channel")),
            "play" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-play")),
            "stop" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-stop")),
            "fade" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-fade")),
            "free" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-free")),
            "load" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-load")),
            "stopall" => Some(fl!(LANGUAGE_LOADER, "hint-member-audio-stop-all")),
            _ => None,
        };
    }
    if id == ERROR_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "ok" => Some(fl!(LANGUAGE_LOADER, "hint-member-error-ok")),
            "kind" => Some(fl!(LANGUAGE_LOADER, "hint-member-error-kind")),
            "code" => Some(fl!(LANGUAGE_LOADER, "hint-member-error-code")),
            "message" => Some(fl!(LANGUAGE_LOADER, "hint-member-error-message")),
            "channel" => Some(fl!(LANGUAGE_LOADER, "hint-member-error-channel")),
            "last" => Some(fl!(LANGUAGE_LOADER, "hint-member-error-last")),
            "clear" => Some(fl!(LANGUAGE_LOADER, "hint-member-error-clear")),
            _ => None,
        };
    }
    if id == EVENT_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "kind" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-kind")),
            "code" | "text" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-key")),
            "scancode" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-scan-code")),
            "pressed" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-pressed")),
            "repeated" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-repeated")),
            "x" | "y" | "pixels" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-position")),
            "button" | "action" | "wheely" | "wheelx" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-mouse")),
            "time" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-time")),
            "channel" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-channel")),
            "dropped" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-dropped")),
            "leftdown" | "middledown" | "rightdown" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-buttons")),
            "shift" | "alt" | "ctrl" | "meta" => Some(fl!(LANGUAGE_LOADER, "hint-member-event-modifiers")),
            _ => None,
        };
    }
    if id == GFX_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "init" => Some(fl!(LANGUAGE_LOADER, "hint-member-gfx-init")),
            "shutdown" => Some(fl!(LANGUAGE_LOADER, "hint-member-gfx-shutdown")),
            "backend" => Some(fl!(LANGUAGE_LOADER, "hint-member-gfx-backend")),
            "setpacing" => Some(fl!(LANGUAGE_LOADER, "hint-member-gfx-set-pacing")),
            _ => None,
        };
    }
    if id == GFX_BACKEND_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "none" => Some(fl!(LANGUAGE_LOADER, "hint-member-gfx-backend-none")),
            "auto" => Some(fl!(LANGUAGE_LOADER, "hint-member-gfx-backend-auto")),
            "sixel" => Some(fl!(LANGUAGE_LOADER, "hint-member-gfx-backend-sixel")),
            "jxl" => Some(fl!(LANGUAGE_LOADER, "hint-member-gfx-backend-jxl")),
            _ => None,
        };
    }
    if id == SURFACE_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "width" | "height" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-dimension")),
            "valid" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-valid")),
            "clear" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-clear")),
            "setpixel" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-set-pixel")),
            "getpixel" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-get-pixel")),
            "fillrect" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-fill-rect")),
            "drawrect" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-draw-rect")),
            "blit" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-blit")),
            "blitrect" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-blit-rect")),
            "present" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-present")),
            "presentat" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-present-at")),
            "presentrect" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-present-rect")),
            "pin" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-pin")),
            "unpin" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-unpin")),
            "free" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-free")),
            "new" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-new")),
            "load" => Some(fl!(LANGUAGE_LOADER, "hint-member-surface-load")),
            _ => None,
        };
    }
    if id == MSG_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "number" | "replyto" | "size" => Some(fl!(LANGUAGE_LOADER, "hint-member-msg-number")),
            "valid" => Some(fl!(LANGUAGE_LOADER, "hint-member-msg-valid")),
            "from" | "to" | "subject" | "status" => Some(fl!(LANGUAGE_LOADER, "hint-member-msg-header")),
            "date" | "time" => Some(fl!(LANGUAGE_LOADER, "hint-member-msg-written")),
            "isprivate" | "isread" | "isdeleted" | "isecho" | "needspassword" => Some(fl!(LANGUAGE_LOADER, "hint-member-msg-flags")),
            "text" => Some(fl!(LANGUAGE_LOADER, "hint-member-msg-text")),
            _ => None,
        };
    }
    if id == CONTACT_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "service" => Some(fl!(LANGUAGE_LOADER, "hint-member-contact-service")),
            "account" => Some(fl!(LANGUAGE_LOADER, "hint-member-contact-account")),
            _ => None,
        };
    }
    if id == CONFERENCE_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "name" | "number" | "valid" => Some(fl!(LANGUAGE_LOADER, "hint-member-conference-identity")),
            "ispublic" | "isreadonly" | "allowaliases" | "echomail" | "autorejoin" | "privateuploads" => {
                Some(fl!(LANGUAGE_LOADER, "hint-member-conference-options"))
            }
            "password" => Some(fl!(LANGUAGE_LOADER, "hint-member-conference-password")),
            "directories" | "areas" | "doors" => Some(fl!(LANGUAGE_LOADER, "hint-member-conference-collections")),
            "hasaccess" | "canpost" | "canattach" => Some(fl!(LANGUAGE_LOADER, "hint-member-conference-access")),
            _ => None,
        };
    }
    if id == MESSAGE_AREA_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "name" | "number" | "valid" => Some(fl!(LANGUAGE_LOADER, "hint-member-area-identity")),
            "isreadonly" | "allowaliases" | "qwkname" | "echotag" => Some(fl!(LANGUAGE_LOADER, "hint-member-area-options")),
            "hasaccess" | "canenter" | "canattach" => Some(fl!(LANGUAGE_LOADER, "hint-member-area-access")),
            "highmsg" | "lowmsg" => Some(fl!(LANGUAGE_LOADER, "hint-member-area-range")),
            "read" => Some(fl!(LANGUAGE_LOADER, "hint-member-area-read")),
            "find" => Some(fl!(LANGUAGE_LOADER, "hint-member-area-find")),
            _ => None,
        };
    }
    if id == FILE_DIRECTORY_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "name" | "number" | "valid" => Some(fl!(LANGUAGE_LOADER, "hint-member-directory-identity")),
            "path" | "isfree" | "hasnewfiles" | "password" => Some(fl!(LANGUAGE_LOADER, "hint-member-directory-options")),
            "hasaccess" | "candownload" => Some(fl!(LANGUAGE_LOADER, "hint-member-directory-access")),
            _ => None,
        };
    }
    if id == DOOR_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "name" | "number" | "valid" => Some(fl!(LANGUAGE_LOADER, "hint-member-door-identity")),
            "description" | "path" | "password" => Some(fl!(LANGUAGE_LOADER, "hint-member-door-options")),
            "hasaccess" => Some(fl!(LANGUAGE_LOADER, "hint-member-door-access")),
            _ => None,
        };
    }
    if id == BOARD_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "name" | "location" | "operator" | "sysopname" | "nodecount" => Some(fl!(LANGUAGE_LOADER, "hint-member-board-property")),
            "conferences" => Some(fl!(LANGUAGE_LOADER, "hint-member-board-conferences")),
            "users" => Some(fl!(LANGUAGE_LOADER, "hint-member-board-users")),
            _ => None,
        };
    }
    if id == SESSION_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "conference" | "area" | "directory" | "user" => Some(fl!(LANGUAGE_LOADER, "hint-member-session-context")),
            "username" | "aliasname" | "securitylevel" | "node" | "minutesleft" | "pagelength" | "language" | "islocal" | "issysop" => {
                Some(fl!(LANGUAGE_LOADER, "hint-member-session-value"))
            }
            _ => None,
        };
    }
    if id == USER_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "valid" => Some(fl!(LANGUAGE_LOADER, "hint-member-user-valid")),
            "recordnumber" => Some(fl!(LANGUAGE_LOADER, "hint-member-user-record-number")),
            "contacts" => Some(fl!(LANGUAGE_LOADER, "hint-member-user-contacts")),
            "notes" => Some(fl!(LANGUAGE_LOADER, "hint-member-user-notes")),
            "setpassword" => Some(fl!(LANGUAGE_LOADER, "hint-member-user-set-password")),
            "addcontact" | "removecontact" => Some(fl!(LANGUAGE_LOADER, "hint-member-user-contact-method")),
            "setnote" => Some(fl!(LANGUAGE_LOADER, "hint-member-user-set-note")),
            "editormode" => Some(fl!(LANGUAGE_LOADER, "hint-member-user-editor-mode")),
            _ => Some(fl!(LANGUAGE_LOADER, "hint-member-user-profile")),
        };
    }
    if id == EVENT_KIND_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "none" => Some(fl!(LANGUAGE_LOADER, "hint-enum-event-kind-none")),
            "key" | "keyedge" | "mouse" | "overflow" | "audio" => Some(fl!(LANGUAGE_LOADER, "hint-enum-event-kind-value")),
            _ => None,
        };
    }
    if id == MOUSE_ACTION_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "none" | "press" | "release" | "motion" | "wheel" => Some(fl!(LANGUAGE_LOADER, "hint-enum-mouse-action")),
            _ => None,
        };
    }
    if id == MOUSE_BUTTON_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "none" | "left" | "middle" | "right" | "wheelup" | "wheeldown" | "wheelleft" | "wheelright" => Some(fl!(LANGUAGE_LOADER, "hint-enum-mouse-button")),
            _ => None,
        };
    }
    if id == MOUSE_MODE_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "text" => Some(fl!(LANGUAGE_LOADER, "hint-enum-mouse-mode-text")),
            "pixels" => Some(fl!(LANGUAGE_LOADER, "hint-enum-mouse-mode-pixels")),
            _ => None,
        };
    }
    if id == MOUSE_TRACKING_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "buttons" | "drag" | "all" => Some(fl!(LANGUAGE_LOADER, "hint-enum-mouse-tracking")),
            _ => None,
        };
    }
    if id == ERR_KIND_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "none" | "file" | "dbase" | "stack" | "gfx" | "font" | "audio" | "term" | "msg" | "net" | "user" | "string" | "regex" => {
                Some(fl!(LANGUAGE_LOADER, "hint-enum-error-kind"))
            }
            _ => None,
        };
    }
    if id == ERR_CODE_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "ok" | "unavailable" | "invalid" | "io" | "format" | "limit" | "unsupported" | "stack" | "denied" | "timeout" => {
                Some(fl!(LANGUAGE_LOADER, "hint-enum-error-code"))
            }
            _ => None,
        };
    }
    if id == EDITOR_MODE_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "yes" | "no" | "ask" => Some(fl!(LANGUAGE_LOADER, "hint-enum-editor-mode")),
            _ => None,
        };
    }
    if id == MSG_FIELD_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "to" | "from" | "subject" => Some(fl!(LANGUAGE_LOADER, "hint-enum-msg-field")),
            _ => None,
        };
    }
    if id == HTTP_METHOD_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "get" | "head" | "post" | "put" | "delete" | "patch" => Some(fl!(LANGUAGE_LOADER, "hint-enum-http-method")),
            _ => None,
        };
    }
    if id == REGEX_OPTIONS_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "none" | "ignorecase" | "multiline" | "dotmatchesnewline" | "ignorewhitespace" | "swapgreed" | "ascii" => {
                Some(fl!(LANGUAGE_LOADER, "hint-enum-regex-options"))
            }
            _ => None,
        };
    }
    if id == STRING_COMPARISON_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "ordinal" | "ordinalignorecase" => Some(fl!(LANGUAGE_LOADER, "hint-enum-string-comparison")),
            _ => None,
        };
    }
    if id == CHECKSUM_ENUM_ID {
        return match member.to_ascii_lowercase().as_str() {
            "crc32" | "md5" | "sha256" => Some(fl!(LANGUAGE_LOADER, "hint-enum-checksum")),
            _ => None,
        };
    }
    if id == HTTP_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "get" => Some(fl!(LANGUAGE_LOADER, "hint-http-get")),
            "new" => Some(fl!(LANGUAGE_LOADER, "hint-http-new")),
            "download" => Some(fl!(LANGUAGE_LOADER, "hint-http-download")),
            "urlencode" => Some(fl!(LANGUAGE_LOADER, "hint-http-url-encode")),
            "urldecode" => Some(fl!(LANGUAGE_LOADER, "hint-http-url-decode")),
            _ => None,
        };
    }
    if id == HTTP_REQUEST_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "url" | "method" => Some(fl!(LANGUAGE_LOADER, "hint-http-request-property")),
            "setheader" => Some(fl!(LANGUAGE_LOADER, "hint-http-request-set-header")),
            "settext" => Some(fl!(LANGUAGE_LOADER, "hint-http-request-set-text")),
            "setform" => Some(fl!(LANGUAGE_LOADER, "hint-http-request-set-form")),
            "send" => Some(fl!(LANGUAGE_LOADER, "hint-http-request-send")),
            _ => None,
        };
    }
    if id == HTTP_RESPONSE_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "valid" | "ok" | "status" | "finalurl" | "size" | "contenttype" => Some(fl!(LANGUAGE_LOADER, "hint-http-response-property")),
            "text" => Some(fl!(LANGUAGE_LOADER, "hint-http-response-text")),
            "bytes" => Some(fl!(LANGUAGE_LOADER, "hint-http-response-bytes")),
            "header" => Some(fl!(LANGUAGE_LOADER, "hint-http-response-header")),
            "save" => Some(fl!(LANGUAGE_LOADER, "hint-http-response-save")),
            _ => None,
        };
    }
    if id == REGEX_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "valid" => Some(fl!(LANGUAGE_LOADER, "hint-regex-valid")),
            "pattern" => Some(fl!(LANGUAGE_LOADER, "hint-regex-pattern")),
            "compile" => Some(fl!(LANGUAGE_LOADER, "hint-regex-compile")),
            "escape" => Some(fl!(LANGUAGE_LOADER, "hint-regex-escape")),
            "isvalid" => Some(fl!(LANGUAGE_LOADER, "hint-regex-is-valid")),
            "ismatch" => Some(fl!(LANGUAGE_LOADER, "hint-regex-is-match")),
            "find" => Some(fl!(LANGUAGE_LOADER, "hint-regex-find")),
            "findall" => Some(fl!(LANGUAGE_LOADER, "hint-regex-find-all")),
            "replace" => Some(fl!(LANGUAGE_LOADER, "hint-regex-replace")),
            "split" => Some(fl!(LANGUAGE_LOADER, "hint-regex-split")),
            _ => None,
        };
    }
    if id == REGEX_MATCH_ID as u8 {
        return match member.to_ascii_lowercase().as_str() {
            "success" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-success")),
            "value" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-value")),
            "start" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-start")),
            "length" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-length")),
            "groupcount" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-group-count")),
            "group" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-group")),
            "namedgroup" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-named-group")),
            "groupmatched" | "namedgroupmatched" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-group-matched")),
            "groupstart" | "namedgroupstart" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-group-start")),
            "grouplength" | "namedgrouplength" => Some(fl!(LANGUAGE_LOADER, "hint-regex-match-group-length")),
            _ => None,
        };
    }
    None
}

/// A short, localized explanation suitable for signature help and generated
/// parameter lists. Parameter names are deliberately shared by the registry,
/// hover and completion so the three views cannot drift apart.
pub fn get_parameter_documentation(name: &str) -> Option<String> {
    let key = match name.to_ascii_lowercase().as_str() {
        "backend" => "hint-param-backend",
        "fullscreen" => "hint-param-fullscreen",
        "enabled" => "hint-param-enabled",
        "top" => "hint-param-top",
        "bottom" => "hint-param-bottom",
        "left" => "hint-param-left",
        "right" => "hint-param-right",
        "timeoutms" => "hint-param-timeout-ms",
        "mode" => "hint-param-mode",
        "tracking" => "hint-param-tracking",
        "echo" => "hint-param-echo",
        "color" => "hint-param-color",
        "rgba" => "hint-param-rgba",
        "slot" => "hint-param-slot",
        "looping" => "hint-param-looping",
        "durationms" => "hint-param-duration-ms",
        "targetvolume" => "hint-param-target-volume",
        "volume" => "hint-param-volume",
        "font" => "hint-param-font",
        "file" => "hint-param-file",
        "password" => "hint-param-password",
        "service" => "hint-param-service",
        "account" => "hint-param-account",
        "index" => "hint-param-index",
        "text" => "hint-param-text",
        "url" => "hint-param-url",
        "method" => "hint-param-method",
        "name" => "hint-param-name",
        "value" => "hint-param-value",
        "contenttype" => "hint-param-content-type",
        "form" => "hint-param-form",
        "pattern" => "hint-param-pattern",
        "options" => "hint-param-options",
        "start" => "hint-param-start",
        "limit" => "hint-param-limit",
        "replacement" => "hint-param-replacement",
        "messagenumber" => "hint-param-message-number",
        "field" => "hint-param-field",
        "startmessage" => "hint-param-start-message",
        "x" => "hint-param-x",
        "y" => "hint-param-y",
        "width" => "hint-param-width",
        "height" => "hint-param-height",
        "source" => "hint-param-source",
        "sourcex" => "hint-param-source-x",
        "sourcey" => "hint-param-source-y",
        "sourcewidth" => "hint-param-source-width",
        "sourceheight" => "hint-param-source-height",
        "destinationx" => "hint-param-destination-x",
        "destinationy" => "hint-param-destination-y",
        "column" => "hint-param-column",
        "row" => "hint-param-row",
        "destinationwidth" => "hint-param-destination-width",
        "destinationheight" => "hint-param-destination-height",
        "flip" => "hint-param-flip",
        _ => return None,
    };
    Some(LANGUAGE_LOADER.get(key))
}

pub fn parameters_title() -> String {
    fl!(LANGUAGE_LOADER, "hint-parameters-title")
}

pub fn optional_parameter_label() -> String {
    fl!(LANGUAGE_LOADER, "hint-param-optional")
}

pub fn get_member_documentation_with_parameters(registry: &UserTypeRegistry, receiver: VariableType, member: &unicase::Ascii<String>) -> Option<String> {
    let mut documentation = get_member_documentation(receiver, member.as_ref()).unwrap_or_default();
    let VariableType::UserData(id) = receiver else {
        return (!documentation.is_empty()).then_some(documentation);
    };
    let Some(object) = registry.get_type_from_id(id) else {
        return (!documentation.is_empty()).then_some(documentation);
    };
    let (parameters, names, required) = if let Some(function) = object.functions.get(member) {
        (&function.parameters, &function.parameter_names, function.required)
    } else if let Some(procedure) = object.procedures.get(member) {
        (&procedure.parameters, &procedure.parameter_names, procedure.required)
    } else {
        return (!documentation.is_empty()).then_some(documentation);
    };
    if parameters.is_empty() || names.len() != parameters.len() {
        return (!documentation.is_empty()).then_some(documentation);
    }
    let _ = write!(documentation, "\n\n**{}**\n", parameters_title());
    for (index, (parameter, name)) in parameters.iter().zip(names).enumerate() {
        let optional = if index < required {
            String::new()
        } else {
            format!(", {}", optional_parameter_label())
        };
        let description = get_parameter_documentation(name).unwrap_or_else(|| format!("Value for `{name}`."));
        let _ = write!(
            documentation,
            "\n- `{name}` (`{}{optional}`) — {description}",
            crate::type_lookup::type_name(registry, *parameter)
        );
    }
    Some(documentation)
}

pub fn get_string_member_documentation(member: &str) -> Option<String> {
    match member.to_ascii_lowercase().as_str() {
        "len" => Some(fl!(LANGUAGE_LOADER, "hint-string-len")),
        "find" => Some(fl!(LANGUAGE_LOADER, "hint-string-find")),
        "findlast" => Some(fl!(LANGUAGE_LOADER, "hint-string-find-last")),
        "contains" => Some(fl!(LANGUAGE_LOADER, "hint-string-contains")),
        "startswith" => Some(fl!(LANGUAGE_LOADER, "hint-string-starts-with")),
        "endswith" => Some(fl!(LANGUAGE_LOADER, "hint-string-ends-with")),
        "count" => Some(fl!(LANGUAGE_LOADER, "hint-string-count")),
        "equals" => Some(fl!(LANGUAGE_LOADER, "hint-string-equals")),
        "replace" => Some(fl!(LANGUAGE_LOADER, "hint-string-replace")),
        "trim" => Some(fl!(LANGUAGE_LOADER, "hint-string-trim")),
        "trimstart" => Some(fl!(LANGUAGE_LOADER, "hint-string-trim-start")),
        "trimend" => Some(fl!(LANGUAGE_LOADER, "hint-string-trim-end")),
        "toupper" => Some(fl!(LANGUAGE_LOADER, "hint-string-to-upper")),
        "tolower" => Some(fl!(LANGUAGE_LOADER, "hint-string-to-lower")),
        "split" => Some(fl!(LANGUAGE_LOADER, "hint-string-split")),
        "join" => Some(fl!(LANGUAGE_LOADER, "hint-string-join")),
        "repeat" => Some(fl!(LANGUAGE_LOADER, "hint-string-repeat")),
        _ => None,
    }
}

fn format_signature(sig: Signature, arg: String) -> MarkupContent {
    // A language-tagged fence lets VS Code tokenize the preview with the PPL
    // grammar. Inline code renders every part in one neutral color.
    let mut value = format!("```PPL\n{}\n```\n\n{arg}", sig.signature);

    for i in 0..sig.args.len() {
        value = value.replace(&format!("@{}", i + 1), &format!("`{}`", sig.args[i]))
    }
    MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    }
}

fn get_sig_hint(sig: Signature, arg: String) -> Option<Hover> {
    Some(Hover {
        contents: HoverContents::Markup(format_signature(sig, arg)),
        range: None,
    })
}

/// Documentation shared by preprocessor-directive hover and completion.
pub fn get_preprocessor_hover(directive: &str) -> Option<Hover> {
    let directive = directive.trim_start_matches(';').trim_start_matches(['$', '#']).to_ascii_uppercase();
    let (signature, documentation) = match directive.as_str() {
        "LANGVERSION" => (";$LANGVERSION number", fl!(LANGUAGE_LOADER, "hint-preprocessor-langversion")),
        "DEFINE" => (";$DEFINE name[=value]", fl!(LANGUAGE_LOADER, "hint-preprocessor-define")),
        "IF" => (";$IF expression", fl!(LANGUAGE_LOADER, "hint-preprocessor-if")),
        "ELSEIF" => (";$ELSEIF expression", fl!(LANGUAGE_LOADER, "hint-preprocessor-elseif")),
        "ELIF" => (";$ELIF expression", fl!(LANGUAGE_LOADER, "hint-preprocessor-elif")),
        "ELSE" => (";$ELSE", fl!(LANGUAGE_LOADER, "hint-preprocessor-else")),
        "ENDIF" => (";$ENDIF", fl!(LANGUAGE_LOADER, "hint-preprocessor-endif")),
        "USEFUNCS" => (";$USEFUNCS", fl!(LANGUAGE_LOADER, "hint-preprocessor-usefuncs")),
        "SUBSTITUTION" => (";#name", fl!(LANGUAGE_LOADER, "hint-preprocessor-substitution")),
        _ => return None,
    };
    get_sig_hint(Signature::new(signature.to_string()), documentation)
}

/// Documentation shared by reserved-word hover and completion.
pub fn get_keyword_hover(keyword: &str) -> Option<Hover> {
    let keyword = keyword.to_ascii_lowercase();
    let (name, mut documentation) = match keyword.as_str() {
        "if" => ("IF", fl!(LANGUAGE_LOADER, "hint-keyword-if")),
        "let" => ("LET", fl!(LANGUAGE_LOADER, "hint-keyword-let")),
        "while" => ("WHILE", fl!(LANGUAGE_LOADER, "hint-keyword-while")),
        "endwhile" => ("ENDWHILE", fl!(LANGUAGE_LOADER, "hint-keyword-endwhile")),
        "else" => ("ELSE", fl!(LANGUAGE_LOADER, "hint-keyword-else")),
        "elseif" => ("ELSEIF", fl!(LANGUAGE_LOADER, "hint-keyword-elseif")),
        "endif" => ("ENDIF", fl!(LANGUAGE_LOADER, "hint-keyword-endif")),
        "for" => ("FOR", fl!(LANGUAGE_LOADER, "hint-keyword-for")),
        "next" => ("NEXT", fl!(LANGUAGE_LOADER, "hint-keyword-next")),
        "endfor" => ("ENDFOR", fl!(LANGUAGE_LOADER, "hint-keyword-endfor")),
        "break" => ("BREAK", fl!(LANGUAGE_LOADER, "hint-keyword-break")),
        "continue" => ("CONTINUE", fl!(LANGUAGE_LOADER, "hint-keyword-continue")),
        "return" => ("RETURN", fl!(LANGUAGE_LOADER, "hint-keyword-return")),
        "gosub" => ("GOSUB", fl!(LANGUAGE_LOADER, "hint-keyword-gosub")),
        "goto" => ("GOTO", fl!(LANGUAGE_LOADER, "hint-keyword-goto")),
        "select" => ("SELECT", fl!(LANGUAGE_LOADER, "hint-keyword-select")),
        "case" => ("CASE", fl!(LANGUAGE_LOADER, "hint-keyword-case")),
        "default" => ("DEFAULT", fl!(LANGUAGE_LOADER, "hint-keyword-default")),
        "endselect" => ("ENDSELECT", fl!(LANGUAGE_LOADER, "hint-keyword-endselect")),
        "declare" => ("DECLARE", fl!(LANGUAGE_LOADER, "hint-keyword-declare")),
        "function" => ("FUNCTION", fl!(LANGUAGE_LOADER, "hint-keyword-function")),
        "procedure" => ("PROCEDURE", fl!(LANGUAGE_LOADER, "hint-keyword-procedure")),
        "endproc" => ("ENDPROC", fl!(LANGUAGE_LOADER, "hint-keyword-endproc")),
        "endfunc" => ("ENDFUNC", fl!(LANGUAGE_LOADER, "hint-keyword-endfunc")),
        "repeat" => ("REPEAT", fl!(LANGUAGE_LOADER, "hint-keyword-repeat")),
        "until" => ("UNTIL", fl!(LANGUAGE_LOADER, "hint-keyword-until")),
        "loop" => ("LOOP", fl!(LANGUAGE_LOADER, "hint-keyword-loop")),
        "endloop" => ("ENDLOOP", fl!(LANGUAGE_LOADER, "hint-keyword-endloop")),
        "const" => ("CONST", fl!(LANGUAGE_LOADER, "hint-keyword-const")),
        "enum" => ("ENUM", fl!(LANGUAGE_LOADER, "hint-keyword-enum")),
        "endenum" => ("ENDENUM", fl!(LANGUAGE_LOADER, "hint-keyword-endenum")),
        "type" => ("TYPE", fl!(LANGUAGE_LOADER, "hint-keyword-type")),
        "endtype" => ("ENDTYPE", fl!(LANGUAGE_LOADER, "hint-keyword-endtype")),
        "begin" => ("BEGIN", fl!(LANGUAGE_LOADER, "hint-keyword-begin")),
        "onerror" => ("ONERROR", fl!(LANGUAGE_LOADER, "hint-keyword-onerror")),
        "foreach" => ("FOREACH", fl!(LANGUAGE_LOADER, "hint-keyword-foreach")),
        "endforeach" => ("ENDFOREACH", fl!(LANGUAGE_LOADER, "hint-keyword-endforeach")),
        "exit" => ("EXIT", fl!(LANGUAGE_LOADER, "hint-keyword-exit")),
        _ => return None,
    };
    let usage = keyword_usage(&keyword)?;
    documentation.push_str(&format!("\n\n**{}**\n\n```PPL\n{usage}\n```", fl!(LANGUAGE_LOADER, "hint-keyword-usage")));
    get_sig_hint(Signature::new(name.to_string()), documentation)
}

fn keyword_usage(keyword: &str) -> Option<&'static str> {
    match keyword {
        "if" | "elseif" | "else" | "endif" => {
            Some("IF condition THEN\n    statements\nELSEIF otherCondition THEN\n    statements\nELSE\n    statements\nENDIF\n\nIF condition statement")
        }
        "let" => Some("LET variable = expression\nvariable = expression"),
        "while" | "endwhile" => Some("WHILE condition DO\n    statements\nENDWHILE\n\nWHILE condition statement"),
        "for" | "next" | "endfor" => Some("FOR variable = start TO stop [STEP increment]\n    statements\nNEXT [variable]"),
        "break" => Some("BREAK"),
        "continue" => Some("CONTINUE"),
        "return" => Some("RETURN [expression]"),
        "gosub" => Some("GOSUB label"),
        "goto" => Some("GOTO label"),
        "select" | "case" | "default" | "endselect" => {
            Some("SELECT CASE expression\n    CASE value1, value2, first..last\n        statements\n    DEFAULT\n        statements\nENDSELECT")
        }
        "declare" => Some("DECLARE FUNCTION name(TYPE parameter, ...) TYPE\nDECLARE PROCEDURE name(TYPE parameter, ...)"),
        "function" | "endfunc" => Some("FUNCTION name(TYPE parameter, ...) TYPE\n    statements\n    RETURN expression\nENDFUNC"),
        "procedure" | "endproc" => Some("PROCEDURE name(TYPE parameter, VAR TYPE output, ...)\n    statements\nENDPROC"),
        "repeat" | "until" => Some("REPEAT\n    statements\nUNTIL condition"),
        "loop" | "endloop" => Some("LOOP\n    statements\n    IF condition BREAK\nENDLOOP"),
        "const" => Some("CONST TYPE name = constantExpression"),
        "enum" | "endenum" => Some("ENUM Name\n    First\n    Second = 5\n    Third\nENDENUM"),
        "type" | "endtype" => Some("TYPE Name\n    TYPE field\n    TYPE values(bound)\nENDTYPE"),
        "begin" => Some("BEGIN\n    statements\nEND"),
        "onerror" => Some("ON ERROR GOTO label\nON ERROR GOSUB label\nON ERROR Handler\nON ERROR OFF"),
        "foreach" | "endforeach" => Some("FOREACH value IN collection\n    statements\nENDFOREACH"),
        "exit" => Some("EXIT"),
        _ => None,
    }
}

pub fn get_function_hover(func: &FunctionDefinition) -> Option<Hover> {
    let sig = func.get_signature();
    match func.opcode {
        FuncOpCode::LEN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-len")),
        FuncOpCode::LOWER => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-lower")),
        FuncOpCode::UPPER => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-upper")),
        FuncOpCode::MID => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mid")),
        FuncOpCode::LEFT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-left")),
        FuncOpCode::RIGHT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-right")),
        FuncOpCode::SPACE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-space")),
        FuncOpCode::FERR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ferr")),
        FuncOpCode::CHR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-chr")),
        FuncOpCode::ASC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-asc")),
        FuncOpCode::INSTR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-instr")),
        FuncOpCode::ABORT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-abort")),
        FuncOpCode::LTRIM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ltrim")),
        FuncOpCode::RTRIM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-rtrim")),
        FuncOpCode::TRIM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-trim")),
        FuncOpCode::RANDOM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-random")),
        FuncOpCode::DATE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-date")),
        FuncOpCode::TIME => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-time")),
        FuncOpCode::U_NAME => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_name")),
        FuncOpCode::U_LDATE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_ldate")),
        FuncOpCode::U_LTIME => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_ltime")),
        FuncOpCode::U_LDIR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_ldir")),
        FuncOpCode::U_LOGONS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_logons")),
        FuncOpCode::U_FUL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_ful")),
        FuncOpCode::U_FDL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_fdl")),
        FuncOpCode::U_BDLDAY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_bdlday")),
        FuncOpCode::U_TIMEON => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_timeon")),
        FuncOpCode::U_BDL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_bdl")),
        FuncOpCode::U_BUL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_bul")),
        FuncOpCode::YEAR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-year")),
        FuncOpCode::MONTH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-month")),
        FuncOpCode::DAY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-day")),
        FuncOpCode::DOW => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dow")),
        FuncOpCode::HOUR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-hour")),
        FuncOpCode::MIN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-min")),
        FuncOpCode::SEC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-sec")),
        FuncOpCode::TIMEAP => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-timeap")),
        FuncOpCode::VER => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ver")),
        FuncOpCode::NOCHAR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-nochar")),
        FuncOpCode::YESCHAR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-yeschar")),
        FuncOpCode::STRIPATX => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-stripatx")),
        FuncOpCode::REPLACE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-replace")),
        FuncOpCode::STRIP => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-strip")),
        FuncOpCode::INKEY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-inkey")),
        FuncOpCode::TOSTRING => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tostring")),
        FuncOpCode::MASK_PWD => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mask_pwd")),
        FuncOpCode::MASK_ALPHA => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mask_alpha")),
        FuncOpCode::MASK_NUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mask_num")),
        FuncOpCode::MASK_ALNUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mask_alnum")),
        FuncOpCode::MASK_FILE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mask_file")),
        FuncOpCode::MASK_PATH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mask_path")),
        FuncOpCode::MASK_ASCII => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mask_ascii")),
        FuncOpCode::CURCONF => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-curconf")),
        FuncOpCode::PCBDAT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-pcbdat")),
        FuncOpCode::PPEPATH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ppepath")),
        FuncOpCode::VALDATE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-valdate")),
        FuncOpCode::VALTIME => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-valtime")),
        FuncOpCode::U_MSGRD => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_msgrd")),
        FuncOpCode::U_MSGWR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_msgwr")),
        FuncOpCode::PCBNODE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-pcbnode")),
        FuncOpCode::READLINE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-readline")),
        FuncOpCode::SYSOPSEC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-sysopsec")),
        FuncOpCode::ONLOCAL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-onlocal")),
        FuncOpCode::UN_STAT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-un_stat")),
        FuncOpCode::UN_NAME => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-un_name")),
        FuncOpCode::UN_CITY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-un_city")),
        FuncOpCode::UN_OPER => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-un_oper")),
        FuncOpCode::CURSEC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-cursec")),
        FuncOpCode::GETTOKEN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-gettoken")),
        FuncOpCode::MINLEFT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-minleft")),
        FuncOpCode::MINON => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-minon")),
        FuncOpCode::GETENV => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-getenv")),
        FuncOpCode::CALLID => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-callid")),
        FuncOpCode::REGAL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regal")),
        FuncOpCode::REGAH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regah")),
        FuncOpCode::REGBL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regbl")),
        FuncOpCode::REGBH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regbh")),
        FuncOpCode::REGCL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regcl")),
        FuncOpCode::REGCH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regch")),
        FuncOpCode::REGDL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regdl")),
        FuncOpCode::REGDH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regdh")),
        FuncOpCode::REGAX => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regax")),
        FuncOpCode::REGBX => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regbx")),
        FuncOpCode::REGCX => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regcx")),
        FuncOpCode::REGDX => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regdx")),
        FuncOpCode::REGSI => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regsi")),
        FuncOpCode::REGDI => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regdi")),
        FuncOpCode::REGF => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regf")),
        FuncOpCode::REGCF => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regcf")),
        FuncOpCode::REGDS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-regds")),
        FuncOpCode::REGES => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-reges")),
        FuncOpCode::B2W => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-b2w")),
        FuncOpCode::PEEKB => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-peekb")),
        FuncOpCode::PEEKW => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-peekw")),
        FuncOpCode::MKADDR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mkaddr")),
        FuncOpCode::EXIST => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-exist")),
        FuncOpCode::I2S => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-i2s")),
        FuncOpCode::S2I => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-s2i")),
        FuncOpCode::CARRIER => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-carrier")),
        FuncOpCode::TOKENSTR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tokenstr")),
        FuncOpCode::CDON => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-cdon")),
        FuncOpCode::LANGEXT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-langext")),
        FuncOpCode::ANSION => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ansion")),
        FuncOpCode::VALCC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-valcc")),
        FuncOpCode::FMTCC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-fmtcc")),
        FuncOpCode::CCTYPE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-cctype")),
        FuncOpCode::GETX => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-getx")),
        FuncOpCode::GETY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-gety")),
        FuncOpCode::BAND => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-band")),
        FuncOpCode::BOR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-bor")),
        FuncOpCode::BXOR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-bxor")),
        FuncOpCode::BNOT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-bnot")),
        FuncOpCode::U_PWDHIST => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_pwdhist")),
        FuncOpCode::U_PWDLC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_pwdlc")),
        FuncOpCode::U_PWDTC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_pwdtc")),
        FuncOpCode::U_STAT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_stat")),
        FuncOpCode::DEFCOLOR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-defcolor")),
        FuncOpCode::ABS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-abs")),
        FuncOpCode::SIN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-sin")),
        FuncOpCode::COS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-cos")),
        FuncOpCode::TAN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tan")),
        FuncOpCode::ATAN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-atan")),
        FuncOpCode::LOG => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-log")),
        FuncOpCode::SQRT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-sqrt")),
        FuncOpCode::GRAFMODE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-grafmode")),
        FuncOpCode::PSA => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-psa")),
        FuncOpCode::FILEINF => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-fileinf")),
        FuncOpCode::PPENAME => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ppename")),
        FuncOpCode::MKDATE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mkdate")),
        FuncOpCode::CURCOLOR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-curcolor")),
        FuncOpCode::KINKEY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-kinkey")),
        FuncOpCode::MINKEY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-minkey")),
        FuncOpCode::MAXNODE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-maxnode")),
        FuncOpCode::SLPATH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-slpath")),
        FuncOpCode::HELPPATH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-helppath")),
        FuncOpCode::TEMPPATH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-temppath")),
        FuncOpCode::MODEM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-modem")),
        FuncOpCode::LOGGEDON => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-loggedon")),
        FuncOpCode::CALLNUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-callnum")),
        FuncOpCode::MGETBYTE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mgetbyte")),
        FuncOpCode::TOKCOUNT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tokcount")),
        FuncOpCode::U_RECNUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_recnum")),
        FuncOpCode::U_INCONF => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_inconf")),
        FuncOpCode::PEEKDW => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-peekdw")),
        FuncOpCode::DBGLEVEL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dbglevel")),
        FuncOpCode::SCRTEXT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-scrtext")),
        FuncOpCode::SHOWSTAT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-showstat")),
        FuncOpCode::PAGESTAT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-pagestat")),
        FuncOpCode::REPLACESTR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-replacestr")),
        FuncOpCode::STRIPSTR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-stripstr")),
        FuncOpCode::TOBIGSTR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tobigstr")),
        FuncOpCode::TOBOOLEAN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-toboolean")),
        FuncOpCode::TOBYTE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tobyte")),
        FuncOpCode::TODATE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-todate")),
        FuncOpCode::TODREAL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-todreal")),
        FuncOpCode::TOEDATE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-toedate")),
        FuncOpCode::TOINTEGER => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tointeger")),
        FuncOpCode::TOLONG64 => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tolong")),
        FuncOpCode::TOULONG64 => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-toulong")),
        FuncOpCode::TOMONEY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tomoney")),
        FuncOpCode::TOREAL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-toreal")),
        FuncOpCode::TOSBYTE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tosbyte")),
        FuncOpCode::TOSWORD => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tosword")),
        FuncOpCode::TOTIME => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-totime")),
        FuncOpCode::TOUNSIGNED => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tounsigned")),
        FuncOpCode::TOWORD => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-toword")),
        FuncOpCode::MIXED => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-mixed")),
        FuncOpCode::ALIAS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-alias")),
        FuncOpCode::CONFREG => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-confreg")),
        FuncOpCode::CONFEXP => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-confexp")),
        FuncOpCode::CONFSEL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-confsel")),
        FuncOpCode::CONFSYS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-confsys")),
        FuncOpCode::CONFMW => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-confmw")),
        FuncOpCode::LPRINTED => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-lprinted")),
        FuncOpCode::ISNONSTOP => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-isnonstop")),
        FuncOpCode::ERRCORRECT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-errcorrect")),
        FuncOpCode::CONFALIAS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-confalias")),
        FuncOpCode::USERALIAS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-useralias")),
        FuncOpCode::CURUSER => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-curuser")),
        FuncOpCode::U_LMR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-u_lmr")),
        FuncOpCode::CHATSTAT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-chatstat")),
        FuncOpCode::DEFANS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-defans")),
        FuncOpCode::LASTANS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-lastans")),
        FuncOpCode::MEGANUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-meganum")),
        FuncOpCode::EVTTIMEADJ => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-evttimeadj")),
        FuncOpCode::ISBITSET => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-isbitset")),
        FuncOpCode::FMTREAL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-fmtreal")),
        FuncOpCode::FLAGCNT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-flagcnt")),
        FuncOpCode::KBDBUFSIZE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-kbdbufsize")),
        FuncOpCode::PPLBUFSIZE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-pplbufsize")),
        FuncOpCode::KBDFILUSED => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-kbdfilused")),
        FuncOpCode::LOMSGNUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-lomsgnum")),
        FuncOpCode::HIMSGNUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-himsgnum")),
        FuncOpCode::DRIVESPACE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-drivespace")),
        FuncOpCode::OUTBYTES => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-outbytes")),
        FuncOpCode::HICONFNUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-hiconfnum")),
        FuncOpCode::INBYTES => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-inbytes")),
        FuncOpCode::CRC32 => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-crc32")),
        FuncOpCode::PCBMAC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-pcbmac")),
        FuncOpCode::ACTMSGNUM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-actmsgnum")),
        FuncOpCode::STACKLEFT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-stackleft")),
        FuncOpCode::STACKERR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-stackerr")),
        FuncOpCode::DGETALIAS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dgetalias")),
        FuncOpCode::DBOF => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dbof")),
        FuncOpCode::DCHANGED => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dchanged")),
        FuncOpCode::DDECIMALS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ddecimals")),
        FuncOpCode::DDELETED => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ddeleted")),
        FuncOpCode::DEOF => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-deof")),
        FuncOpCode::DERR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-derr")),
        FuncOpCode::DFIELDS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dfields")),
        FuncOpCode::DLENGTH => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dlength")),
        FuncOpCode::DNAME => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dname")),
        FuncOpCode::DRECCOUNT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dreccount")),
        FuncOpCode::DRECNO => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-drecno")),
        FuncOpCode::DTYPE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dtype")),
        FuncOpCode::FNEXT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-fnext")),
        FuncOpCode::DNEXT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dnext")),
        FuncOpCode::TODDATE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-toddate")),
        FuncOpCode::DCLOSEALL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dcloseall")),
        FuncOpCode::DOPEN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dopen")),
        FuncOpCode::DCLOSE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dclose")),
        FuncOpCode::DSETALIAS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dsetalias")),
        FuncOpCode::DPACK => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dpack")),
        FuncOpCode::DLOCKF => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dlockf")),
        FuncOpCode::DLOCK => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dlock")),
        FuncOpCode::DLOCKR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dlockr")),
        FuncOpCode::DUNLOCK => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dunlock")),
        FuncOpCode::DNOPEN => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dnopen")),
        FuncOpCode::DNCLOSE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dnclose")),
        FuncOpCode::DNCLOSEALL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dncloseall")),
        FuncOpCode::DNEW => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dnew")),
        FuncOpCode::DADD => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dadd")),
        FuncOpCode::DAPPEND => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dappend")),
        FuncOpCode::DTOP => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dtop")),
        FuncOpCode::DGO => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dgo")),
        FuncOpCode::DBOTTOM => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dbottom")),
        FuncOpCode::DSKIP => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dskip")),
        FuncOpCode::DBLANK => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dblank")),
        FuncOpCode::DDELETE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ddelete")),
        FuncOpCode::DRECALL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-drecall")),
        FuncOpCode::DTAG => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dtag")),
        FuncOpCode::DSEEK => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dseek")),
        FuncOpCode::DFBLANK => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dfblank")),
        FuncOpCode::DGET => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dget")),
        FuncOpCode::DPUT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dput")),
        FuncOpCode::DFCOPY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dfcopy")),
        FuncOpCode::DSELECT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dselect")),
        FuncOpCode::DCHKSTAT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-dchkstat")),
        FuncOpCode::PCBACCOUNT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-pcbaccount")),
        FuncOpCode::PCBACCSTAT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-pcbaccstat")),
        FuncOpCode::DERRMSG => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-derrmsg")),
        FuncOpCode::ACCOUNT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-account")),
        FuncOpCode::SCANMSGHDR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-scanmsghdr")),
        FuncOpCode::CHECKRIP => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-checkrip")),
        FuncOpCode::RIPVER => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ripver")),
        FuncOpCode::QWKLIMITS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-qwklimits")),
        FuncOpCode::FINDFIRST => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-findfirst")),
        FuncOpCode::FINDNEXT => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-findnext")),
        FuncOpCode::USELMRS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-uselmrs")),
        FuncOpCode::CONFINFO => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-confinfo")),
        FuncOpCode::TINKEY => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tinkey")),
        FuncOpCode::CWD => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-cwd")),
        FuncOpCode::INSTRR => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-instrr")),
        FuncOpCode::FDORDAKA => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-fdordaka")),
        FuncOpCode::FDORDORG => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-fdordorg")),
        FuncOpCode::FDORDAREA => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-fdordarea")),
        FuncOpCode::FDOQRD => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-fdoqrd")),
        FuncOpCode::GETDRIVE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-getdrive")),
        FuncOpCode::SETDRIVE => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-setdrive")),
        FuncOpCode::BS2I => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-bs2i")),
        FuncOpCode::BD2I => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-bd2i")),
        FuncOpCode::I2BS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-i2bs")),
        FuncOpCode::I2BD => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-i2bd")),
        FuncOpCode::FTELL => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-ftell")),
        FuncOpCode::OS => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-os")),
        FuncOpCode::SHORT_DESC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-short_desc")),
        FuncOpCode::GetBankBal => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-getbankbal")),
        FuncOpCode::GetMsgHdr => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-getmsghdr")),
        FuncOpCode::SetMsgHdr => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-setmsghdr")),
        FuncOpCode::AreaId => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-areaid")),
        FuncOpCode::Len_Dim => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-len_dim")),
        FuncOpCode::BASE64ENC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-base64enc")),
        FuncOpCode::BASE64DEC => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-base64dec")),
        FuncOpCode::ToBytes => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-tobytes")),
        FuncOpCode::Rgb | FuncOpCode::RgbAlpha => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-rgb")),
        FuncOpCode::Terminal => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-terminal")),
        FuncOpCode::Board => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-board")),
        FuncOpCode::Session => get_sig_hint(sig, fl!(crate::LANGUAGE_LOADER, "hint-function-session")),
        _ => None,
    }
}

pub fn get_statement_hover(stmt: &StatementDefinition) -> Option<Hover> {
    let sig = stmt.get_signature();
    match stmt.opcode {
        OpCode::END => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-end")),
        OpCode::CLS => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-cls")),
        OpCode::CLREOL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-clreol")),
        OpCode::MORE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-more")),
        OpCode::WAIT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-wait")),
        OpCode::COLOR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-color")),
        OpCode::GOTO => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-goto")),
        OpCode::LET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-let")),
        OpCode::PRINT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-print")),
        OpCode::PRINTLN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-println")),
        OpCode::CONFFLAG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-confflag")),
        OpCode::CONFUNFLAG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-confunflag")),
        OpCode::DISPFILE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dispfile")),
        OpCode::INPUT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-input")),
        OpCode::FCREATE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fcreate")),
        OpCode::FOPEN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fopen")),
        OpCode::FAPPEND => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fappend")),
        OpCode::FCLOSE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fclose")),
        OpCode::FGET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fget")),
        OpCode::FPUT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fput")),
        OpCode::FPUTLN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fputln")),
        OpCode::RESETDISP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-resetdisp")),
        OpCode::STARTDISP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-startdisp")),
        OpCode::FPUTPAD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fputpad")),
        OpCode::HANGUP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-hangup")),
        OpCode::GETUSER => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-getuser")),
        OpCode::PUTUSER => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-putuser")),
        OpCode::DEFCOLOR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-defcolor")),
        OpCode::DELETE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-delete")),
        OpCode::DELUSER => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-deluser")),
        OpCode::ADJTIME => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-adjtime")),
        OpCode::LOG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-log")),
        OpCode::INPUTSTR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inputstr")),
        OpCode::INPUTYN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inputyn")),
        OpCode::INPUTMONEY => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inputmoney")),
        OpCode::INPUTINT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inputint")),
        OpCode::INPUTCC => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inputcc")),
        OpCode::INPUTDATE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inputdate")),
        OpCode::INPUTTIME => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inputtime")),
        OpCode::GOSUB => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-gosub")),
        OpCode::RETURN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-return")),
        OpCode::PROMPTSTR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-promptstr")),
        OpCode::DTRON => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dtron")),
        OpCode::DTROFF => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dtroff")),
        OpCode::CDCHKON => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-cdchkon")),
        OpCode::CDCHKOFF => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-cdchkoff")),
        OpCode::DELAY => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-delay")),
        OpCode::SENDMODEM => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-sendmodem")),
        OpCode::INC => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inc")),
        OpCode::DEC => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dec")),
        OpCode::NEWLINE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-newline")),
        OpCode::NEWLINES => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-newlines")),
        OpCode::TOKENIZE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tokenize")),
        OpCode::GETTOKEN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-gettoken")),
        OpCode::SHELL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-shell")),
        OpCode::DISPTEXT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-disptext")),
        OpCode::STOP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-stop")),
        OpCode::INPUTTEXT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-inputtext")),
        OpCode::BEEP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-beep")),
        OpCode::PUSH => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-push")),
        OpCode::POP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-pop")),
        OpCode::KBDSTUFF => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-kbdstuff")),
        OpCode::CALL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-call")),
        OpCode::JOIN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-join")),
        OpCode::QUEST => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-quest")),
        OpCode::BLT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-blt")),
        OpCode::DIR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dir")),
        OpCode::KBDFILE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-kbdfile")),
        OpCode::BYE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-bye")),
        OpCode::GOODBYE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-goodbye")),
        OpCode::BROADCAST => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-broadcast")),
        OpCode::WAITFOR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-waitfor")),
        OpCode::KBDCHKON => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-kbdchkon")),
        OpCode::KBDCHKOFF => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-kbdchkoff")),
        OpCode::OPTEXT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-optext")),
        OpCode::DISPSTR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dispstr")),
        OpCode::RDUNET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-rdunet")),
        OpCode::WRUNET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-wrunet")),
        OpCode::DOINTR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dointr")),
        OpCode::VARSEG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-varseg")),
        OpCode::VAROFF => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-varoff")),
        OpCode::POKEB => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-pokeb")),
        OpCode::POKEW => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-pokew")),
        OpCode::VARADDR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-varaddr")),
        OpCode::ANSIPOS => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-ansipos")),
        OpCode::BACKUP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-backup")),
        OpCode::FORWARD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-forward")),
        OpCode::FRESHLINE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-freshline")),
        OpCode::WRUSYS => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-wrusys")),
        OpCode::RDUSYS => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-rdusys")),
        OpCode::NEWPWD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-newpwd")),
        OpCode::OPENCAP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-opencap")),
        OpCode::CLOSECAP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-closecap")),
        OpCode::MESSAGE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-message")),
        OpCode::SAVESCRN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-savescrn")),
        OpCode::RESTSCRN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-restscrn")),
        OpCode::SOUND => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-sound")),
        OpCode::CHAT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-chat")),
        OpCode::SPRINT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-sprint")),
        OpCode::SPRINTLN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-sprintln")),
        OpCode::MPRINT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-mprint")),
        OpCode::MPRINTLN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-mprintln")),
        OpCode::RENAME => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-rename")),
        OpCode::FREWIND => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-frewind")),
        OpCode::POKEDW => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-pokedw")),
        OpCode::DBGLEVEL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dbglevel")),
        OpCode::SHOWON => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-showon")),
        OpCode::SHOWOFF => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-showoff")),
        OpCode::PAGEON => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-pageon")),
        OpCode::PAGEOFF => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-pageoff")),
        OpCode::FSEEK => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fseek")),
        OpCode::FFLUSH => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fflush")),
        OpCode::FREAD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fread")),
        OpCode::FWRITE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fwrite")),
        OpCode::FDEFIN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdefin")),
        OpCode::FDEFOUT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdefout")),
        OpCode::FDGET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdget")),
        OpCode::FDPUT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdput")),
        OpCode::FDPUTLN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdputln")),
        OpCode::FDPUTPAD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdputpad")),
        OpCode::FDREAD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdread")),
        OpCode::FDWRITE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdwrite")),
        OpCode::ADJBYTES => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-adjbytes")),
        OpCode::KBDSTRING => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-kbdstring")),
        OpCode::ALIAS => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-alias")),
        OpCode::REDIM => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-redim")),
        OpCode::APPEND => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-append")),
        OpCode::COPY => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-copy")),
        OpCode::KBDFLUSH => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-kbdflush")),
        OpCode::MDMFLUSH => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-mdmflush")),
        OpCode::KEYFLUSH => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-keyflush")),
        OpCode::LASTIN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-lastin")),
        OpCode::FLAG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-flag")),
        OpCode::DOWNLOAD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-download")),
        OpCode::WRUSYSDOOR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-wrusysdoor")),
        OpCode::GETALTUSER => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-getaltuser")),
        OpCode::ADJDBYTES => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-adjdbytes")),
        OpCode::ADJTBYTES => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-adjtbytes")),
        OpCode::ADJTFILES => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-adjtfiles")),
        OpCode::LANG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-lang")),
        OpCode::SORT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-sort")),
        OpCode::MOUSEREG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-mousereg")),
        OpCode::SCRFILE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-scrfile")),
        OpCode::SEARCHINIT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-searchinit")),
        OpCode::SEARCHFIND => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-searchfind")),
        OpCode::SEARCHSTOP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-searchstop")),
        OpCode::PRFOUND => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-prfound")),
        OpCode::PRFOUNDLN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-prfoundln")),
        OpCode::TPAGET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tpaget")),
        OpCode::TPAPUT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tpaput")),
        OpCode::TPACGET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tpacget")),
        OpCode::TPACPUT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tpacput")),
        OpCode::TPAREAD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tparead")),
        OpCode::TPAWRITE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tpawrite")),
        OpCode::TPACREAD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tpacread")),
        OpCode::TPACWRITE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-tpacwrite")),
        OpCode::BITSET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-bitset")),
        OpCode::BITCLEAR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-bitclear")),
        OpCode::BRAG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-brag")),
        OpCode::FREALTUSER => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-frealtuser")),
        OpCode::SETLMR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-setlmr")),
        OpCode::SETENV => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-setenv")),
        OpCode::FCLOSEALL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fcloseall")),
        OpCode::STACKABORT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-stackabort")),
        OpCode::DCREATE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dcreate")),
        OpCode::DOPEN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dopen")),
        OpCode::DCLOSE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dclose")),
        OpCode::DSETALIAS => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dsetalias")),
        OpCode::DPACK => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dpack")),
        OpCode::DCLOSEALL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dcloseall")),
        OpCode::DLOCK => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dlock")),
        OpCode::DLOCKR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dlockr")),
        OpCode::DLOCKG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dlockg")),
        OpCode::DUNLOCK => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dunlock")),
        OpCode::DNCREATE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dncreate")),
        OpCode::DNOPEN => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dnopen")),
        OpCode::DNCLOSE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dnclose")),
        OpCode::DNCLOSEALL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dncloseall")),
        OpCode::DNEW => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dnew")),
        OpCode::DADD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dadd")),
        OpCode::DAPPEND => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dappend")),
        OpCode::DTOP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dtop")),
        OpCode::DGO => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dgo")),
        OpCode::DBOTTOM => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dbottom")),
        OpCode::DSKIP => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dskip")),
        OpCode::DBLANK => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dblank")),
        OpCode::DDELETE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-ddelete")),
        OpCode::DRECALL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-drecall")),
        OpCode::DTAG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dtag")),
        OpCode::DSEEK => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dseek")),
        OpCode::DFBLANK => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dfblank")),
        OpCode::DGET => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dget")),
        OpCode::DPUT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dput")),
        OpCode::DFCOPY => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-dfcopy")),
        OpCode::ACCOUNT => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-account")),
        OpCode::RECORDUSAGE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-recordusage")),
        OpCode::MSGTOFILE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-msgtofile")),
        OpCode::QWKLIMITS => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-qwklimits")),
        OpCode::COMMAND => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-command")),
        OpCode::USELMRS => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-uselmrs")),
        OpCode::CONFINFO => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-confinfo")),
        OpCode::ADJTUBYTES => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-adjtubytes")),
        OpCode::GRAFMODE => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-grafmode")),
        OpCode::ADDUSER => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-adduser")),
        OpCode::KILLMSG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-killmsg")),
        OpCode::CHDIR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-chdir")),
        OpCode::MKDIR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-mkdir")),
        OpCode::RMDIR => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-rmdir")),
        OpCode::FDOWRAKA => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdowraka")),
        OpCode::FDOADDAKA => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdoaddaka")),
        OpCode::FDOWRORG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdowrorg")),
        OpCode::FDOADDORG => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdoaddorg")),
        OpCode::FDOQMOD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdoqmod")),
        OpCode::FDOQADD => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdoqadd")),
        OpCode::FDOQDEL => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fdoqdel")),
        OpCode::SOUNDDELAY => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-sounddelay")),
        OpCode::ShortDesc => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-shortdesc")),
        OpCode::MoveMsg => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-movemsg")),
        OpCode::SetBankBal => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-setbankbal")),
        OpCode::OnError => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-on-error")),
        OpCode::FGetRec => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fgetrec")),
        OpCode::FPutRec => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fputrec")),
        OpCode::FReadRec => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-freadrec")),
        OpCode::FWriteRec => get_sig_hint(sig, fl!(LANGUAGE_LOADER, "hint-statement-fwriterec")),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use icy_board_engine::{
        executable::{FUNCTION_DEFINITIONS, FunctionSignature, STATEMENT_DEFINITIONS, VariableType},
        parser::{
            BOARD_ID, CHECKSUM_ENUM_ID, EDITOR_MODE_ENUM_ID, ERR_CODE_ENUM_ID, ERR_KIND_ENUM_ID, EVENT_KIND_ENUM_ID, GFX_BACKEND_ENUM_ID, GFX_ID, HTTP_ID,
            HTTP_METHOD_ENUM_ID, HTTP_REQUEST_ID, HTTP_RESPONSE_ID, MOUSE_ACTION_ENUM_ID, MOUSE_BUTTON_ENUM_ID, MOUSE_MODE_ENUM_ID, MOUSE_TRACKING_ENUM_ID,
            MSG_FIELD_ENUM_ID, REGEX_OPTIONS_ENUM_ID, SESSION_ID, STRING_COMPARISON_ENUM_ID, SURFACE_ID, USER_ID, UserTypeRegistry,
        },
    };

    #[test]
    fn test_function_translations() {
        for f in FUNCTION_DEFINITIONS.iter() {
            // Names in angle brackets are the compiler's own; they cannot be written.
            if f.name.starts_with('<') {
                continue;
            }
            if let FunctionSignature::FixedParameters(_) = f.signature {
                assert!(super::get_function_hover(f).is_some(), "Function {:?} failed", f.opcode);
            }
        }
    }

    #[test]
    fn classic_signatures_are_ppl_highlighted_code() {
        let definition = STATEMENT_DEFINITIONS
            .iter()
            .find(|definition| definition.name.eq_ignore_ascii_case("ANSIPOS"))
            .unwrap();
        let hover = super::get_statement_hover(definition).unwrap();
        let tower_lsp::lsp_types::HoverContents::Markup(content) = hover.contents else {
            panic!("expected markdown hover");
        };
        assert!(content.value.starts_with("```PPL\nANSIPOS "), "{}", content.value);
        assert!(content.value.contains("\n```\n\n"), "{}", content.value);
    }

    #[test]
    fn built_in_constants_show_kind_type_name_and_value() {
        let hover = super::get_const_hover(&icy_board_engine::ast::constant::BuiltinConst::TRUE).unwrap();
        let tower_lsp::lsp_types::HoverContents::Markup(content) = hover.contents else {
            panic!("expected markdown hover");
        };
        assert!(content.value.starts_with("```PPL\nCONSTANT BOOLEAN TRUE = 1h\n```"), "{}", content.value);

        let key = icy_board_engine::ast::constant::BUILTIN_CONSTS
            .iter()
            .find(|constant| constant.name == "KEY_ESCAPE")
            .unwrap();
        let hover = super::get_const_hover(key).expect("every predefined constant has a tooltip");
        let tower_lsp::lsp_types::HoverContents::Markup(content) = hover.contents else {
            panic!("expected markdown hover");
        };
        assert!(content.value.starts_with("```PPL\nCONSTANT INTEGER KEY_ESCAPE = 1Bh\n```"), "{}", content.value);
    }

    #[test]
    fn string_hover_changes_at_language_version_400() {
        let text = |version| {
            let hover = super::get_type_hover_for_version(VariableType::String, version).unwrap();
            let tower_lsp::lsp_types::HoverContents::Markup(content) = hover.contents else {
                panic!("expected markdown hover");
            };
            content.value
        };

        let legacy = text(350);
        assert!(legacy.contains("256"), "{legacy}");
        assert!(!legacy.to_ascii_lowercase().contains("unbounded"), "{legacy}");

        let modern = text(400);
        assert!(modern.to_ascii_lowercase().contains("unbounded"), "{modern}");
        assert!(modern.contains("400"), "{modern}");
    }

    #[test]
    fn every_reserved_keyword_has_hover_documentation() {
        for keyword in icy_board_engine::parser::lexer::KEYWORDS {
            let hover = super::get_keyword_hover(keyword.name).unwrap_or_else(|| panic!("missing documentation for {}", keyword.name));
            let tower_lsp::lsp_types::HoverContents::Markup(content) = hover.contents else {
                panic!("expected markdown hover for {}", keyword.name);
            };
            assert!(content.value.starts_with("```PPL\n"), "{}", keyword.name);
            assert!(
                content.value.contains("\n```PPL\n"),
                "missing usage syntax for {}: {}",
                keyword.name,
                content.value
            );
        }
        let exit = super::get_keyword_hover("EXIT").unwrap();
        let tower_lsp::lsp_types::HoverContents::Markup(exit) = exit.contents else {
            panic!("expected markdown hover for EXIT");
        };
        assert!(exit.value.contains("\n```PPL\nEXIT\n```"), "{exit:?}");
    }

    #[test]
    fn conditional_hover_documents_inline_and_block_usage() {
        let hover = super::get_keyword_hover("IF").unwrap();
        let tower_lsp::lsp_types::HoverContents::Markup(content) = hover.contents else {
            panic!("expected markdown hover");
        };
        for syntax in ["IF condition THEN", "ELSEIF otherCondition THEN", "ELSE", "ENDIF", "IF condition statement"] {
            assert!(content.value.contains(syntax), "missing {syntax:?}: {}", content.value);
        }
    }

    #[test]
    fn german_catalog_covers_all_classic_statements_and_functions() {
        fn classic_keys(catalog: &str) -> std::collections::BTreeSet<&str> {
            catalog
                .lines()
                .filter_map(|line| line.split_once('=').map(|(key, _)| key))
                .filter(|key| key.starts_with("hint-statement-") || key.starts_with("hint-function-"))
                .collect()
        }

        let english = classic_keys(include_str!("../i18n/en/ppl_lsp.ftl"));
        let german = classic_keys(include_str!("../i18n/de/ppl_lsp.ftl"));
        let missing: Vec<_> = english.difference(&german).copied().collect();
        assert!(missing.is_empty(), "German classic documentation is missing: {missing:?}");
    }

    #[test]
    fn localized_documentation_contains_no_todo_placeholders() {
        for (language, catalog) in [
            ("English", include_str!("../i18n/en/ppl_lsp.ftl")),
            ("German", include_str!("../i18n/de/ppl_lsp.ftl")),
        ] {
            for (line_number, line) in catalog.lines().enumerate() {
                assert!(
                    !line.to_ascii_lowercase().contains("todo"),
                    "{language} documentation contains a TODO placeholder at line {}: {line}",
                    line_number + 1
                );
            }
        }
    }

    #[test]
    fn german_catalog_covers_the_new_api() {
        fn new_api_keys(catalog: &str) -> std::collections::BTreeSet<&str> {
            const PREFIXES: &[&str] = &[
                "hint-type-",
                "hint-member-",
                "hint-enum-",
                "hint-bytes-",
                "hint-http-",
                "hint-regex-",
                "hint-string-",
            ];

            catalog
                .lines()
                .filter_map(|line| line.split_once('=').map(|(key, _)| key))
                .filter(|key| PREFIXES.iter().any(|prefix| key.starts_with(prefix)))
                .collect()
        }

        let english = new_api_keys(include_str!("../i18n/en/ppl_lsp.ftl"));
        let german = new_api_keys(include_str!("../i18n/de/ppl_lsp.ftl"));
        let missing: Vec<_> = english.difference(&german).copied().collect();
        assert!(missing.is_empty(), "German PPL 400 API documentation is missing: {missing:?}");
    }

    #[test]
    fn new_runtime_types_have_hover_documentation() {
        for variable_type in [
            VariableType::Bytes,
            VariableType::UserData(BOARD_ID as u8),
            VariableType::UserData(SESSION_ID as u8),
            VariableType::UserData(USER_ID as u8),
            VariableType::UserData(HTTP_ID as u8),
            VariableType::UserData(HTTP_REQUEST_ID as u8),
            VariableType::UserData(HTTP_RESPONSE_ID as u8),
            VariableType::UserData(CHECKSUM_ENUM_ID),
            VariableType::UserData(GFX_ID as u8),
            VariableType::UserData(SURFACE_ID as u8),
            VariableType::UserData(GFX_BACKEND_ENUM_ID),
        ] {
            assert!(super::get_type_hover(variable_type).is_some(), "missing hover for {variable_type}");
        }
    }

    #[test]
    fn every_ppl_400_object_and_enum_member_has_documentation() {
        let registry = UserTypeRegistry::icy_board_registry();
        for (type_name, variable_type) in &registry.registered_types {
            let VariableType::UserData(id) = variable_type else {
                continue;
            };
            assert!(super::get_type_hover(*variable_type).is_some(), "missing type documentation for {type_name}");
            if let Some(record) = registry.get_record_type_from_id(*id) {
                for (member, _) in &record.fields {
                    assert!(
                        super::get_member_documentation(*variable_type, member.as_ref()).is_some(),
                        "missing documentation for {type_name}.{member}"
                    );
                }
                continue;
            }
            let Some(object) = registry.get_type_from_id(*id) else {
                continue;
            };
            for member in object.fields.keys().chain(object.functions.keys()).chain(object.procedures.keys()) {
                assert!(
                    super::get_member_documentation(*variable_type, member.as_ref()).is_some(),
                    "missing documentation for {type_name}.{member}"
                );
            }
        }

        for id in [
            EVENT_KIND_ENUM_ID,
            MOUSE_ACTION_ENUM_ID,
            MOUSE_BUTTON_ENUM_ID,
            MOUSE_MODE_ENUM_ID,
            MOUSE_TRACKING_ENUM_ID,
            GFX_BACKEND_ENUM_ID,
            ERR_KIND_ENUM_ID,
            ERR_CODE_ENUM_ID,
            EDITOR_MODE_ENUM_ID,
            MSG_FIELD_ENUM_ID,
            HTTP_METHOD_ENUM_ID,
            REGEX_OPTIONS_ENUM_ID,
            STRING_COMPARISON_ENUM_ID,
            CHECKSUM_ENUM_ID,
        ] {
            let definition = registry.get_enum_from_id(id).unwrap();
            for (member, _) in &definition.variants {
                assert!(
                    super::get_member_documentation(VariableType::UserData(id), member.as_ref()).is_some(),
                    "missing documentation for {}.{member}",
                    definition.name
                );
            }
        }
    }

    #[test]
    fn every_object_api_parameter_has_a_name_and_description() {
        let registry = UserTypeRegistry::icy_board_registry();
        for (type_name, variable_type) in &registry.registered_types {
            let VariableType::UserData(id) = variable_type else {
                continue;
            };
            let Some(object) = registry.get_type_from_id(*id) else {
                continue;
            };
            for (member, function) in &object.functions {
                assert_eq!(
                    function.parameter_names.len(),
                    function.parameters.len(),
                    "{type_name}.{member} has unnamed parameters"
                );
                for name in &function.parameter_names {
                    assert!(
                        super::get_parameter_documentation(name).is_some(),
                        "{type_name}.{member} parameter {name} is undocumented"
                    );
                }
            }
            for (member, procedure) in &object.procedures {
                assert_eq!(
                    procedure.parameter_names.len(),
                    procedure.parameters.len(),
                    "{type_name}.{member} has unnamed parameters"
                );
                for name in &procedure.parameter_names {
                    assert!(
                        super::get_parameter_documentation(name).is_some(),
                        "{type_name}.{member} parameter {name} is undocumented"
                    );
                }
            }
        }
    }
}
