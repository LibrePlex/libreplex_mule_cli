use anyhow::{anyhow, Result};
use retry::{delay::Exponential, retry};
use solana_client::{rpc_client::RpcClient, rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig}};
use solana_program::instruction::Instruction;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

pub const MAX_TX_SIZE: usize = 1232;
pub const DEFAULT_CU: u64 = 15_000;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum Priority {
    None,
    #[default]
    Low,
    Medium,
    High,
    Max,
}

impl FromStr for Priority {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "max" => Ok(Self::Max),
            _ => Err(anyhow!("Invalid priority".to_string())),
        }
    }
}

impl Display for Priority {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Max => write!(f, "Max"),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum Filter {
    #[default]
    All,
    Group {
        group_id: Pubkey
    },
    Creator {
        creator_id: Pubkey
    }
}

impl FromStr for Filter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(":");
        let filter_type = parts.next();

        match filter_type {
            None => {
                panic!("Invalid filter type. Filter form is <type>:<pubkey> where <type> is 'g' for group or 'a' for all (permits everything)")
            }
            Some(x) => {
                match x {
                    "a" => {
                        Ok(Filter::All)
                    },
                    "g" => {
                        Ok(Filter::Group { group_id: Pubkey::from_str(parts.next().unwrap()).unwrap() })
                    },
                    _ => {
                        panic!("Invalid filter type. Filter form is <type>:<pubkey> where <type> is 'g' for group")
                    }
                }
            }
        }

        
    }
}

impl Display for Filter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "a"),
            Self::Group {group_id}=> write!(f, "g:{}", group_id),
            Self::Creator { creator_id } => write!(f, "c:{}", creator_id),
        }
    }
}



pub fn get_priority_fee(priority: &Priority) -> u64 {
    match priority {
        Priority::None => 1_000,
        Priority::Low => 50_000,
        Priority::Medium => 200_000,
        Priority::High => 1_000_000,
        Priority::Max => 2_000_000,
    }
}

#[macro_export]
macro_rules! transaction {
    ($client:expr, $signers:expr, $instructions:expr) => {
        Transaction::new_signed_with_payer(
            $instructions,
            Some(&$signers[0].pubkey()),
            $signers,
            $client.get_latest_blockhash()?,
        )
    };
}

pub fn send_and_confirm_tx(
    client: &RpcClient,
    signers: &[&Keypair],
    ixs: &[Instruction],
) -> Result<Signature> {
    let tx = transaction!(client, signers, ixs);

    let signature = client.send_and_confirm_transaction(&tx)?;

    Ok(signature)
}

pub fn send_and_confirm_tx_with_spinner_with_config(
    client: &RpcClient,
    signers: &[&Keypair],
    ixs: &[Instruction],
    config: RpcSendTransactionConfig
) -> Result<Signature> {
    let tx = transaction!(client, signers, ixs);

    let signature = client.send_transaction_with_config(
        &tx,
        config
    )?;

    Ok(signature)
}

pub fn send_and_confirm_tx_with_spinner(
    client: &RpcClient,
    signers: &[&Keypair],
    ixs: &[Instruction],
) -> Result<Signature> {
    let tx = transaction!(client, signers, ixs);

    let signature = client.send_and_confirm_transaction_with_spinner(&tx)?;

    Ok(signature)
}

pub fn send_and_confirm_tx_with_retries(
    client: &RpcClient,
    signers: &[&Keypair],
    ixs: &[Instruction],
) -> Result<Signature> {
    let tx = transaction!(client, signers, ixs);

    // Send tx with retries.
    let res = retry(
        Exponential::from_millis_with_factor(250, 2.0).take(3),
        || client.send_and_confirm_transaction_with_spinner(&tx),
    )?;

    Ok(res)
}

pub fn pack_instructions<'a>(
    num_signers: u32,
    payer: &'a Pubkey,
    ixs: &'a [Instruction],
) -> Vec<Vec<Instruction>> {
    // This contains the instructions that will be sent in each transaction.
    let mut transactions: Vec<Vec<Instruction>> = vec![];
    // Batch instructions for each tx into this vector, ensuring we don't exceed max payload size.
    let mut tx_instructions: Vec<Instruction> = vec![];

    // 64 bytes for each signature + Message size
    let max_payload_size = MAX_TX_SIZE - std::mem::size_of::<Signature>() * num_signers as usize;

    for ix in ixs {
        tx_instructions.push(ix.clone());
        let tx = Transaction::new_with_payer(tx_instructions.as_slice(), Some(payer));
        let tx_len = bincode::serialize(&tx).unwrap().len();

        if tx_len > max_payload_size {
            let last_ix = tx_instructions.pop().unwrap();
            transactions.push(tx_instructions.clone());
            tx_instructions.clear();
            tx_instructions.push(last_ix);
        }
    }
    transactions.push(tx_instructions);

    transactions
}

pub fn get_compute_units(
    client: &RpcClient,
    ixs: &[Instruction],
    signers: &[&Keypair],
) -> Result<u64> {
    let config = RpcSimulateTransactionConfig {
        sig_verify: false,
        replace_recent_blockhash: true,
        commitment: Some(CommitmentConfig::confirmed()),
        ..Default::default()
    };

    let tx = Transaction::new_signed_with_payer(
        ixs,
        Some(&signers[0].pubkey()),
        signers,
        Hash::new(Pubkey::default().as_ref()), // dummy value
    );

    // This doesn't return an error if the simulation fails
    let sim_result = client.simulate_transaction_with_config(&tx, config)?;

    // it sets the error Option on the value in the Ok variant, so we check here
    // and return the error manually.
    if let Some(err) = sim_result.value.err {
        return Err(err.into());
    }

    // Otherwise, we can get the compute units from the simulation result
    let units = sim_result.value.units_consumed.unwrap_or(DEFAULT_CU);

    Ok(units)
}
