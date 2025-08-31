use crate::error::Result;
use crate::utils::{deserialize, serialize};
use crate::wallet::Wallet;
use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};

pub const WALLET_FILE: &str = "wallet.dat";

pub struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Default for Wallets {
    fn default() -> Self {
        Self::new()
    }
}

impl Wallets {
    pub fn new() -> Wallets {
        let mut wallets = Wallets {
            wallets: HashMap::new(),
        };
        wallets.load_from_file();
        wallets
    }

    pub fn create_wallet(&mut self) -> Result<String> {
        let wallet = Wallet::new()?;
        let address = wallet.get_address();
        self.wallets.insert(address.clone(), wallet);
        self.save_to_file();
        Ok(address)
    }

    pub fn get_addresses(&self) -> Vec<String> {
        let mut addresses = vec![];
        for address in self.wallets.keys() {
            addresses.push(address.clone())
        }
        addresses
    }

    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        if let Some(wallet) = self.wallets.get(address) {
            return Some(wallet);
        }
        None
    }

    pub fn load_from_file(&mut self) {
        // Ignore errors during wallet loading - just start with empty wallet set
        if let Err(e) = self.load_from_file_safe() {
            log::warn!("Could not load wallets from file: {e}");
        }
    }

    fn load_from_file_safe(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let path = current_dir()?.join(WALLET_FILE);
        if !path.exists() {
            return Ok(());
        }

        let mut file = File::open(path)?;
        let metadata = file.metadata()?;
        let mut buf = vec![0; metadata.len() as usize];
        file.read_exact(&mut buf)?;
        let wallets = deserialize(&buf[..])?;
        self.wallets = wallets;
        Ok(())
    }

    fn save_to_file(&self) {
        // Ignore errors during wallet saving - log but don't crash
        if let Err(e) = self.save_to_file_safe() {
            log::error!("Could not save wallets to file: {e}");
        }
    }

    fn save_to_file_safe(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let path = current_dir()?.join(WALLET_FILE);
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)?;
        let mut writer = BufWriter::new(file);
        let wallets_bytes = serialize(&self.wallets)?;
        writer.write_all(wallets_bytes.as_slice())?;
        writer.flush()?;
        Ok(())
    }
}
