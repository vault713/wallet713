// Copyright 2019 The vault713 Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
                    log::info!($fmt_string, $( $arg ),*);
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
                    log::info!($fmt_string);
                }
            }
        };
    }
