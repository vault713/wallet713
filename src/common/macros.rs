#[macro_export]
macro_rules! cli_message {
        () => {
            {
                use std::io::Write;
                use crate::common::{is_cli, COLORED_PROMPT};
                if is_cli() {
                    print!("\r{}", COLORED_PROMPT);
                    std::io::stdout().flush().unwrap();
                }
            }
        };

        ($fmt_string:expr, $( $arg:expr ),+) => {
            {
                use std::io::Write;
                use crate::common::{is_cli, COLORED_PROMPT};
                if is_cli() {
                    print!("\r");
                    print!($fmt_string, $( $arg ),*);
                    print!("\n{}", COLORED_PROMPT);
                    std::io::stdout().flush().unwrap();
                } else {
                    info!($fmt_string, $( $arg ),*);
                }
            }
        };

        ($fmt_string:expr) => {
            {
                use std::io::Write;
                use crate::common::{is_cli, COLORED_PROMPT};
                if is_cli() {
                    print!("\r");
                    print!($fmt_string);
                    print!("\n{}", COLORED_PROMPT);
                    std::io::stdout().flush().unwrap();
                } else {
                    info!($fmt_string);
                }
            }
        };
    }
