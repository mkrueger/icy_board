//! The `AUDIO` object a PPE plays through.
//!
//! Like a surface, the value a PPE holds is only the channel the engine handed out.
//! The file it was loaded from lives in the session, so the object stays immutable.

use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableData, VariableValue},
    parser::AUDIO_ID,
};

#[derive(Clone, Copy)]
pub struct PplAudio {
    pub channel: i32,
}

impl PplAudio {
    pub fn value(channel: i32) -> VariableValue {
        let mut value = user_data_value(PplAudio { channel }, AUDIO_ID);
        value.data = VariableData::from_int(channel);
        value
    }

    /// An answer for audio that could not be loaded, so its members stay callable.
    /// Why it failed is `Error.Last()`'s to tell.
    pub fn invalid() -> VariableValue {
        Self::value(-1)
    }
}

pub static VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Valid".to_string()));
pub static PLAYING: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Playing".to_string()));
pub static CHANNEL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Channel".to_string()));
pub static SET_VOLUME: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("SetVolume".to_string()));
pub static PLAY: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Play".to_string()));
pub static STOP: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Stop".to_string()));
pub static FADE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Fade".to_string()));
pub static FREE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Free".to_string()));
pub static LOAD: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Load".to_string()));
pub static STOP_ALL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("StopAll".to_string()));

impl UserData for PplAudio {
    const TYPE_NAME: &'static str = "Audio";
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(PplAudio::invalid);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(AUDIO_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplAudio {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let loaded = vm.icy_board_state.ppl_audio_file(self.channel).is_some();
        if *name == *VALID {
            return Ok(VariableValue::new_bool(loaded));
        }
        if *name == *CHANNEL {
            return Ok(VariableValue::new_int(self.channel));
        }
        if *name == *PLAYING {
            let playing = loaded
                && vm
                    .icy_board_state
                    .sound_active
                    .get(self.channel.unsigned_abs() as usize)
                    .is_some_and(|active| *active);
            return Ok(VariableValue::new_bool(playing));
        }
        log::error!("Invalid user data call on Audio ({name})");
        Ok(VariableValue::new_int(-1))
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("AUDIO property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *LOAD {
            let file_name = arguments.first().map(VariableValue::as_string).unwrap_or_default();
            return crate::vm::statements::predefined_procedures::audio_load(vm, &file_name).await;
        }
        if *name == *STOP_ALL {
            return Ok(VariableValue::new_bool(crate::vm::statements::predefined_procedures::sound_stop_all(vm).await?));
        }
        let handled = crate::vm::statements::predefined_procedures::sound_member(vm, self.channel, name, arguments).await?;
        Ok(VariableValue::new_bool(handled))
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on Audio ({name})");
        Err("Function not found".into())
    }
}
