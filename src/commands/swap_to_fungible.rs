use std::str::FromStr;

use crate::transaction::send_and_confirm_tx_with_spinner_with_config;

use super::*;

use libreplex_nico::{AccountData, Nico};
use mpl_token_metadata::{accounts::Metadata, types::TokenStandard};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_program::pubkey;
use solana_sdk::{
    account::{self, ReadableAccount},
    account_info::AccountInfo,
    instruction::AccountMeta,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;

pub const AUTH_RULES_PROGRAM_ID: Pubkey = pubkey!("auth9SigNpDKz4sJJ1DfCTuZrZNSAgh9sFD3rboVmgg");
pub const MPL_CORE_ID: Pubkey = pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
pub const MPL_TOKEN_METADATA_ID: Pubkey = pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
pub const SYSVAR_INSTRUCTIONS_PROGRAM_ID: Pubkey =
    pubkey!("Sysvar1nstructions1111111111111111111111111");

pub struct SwapToFungibleArgs {
    pub keypair_path: Option<PathBuf>,
    pub rpc_url: Option<String>,
    pub mule: Pubkey,
    pub non_fungible_asset: Pubkey,
    pub asset_group: Option<Pubkey>,
    // required unless asset is nifty or a core
    pub non_fungible_source_token_account: Option<Pubkey>,
    pub priority: Priority,
}

pub fn handle_swap_to_fungible(args: SwapToFungibleArgs) -> Result<()> {
    let config = CliConfig::new(args.keypair_path, args.rpc_url)?;

    let authority_sk = config.keypair;

    let authority = authority_sk.pubkey();

    let data = config.client.get_account_data(&args.mule)?;

    let mule_obj = Mule::from_bytes(&data).unwrap();

    let data_fungible = config.client.get_account(&mule_obj.fungible_asset)?;

    let swap_marker = Pubkey::find_program_address(
        &[
            b"swap_marker",
            args.mule.as_ref(),
            args.non_fungible_asset.as_ref(),
        ],
        &libreplex_mule_client::ID,
    )
    .0;

    let fungible_source_token_account = get_associated_token_address_with_program_id(
        &args.mule,
        &mule_obj.fungible_asset,
        data_fungible.owner(),
    );

    let fungible_target_token_account = get_associated_token_address_with_program_id(
        &authority,
        &mule_obj.fungible_asset,
        data_fungible.owner(),
    );

    let mut remaining_accounts = vec![
        AccountMeta {
            pubkey: spl_token::ID,
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: system_program::ID,
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: MPL_CORE_ID,
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: MPL_TOKEN_METADATA_ID,
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: spl_associated_token_account::ID,
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: AUTH_RULES_PROGRAM_ID,
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: SYSVAR_INSTRUCTIONS_PROGRAM_ID,
            is_signer: false,
            is_writable: false,
        },
    ];

    if let Some(x) = args.asset_group {
        remaining_accounts.push(AccountMeta {
            pubkey: x,
            is_signer: false,
            is_writable: true,
        });
    }

    let metadata = Pubkey::find_program_address(
        &[
            b"metadata",
            MPL_TOKEN_METADATA_ID.as_ref(),
            &args.non_fungible_asset.as_ref(),
        ],
        &MPL_TOKEN_METADATA_ID,
    )
    .0;
    if let Some(x) = args.non_fungible_source_token_account {
        // add metadata
        remaining_accounts.push(AccountMeta {
            pubkey: metadata,
            is_signer: false,
            is_writable: true,
        });

        // add target token record
        remaining_accounts.push(AccountMeta {
            pubkey: Pubkey::find_program_address(
                &[
                    b"metadata",
                    MPL_TOKEN_METADATA_ID.as_ref(),
                    args.non_fungible_asset.as_ref(),
                    b"token_record",
                    x.as_ref(),
                ],
                &MPL_TOKEN_METADATA_ID,
            )
            .0,
            is_signer: false,
            is_writable: true,
        });
    }

    let account_non_fungible = config.client.get_account(&args.non_fungible_asset)?;
    let data_non_fungible = account_non_fungible.data();

    // if it is a mint, then grab some metadata as well

    let mut account_datas: Vec<AccountData> = vec![];
    let account_metadata = config.client.get_account(&metadata);
    let mut data_metadata: Vec<u8> = vec![];
    if let Some(md) = account_metadata.ok() {
        if md.owner == MPL_TOKEN_METADATA_ID {
            let metadata_obj = Metadata::from_bytes(md.data())?;
            data_metadata.append(&mut md.data().to_vec());
            match metadata_obj.token_standard {
                Some(x) => match &x {
                    TokenStandard::ProgrammableNonFungible => {
                        match metadata_obj.programmable_config {
                            Some(x) => match &x {
                                mpl_token_metadata::types::ProgrammableConfig::V1 { rule_set } => {
                                    if let Some(x) = rule_set {
                                        remaining_accounts.push(AccountMeta {
                                            pubkey: *x,
                                            is_signer: false,
                                            is_writable: false,
                                        });
                                    }
                                }
                            },
                            None => {}
                        }
                    }
                    _ => {}
                },
                None => todo!(),
            }
            account_datas.push(AccountData {
                pubkey: metadata,
                data: &data_metadata,
            });
        }
    }
    let nico: Nico = Nico::from_raw_data(
        args.non_fungible_asset,
        account_non_fungible.owner,
        data_non_fungible,
        None,
        None,
        &account_datas,
    );

    let target_ata = get_associated_token_address_with_program_id(
        &args.mule,
        &args.non_fungible_asset,
        &spl_token::ID,
    );
    remaining_accounts.push(AccountMeta {
        pubkey: target_ata,
        is_signer: false,
        is_writable: true,
    });

    remaining_accounts.push(AccountMeta {
        pubkey: Pubkey::find_program_address(
            &[
                b"metadata",
                mpl_token_metadata::ID.as_ref(),
                args.non_fungible_asset.as_ref(),
                b"token_record",
                target_ata.as_ref(),
            ],
            &mpl_token_metadata::ID,
        )
        .0,
        is_signer: false,
        is_writable: true,
    });

    remaining_accounts.push(AccountMeta {
        pubkey: Pubkey::find_program_address(
            &[
                b"metadata",
                &mpl_token_metadata::ID.as_ref(),
                &args.non_fungible_asset.as_ref(),
                b"edition",
            ],
            &mpl_token_metadata::ID,
        )
        .0,
        is_signer: false,
        is_writable: false,
    });

    if let Some(x) = nico.group {
        remaining_accounts.push(AccountMeta {
            pubkey: x,
            is_signer: false,
            is_writable: false,
        });
    }

    let ix = SwapToFungible {
        payer: authority,
        swapper: authority,
        mule: args.mule,
        authority,
        cosigner: None,
        swap_marker: swap_marker,
        non_fungible_asset: args.non_fungible_asset,
        fungible_asset: mule_obj.fungible_asset,
        fungible_source_token_account,
        fungible_target_token_account,
        non_fungible_source_token_account: args.non_fungible_source_token_account,
        system_program: system_program::ID,
        associated_token_program: spl_associated_token_account::ID,
        non_fungible_current_owner: Some(authority),
    }
    .instruction_with_remaining_accounts(remaining_accounts.as_slice());

    let signers = vec![&authority_sk];

    let micro_lamports = get_priority_fee(&args.priority);
    let compute_units = 500_000; //get_compute_units(&config.client, &[ix.clone()], &signers)?;

    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(compute_units as u32),
        ComputeBudgetInstruction::set_compute_unit_price(micro_lamports),
        ix,
    ];

    println!("Sending transaction");

    let sig = send_and_confirm_tx_with_spinner_with_config(
        &config.client,
        &signers,
        &instructions,
        RpcSendTransactionConfig {
            skip_preflight: true,
            preflight_commitment: None,
            encoding: None,
            max_retries: None,
            min_context_slot: None,
        },
    )?;

    println!("Swapped asset to fungible. Tx: {sig}");

    Ok(())
}
