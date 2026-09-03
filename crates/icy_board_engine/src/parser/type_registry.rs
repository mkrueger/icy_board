use std::{collections::HashMap, sync::RwLock};

use crate::{
    compiler::user_data::{UserData, UserDataRegistry},
    executable::{RecordField, VariableType},
    icy_board::{conferences::Conference, doors::Door, file_directory::FileDirectory, message_area::MessageArea},
};

/// A record a program declared with `TYPE ... ENDTYPE`.
#[derive(Debug, Clone, PartialEq)]
pub struct UserTypeDefinition {
    pub id: usize,
    pub name: unicase::Ascii<String>,
    pub fields: Vec<(unicase::Ascii<String>, RecordField)>,
}

/// An integer-backed type that exists in source only.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDefinition {
    pub id: u8,
    pub name: unicase::Ascii<String>,
    pub variants: Vec<(unicase::Ascii<String>, i32)>,
}

impl EnumDefinition {
    pub fn value(&self, variant: &unicase::Ascii<String>) -> Option<i32> {
        self.variants.iter().find_map(|(name, value)| (name == variant).then_some(*value))
    }

    /// A member standing for `value`, so a lowered value can be written as itself again.
    pub fn variant_name(&self, value: i32) -> Option<&unicase::Ascii<String>> {
        self.variants.iter().find_map(|(name, variant_value)| (*variant_value == value).then_some(name))
    }
}

impl UserTypeDefinition {
    pub fn field_index(&self, name: &unicase::Ascii<String>) -> Option<usize> {
        self.fields.iter().position(|(field, _)| field == name)
    }

    pub fn field_type(&self, index: usize) -> Option<VariableType> {
        self.fields.get(index).map(|(_, field)| field.variable_type)
    }

    pub fn field(&self, index: usize) -> Option<RecordField> {
        self.fields.get(index).map(|(_, field)| *field)
    }
}

#[derive(Default)]
pub struct UserTypeRegistry {
    pub registered_types: HashMap<unicase::Ascii<String>, VariableType>,
    pub types: HashMap<u8, UserDataRegistry>,
    built_in_records: HashMap<u8, UserTypeDefinition>,
    /// Records the compiled program declares. Shared across every file of a
    /// compilation so a type declared in one is visible in the next.
    user_types: RwLock<Vec<UserTypeDefinition>>,
    enums: RwLock<Vec<EnumDefinition>>,
}

pub const FIRST_ID: usize = 30;
pub const CONFERENCE_ID: usize = 30;
pub const MESSAGE_AREA_ID: usize = 31;
pub const FILE_DIRECTORY_ID: usize = 32;
pub const DOOR_ID: usize = 33;
pub const CONTACT_ID: usize = 34;
pub const SURFACE_ID: usize = 35;
pub const EVENT_ID: usize = 36;
pub const AUDIO_ID: usize = 37;
pub const ERROR_ID: usize = 38;
pub const TERM_INFO_ID: usize = 39;
pub const TERM_INPUT_ID: usize = 40;
pub const TERMINAL_ID: usize = 41;
pub const GFX_ID: usize = 42;
pub const MARGINS_ID: usize = 43;
pub const PALETTE_ID: usize = 44;
pub const MACROS_ID: usize = 45;
pub const BOARD_ID: usize = 46;
pub const SESSION_ID: usize = 47;
pub const USER_ID: usize = 48;
pub const MSG_ID: usize = 49;
pub const HTTP_ID: usize = 50;
pub const HTTP_REQUEST_ID: usize = 51;
pub const HTTP_RESPONSE_ID: usize = 52;
pub const REGEX_ID: usize = 53;
pub const REGEX_MATCH_ID: usize = 54;

/// Builtin enums take the top of the id space and a program's own enums grow down from
/// below them. Their current compact order is what a PPE stores.
pub const EVENT_KIND_ENUM_ID: u8 = 255;
pub const MOUSE_ACTION_ENUM_ID: u8 = 254;
pub const MOUSE_BUTTON_ENUM_ID: u8 = 253;
pub const MOUSE_MODE_ENUM_ID: u8 = 252;
pub const MOUSE_TRACKING_ENUM_ID: u8 = 251;
pub const GFX_BACKEND_ENUM_ID: u8 = 250;
pub const ERR_KIND_ENUM_ID: u8 = 249;
pub const ERR_CODE_ENUM_ID: u8 = 248;
pub const EDITOR_MODE_ENUM_ID: u8 = 247;
pub const MSG_FIELD_ENUM_ID: u8 = 246;
pub const HTTP_METHOD_ENUM_ID: u8 = 245;
pub const REGEX_OPTIONS_ENUM_ID: u8 = 244;
pub const STRING_COMPARISON_ENUM_ID: u8 = 243;
pub const CHECKSUM_ENUM_ID: u8 = 242;

/// The board objects are ours, so no `PCBoard` language knows their names.
pub const FIRST_BOARD_OBJECT_LANGUAGE_VERSION: u16 = 400;

/// Types a program declares itself start here, so the board can keep adding
/// objects of its own below without ever running into them.
pub const FIRST_USER_TYPE_ID: usize = 100;

/// How many records one program may declare, ids 100..=255.
/// How many enums the board provides. They sit at the top of the id space, so a program
/// declares that many fewer records of its own.
pub const BUILTIN_ENUM_COUNT: usize = 14;

/// How many records one program may declare, ids 100..=255 less the builtin enums.
pub const MAX_USER_TYPES: usize = u8::MAX as usize - FIRST_USER_TYPE_ID + 1 - BUILTIN_ENUM_COUNT;

/// How many fields one record may hold - the PPE stores the count in a byte.
pub const MAX_TYPE_FIELDS: usize = u8::MAX as usize;

/// True for a type a program declared rather than one the board provides.
pub fn is_user_declared_type(id: u8) -> bool {
    id as usize >= FIRST_USER_TYPE_ID
}

impl UserTypeRegistry {
    pub fn icy_board_registry() -> Self {
        let mut registry = UserTypeRegistry::default();
        registry.register_builtin_enums();
        registry.register::<Conference>(CONFERENCE_ID);
        registry.register::<MessageArea>(MESSAGE_AREA_ID);
        registry.register::<FileDirectory>(FILE_DIRECTORY_ID);
        registry.register::<Door>(DOOR_ID);
        registry.types.get_mut(&(DOOR_ID as u8)).unwrap().empty_value =
            Some(|| crate::compiler::user_data::user_data_value(crate::icy_board::doors::Door::default(), DOOR_ID));
        registry.register_record(
            CONTACT_ID,
            "CONTACT",
            vec![
                (unicase::Ascii::new("Service".to_string()), VariableType::UnboundedString),
                (unicase::Ascii::new("Account".to_string()), VariableType::UnboundedString),
            ],
        );
        registry.register::<crate::icy_board::state::ppl_surface::PplSurface>(SURFACE_ID);
        registry.register::<crate::icy_board::state::ppl_events::PplEvent>(EVENT_ID);
        registry.register::<crate::icy_board::state::ppl_audio::PplAudio>(AUDIO_ID);
        registry.register::<crate::icy_board::state::ppl_error::PplError>(ERROR_ID);
        registry.register::<crate::icy_board::state::ppl_terminal_info::PplTerminalInfo>(TERM_INFO_ID);
        registry.register::<crate::icy_board::state::ppl_terminal_input::PplTerminalInput>(TERM_INPUT_ID);
        registry.register::<crate::icy_board::state::ppl_terminal::PplTerminal>(TERMINAL_ID);
        registry.register::<crate::icy_board::state::ppl_gfx::PplGfx>(GFX_ID);
        registry.register::<crate::icy_board::state::ppl_margins::PplMargins>(MARGINS_ID);
        registry.register::<crate::icy_board::state::ppl_palette::PplPalette>(PALETTE_ID);
        registry.register::<crate::icy_board::state::ppl_macros::PplMacros>(MACROS_ID);
        registry.register::<crate::icy_board::state::ppl_board::PplBoard>(BOARD_ID);
        registry.register::<crate::icy_board::state::ppl_session::PplSession>(SESSION_ID);
        registry.register::<crate::icy_board::state::ppl_user::PplUser>(USER_ID);
        registry.register::<crate::icy_board::state::ppl_message::PplMessage>(MSG_ID);
        registry.register::<crate::icy_board::state::ppl_http::PplHttp>(HTTP_ID);
        registry.register::<crate::icy_board::state::ppl_http::PplHttpRequest>(HTTP_REQUEST_ID);
        registry.register::<crate::icy_board::state::ppl_http::PplHttpResponse>(HTTP_RESPONSE_ID);
        registry.register::<crate::icy_board::state::ppl_regex::PplRegex>(REGEX_ID);
        registry.register::<crate::icy_board::state::ppl_regex::PplRegexMatch>(REGEX_MATCH_ID);

        registry
    }

    pub fn module_type_name(module: &unicase::Ascii<String>, name: &unicase::Ascii<String>) -> unicase::Ascii<String> {
        unicase::Ascii::new(format!("__T_{}_{}", module.as_str(), name.as_str()))
    }

    pub fn get_type(&self, identifier: &unicase::Ascii<String>) -> Option<VariableType> {
        self.get_board_object(identifier).or_else(|| self.get_declared_type(identifier))
    }

    /// One of the objects the board itself provides, such as a conference.
    pub fn get_board_object(&self, identifier: &unicase::Ascii<String>) -> Option<VariableType> {
        self.registered_types.get(identifier).copied()
    }

    /// A record or an enum the program declared for itself.
    pub fn get_declared_type(&self, identifier: &unicase::Ascii<String>) -> Option<VariableType> {
        self.get_user_type(identifier)
            .map(|definition| VariableType::UserData(definition.id as u8))
            .or_else(|| self.get_enum(identifier).map(|definition| VariableType::UserData(definition.id)))
    }

    pub fn get_module_declared_type(&self, module: Option<&unicase::Ascii<String>>, identifier: &unicase::Ascii<String>) -> Option<VariableType> {
        module
            .and_then(|module| self.get_declared_type(&Self::module_type_name(module, identifier)))
            .or_else(|| self.get_declared_type(identifier))
    }

    /// The record declared under `identifier`, if the program declared one.
    pub fn get_user_type(&self, identifier: &unicase::Ascii<String>) -> Option<UserTypeDefinition> {
        self.user_types
            .read()
            .unwrap()
            .iter()
            .find(|definition| definition.name == *identifier)
            .cloned()
    }

    pub fn get_user_type_from_id(&self, id: u8) -> Option<UserTypeDefinition> {
        let id = id as usize;
        if id < FIRST_USER_TYPE_ID {
            return None;
        }
        self.user_types.read().unwrap().get(id - FIRST_USER_TYPE_ID).cloned()
    }

    pub fn get_record_type_from_id(&self, id: u8) -> Option<UserTypeDefinition> {
        self.built_in_records.get(&id).cloned().or_else(|| self.get_user_type_from_id(id))
    }

    pub fn is_record_type(&self, id: u8) -> bool {
        self.built_in_records.contains_key(&id) || is_user_declared_type(id)
    }

    pub fn user_types(&self) -> Vec<UserTypeDefinition> {
        self.user_types.read().unwrap().clone()
    }

    pub fn get_enum(&self, identifier: &unicase::Ascii<String>) -> Option<EnumDefinition> {
        self.enums.read().unwrap().iter().find(|definition| definition.name == *identifier).cloned()
    }

    pub fn enums(&self) -> Vec<EnumDefinition> {
        self.enums.read().unwrap().clone()
    }

    pub fn get_enum_from_id(&self, id: u8) -> Option<EnumDefinition> {
        self.enums.read().unwrap().iter().find(|definition| definition.id == id).cloned()
    }

    pub fn is_enum_type(&self, variable_type: VariableType) -> bool {
        matches!(variable_type, VariableType::UserData(id) if self.get_enum_from_id(id).is_some())
    }

    /// Records grow upward from 100, enums downward from 255; neither kind is
    /// serialized under the other's representation.
    pub fn declare_enum(&self, name: unicase::Ascii<String>, variants: Vec<(unicase::Ascii<String>, i32)>) -> Option<u8> {
        let mut enums = self.enums.write().unwrap();
        let id = u8::MAX as usize - enums.len();
        let next_record = FIRST_USER_TYPE_ID + self.user_types.read().unwrap().len();
        if id < next_record {
            return None;
        }
        enums.push(EnumDefinition { id: id as u8, name, variants });
        Some(id as u8)
    }

    /// Claims one of the fixed ids at the top of the space for an enum the board provides.
    fn register_enum(&self, id: u8, name: &str, variants: &[(&str, i32)]) {
        let mut enums = self.enums.write().unwrap();
        assert_eq!(
            id as usize,
            u8::MAX as usize - enums.len(),
            "builtin enum '{name}' wants id {id}, which is not the next one free"
        );
        enums.push(EnumDefinition {
            id,
            name: unicase::Ascii::new(name.to_string()),
            variants: variants
                .iter()
                .map(|(variant, value)| (unicase::Ascii::new((*variant).to_string()), *value))
                .collect(),
        });
    }

    /// The enums the board provides. Values match what the runtime already reports, so
    /// naming one is a way of writing the number rather than a change of meaning.
    fn register_builtin_enums(&self) {
        self.register_enum(
            EVENT_KIND_ENUM_ID,
            "EventKind",
            &[("None", 0), ("Key", 1), ("KeyEdge", 2), ("Mouse", 3), ("Overflow", 4), ("Audio", 5)],
        );
        self.register_enum(
            MOUSE_ACTION_ENUM_ID,
            "MouseAction",
            &[("None", 0), ("Press", 1), ("Release", 2), ("Motion", 3), ("Wheel", 4)],
        );
        // A button names itself; the buttons a caller is holding are separate booleans,
        // because a set of them cannot be one of these.
        self.register_enum(
            MOUSE_BUTTON_ENUM_ID,
            "MouseButton",
            &[
                ("None", -1),
                ("Left", 0),
                ("Middle", 1),
                ("Right", 2),
                ("WheelUp", 3),
                ("WheelDown", 4),
                ("WheelLeft", 5),
                ("WheelRight", 6),
            ],
        );
        self.register_enum(MOUSE_MODE_ENUM_ID, "MouseMode", &[("Text", 0), ("Pixels", 1)]);
        self.register_enum(MOUSE_TRACKING_ENUM_ID, "MouseTracking", &[("Buttons", 0), ("Drag", 1), ("All", 2)]);
        self.register_enum(GFX_BACKEND_ENUM_ID, "GfxBackend", &[("None", -1), ("Auto", 0), ("Sixel", 2), ("Jxl", 3)]);
        self.register_enum(
            ERR_KIND_ENUM_ID,
            "ErrKind",
            &[
                ("None", 0),
                ("File", 1),
                ("DBase", 2),
                ("Stack", 3),
                ("Gfx", 4),
                ("Font", 5),
                ("Audio", 6),
                ("Term", 7),
                ("Msg", 8),
                ("Net", 9),
                ("User", 10),
                ("String", 11),
                ("Regex", 12),
            ],
        );
        self.register_enum(
            ERR_CODE_ENUM_ID,
            "ErrCode",
            &[
                ("Ok", 0),
                ("Unavailable", 1),
                ("Invalid", 2),
                ("Io", 3),
                ("Format", 4),
                ("Limit", 5),
                ("Unsupported", 6),
                ("Stack", 7),
                ("Denied", 8),
                ("Timeout", 9),
            ],
        );
        self.register_enum(EDITOR_MODE_ENUM_ID, "EditorMode", &[("Yes", 0), ("No", 1), ("Ask", 2)]);
        self.register_enum(MSG_FIELD_ENUM_ID, "MsgField", &[("To", 0x07), ("From", 0x0B), ("Subject", 0x0C)]);
        self.register_enum(
            HTTP_METHOD_ENUM_ID,
            "HttpMethod",
            &[("Get", 0), ("Head", 1), ("Post", 2), ("Put", 3), ("Delete", 4), ("Patch", 5)],
        );
        self.register_enum(
            REGEX_OPTIONS_ENUM_ID,
            "RegexOptions",
            &[
                ("None", 0),
                ("IgnoreCase", 1),
                ("MultiLine", 2),
                ("DotMatchesNewLine", 4),
                ("IgnoreWhitespace", 8),
                ("SwapGreed", 16),
                ("Ascii", 32),
            ],
        );
        self.register_enum(STRING_COMPARISON_ENUM_ID, "StringComparison", &[("Ordinal", 0), ("OrdinalIgnoreCase", 1)]);
        self.register_enum(CHECKSUM_ENUM_ID, "Checksum", &[("CRC32", 0), ("MD5", 1), ("SHA256", 2)]);
    }

    /// The position of a field inside a record, which doubles
    /// as its member id in the generated code.
    pub fn record_field_index(&self, id: u8, field: &unicase::Ascii<String>) -> Option<usize> {
        self.get_record_type_from_id(id)?.field_index(field)
    }

    /// Adds a record and hands back its type id, or `None` when the id space is full.
    pub fn declare_user_type(&self, name: unicase::Ascii<String>, fields: Vec<(unicase::Ascii<String>, RecordField)>) -> Option<usize> {
        let mut user_types = self.user_types.write().unwrap();
        let id = FIRST_USER_TYPE_ID + user_types.len();
        let lowest_enum = self
            .enums
            .read()
            .unwrap()
            .last()
            .map_or(u8::MAX as usize + 1, |definition| definition.id as usize);
        if id >= lowest_enum {
            return None;
        }
        user_types.push(UserTypeDefinition { id, name, fields });
        Some(id)
    }

    /// Every PPE stores the type id, so an id that is taken can never move.
    fn claim_id(&self, id: usize, name: &str) {
        assert!(
            (FIRST_ID..FIRST_USER_TYPE_ID).contains(&id),
            "board object '{name}' wants id {id}, which is outside the board object range"
        );
        assert!(
            !self.types.contains_key(&(id as u8)) && !self.built_in_records.contains_key(&(id as u8)),
            "board object '{name}' wants id {id}, which is already taken"
        );
    }

    pub fn register<T: UserData>(&mut self, id: usize) {
        self.claim_id(id, T::TYPE_NAME);
        let mut registry = UserDataRegistry {
            instance_provider: T::INSTANCE_PROVIDER,
            static_receiver: T::STATIC_RECEIVER,
            empty_value: T::EMPTY_VALUE,
            ..Default::default()
        };
        T::register_members(&mut registry);
        self.registered_types
            .insert(unicase::Ascii::new(T::TYPE_NAME.to_string()), VariableType::UserData(id as u8));
        self.types.insert(id as u8, registry);
    }

    fn register_record(&mut self, id: usize, name: &str, fields: Vec<(unicase::Ascii<String>, VariableType)>) {
        self.claim_id(id, name);
        self.registered_types
            .insert(unicase::Ascii::new(name.to_string()), VariableType::UserData(id as u8));
        self.built_in_records.insert(
            id as u8,
            UserTypeDefinition {
                id,
                name: unicase::Ascii::new(name.to_string()),
                fields: fields
                    .into_iter()
                    .map(|(name, variable_type)| (name, RecordField::scalar(variable_type)))
                    .collect(),
            },
        );
    }

    pub fn get_type_from_id(&self, id: u8) -> Option<&UserDataRegistry> {
        self.types.get(&id)
    }
}
