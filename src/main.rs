use anyhow::Result;
use clap::Parser;

use libreplex_mule_client::types::Filter;
use mule_cli::{
    args::{Args, Commands},
    commands::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    solana_logger::setup_with_default("solana=error");

    let args = Args::parse();

    let keypair_path = args.keypair_path.clone();
    let rpc_url = args.rpc_url.clone();

    match args.command {
        Commands::Create {
            priority,
            base_swap_rate,
            auto_generate_swap_marker,
            filter,
            fungible_mint,
            update_auth,
            fee_per_swap_lamports,
            fee_rate_per_swap_basis_points,
            swap_fee_treasury,
            fee_per_swap_spl_amount,
            burn_spl_basis_points,
            name,
        } => handle_create(CreateArgs {
            keypair_path,
            rpc_url,
            update_auth,
            base_swap_rate,
            auto_generate_swap_marker,
            fee_per_swap_lamports,
            fee_rate_per_swap_basis_points,
            swap_fee_treasury,
            burn_spl_basis_points,
            fee_per_swap_spl_amount,
            filter: match filter {
                mule_cli::transaction::Filter::All => Filter::All,
                mule_cli::transaction::Filter::Group { group_id } => Filter::Group { group_id },
                mule_cli::transaction::Filter::Creator { creator_id } => {
                    Filter::Creator { creator_id }
                }
            },
            fungible_mint,
            priority,
            name
        }),
        Commands::SwapToFungible {
            priority,
            mule,
            asset_group,
            non_fungible_asset,
            non_fungible_source_token_account,
        } => handle_swap_to_fungible(SwapToFungibleArgs {
            keypair_path,
            rpc_url,
            mule,
            asset_group,
            non_fungible_asset,
            non_fungible_source_token_account,
            priority,
        }),
    }
}
