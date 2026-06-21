//! mpl-core's instructions, run through the TestSVM mollusk adapter against the
//! project's own built `spl`/`mpl_core_program.so`, asserting on the returned
//! model + read-back state instead of mollusk's `process_instruction` +
//! `matches!`. The instruction builders and fabricators live in the crate lib.

use {
    mpl_core_babelfish_tests::*,
    mpl_core_program::ID as MPL_CORE_ID,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    testsvm::TestSVM,
};

#[test]
fn create_v1_allocates_via_system_cpi() {
    let mut engine = engine_with_program();
    let payer = engine.actor("payer", ACCOUNT_LAMPORTS);
    // The asset is a fresh signer with an empty, system-owned account: the
    // uninitialized target the program's System CPI allocates.
    let asset = engine.actor("Asset", 0);

    let tx = engine.send(
        &[create_v1(asset.pubkey(), payer.pubkey(), "Babelfish Asset", "https://example.com/a")],
        &[&payer, &asset],
    );

    assert!(tx.error.is_none(), "create failed: {:?}", tx.error);
    let created = read_asset(&engine, &asset.pubkey()).expect("asset created");
    assert_eq!(created.owner, payer.pubkey(), "owner defaults to the authority (payer)");
    // The System allocate CPI nests under the program frame.
    assert!(tx.pretty_cpi_tree().contains("CreateAccount System"));
}

#[test]
fn transfer_succeeds_with_valid_asset() {
    let mut engine = engine_with_program();
    let payer = engine.actor("payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();
    engine.register_alias(&asset, "Asset");

    engine.set_account(&asset, asset_account(&payer.pubkey(), &MPL_CORE_ID));
    engine.set_account(&new_owner, solana_account::Account::default());

    let tx = engine.send(&[transfer_v1(asset, payer.pubkey(), new_owner)], &[&payer]);

    assert!(tx.error.is_none(), "transfer failed: {:?}", tx.error);
    let after = read_asset(&engine, &asset).expect("asset present");
    assert_eq!(after.owner, new_owner, "ownership transferred");
}
