use ruffle_core::context::UpdateContext;
use ruffle_core::external::Value as ExternalValue;
use ruffle_core::external::{ExternalInterfaceMethod, ExternalInterfaceProvider};

pub mod tests;

#[derive(Default)]
pub struct ExternalInterfaceTestProvider {}

impl ExternalInterfaceTestProvider {
    pub fn new() -> Self {
        Default::default()
    }
}

fn do_trace(context: &mut UpdateContext<'_, '_>, args: &[ExternalValue]) -> ExternalValue {
    context.avm_trace(&format!("[ExternalInterface] trace: {args:?}"));
    "Traced!".into()
}

fn do_ping(context: &mut UpdateContext<'_, '_>, _args: &[ExternalValue]) -> ExternalValue {
    context.avm_trace("[ExternalInterface] ping");
    "Pong!".into()
}

fn do_reentry(context: &mut UpdateContext<'_, '_>, _args: &[ExternalValue]) -> ExternalValue {
    context.avm_trace("[ExternalInterface] starting reentry");
    if let Some(callback) = context.external_interface.get_callback("callWith") {
        callback.call(
            context,
            "callWith",
            vec!["trace".into(), "successful reentry!".into()],
        )
    } else {
        ExternalValue::Null
    }
}

impl ExternalInterfaceProvider for ExternalInterfaceTestProvider {
    fn get_method(&self, name: &str) -> Option<Box<dyn ExternalInterfaceMethod>> {
        match name {
            "trace" => Some(Box::new(do_trace)),
            "ping" => Some(Box::new(do_ping)),
            "reentry" => Some(Box::new(do_reentry)),
            _ => None,
        }
    }

    fn on_callback_available(&self, _name: &str) {}
}
