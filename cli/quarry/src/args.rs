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
    default_value_t = quarry::id().to_string()
    )]
    pub program_id: String,
}

#[derive(Parser, Debug)]
pub enum CliCommand {
    NewRewarder {
        #[clap(long)]
        rewards_token_mint: Pubkey,
    },
    SetGaugeFactoryMintAuthority {},
    SetPauseAuthority {
        #[clap(long)]
        new_pause_authority: Pubkey,
    },
    Pause {},
    Unpause {},
    TransferAdmin {
        #[clap(long)]
        new_admin: Pubkey,
    },
    AcceptAdmin {},
    SetAnnualRewards {
        #[clap(long)]
        new_rate: u64,
    },
    CreateQuarry {
        #[clap(long)]
        system_base: String,
    },
    SetRewardsShare {
        #[clap(long)]
        new_share: u64,
        #[clap(long)]
        token_mint: Pubkey,
    },
    SetFamine {
        #[clap(long)]
        famine_ts: i64,
        #[clap(long)]
        system_base: String,
    },
    UpdateQuarryRewards {
        #[clap(long)]
        token_mint: Pubkey,
    },
    CreateMiner {
        #[clap(long)]
        token_mint: Pubkey,
    },
    ClaimRewards {
        #[clap(long)]
        token_mint: Pubkey,
    },
    StakeToken {
        #[clap(long)]
        token_mint: Pubkey,
        #[clap(long)]
        amount: u64,
    },
    UnstakeToken {
        #[clap(long)]
        token_mint: Pubkey,
        #[clap(long)]
        amount: u64,
    },
    ViewMiner {
        #[clap(long)]
        token_mint: Pubkey,
    },
}

#[derive(Parser, Debug)]
#[clap(version, about, author)]
pub struct Opts {
    #[clap(flatten)]
    pub config_override: ConfigOverride,
    #[clap(subcommand)]
    pub command: CliCommand,
}
