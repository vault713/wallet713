pub mod error;
pub mod config;
pub mod base58;
pub mod crypto;
pub use self::error::Error;

macro_rules! cli_message {
    () => {
        {
            use std::io::Write;
            use colored::*;
            print!("\r{}", "wallet713> ".bright_green());
            std::io::stdout().flush().unwrap();
        }
    };

    ($fmt_string:expr, $( $arg:expr ),+) => {
        {
            use std::io::Write;
            use colored::*;
            print!("\r");
            print!($fmt_string, $( $arg ),*);
            print!("\n{}", "wallet713> ".bright_green());
            std::io::stdout().flush().unwrap();
        }
    };

    ($fmt_string:expr) => {
        {
            use std::io::Write;
            use colored::*;
            print!("\r");
            print!($fmt_string);
            print!("\n{}", "wallet713> ".bright_green());
            std::io::stdout().flush().unwrap();
        }
    };

}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
    }
}