use color_eyre::config::Theme;
use std::error::Error;
use tracing::error;

pub use color_eyre;

pub enum PanicBehavior {
    Print,
    Log,
}

impl Default for PanicBehavior {
    fn default() -> Self {
        Self::Log
    }
}

#[derive(Default)]
pub struct ErrorHandler {
    theme: Theme,
    panic_behavior: PanicBehavior,
}

impl ErrorHandler {
    pub fn with_theme(self, theme: Theme) -> Self {
        Self { theme, ..self }
    }

    pub fn with_panic_behavior(self, panic_behavior: PanicBehavior) -> Self {
        Self {
            panic_behavior,
            ..self
        }
    }

    pub fn install(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
            .add_default_filters()
            .capture_span_trace_by_default(false)
            .theme(self.theme)
            .into_hooks();

        eyre_hook.install()?;
        let panic_behavior = self.panic_behavior;
        std::panic::set_hook(Box::new(move |pi| {
            match panic_behavior {
                PanicBehavior::Log => error!("{}", panic_hook.panic_report(pi)),
                PanicBehavior::Print => eprintln!("{}", panic_hook.panic_report(pi)),
            };
        }));
        Ok(())
    }
}
