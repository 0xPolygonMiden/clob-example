use clap::Parser;
use colored::Colorize;
use log::warn;
use miden_order_book::constants::{ACCOUNTS_DIR, CLOB_DATA_FILE_PATH, DB_FILE_PATH};
use std::{
    fs::{self},
    path::Path,
};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Initialize the order book")]
pub struct InitCmd {}

impl InitCmd {
    pub fn execute(&self) -> Result<(), String> {
        self.remove_file_if_exists(DB_FILE_PATH)?;
        self.remove_file_if_exists(CLOB_DATA_FILE_PATH)?;
        self.remove_folder_if_exists(ACCOUNTS_DIR)?;
        self.print_cool_start_message();
        println!("State successfully initialized.");
        Ok(())
    }

    pub fn remove_file_if_exists(&self, file_path: &str) -> Result<(), String> {
        let path = Path::new(file_path);
        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| format!("Failed to remove file {}: {}", file_path, e))?;
        }
        Ok(())
    }

    fn remove_folder_if_exists(&self, folder_path: &str) -> Result<(), String> {
        let path = Path::new(folder_path);
        if path.exists() && path.is_dir() {
            fs::remove_dir_all(path)
                .map_err(|e| format!("Failed to remove folder {}: {}", folder_path, e))?;
        }
        Ok(())
    }

    fn print_cool_start_message(&self) {
        println!(
            "{}",
            r#"
 __  __ ___ ___  ___  _  _    ___  ___  ___  ___  ___    ___   ___   ___  _  __
|  \/  |_ _|   \| __|| \| |  / _ \| _ \|   \| __|| _ \  | _ ) / _ \ / _ \| |/ /
| |\/| || || |) | _| | .` | | (_) |   /| |) | _| |   /  | _ \| (_) | (_) | ' <
|_|  |_|___|___/|___||_|\_|  \___/|_|_\|___/|___||_|_\  |___/ \___/ \___/|_|\_\

"#
            .bright_cyan()
        );
        println!("{}", "MIDEN ORDER BOOK".bold().green());
        warn!("Fasten your seatbelts! We're about to take off into the world of decentralized finance!");
    }
}
