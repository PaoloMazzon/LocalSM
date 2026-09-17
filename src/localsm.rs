extern crate core;

pub mod util;
use spdlog::prelude::*;
use crate::util::config::Config;

pub struct LocalSm {
    logger: Logger,
    config: Config
}

impl LocalSm {
    pub fn init(logger: Logger, config: Config) -> Self {
        info!(logger: logger, "Hello from LocalSM!");
        LocalSm {
            logger,
            config
        }
    }
}