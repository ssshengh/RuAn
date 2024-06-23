use std::fmt;
use ::log::{LevelFilter, Log, Metadata, Record};

use env_logger::{Builder, Logger, Target};
use log::{info, SetLoggerError};

/// 外部传入的函数指针，用于 SDK 调用类似安卓 logcat 之类的功能来打印日志
pub type SdkLogMessage = Option<Box<dyn Fn(&str, log::Level, &str) + Send + Sync>>;

struct SdkLogger {
    callback: SdkLogMessage,
    logger: Logger,
}

impl SdkLogger {
    fn log_message(&self, category: &str, level: log::Level, msg: &str) {
        if let Some(cb) = self.callback.as_ref() {
            cb(category, level, msg)
        } else {
            eprint!("Callback logger should be set")
        }
    }
}

impl Log for SdkLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.logger.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        let category = "sdk";

        let mut message = if let (Some(file), Some(line)) = (record.file(), record.line()) {
            format!("[{file}:{line}] ")
        } else {
            format!("[{}]", record.target())
        };

        if let Err(e) = fmt::write(&mut message, *record.args()) {
            eprintln!("Can not format log, error: {e}")
        }

        self.log_message(category, record.level(), &message);
    }

    fn flush(&self) {}
}

/// 外部注入日志打印回调时，使用这个方法，里面没有设置输出 target
fn create_logger(config_str: &str) -> Logger {
    let mut builder = Builder::new();

    if !config_str.is_empty() {
        // 优先使用参数传入的过滤器
        builder.parse_filters(config_str);
    } else if let Ok(var_str) = std::env::var("SDK_LOG") {
        // 次优使用环境变量设置的过滤器
        builder.parse_filters(&var_str);
    } else {
        // 默认过滤器
        builder.filter(Some("core"), LevelFilter::Info);
        builder.filter(Some("logging"), LevelFilter::Info);
        builder.filter(Some("sdk-java"), LevelFilter::Info);
        builder.filter(Some("sdk-li"), LevelFilter::Info);
        builder.filter(None, LevelFilter::Warn);
    }

    builder.build()
}

fn init_env_logger(filter_str: &str) -> Result<(), SetLoggerError> {
    let mut builder = env_logger::Builder::new();
    builder.target(Target::Stdout);

    if !filter_str.is_empty() {
        // 优先使用参数传入的过滤器
        builder.parse_filters(filter_str);
    } else if let Ok(var_str) = std::env::var("SDK_LOG") {
        builder.parse_filters(&var_str);
    } else {
        builder.filter(Some("core"), LevelFilter::Info);
        builder.filter(Some("logging"), LevelFilter::Info);
        builder.filter(Some("sdk-java"), LevelFilter::Info);
        builder.filter(Some("sdk-li"), LevelFilter::Info);
        builder.filter(None, LevelFilter::Warn);
    };
    builder.try_init()
}

pub fn init(callback: SdkLogMessage) -> Result<(), SetLoggerError> {
    init_with_config(callback, false, "")
}

/// 使用 stdout 的版本
pub fn init_with_config(
    callback: SdkLogMessage,
    force_stdout: bool,
    filter_string: &str,
) -> Result<(), SetLoggerError> {
    let force_stdout = std::env::var("SDK_LOG_STDOUT").is_ok() || force_stdout;

    match (&callback, force_stdout) {
        (Some(_), false) => {
            let log_backend: Box<SdkLogger> = Box::new(SdkLogger {
                callback,
                logger: create_logger(filter_string),
            });
            log::set_max_level(log_backend.logger.filter());
            log::set_boxed_logger(log_backend)?;
            info!("Using custom callback logger");
        }
        _ => {
            init_env_logger(filter_string)?;
            info!("Using evn_logger for debugging");
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use crossbeam::channel::unbounded;
    use super::{init, init_with_config, SdkLogMessage};

    /// 测试外部注入日志打印回调，且不使用 stdout 的版本
    #[test]
    fn test_log() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = unbounded();

        let wb_log_message: SdkLogMessage = Some(Box::new(
            move |category: &str, level: log::Level, msg: &str| {
                tx.send((category.to_string(), level, msg.to_string()))
                    .unwrap()
            },
        ));

        init(wb_log_message)?;

        while rx.try_recv().is_ok() {}
        log::info!("LOG MESSAGE FROM WB LOGGER");

        let (category, level, msg) = rx.recv_timeout(Duration::from_millis(2))?;

        assert_eq!(category, "sdk");
        assert_eq!(level, log::Level::Info);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(
            msg,
            "[logging/src/lib.rs:140] LOG MESSAGE FROM WB LOGGER"
        );
        #[cfg(target_os = "windows")]
        assert_eq!(
            msg,
            "[logging\\src\\lib.rs:140] LOG MESSAGE FROM WB LOGGER"
        );
        Ok(())
    }

    /// 测试使用 stdout 的情况，此时应该会打印日志
    /// Using evn_logger for debugging
    /// LOG MESSAGE FROM WB LOGGER
    #[test]
    fn test_std_out() -> Result<(), Box<dyn std::error::Error>> {
        let wb_log_message: SdkLogMessage = Some(Box::new(
            move |category: &str, level: log::Level, msg: &str| {
                println!("category={}, level={}, msg={}", category, level, msg)
            },
        ));

        init_with_config(
            wb_log_message, true, ""
        )?;

        log::info!("LOG MESSAGE FROM WB LOGGER");

        Ok(())
    }
}
