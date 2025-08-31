use clap::{Parser, Subcommand};
use std::str::FromStr;

/// Fee priority levels for transactions
#[derive(Debug, Clone, Copy)]
pub enum FeePriorityArg {
    Low,
    Normal,
    High,
    Urgent,
}

impl FromStr for FeePriorityArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(FeePriorityArg::Low),
            "normal" => Ok(FeePriorityArg::Normal),
            "high" => Ok(FeePriorityArg::High),
            "urgent" => Ok(FeePriorityArg::Urgent),
            _ => Err(format!(
                "Invalid priority: {s}. Valid options: low, normal, high, urgent"
            )),
        }
    }
}

impl std::fmt::Display for FeePriorityArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeePriorityArg::Low => write!(f, "low"),
            FeePriorityArg::Normal => write!(f, "normal"),
            FeePriorityArg::High => write!(f, "high"),
            FeePriorityArg::Urgent => write!(f, "urgent"),
        }
    }
}

/// Fee mode for configuration
#[derive(Debug, Clone)]
pub enum FeeModeArg {
    Fixed(u64),
    Dynamic,
}

impl FromStr for FeeModeArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.to_lowercase() == "dynamic" {
            Ok(FeeModeArg::Dynamic)
        } else if let Ok(amount) = s.parse::<u64>() {
            Ok(FeeModeArg::Fixed(amount))
        } else {
            Err(format!(
                "Invalid fee mode: {s}. Use 'dynamic' or a fixed amount (e.g., '1')"
            ))
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "architect-chain")]
pub struct Opt {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(name = "createblockchain", about = "Create a new blockchain")]
    Createblockchain {
        #[arg(help = "The address to send genesis block reward to")]
        address: String,
    },
    #[command(name = "createwallet", about = "Create a new wallet")]
    Createwallet,
    #[command(
        name = "getbalance",
        about = "Get the wallet balance of the target address"
    )]
    GetBalance {
        #[arg(help = "The wallet address")]
        address: String,
    },
    #[command(name = "listaddresses", about = "Print local wallet addresses")]
    ListAddresses,
    #[command(name = "send", about = "Send transaction between addresses")]
    Send {
        #[arg(help = "Source wallet address")]
        from: String,
        #[arg(help = "Destination wallet address")]
        to: String,
        #[arg(help = "Amount to send (in satoshis)")]
        amount: u64,
        #[arg(help = "Mine immediately on the same node")]
        mine: usize,
        #[arg(
            long = "priority",
            help = "Transaction priority (low, normal, high, urgent)"
        )]
        priority: Option<FeePriorityArg>,
    },
    #[command(name = "printchain", about = "Print all blocks in the blockchain")]
    Printchain,
    #[command(name = "reindexutxo", about = "Rebuild UTXO index set")]
    Reindexutxo,
    #[command(name = "startnode", about = "Start a blockchain node")]
    StartNode {
        #[arg(help = "Enable mining mode and send reward to ADDRESS")]
        miner: Option<String>,
    },
    #[command(
        name = "estimatefee",
        about = "Estimate transaction fee for given priority"
    )]
    EstimateFee {
        #[arg(help = "Transaction priority (low, normal, high, urgent)")]
        priority: FeePriorityArg,
    },
    #[command(name = "feestatus", about = "Show current fee system status")]
    FeeStatus,
    #[command(name = "setfeemode", about = "Set fee calculation mode")]
    SetFeeMode {
        #[arg(help = "Fee mode: 'dynamic' or fixed amount (e.g., '1')")]
        mode: FeeModeArg,
    },
}
