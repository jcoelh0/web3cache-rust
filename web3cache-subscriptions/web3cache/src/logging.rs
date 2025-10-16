#[macro_export]
macro_rules! custom_info {
    ($($arg:tt)*) => {{
        #[cfg_attr(feature = "exclude_logs_from_coverage", allow(unused))]
        {
            use log::info;
            info!($($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! custom_error {
    ($($arg:tt)*) => {{
        #[cfg_attr(feature = "exclude_logs_from_coverage", allow(unused))]
        {
            use log::error;
            error!($($arg)*);
        }
    }};
}
