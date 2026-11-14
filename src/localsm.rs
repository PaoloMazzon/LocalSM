#![feature(read_le)]
#![cfg_attr(test, allow(unused))]
extern crate core;

use spdlog::prelude::*;
pub mod util;
mod ilog;
mod controller;
use crate::util::config::Config;

pub struct LocalSm {
    logger: Logger,
    config: Config
}

impl LocalSm {
    pub fn init(logger: Logger, config: Config) -> Self {
        debug!(logger: logger, "Config: {:#?}", config);
        info!(logger: logger, "Hello from LocalSM!");
        LocalSm {
            logger,
            config
        }
    }
}
