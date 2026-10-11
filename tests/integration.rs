use spdlog::{LevelFilter, Logger};
use spdlog::sink::StdStreamSink;
use spdlog::terminal_style::StyleMode;
use localsm::LocalSm;
use localsm::util::config::Config;

fn get_logger() -> Logger {
    Logger::builder()
        .name("localsm")
        .sink(StdStreamSink::builder().stdout().style_mode(StyleMode::Always).build_arc().unwrap())
        .level_filter(LevelFilter::All)
        .build().unwrap()
}

#[test]
fn full_integration() {
    LocalSm::init(get_logger(), Config::default());
}
