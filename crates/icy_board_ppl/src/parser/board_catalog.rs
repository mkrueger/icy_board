use super::{
    AUDIO_ID, BOARD_ID, CONFERENCE_ID, CONTACT_ID, DOOR_ID, EDITOR_MODE_ENUM_ID, ERR_CODE_ENUM_ID, ERR_KIND_ENUM_ID, ERROR_ID, EVENT_ID, EVENT_KIND_ENUM_ID,
    FILE_DIRECTORY_ID, GFX_BACKEND_ENUM_ID, GFX_ID, HTTP_ID, HTTP_METHOD_ENUM_ID, HTTP_REQUEST_ID, HTTP_RESPONSE_ID, MACROS_ID, MARGINS_ID, MESSAGE_AREA_ID,
    MOUSE_ACTION_ENUM_ID, MOUSE_BUTTON_ENUM_ID, MOUSE_MODE_ENUM_ID, MOUSE_TRACKING_ENUM_ID, MSG_FIELD_ENUM_ID, MSG_ID, PALETTE_ID, REGEX_ID, REGEX_MATCH_ID,
    REGEX_OPTIONS_ENUM_ID, SESSION_ID, SURFACE_ID, TERM_INFO_ID, TERM_INPUT_ID, TERMINAL_ID, USER_ID,
};
use crate::{
    compiler::user_data::UserDataMemberRegistry,
    executable::{FuncOpCode, VariableType as V},
};

pub const TYPES: &[(usize, &str, Option<FuncOpCode>)] = &[
    (CONFERENCE_ID, "Conference", None),
    (MESSAGE_AREA_ID, "Area", None),
    (FILE_DIRECTORY_ID, "Directory", None),
    (DOOR_ID, "Door", None),
    (SURFACE_ID, "Surface", None),
    (EVENT_ID, "Event", None),
    (AUDIO_ID, "Audio", None),
    (ERROR_ID, "Error", None),
    (TERM_INFO_ID, "TermInfo", None),
    (TERM_INPUT_ID, "TermInput", None),
    (TERMINAL_ID, "Terminal", Some(FuncOpCode::Terminal)),
    (GFX_ID, "Gfx", None),
    (MARGINS_ID, "Margins", None),
    (PALETTE_ID, "Palette", None),
    (MACROS_ID, "Macros", None),
    (BOARD_ID, "Board", Some(FuncOpCode::Board)),
    (SESSION_ID, "Session", Some(FuncOpCode::Session)),
    (USER_ID, "User", None),
    (MSG_ID, "Msg", None),
    (HTTP_ID, "Http", None),
    (HTTP_REQUEST_ID, "HttpRequest", None),
    (HTTP_RESPONSE_ID, "HttpResponse", None),
    (REGEX_ID, "Regex", None),
    (REGEX_MATCH_ID, "RegexMatch", None),
];

fn n(name: &str) -> unicase::Ascii<String> {
    unicase::Ascii::new(name.to_string())
}

fn register_data_members<F: UserDataMemberRegistry>(id: usize, registry: &mut F) {
    match id {
        USER_ID => {
            for name in [
                "Alias",
                "VerifyAnswer",
                "Street1",
                "Street2",
                "City",
                "State",
                "Zip",
                "Country",
                "BusinessPhone",
                "HomePhone",
                "Email",
                "Web",
                "Gender",
                "Comment",
                "SysopComment",
                "Protocol",
            ] {
                registry.add_property(n(name), V::UnboundedString, true);
            }
            for name in ["Name", "Language", "DateFormat"] {
                registry.add_property(n(name), V::UnboundedString, false);
            }
            registry.add_property(n("Valid"), V::Boolean, false);
            registry.add_property(n("RecordNumber"), V::Integer, false);
            for name in ["BirthDate", "ExpirationDate", "PasswordExpires"] {
                registry.add_property(n(name), V::Date, true);
            }
            for name in ["FirstDateOn", "LastDateOn", "LastDirRead"] {
                registry.add_property(n(name), V::Date, false);
            }
            for name in ["PageLength", "SecurityLevel", "ExpiredSecurityLevel"] {
                registry.add_property(n(name), V::Integer, true);
            }
            registry.add_property(n("MinutesToday"), V::Integer, false);
            for name in [
                "TimesOn",
                "MessagesRead",
                "MessagesLeft",
                "Uploads",
                "Downloads",
                "UploadBytes",
                "DownloadBytes",
                "DownloadBytesToday",
            ] {
                registry.add_property(n(name), V::ULong, false);
            }
            for name in [
                "ExpertMode",
                "ClearScreen",
                "ScrollMessageBody",
                "ShortDescriptions",
                "LongHeader",
                "WideEditor",
            ] {
                registry.add_property(n(name), V::Boolean, true);
            }
            for name in ["UseGraphics", "UseAlias"] {
                registry.add_property(n(name), V::Boolean, false);
            }
            registry.add_property(n("EditorMode"), V::UserData(EDITOR_MODE_ENUM_ID), true);
            registry.add_array_property(n("Notes"), V::UnboundedString, 1);
            registry.add_array_property(n("Contacts"), V::UserData(CONTACT_ID as u8), 1);
            registry.add_named_function(n("SetPassword"), vec![("password", V::UnboundedString)], V::Boolean);
            registry.add_named_function(
                n("AddContact"),
                vec![("service", V::UnboundedString), ("account", V::UnboundedString)],
                V::Boolean,
            );
            registry.add_named_function(n("RemoveContact"), vec![("index", V::Integer)], V::Boolean);
            registry.add_named_function(n("SetNote"), vec![("index", V::Integer), ("text", V::UnboundedString)], V::Boolean);
        }
        MSG_ID => {
            registry.add_property(n("Number"), V::Long, false);
            registry.add_property(n("Valid"), V::Boolean, false);
            for name in ["From", "To", "Subject", "Status"] {
                registry.add_property(n(name), V::UnboundedString, false);
            }
            registry.add_property(n("Date"), V::Date, false);
            registry.add_property(n("Time"), V::Time, false);
            registry.add_property(n("ReplyTo"), V::Long, false);
            registry.add_property(n("Size"), V::Long, false);
            for name in ["IsPrivate", "IsRead", "IsDeleted", "IsEcho", "NeedsPassword"] {
                registry.add_property(n(name), V::Boolean, false);
            }
            registry.add_function(n("Text"), Vec::new(), V::UnboundedString);
        }
        HTTP_ID => {
            registry.add_named_static_function(n("Get"), vec![("url", V::UnboundedString)], V::UserData(HTTP_RESPONSE_ID as u8));
            registry.add_named_static_function(
                n("New"),
                vec![("method", V::UserData(HTTP_METHOD_ENUM_ID)), ("url", V::UnboundedString)],
                V::UserData(HTTP_REQUEST_ID as u8),
            );
            registry.add_named_static_function(
                n("Download"),
                vec![("url", V::UnboundedString), ("file", V::UnboundedString)],
                V::UserData(HTTP_RESPONSE_ID as u8),
            );
            for name in ["UrlEncode", "UrlDecode", "FormEncode", "FormDecode"] {
                registry.add_named_static_function(n(name), vec![("text", V::UnboundedString)], V::UnboundedString);
            }
        }
        HTTP_REQUEST_ID => {
            registry.add_property(n("Url"), V::UnboundedString, false);
            registry.add_property(n("Method"), V::UserData(HTTP_METHOD_ENUM_ID), false);
            for name in ["SetQuery", "SetHeader"] {
                registry.add_named_function(n(name), vec![("name", V::UnboundedString), ("value", V::UnboundedString)], V::Boolean);
            }
            registry.add_named_function_with(
                n("SetText"),
                vec![("text", V::UnboundedString), ("contentType", V::UnboundedString)],
                1,
                V::Boolean,
            );
            registry.add_named_function_with(n("SetBytes"), vec![("data", V::Bytes), ("contentType", V::UnboundedString)], 1, V::Boolean);
            registry.add_named_function(n("SetForm"), vec![("name", V::UnboundedString), ("value", V::UnboundedString)], V::Boolean);
            registry.add_function(n("Send"), Vec::new(), V::UserData(HTTP_RESPONSE_ID as u8));
        }
        HTTP_RESPONSE_ID => {
            registry.add_property(n("Valid"), V::Boolean, false);
            registry.add_property(n("OK"), V::Boolean, false);
            registry.add_property(n("Status"), V::Integer, false);
            registry.add_property(n("FinalUrl"), V::UnboundedString, false);
            registry.add_property(n("Size"), V::Long, false);
            registry.add_property(n("ContentType"), V::UnboundedString, false);
            registry.add_function(n("Text"), Vec::new(), V::UnboundedString);
            registry.add_function(n("Bytes"), Vec::new(), V::Bytes);
            registry.add_named_function(n("Header"), vec![("name", V::UnboundedString)], V::UnboundedString);
            registry.add_named_function(n("Save"), vec![("file", V::UnboundedString)], V::Boolean);
        }
        REGEX_ID => {
            registry.add_property(n("Valid"), V::Boolean, false);
            registry.add_property(n("Pattern"), V::UnboundedString, false);
            registry.add_named_static_function_with(
                n("Compile"),
                vec![("pattern", V::UnboundedString), ("options", V::UserData(REGEX_OPTIONS_ENUM_ID))],
                1,
                V::UserData(REGEX_ID as u8),
            );
            registry.add_named_static_function(n("Escape"), vec![("text", V::UnboundedString)], V::UnboundedString);
            registry.add_named_static_function_with(
                n("IsValid"),
                vec![("pattern", V::UnboundedString), ("options", V::UserData(REGEX_OPTIONS_ENUM_ID))],
                1,
                V::Boolean,
            );
            registry.add_named_function_with(n("IsMatch"), vec![("text", V::UnboundedString), ("start", V::Integer)], 1, V::Boolean);
            registry.add_named_function_with(
                n("Find"),
                vec![("text", V::UnboundedString), ("start", V::Integer)],
                1,
                V::UserData(REGEX_MATCH_ID as u8),
            );
            registry.add_named_array_function_with(
                n("FindAll"),
                vec![("text", V::UnboundedString), ("start", V::Integer), ("limit", V::Integer)],
                1,
                V::UserData(REGEX_MATCH_ID as u8),
                1,
            );
            registry.add_named_function_with(
                n("Replace"),
                vec![("text", V::UnboundedString), ("replacement", V::UnboundedString), ("limit", V::Integer)],
                2,
                V::UnboundedString,
            );
            registry.add_named_array_function_with(n("Split"), vec![("text", V::UnboundedString), ("limit", V::Integer)], 1, V::UnboundedString, 1);
        }
        REGEX_MATCH_ID => {
            registry.add_property(n("Success"), V::Boolean, false);
            registry.add_property(n("Value"), V::UnboundedString, false);
            registry.add_property(n("Start"), V::Integer, false);
            registry.add_property(n("Length"), V::Integer, false);
            registry.add_property(n("GroupCount"), V::Integer, false);
            for (indexed, named, result) in [
                ("Group", "NamedGroup", V::UnboundedString),
                ("GroupMatched", "NamedGroupMatched", V::Boolean),
                ("GroupStart", "NamedGroupStart", V::Integer),
                ("GroupLength", "NamedGroupLength", V::Integer),
            ] {
                registry.add_named_function(n(indexed), vec![("index", V::Integer)], result);
                registry.add_named_function(n(named), vec![("name", V::UnboundedString)], result);
            }
        }
        _ => panic!("unknown board object id {id}"),
    }
}

fn register_remaining_members<F: UserDataMemberRegistry>(id: usize, registry: &mut F) {
    match id {
        TERM_INFO_ID => {
            for name in ["Program", "DeviceAttrs", "RipVersion"] {
                registry.add_property(n(name), V::UnboundedString, false);
            }
            for name in ["Columns", "Rows", "CTermLevel", "CellWidth", "CellHeight", "ScreenWidth", "ScreenHeight"] {
                registry.add_property(n(name), V::Integer, false);
            }
            for name in [
                "Utf8",
                "Sixel",
                "Jxl",
                "InlineGraphics",
                "Audio",
                "PhysicalKeys",
                "PixelMouse",
                "ClientBlit",
                "SynchronizedOutput",
                "TerminalMacros",
            ] {
                registry.add_property(n(name), V::Boolean, false);
            }
        }
        TERM_INPUT_ID => {
            registry.add_function(n("Poll"), Vec::new(), V::UserData(EVENT_ID as u8));
            registry.add_named_function(n("Wait"), vec![("timeoutMs", V::Integer)], V::UserData(EVENT_ID as u8));
            registry.add_named_function_with(
                n("MouseOn"),
                vec![("mode", V::UserData(MOUSE_MODE_ENUM_ID)), ("tracking", V::UserData(MOUSE_TRACKING_ENUM_ID))],
                1,
                V::Boolean,
            );
            registry.add_function(n("MouseOff"), Vec::new(), V::Boolean);
            registry.add_named_function_with(n("KeyboardOn"), vec![("echo", V::Boolean)], 0, V::Boolean);
            registry.add_function(n("KeyboardOff"), Vec::new(), V::Boolean);
            registry.add_function(n("Release"), Vec::new(), V::Boolean);
        }
        TERMINAL_ID => {
            for (name, id) in [
                ("Info", TERM_INFO_ID),
                ("Gfx", GFX_ID),
                ("Input", TERM_INPUT_ID),
                ("Margins", MARGINS_ID),
                ("Palette", PALETTE_ID),
                ("Macros", MACROS_ID),
            ] {
                registry.add_property(n(name), V::UserData(id as u8), false);
            }
            registry.add_function(n("BeginUpdate"), Vec::new(), V::Boolean);
            registry.add_function(n("EndUpdate"), Vec::new(), V::Boolean);
            registry.add_named_function_with(n("SetFont"), vec![("font", V::Integer), ("slot", V::Integer)], 1, V::Boolean);
            registry.add_named_function(n("LoadFont"), vec![("font", V::Integer), ("file", V::UnboundedString)], V::Boolean);
        }
        GFX_ID => {
            let backend = V::UserData(GFX_BACKEND_ENUM_ID);
            registry.add_property(n("Backend"), backend, false);
            registry.add_named_function_with(n("Init"), vec![("backend", backend), ("fullscreen", V::Boolean)], 0, V::Boolean);
            registry.add_named_function(n("SetPacing"), vec![("enabled", V::Boolean)], V::Boolean);
            registry.add_function(n("Shutdown"), Vec::new(), V::Boolean);
        }
        MARGINS_ID => {
            for name in ["Top", "Bottom", "Left", "Right"] {
                registry.add_property(n(name), V::Integer, false);
            }
            registry.add_property(n("HasVertical"), V::Boolean, false);
            registry.add_property(n("HasHorizontal"), V::Boolean, false);
            registry.add_named_function(n("SetVertical"), vec![("top", V::Integer), ("bottom", V::Integer)], V::Boolean);
            registry.add_named_function(n("SetHorizontal"), vec![("left", V::Integer), ("right", V::Integer)], V::Boolean);
            for name in ["ResetVertical", "ResetHorizontal", "ResetAll"] {
                registry.add_function(n(name), Vec::new(), V::Boolean);
            }
        }
        PALETTE_ID => {
            registry.add_named_function(n("Set"), vec![("color", V::Integer), ("rgba", V::Unsigned)], V::Boolean);
            registry.add_named_function(n("Reset"), vec![("color", V::Integer)], V::Boolean);
            registry.add_function(n("ResetAll"), Vec::new(), V::Boolean);
        }
        MACROS_ID => {
            registry.add_property(n("Recording"), V::Boolean, false);
            registry.add_named_function(n("BeginRecord"), vec![("slot", V::Integer)], V::Boolean);
            registry.add_function(n("EndRecord"), Vec::new(), V::Boolean);
            registry.add_named_function(n("Play"), vec![("slot", V::Integer)], V::Boolean);
            registry.add_named_function(n("Delete"), vec![("slot", V::Integer)], V::Boolean);
            registry.add_function(n("DeleteAll"), Vec::new(), V::Boolean);
        }
        BOARD_ID => {
            for name in ["Name", "Location", "Operator", "SysopName"] {
                registry.add_property(n(name), V::UnboundedString, false);
            }
            registry.add_property(n("NodeCount"), V::Integer, false);
            registry.add_array_property(n("Conferences"), V::UserData(CONFERENCE_ID as u8), 1);
            registry.add_array_property(n("Users"), V::UserData(USER_ID as u8), 1);
        }
        SESSION_ID => {
            for (name, id) in [
                ("Conference", CONFERENCE_ID),
                ("User", USER_ID),
                ("Area", MESSAGE_AREA_ID),
                ("Directory", FILE_DIRECTORY_ID),
            ] {
                registry.add_property(n(name), V::UserData(id as u8), false);
            }
            for name in ["UserName", "AliasName", "Language"] {
                registry.add_property(n(name), V::UnboundedString, false);
            }
            for name in ["SecurityLevel", "Node", "MinutesLeft", "PageLength"] {
                registry.add_property(n(name), V::Integer, false);
            }
            for name in ["IsLocal", "IsSysop"] {
                registry.add_property(n(name), V::Boolean, false);
            }
            registry.add_named_function(n("RequestPasswordRecovery"), vec![("userName", V::UnboundedString)], V::Boolean);
        }
        _ => register_data_members(id, registry),
    }
}

/// Member order is part of the PPE ABI; both the compiler and runtime use this catalog.
pub fn register_members<F: UserDataMemberRegistry>(id: usize, registry: &mut F) {
    match id {
        CONFERENCE_ID => {
            registry.add_property(n("Name"), V::UnboundedString, false);
            registry.add_property(n("Number"), V::Integer, false);
            for name in ["Valid", "IsPublic", "IsReadOnly", "AllowAliases", "EchoMail", "AutoRejoin", "PrivateUploads"] {
                registry.add_property(n(name), V::Boolean, false);
            }
            registry.add_property(n("Password"), V::Password, false);
            registry.add_array_property(n("Directories"), V::UserData(FILE_DIRECTORY_ID as u8), 1);
            registry.add_array_property(n("Areas"), V::UserData(MESSAGE_AREA_ID as u8), 1);
            registry.add_array_property(n("Doors"), V::UserData(DOOR_ID as u8), 1);
            for name in ["HasAccess", "CanPost", "CanAttach"] {
                registry.add_function(n(name), Vec::new(), V::Boolean);
            }
        }
        MESSAGE_AREA_ID => {
            registry.add_property(n("Name"), V::UnboundedString, false);
            registry.add_property(n("Number"), V::Integer, false);
            for name in ["Valid", "IsReadOnly", "AllowAliases"] {
                registry.add_property(n(name), V::Boolean, false);
            }
            for name in ["QwkName", "EchoTag", "EchoOrigin"] {
                registry.add_property(n(name), V::UnboundedString, false);
            }
            for name in ["HasAccess", "CanEnter", "CanAttach"] {
                registry.add_function(n(name), Vec::new(), V::Boolean);
            }
            for name in ["HighMsg", "LowMsg"] {
                registry.add_function(n(name), Vec::new(), V::Long);
            }
            registry.add_named_function(n("Read"), vec![("messageNumber", V::Long)], V::UserData(MSG_ID as u8));
            registry.add_named_function_with(
                n("Find"),
                vec![
                    ("field", V::UserData(MSG_FIELD_ENUM_ID)),
                    ("text", V::UnboundedString),
                    ("startMessage", V::Long),
                ],
                2,
                V::UserData(MSG_ID as u8),
            );
        }
        FILE_DIRECTORY_ID => {
            registry.add_property(n("Name"), V::UnboundedString, false);
            registry.add_property(n("Number"), V::Integer, false);
            registry.add_property(n("Valid"), V::Boolean, false);
            registry.add_property(n("Path"), V::UnboundedString, false);
            registry.add_property(n("IsFree"), V::Boolean, false);
            registry.add_property(n("HasNewFiles"), V::Boolean, false);
            registry.add_property(n("Password"), V::Password, false);
            registry.add_function(n("HasAccess"), Vec::new(), V::Boolean);
            registry.add_function(n("CanDownload"), Vec::new(), V::Boolean);
        }
        DOOR_ID => {
            registry.add_property(n("Name"), V::UnboundedString, false);
            registry.add_property(n("Number"), V::Integer, false);
            registry.add_property(n("Valid"), V::Boolean, false);
            registry.add_property(n("Description"), V::UnboundedString, false);
            registry.add_property(n("Path"), V::UnboundedString, false);
            registry.add_property(n("Password"), V::Password, false);
            registry.add_function(n("HasAccess"), Vec::new(), V::Boolean);
        }
        SURFACE_ID => {
            let surface = V::UserData(SURFACE_ID as u8);
            registry.add_property(n("Width"), V::Integer, false);
            registry.add_property(n("Height"), V::Integer, false);
            registry.add_property(n("Valid"), V::Boolean, false);
            registry.add_named_function(n("Clear"), vec![("rgba", V::Unsigned)], V::Boolean);
            registry.add_named_function(n("SetPixel"), vec![("x", V::Integer), ("y", V::Integer), ("rgba", V::Unsigned)], V::Boolean);
            registry.add_named_function(n("GetPixel"), vec![("x", V::Integer), ("y", V::Integer)], V::Unsigned);
            for name in ["FillRect", "DrawRect"] {
                registry.add_named_function(
                    n(name),
                    vec![
                        ("x", V::Integer),
                        ("y", V::Integer),
                        ("width", V::Integer),
                        ("height", V::Integer),
                        ("rgba", V::Unsigned),
                    ],
                    V::Boolean,
                );
            }
            registry.add_named_function(
                n("Blit"),
                vec![("source", surface), ("destinationX", V::Integer), ("destinationY", V::Integer)],
                V::Boolean,
            );
            registry.add_named_function(
                n("BlitRect"),
                vec![
                    ("source", surface),
                    ("sourceX", V::Integer),
                    ("sourceY", V::Integer),
                    ("width", V::Integer),
                    ("height", V::Integer),
                    ("destinationX", V::Integer),
                    ("destinationY", V::Integer),
                ],
                V::Boolean,
            );
            registry.add_function(n("Present"), Vec::new(), V::Boolean);
            registry.add_named_function(n("PresentAt"), vec![("column", V::Integer), ("row", V::Integer)], V::Boolean);
            registry.add_named_function_with(
                n("PresentRect"),
                vec![
                    ("sourceX", V::Integer),
                    ("sourceY", V::Integer),
                    ("sourceWidth", V::Integer),
                    ("sourceHeight", V::Integer),
                    ("column", V::Integer),
                    ("row", V::Integer),
                    ("destinationWidth", V::Integer),
                    ("destinationHeight", V::Integer),
                    ("flip", V::Integer),
                ],
                4,
                V::Boolean,
            );
            for name in ["Pin", "Unpin", "Free"] {
                registry.add_function(n(name), Vec::new(), V::Boolean);
            }
            registry.add_named_static_function(n("New"), vec![("width", V::Integer), ("height", V::Integer)], surface);
            registry.add_named_static_function(n("Load"), vec![("file", V::UnboundedString)], surface);
        }
        EVENT_ID => {
            registry.add_property(n("Kind"), V::UserData(EVENT_KIND_ENUM_ID), false);
            registry.add_property(n("Code"), V::Integer, false);
            registry.add_property(n("ScanCode"), V::Integer, false);
            registry.add_property(n("Text"), V::UnboundedString, false);
            registry.add_property(n("Pressed"), V::Boolean, false);
            registry.add_property(n("X"), V::Integer, false);
            registry.add_property(n("Y"), V::Integer, false);
            registry.add_property(n("Button"), V::UserData(MOUSE_BUTTON_ENUM_ID), false);
            registry.add_property(n("Pixels"), V::Boolean, false);
            registry.add_property(n("Repeated"), V::Boolean, false);
            registry.add_property(n("WheelX"), V::Integer, false);
            registry.add_property(n("WheelY"), V::Integer, false);
            registry.add_property(n("Time"), V::Unsigned, false);
            registry.add_property(n("Action"), V::UserData(MOUSE_ACTION_ENUM_ID), false);
            registry.add_property(n("Channel"), V::Integer, false);
            registry.add_property(n("Dropped"), V::Integer, false);
            for name in ["LeftDown", "MiddleDown", "RightDown", "Shift", "Alt", "Ctrl", "Meta"] {
                registry.add_property(n(name), V::Boolean, false);
            }
        }
        AUDIO_ID => {
            registry.add_property(n("Valid"), V::Boolean, false);
            registry.add_property(n("Playing"), V::Boolean, false);
            registry.add_property(n("Channel"), V::Integer, false);
            registry.add_named_function(n("SetVolume"), vec![("volume", V::Integer)], V::Boolean);
            registry.add_named_function_with(n("Play"), vec![("looping", V::Boolean)], 0, V::Boolean);
            registry.add_function(n("Stop"), Vec::new(), V::Boolean);
            registry.add_named_function(n("Fade"), vec![("durationMs", V::Integer), ("targetVolume", V::Integer)], V::Boolean);
            registry.add_function(n("Free"), Vec::new(), V::Boolean);
            registry.add_named_static_function(n("Load"), vec![("file", V::UnboundedString)], V::UserData(AUDIO_ID as u8));
            registry.add_static_function(n("StopAll"), Vec::new(), V::Boolean);
        }
        ERROR_ID => {
            registry.add_property(n("OK"), V::Boolean, false);
            registry.add_property(n("Kind"), V::UserData(ERR_KIND_ENUM_ID), false);
            registry.add_property(n("Code"), V::UserData(ERR_CODE_ENUM_ID), false);
            registry.add_property(n("Message"), V::UnboundedString, false);
            registry.add_property(n("Channel"), V::Integer, false);
            registry.add_static_function(n("Last"), Vec::new(), V::UserData(ERROR_ID as u8));
            registry.add_static_function(n("Clear"), Vec::new(), V::Boolean);
        }
        _ => register_remaining_members(id, registry),
    }
}
