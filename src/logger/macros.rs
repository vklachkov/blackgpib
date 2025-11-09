#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::logger::Logger::log(
            $crate::logger::get_logger(),
            std::time::Instant::now(),
            file!(),
            line!(),
            $level,
            format_args!($($arg)*)
        );
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::log!($crate::logger::LogLevel::Trace, $($arg)*);
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::log!($crate::logger::LogLevel::Debug, $($arg)*);
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log!($crate::logger::LogLevel::Info, $($arg)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log!($crate::logger::LogLevel::Warn, $($arg)*);
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log!($crate::logger::LogLevel::Error, $($arg)*);
    };
}
