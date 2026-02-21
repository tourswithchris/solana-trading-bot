use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta,
    option_serializer::OptionSerializer,
};

#[derive(Debug, Clone)]
pub struct TokenDelta {
    pub mint: String,
    pub delta_ui: f64,
}

pub fn extract_fee_sol(tx: &EncodedConfirmedTransactionWithStatusMeta) -> f64 {
    let fee_lamports = tx.transaction.meta.as_ref().map(|m| m.fee).unwrap_or(0);
    fee_lamports as f64 / 1_000_000_000.0
}

/// Sum token balance deltas for a specific owner (your wallet).
/// delta_ui = post_ui - pre_ui for each mint, summed over all token accounts owned by `owner`.
pub fn extract_owner_token_deltas(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
    owner: &Pubkey,
) -> Vec<TokenDelta> {
    let mut deltas: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

    let Some(meta) = &tx.transaction.meta else { return vec![]; };

    // Helper to index balances by (account_index, mint) for owner
    fn collect(
        balances: &Vec<solana_transaction_status::UiTransactionTokenBalance>,
        owner: &Pubkey,
    ) -> std::collections::HashMap<(u64, String), f64> {
        let mut map = std::collections::HashMap::new();
        for b in balances {
            // owner is optional in some responses; skip if missing
            let Some(o) = &b.owner else { continue; };
            if o != &owner.to_string() { continue; }

            let mint = b.mint.clone();
            let idx = b.account_index;
            let ui = b.ui_token_amount.ui_amount.unwrap_or(0.0);
            map.insert((idx, mint), ui);
        }
        map
    }

    let pre = match &meta.pre_token_balances {
        OptionSerializer::Some(v) => collect(v, owner),
        _ => std::collections::HashMap::new(),
    };

    let post = match &meta.post_token_balances {
        OptionSerializer::Some(v) => collect(v, owner),
        _ => std::collections::HashMap::new(),
    };

    // Union keys
    let mut keys: std::collections::HashSet<(u64, String)> = std::collections::HashSet::new();
    for k in pre.keys() { keys.insert(k.clone()); }
    for k in post.keys() { keys.insert(k.clone()); }

    for (_idx, mint) in keys.into_iter() {
        let mut sum_delta = 0.0;

        // sum across accounts of same mint
        for ((i, m), pre_ui) in pre.iter() {
            if m == &mint {
                let post_ui = post.get(&(*i, m.clone())).cloned().unwrap_or(0.0);
                sum_delta += post_ui - pre_ui;
            }
        }
        // also include accounts that only exist post (created during swap)
        for ((i, m), post_ui) in post.iter() {
            if m == &mint && !pre.contains_key(&(*i, m.clone())) {
                sum_delta += *post_ui;
            }
        }

        *deltas.entry(mint).or_insert(0.0) += sum_delta;
    }

    deltas
        .into_iter()
        .map(|(mint, delta_ui)| TokenDelta { mint, delta_ui })
        .collect()
}
