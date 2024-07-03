use std::path::PathBuf;

use clap::{Parser, Subcommand};

use solana_program::pubkey::Pubkey;

use crate::transaction::{Filter, Priority};

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Args {
    /// Path to the keypair file.
    #[arg(short, long, global = true)]
    pub keypair_path: Option<PathBuf>,

    /// RPC URL for the Solana cluster.
    #[arg(short, long, global = true)]
    pub rpc_url: Option<String>,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Subcommand)]
pub enum Commands {
   
    /// Create a mule
    Create {
        /// base swap rate to use for swapping NFTs to SPL tokens. Do not forget decimals 
        #[arg(long)]
        base_swap_rate: u64,

        /// auto-generate swap markers on swap as long as the NFT matches the filter.
        #[arg(long)]
        auto_generate_swap_marker: bool,

        #[arg(long)]
        update_auth: Option<Pubkey>,
  
        /// Filter that defines what assets are swappable in this mule deployment
        #[arg(long)]
        filter: Filter,

        /// Fungible mint
        #[arg(long)]
        fungible_mint: Pubkey,

        #[arg(short = 'P', long, default_value = "low")]
        priority: Priority,

        #[arg(long)]
        fee_per_swap_lamports: Option<u64>,
        #[arg(long)]
        fee_rate_per_swap_basis_points: Option<u16>,
        #[arg(long)]
        swap_fee_treasury: Option<Pubkey>,
        #[arg(long)]
        fee_per_swap_spl_amount: Option<u64>,
        #[arg(long)]
        burn_spl_basis_points: Option<u16>,
        #[arg(long)]
        name: String,
        

    },
    /// Swap NFT to fungible under a given mule deployment
    SwapToFungible {
        /// The mule deployment key
        #[arg(long)]
        mule: Pubkey,

        /// The NFT key (mint / nifty asset / core asset)
        #[arg(long)]
        non_fungible_asset: Pubkey,

        #[arg(long)]
        asset_group: Option<Pubkey>,
        
        /// The NFT source token account. Required for mints only
        #[arg(long)]
        non_fungible_source_token_account: Option<Pubkey>,

        #[arg(short = 'P', long, default_value = "low")]
        priority: Priority,
    }
}
