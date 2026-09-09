pub use icy_board_ppl::compiler::user_data::*;

#[cfg(feature = "bbs")]
pub struct RuntimeUserData(pub std::sync::Arc<dyn UserDataValue>);

#[cfg(feature = "bbs")]
pub fn user_data_value<T: UserDataValue + 'static>(value: T, type_id: usize) -> crate::executable::VariableValue {
    icy_board_ppl::compiler::user_data::user_data_value(RuntimeUserData(std::sync::Arc::new(value)), type_id)
}

#[cfg(feature = "bbs")]
#[async_trait::async_trait(?Send)]
pub trait UserDataValue: Send + Sync {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<crate::executable::VariableValue>;
    async fn set_property_value(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        val: crate::executable::VariableValue,
    ) -> crate::Res<()>;
    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[crate::executable::VariableValue],
    ) -> crate::Res<crate::executable::VariableValue>;
    async fn call_method(
        &mut self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[crate::executable::VariableValue],
    ) -> crate::Res<()>;
}

#[cfg(feature = "bbs")]
pub(crate) fn runtime_object(object: &(dyn std::any::Any + Send + Sync), type_id: u8) -> crate::Res<&dyn UserDataValue> {
    object
        .downcast_ref::<RuntimeUserData>()
        .map(|value| value.0.as_ref())
        .ok_or_else(|| crate::vm::VMError::NoObjectFound(type_id).into())
}
