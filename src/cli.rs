use std::{error::Error, marker::PhantomData};

use crate::{
    platform::Manager,
    service_manager::{Service, ServiceHandler, ServiceManager},
};

pub struct Cli<'a, H>
where
    H: Service + ServiceHandler,
{
    _phantom: PhantomData<H>,
    manager: Manager,
    cmd: clap::Command<'a>,
}

impl<'a, H> Cli<'a, H>
where
    H: Service + ServiceHandler,
{
    pub fn new(manager: Manager) -> Self {
        let cmd = clap::Command::new(manager.display_name())
            .subcommand(clap::command!("install"))
            .subcommand(clap::command!("uninstall"))
            .subcommand(clap::command!("status"))
            .subcommand(clap::command!("start"))
            .subcommand(clap::command!("run"))
            .subcommand(clap::command!("stop"));
        Self {
            manager,
            cmd,
            _phantom: PhantomData::default(),
        }
    }

    #[maybe_async::maybe_async]
    pub async fn handle_input(self) -> Result<(), Box<dyn Error>> {
        let matches = self.cmd.get_matches();

        match matches.subcommand() {
            Some(("install", _)) => self.manager.install(),
            Some(("uninstall", _)) => self.manager.uninstall(),
            Some(("status", _)) => {
                println!("{:?}", self.manager.query_status()?);
                Ok(())
            }
            Some(("start", _)) => self.manager.start(),
            Some(("stop", _)) => self.manager.stop(),
            Some(("run", _)) => {
                H::run_service_main().await;
                Ok(())
            }
            Some((_, _)) => Ok(()),
            None => {
                #[cfg(feature = "direct")]
                {
                    let handler = H::new();
                    handler.run_service_direct().await;
                }

                Ok(())
            }
        }
    }
}
