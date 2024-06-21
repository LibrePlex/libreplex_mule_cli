use libreplex_mule_client::{instructions::CreateMuleInstructionArgs, types::Filter};

use super::*;

pub struct CreateArgs {
    pub keypair_path: Option<PathBuf>,
    pub rpc_url: Option<String>,
    pub base_swap_rate: u64,
    pub auto_generate_swap_marker: bool,
    pub filter: Filter,
    pub fungible_mint: Pubkey,
    pub priority: Priority,
    pub update_auth: Option<Pubkey>
}

pub fn handle_create(args: CreateArgs) -> Result<()> {
    let config = CliConfig::new(args.keypair_path, args.rpc_url)?;

  
    let authority_sk = config.keypair;

    let authority = authority_sk.pubkey();


    let seed = Keypair::new();

    let mule = Pubkey::find_program_address(
        &[b"mule", seed.pubkey().as_ref()],
        &libreplex_mule_client::ID,
    )
    .0;

    let ix_args = CreateMuleInstructionArgs {
        seed: seed.pubkey(),
        base_swap_rate: args.base_swap_rate,
        update_auth: args.update_auth,
        auto_generate_swap_marker: args.auto_generate_swap_marker,
        filter: args.filter,
    };



    let ix = CreateMule {
        payer: authority,
        authority: authority,
        mule: mule,
        cosigner: None,
        fungible_asset: args.fungible_mint,
        handler_program: None,
        system_program: system_program::ID,
    }
    .instruction(ix_args);

    let signers = vec![&authority_sk];

    let micro_lamports = get_priority_fee(&args.priority);
    let compute_units = 500_000; //get_compute_units(&config.client, &[ix.clone()], &signers)?;

    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(compute_units as u32),
        ComputeBudgetInstruction::set_compute_unit_price(micro_lamports),
        ix,
    ];

    println!("Sending transaction");



    let sig = send_and_confirm_tx_with_spinner(&config.client, &signers, &instructions)?;

    println!("Mule {mule} created in tx: {sig}");

    Ok(())
}
