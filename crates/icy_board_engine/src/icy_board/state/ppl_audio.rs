//! The `AUDIO` object a PPE plays through.
//!
//! Like a surface, the value a PPE holds is only the channel the engine handed out.
//! The file it was loaded from lives in the session, so the object stays immutable.

use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{GenericVariableData, VariableData, VariableType, VariableValue},
    parser::AUDIO_ID,
};

#[derive(Clone, Copy)]
pub struct PplAudio {
    pub channel: i32,
}

impl PplAudio {
    pub fn value(channel: i32) -> VariableValue {
        VariableValue {
            vtype: VariableType::UserData(AUDIO_ID as u8),
            data: VariableData::from_int(channel),
            generic_data: GenericVariableData::UserData(std::sync::Arc::new(PplAudio { channel })),
        }
    }

    /// An answer for audio that could not be loaded, so its members stay callable.
    /// Why it failed is `ERR()`'s to tell.
    pub fn invalid() -> VariableValue {
        VariableValue {
            vtype: VariableType::UserData(AUDIO_ID as u8),
            data: VariableData::from_int(-1),
            generic_data: GenericVariableData::UserData(std::sync::Arc::new(PplAudio { channel: -1 })),
        }
    }
}

pub static VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Valid".to_string()));
pub static PLAYING: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Playing".to_string()));
pub static VOLUME: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Volume".to_string()));
pub static CHANNEL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Channel".to_string()));
pub static PLAY: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Play".to_string()));
pub static STOP: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Stop".to_string()));
pub static SET_VOLUME: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("SetVolume".to_string()));
pub static FADE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Fade".to_string()));
pub static FREE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Free".to_string()));

impl UserData for PplAudio {
    const TYPE_NAME: &'static str = "Audio";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(VALID.clone(), VariableType::Boolean, false);
        registry.add_property(PLAYING.clone(), VariableType::Boolean, false);
        registry.add_property(VOLUME.clone(), VariableType::Integer, false);
        registry.add_property(CHANNEL.clone(), VariableType::Integer, false);

        // Looping is the only thing a play has to be told, and it may be left out.
        registry.add_function_with(PLAY.clone(), vec![VariableType::Boolean], 0, VariableType::Boolean);
        registry.add_function(STOP.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(SET_VOLUME.clone(), vec![VariableType::Integer], VariableType::Boolean);
        registry.add_function(FADE.clone(), vec![VariableType::Integer, VariableType::Integer], VariableType::Boolean);
        registry.add_function(FREE.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait]
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
        if *name == *VOLUME {
            let volume = vm.icy_board_state.sound_volume.get(self.channel.unsigned_abs() as usize).copied().unwrap_or(0);
            return Ok(VariableValue::new_int(if loaded { volume } else { 0 }));
        }
        log::error!("Invalid user data call on Audio ({name})");
        Ok(VariableValue::new_int(-1))
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Ok(())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        let handled = crate::vm::statements::predefined_procedures::sound_member(vm, self.channel, name, arguments).await?;
        Ok(VariableValue::new_bool(handled))
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on Audio ({name})");
        Err("Function not found".into())
    }
}
