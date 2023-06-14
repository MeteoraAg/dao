use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::Cluster;
use clap::*;

#[derive(Parser, Debug)]
pub struct ConfigOverride {
    /// Cluster override
    ///
    /// Values = Mainnet, Testnet, Devnet, Localnet.
    /// Default: Devnet
    #[clap(global = true, short, long, default_value_t = Cluster::Devnet)]
    pub cluster: Cluster,
    /// Wallet override
    ///
    /// Example: /path/to/wallet/keypair.json
    /// Default: ~/.config/solana/id.json
    #[clap(
        global = true,
        short,
        long,
        default_value_t = String::from(shellexpand::tilde("~/.config/solana/id.json"))
    )]
    pub wallet_path: String,

    /// Base keypair file required to initialize the vault
    ///
    /// /path/to/base/keypair.json
    #[clap(global = true, short, long)]
    pub base_path: Option<String>,

    #[clap(
    global = true,
    short,
    long,
    default_value_t = minter::id().to_string()
    )]
    pub program_id: String,
}

#[derive(Parser, Debug)]
pub enum CliCommand {
    SetMintAuthority {
        #[clap(long)]
        token_mint: Pubkey,
    },

    NewMintWrapper {
        #[clap(long)]
        hard_cap: u64,
        #[clap(long)]
        token_mint: Pubkey,
    },
    NewMinterForRewarder {},
    NewMinter {
        #[clap(long)]
        minter_authority: Pubkey,
    },
    TransferAdmin {
        #[clap(long)]
        next_admin: Pubkey,
    },
    AcceptAdmin {},
    SetAllowance {
        #[clap(long)]
        allowance: u64,
        #[clap(long)]
        minter_authority: Pubkey,
    },
    SetAllowanceForRewarder {
        #[clap(long)]
        allowance: u64,
    },
    ViewMintWrapper {},
}

#[derive(Parser, Debug)]
#[clap(version, about, author)]
pub struct Opts {
    #[clap(flatten)]
    pub config_override: ConfigOverride,
    #[clap(subcommand)]
    pub command: CliCommand,
}
