use spdlog::LevelFilter;
use spdlog::sink::StdStreamSink;
use spdlog::terminal_style::StyleMode;
use localsm::LocalSm;
use localsm::util::config::Config;

#[test]
fn full_integration() {
    let logger = spdlog::Logger::builder()
        .name("localsm")
        .sink(StdStreamSink::builder().stdout().style_mode(StyleMode::Always).build_arc().unwrap())
        .level_filter(LevelFilter::All)
        .build().unwrap();

    LocalSm::init(logger, Config::default());
}
