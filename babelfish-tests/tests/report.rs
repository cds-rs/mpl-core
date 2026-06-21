//! The core create/transfer/burn scenarios, rendered to `report/`. The suite
//! files (account_ownership, agent_identity, execution_delegate) render their
//! own pages; the master index is regenerated over all of them.

use {
    mpl_core_babelfish_tests::*,
    mpl_core_program::ID as MPL_CORE_ID,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    testsvm::{model::Transaction, TestSVM},
    testsvm_mollusk::MolluskBackend,
};

const FOREIGN_PROGRAM: Pubkey = Pubkey::new_from_array([0xAAu8; 32]);

fn create(engine: &mut MolluskBackend) -> Transaction {
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = engine.actor("Asset", 0);
    engine.send(
        &[create_v1(asset.pubkey(), payer.pubkey(), "Report Asset", "https://example.com/a")],
        &[&payer, &asset],
    )
}

fn transfer(engine: &mut MolluskBackend) -> Transaction {
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let new_owner = engine.actor("NewOwner", 0);
    let asset = engine.actor("Asset", 0).pubkey();
    engine.set_account(&asset, asset_account(&payer.pubkey(), &MPL_CORE_ID));
    engine.send(&[transfer_v1(asset, payer.pubkey(), new_owner.pubkey())], &[&payer])
}

fn burn(engine: &mut MolluskBackend) -> Transaction {
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = engine.actor("Asset", 0).pubkey();
    engine.set_account(&asset, asset_account(&payer.pubkey(), &MPL_CORE_ID));
    engine.send(&[burn_v1(asset, payer.pubkey())], &[&payer])
}

fn reject_foreign_owned(engine: &mut MolluskBackend) -> Transaction {
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let new_owner = engine.actor("NewOwner", 0);
    let asset = engine.actor("Asset", 0).pubkey();
    engine.register_alias(&FOREIGN_PROGRAM, "ForeignProgram");
    engine.set_account(&asset, asset_account(&payer.pubkey(), &FOREIGN_PROGRAM));
    engine.send(&[transfer_v1(asset, payer.pubkey(), new_owner.pubkey())], &[&payer])
}

type Scenario = fn(&mut MolluskBackend) -> Transaction;

#[test]
fn generate_report() {
    let scenarios: &[(&str, &str, &str, Scenario)] = &[
        ("create", "Create an asset", "Allocate a fresh asset; the program creates the account through a System CPI.", create),
        ("transfer", "Transfer an asset", "Hand a valid asset to a new owner.", transfer),
        ("burn", "Burn an asset", "Close a valid asset, returning its lamports to the payer.", burn),
        ("reject_foreign_owned", "Reject a foreign-owned asset", "Transfer an account that carries valid AssetV1 bytes but is owned by another program.", reject_foreign_owned),
    ];

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/report");
    std::fs::create_dir_all(dir).unwrap();
    for (test_fn, title, intent, run) in scenarios {
        let tx = run(&mut engine_with_program());
        let page = format!("{dir}/{}.md", test_fn.replace('_', "-"));
        std::fs::write(page, render_scenario(title, intent, "tests/report.rs", test_fn, &tx)).unwrap();
    }
}
