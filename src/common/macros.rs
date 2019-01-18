#[macro_export]
macro_rules! cli_message {
        () => {
            unsafe {
                use std::io::Write;
                use crate::common::is_cli;
                if is_cli() {
                    print!("\r{}", "wallet713> ");
                    std::io::stdout().flush().unwrap();
                }
            }
        };

        ($fmt_string:expr, $( $arg:expr ),+) => {
            unsafe {
                use std::io::Write;
                use crate::common::is_cli;
                if is_cli() {
                    print!("\r");
                    print!($fmt_string, $( $arg ),*);
                    print!("\n{}", "wallet713> ");
                    std::io::stdout().flush().unwrap();
                } else {
                    info!($fmt_string, $( $arg ),*);
                }
            }
        };

        ($fmt_string:expr) => {
            unsafe {
                use std::io::Write;
                use crate::common::is_cli;
                if is_cli() {
                    print!("\r");
                    print!($fmt_string);
                    print!("\n{}", "wallet713> ");
                    std::io::stdout().flush().unwrap();
                } else {
                    info!($fmt_string);
                }
            }
        };
    }
