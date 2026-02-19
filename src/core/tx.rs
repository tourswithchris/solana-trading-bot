use solana_sdk::signature::Signer;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    signature::Keypair,
    transaction::Transaction,
};
use std::env;
use std::str::FromStr;
use std::time::Instant;

fn get_unit_price() -> u64 {
    env::var("UNIT_PRICE")
        .ok()
        .and_then(|v| u64::from_str(&v).ok())
        .unwrap_or(1)
}

fn get_unit_limit() -> u32 {
    env::var("UNIT_LIMIT")
        .ok()
        .and_then(|v| u32::from_str(&v).ok())
        .unwrap_or(300_000)
}

pub fn add_compute_budget_instructions(instructions: &mut Vec<Instruction>) {
    let unit_price = get_unit_price();
    let unit_limit = get_unit_limit();

    instructions.insert(0, ComputeBudgetInstruction::set_compute_unit_limit(unit_limit));
    instructions.insert(1, ComputeBudgetInstruction::set_compute_unit_price(unit_price));
}

pub fn create_transaction(
    instructions: Vec<Instruction>,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
) -> Transaction {
    let mut tx = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    tx.sign(&[payer], recent_blockhash);
    tx
}

pub fn send_and_confirm_transaction(
    rpc_client: &solana_client::rpc_client::RpcClient,
    transaction: &Transaction,
) -> Result<solana_sdk::signature::Signature, Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    let signature = rpc_client.send_and_confirm_transaction(transaction)?;

    println!("Transaction confirmed in {:?}", start_time.elapsed());
    println!("Signature: {}", signature);

    Ok(signature)
}
