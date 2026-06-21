//! mpl-core's account-ownership rejection suite, ported from the project's native
//! mollusk tests (`programs/mpl-core/tests/account_ownership.rs`) to run through
//! the TestSVM mollusk adapter.
//!
//! The native suite calls `mollusk.process_instruction` (which bypasses
//! sigverify) and asserts `matches!(result, Success)`. Here we drive the same
//! hand-encoded instructions through `engine.send`, which needs real signers, so
//! the fee payer (and the asset's authority, which is the same key) is an
//! `actor`. We assert on the returned model's `tx.error` instead.
//!
//! Every one of the 25 native `#[test]`s is ported with the same name and the
//! same intent: a foreign-owned / wrong-discriminator / garbage / empty / frozen
//! / unauthorized account is rejected; a valid owner's transfer succeeds.

use {
    mpl_core_babelfish_tests::{world::*, *},
    mpl_core_program::{
        plugins::{
            FreezeDelegate, PermanentBurnDelegate, PermanentFreezeDelegate,
            PermanentTransferDelegate, Plugin,
        },
        state::{AssetV1, Authority, CollectionV1, Key, UpdateAuthority},
        ID as MPL_CORE_ID, SPL_NOOP_ID,
    },
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    testsvm::TestSVM,
    testsvm_mollusk::MolluskBackend,
};

// Instruction builders (ported; `system_program::ID` -> SYSTEM_PROGRAM)

/// TransferV1 (disc 14) with the collection account set to a real pubkey.
fn transfer_v1_with_collection_instruction(
    asset: Pubkey,
    collection: Pubkey,
    payer: Pubkey,
    new_owner: Pubkey,
) -> Instruction {
    let data = vec![14u8, 0u8]; // TransferV1 discriminator + None compression_proof
    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, false),               // 0: asset
            AccountMeta::new_readonly(collection, false), // 1: collection
            AccountMeta::new(payer, true),                // 2: payer
            AccountMeta::new_readonly(payer, true),       // 3: authority
            AccountMeta::new_readonly(new_owner, false),  // 4: new_owner
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false), // 5: system_program
            AccountMeta::new_readonly(SPL_NOOP_ID, false), // 6: log_wrapper (optional)
        ],
    )
}

/// TransferV1 (disc 14), with Option::None compression_proof.
fn transfer_v1_instruction(asset: Pubkey, payer: Pubkey, new_owner: Pubkey) -> Instruction {
    let data = vec![14u8, 0u8];
    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, false),                // 0: asset
            AccountMeta::new_readonly(MPL_CORE_ID, false), // 1: collection (optional, program ID stands in for None)
            AccountMeta::new(payer, true),                 // 2: payer
            AccountMeta::new_readonly(payer, true),        // 3: authority
            AccountMeta::new_readonly(new_owner, false),   // 4: new_owner
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false), // 5: system_program
            AccountMeta::new_readonly(SPL_NOOP_ID, false), // 6: log_wrapper (optional)
        ],
    )
}

/// BurnV1 (disc 12), with Option::None compression_proof.
fn burn_v1_instruction(asset: Pubkey, payer: Pubkey) -> Instruction {
    let data = vec![12u8, 0u8];
    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, false),                // 0: asset
            AccountMeta::new_readonly(MPL_CORE_ID, false), // 1: collection (optional)
            AccountMeta::new(payer, true),                 // 2: payer
            AccountMeta::new_readonly(payer, true),        // 3: authority
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false), // 4: system_program (optional)
            AccountMeta::new_readonly(MPL_CORE_ID, false), // 5: log_wrapper (optional)
        ],
    )
}

/// UpdateV1 (disc 15) that changes the name to "X".
fn update_v1_instruction(asset: Pubkey, payer: Pubkey) -> Instruction {
    let mut data = vec![15u8];
    data.push(1); // Option::Some for new_name
    data.extend_from_slice(&1u32.to_le_bytes()); // string length = 1
    data.push(b'X'); // "X"
    data.push(0); // Option::None for new_uri
    data.push(0); // Option::None for new_update_authority
    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, false),                // 0: asset
            AccountMeta::new_readonly(MPL_CORE_ID, false), // 1: collection (optional)
            AccountMeta::new(payer, true),                 // 2: payer
            AccountMeta::new_readonly(payer, true),        // 3: authority
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false), // 4: system_program
            AccountMeta::new_readonly(MPL_CORE_ID, false), // 5: log_wrapper (optional)
        ],
    )
}

/// UpdateCollectionV1 (disc 16) that changes the name to "X".
fn update_collection_v1_instruction(collection: Pubkey, payer: Pubkey) -> Instruction {
    let mut data = vec![16u8];
    data.push(1); // Option::Some for new_name
    data.extend_from_slice(&1u32.to_le_bytes()); // string length = 1
    data.push(b'X'); // "X"
    data.push(0); // Option::None for new_uri
    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(collection, false),    // 0: collection
            AccountMeta::new(payer, true),          // 1: payer
            AccountMeta::new_readonly(payer, true), // 2: authority
            AccountMeta::new_readonly(MPL_CORE_ID, false), // 3: new_update_authority (optional)
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false), // 4: system_program
            AccountMeta::new_readonly(MPL_CORE_ID, false), // 5: log_wrapper (optional)
        ],
    )
}

// Section 1: Fake assets owned by a different program

// Report: ../report/transfer-rejects-fake-asset-owned-by-different-program.md
#[test]
fn transfer_rejects_fake_asset_owned_by_different_program() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(&asset, fake_asset_account(&payer.pubkey(), &FAKE_PROGRAM_ID));
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of foreign-owned asset");
}

// Report: ../report/burn-rejects-fake-asset-owned-by-different-program.md
#[test]
fn burn_rejects_fake_asset_owned_by_different_program() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();

    engine.set_account(&asset, fake_asset_account(&payer.pubkey(), &FAKE_PROGRAM_ID));

    let tx = engine.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer]);
    assert!(tx.error.is_some(), "expected rejection of foreign-owned asset");
}

// Report: ../report/update-rejects-fake-asset-owned-by-different-program.md
#[test]
fn update_rejects_fake_asset_owned_by_different_program() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();

    engine.set_account(&asset, fake_asset_account(&payer.pubkey(), &FAKE_PROGRAM_ID));

    let tx = engine.send(&[update_v1_instruction(asset, payer.pubkey())], &[&payer]);
    assert!(tx.error.is_some(), "expected rejection of foreign-owned asset");
}

// Section 2: Fake collections owned by a different program

// Report: ../report/update-collection-rejects-fake-collection-owned-by-different-program.md
#[test]
fn update_collection_rejects_fake_collection_owned_by_different_program() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let collection = Pubkey::new_unique();

    engine.set_account(&collection, fake_collection_account(&payer.pubkey(), &FAKE_PROGRAM_ID));

    let tx = engine.send(
        &[update_collection_v1_instruction(collection, payer.pubkey())],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of foreign-owned collection");
}

// Section 3: Accounts with wrong discriminator

// Report: ../report/transfer-rejects-account-with-wrong-discriminator.md
#[test]
fn transfer_rejects_account_with_wrong_discriminator() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    // CollectionV1 discriminator (5) where AssetV1 (1) is expected, but owned by mpl-core.
    let collection = CollectionV1::new(
        payer.pubkey(),
        "Wrong".to_string(),
        "https://wrong.com".to_string(),
        0,
        0,
    );
    let wrong_disc_account = Account {
        lamports: ACCOUNT_LAMPORTS,
        data: borsh::to_vec(&collection).unwrap(),
        owner: MPL_CORE_ID,
        executable: false,
        rent_epoch: 0,
    };
    engine.set_account(&asset, wrong_disc_account);
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of wrong discriminator");
}

// Report: ../report/burn-rejects-account-with-wrong-discriminator.md
#[test]
fn burn_rejects_account_with_wrong_discriminator() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();

    // Key::Uninitialized (0): a completely wrong discriminator.
    let mut data = vec![0u8; 100];
    data[0] = Key::Uninitialized as u8;
    let wrong_disc_account = Account {
        lamports: ACCOUNT_LAMPORTS,
        data,
        owner: MPL_CORE_ID,
        executable: false,
        rent_epoch: 0,
    };
    engine.set_account(&asset, wrong_disc_account);

    let tx = engine.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer]);
    assert!(tx.error.is_some(), "expected rejection of wrong discriminator");
}

// Section 4: Random / garbage data accounts

// Report: ../report/transfer-rejects-random-data-account.md
#[test]
fn transfer_rejects_random_data_account() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    let garbage_account = Account {
        lamports: ACCOUNT_LAMPORTS,
        data: vec![0xFF; 200],
        owner: MPL_CORE_ID,
        executable: false,
        rent_epoch: 0,
    };
    engine.set_account(&asset, garbage_account);
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of garbage data");
}

// Section 5: Empty accounts

// Report: ../report/transfer-rejects-empty-account.md
#[test]
fn transfer_rejects_empty_account() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    let empty_account = Account {
        lamports: ACCOUNT_LAMPORTS,
        data: vec![],
        owner: MPL_CORE_ID,
        executable: false,
        rent_epoch: 0,
    };
    engine.set_account(&asset, empty_account);
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of empty account");
}

// Section 6: Valid accounts work correctly (sanity check)

// Report: ../report/transfer-succeeds-with-valid-asset.md
#[test]
fn transfer_succeeds_with_valid_asset() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(&asset, valid_asset_account(&payer.pubkey()));
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Section 7: System-program-owned accounts (uninitialized)

// Report: ../report/transfer-rejects-system-owned-account-with-asset-discriminator.md
#[test]
fn transfer_rejects_system_owned_account_with_asset_discriminator() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(&asset, fake_asset_account(&payer.pubkey(), &SYSTEM_PROGRAM));
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of system-owned account");
}

// Report: ../report/update-rejects-system-owned-account-with-asset-discriminator.md
#[test]
fn update_rejects_system_owned_account_with_asset_discriminator() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();

    engine.set_account(&asset, fake_asset_account(&payer.pubkey(), &SYSTEM_PROGRAM));

    let tx = engine.send(&[update_v1_instruction(asset, payer.pubkey())], &[&payer]);
    assert!(tx.error.is_some(), "expected rejection of system-owned account");
}

// Section 8: Fake asset with mismatched authority

// Report: ../report/transfer-rejects-unauthorized-caller-on-valid-asset.md
#[test]
fn transfer_rejects_unauthorized_caller_on_valid_asset() {
    let mut engine = engine_with_program();
    let actual_owner = Pubkey::new_unique();
    // The attacker is the signer (fee payer + authority), not the asset's owner.
    let attacker = engine.actor("Attacker", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(&asset, valid_asset_account(&actual_owner));
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, attacker.pubkey(), new_owner)],
        &[&attacker],
    );
    assert!(tx.error.is_some(), "expected rejection of unauthorized caller");
}

// Report: ../report/burn-rejects-unauthorized-caller-on-valid-asset.md
#[test]
fn burn_rejects_unauthorized_caller_on_valid_asset() {
    let mut engine = engine_with_program();
    let actual_owner = Pubkey::new_unique();
    let attacker = engine.actor("Attacker", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();

    engine.set_account(&asset, valid_asset_account(&actual_owner));

    let tx = engine.send(&[burn_v1_instruction(asset, attacker.pubkey())], &[&attacker]);
    assert!(tx.error.is_some(), "expected rejection of unauthorized caller");
}

// Section 9: Fake assets with freeze plugins (owned by different program)

// Report: ../report/transfer-rejects-fake-frozen-asset-owned-by-different-program.md
#[test]
fn transfer_rejects_fake_frozen_asset_owned_by_different_program() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[(
                Plugin::FreezeDelegate(FreezeDelegate { frozen: true }),
                Authority::Owner,
            )],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of foreign-owned frozen asset");
}

// Report: ../report/transfer-rejects-fake-unfrozen-asset-owned-by-different-program.md
#[test]
fn transfer_rejects_fake_unfrozen_asset_owned_by_different_program() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[(
                Plugin::FreezeDelegate(FreezeDelegate { frozen: false }),
                Authority::Owner,
            )],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of foreign-owned unfrozen asset");
}

// Report: ../report/burn-rejects-fake-frozen-asset-owned-by-different-program.md
#[test]
fn burn_rejects_fake_frozen_asset_owned_by_different_program() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[(
                Plugin::FreezeDelegate(FreezeDelegate { frozen: true }),
                Authority::Owner,
            )],
        ),
    );

    let tx = engine.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer]);
    assert!(tx.error.is_some(), "expected rejection of foreign-owned frozen asset");
}

// Section 10: Fake assets with permanent delegate plugins

// Report: ../report/transfer-rejects-fake-asset-with-permanent-transfer-delegate.md
#[test]
fn transfer_rejects_fake_asset_with_permanent_transfer_delegate() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[(
                Plugin::PermanentTransferDelegate(PermanentTransferDelegate {}),
                Authority::Owner,
            )],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection despite permanent transfer delegate");
}

// Report: ../report/burn-rejects-fake-asset-with-permanent-burn-delegate.md
#[test]
fn burn_rejects_fake_asset_with_permanent_burn_delegate() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[(
                Plugin::PermanentBurnDelegate(PermanentBurnDelegate {}),
                Authority::Owner,
            )],
        ),
    );

    let tx = engine.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer]);
    assert!(tx.error.is_some(), "expected rejection despite permanent burn delegate");
}

// Report: ../report/transfer-rejects-fake-asset-with-permanent-freeze-frozen.md
#[test]
fn transfer_rejects_fake_asset_with_permanent_freeze_frozen() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[(
                Plugin::PermanentFreezeDelegate(PermanentFreezeDelegate { frozen: true }),
                Authority::Owner,
            )],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of foreign-owned permanent-freeze asset");
}

// Section 11: Fake assets with multiple conflicting plugins

// Report: ../report/transfer-rejects-fake-asset-frozen-but-with-permanent-transfer.md
#[test]
fn transfer_rejects_fake_asset_frozen_but_with_permanent_transfer() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[
                (
                    Plugin::FreezeDelegate(FreezeDelegate { frozen: true }),
                    Authority::Owner,
                ),
                (
                    Plugin::PermanentTransferDelegate(PermanentTransferDelegate {}),
                    Authority::Owner,
                ),
            ],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of foreign-owned asset");
}

// Report: ../report/burn-rejects-fake-asset-unfrozen-with-permanent-burn.md
#[test]
fn burn_rejects_fake_asset_unfrozen_with_permanent_burn() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[
                (
                    Plugin::FreezeDelegate(FreezeDelegate { frozen: false }),
                    Authority::Owner,
                ),
                (
                    Plugin::PermanentBurnDelegate(PermanentBurnDelegate {}),
                    Authority::Owner,
                ),
            ],
        ),
    );

    let tx = engine.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer]);
    assert!(tx.error.is_some(), "expected rejection of foreign-owned asset");
}

// Section 12: Fake collections with plugins

// Report: ../report/transfer-rejects-when-fake-collection-has-permanent-freeze.md
#[test]
fn transfer_rejects_when_fake_collection_has_permanent_freeze() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let collection = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    // A genuine asset that points at a collection we will make foreign-owned.
    let real_asset = AssetV1::new(
        payer.pubkey(),
        UpdateAuthority::Collection(collection),
        "Real Asset".to_string(),
        "https://example.com/real".to_string(),
    );
    engine.set_account(
        &asset,
        Account {
            lamports: ACCOUNT_LAMPORTS,
            data: borsh::to_vec(&real_asset).unwrap(),
            owner: MPL_CORE_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    engine.set_account(
        &collection,
        build_collection_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[(
                Plugin::PermanentFreezeDelegate(PermanentFreezeDelegate { frozen: false }),
                Authority::UpdateAuthority,
            )],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_with_collection_instruction(
            asset,
            collection,
            payer.pubkey(),
            new_owner,
        )],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of foreign-owned collection");
}

// Report: ../report/transfer-rejects-when-fake-collection-has-permanent-transfer-delegate.md
#[test]
fn transfer_rejects_when_fake_collection_has_permanent_transfer_delegate() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let collection = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    let real_asset = AssetV1::new(
        payer.pubkey(),
        UpdateAuthority::Collection(collection),
        "Real Asset".to_string(),
        "https://example.com/real".to_string(),
    );
    engine.set_account(
        &asset,
        Account {
            lamports: ACCOUNT_LAMPORTS,
            data: borsh::to_vec(&real_asset).unwrap(),
            owner: MPL_CORE_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    engine.set_account(
        &collection,
        build_collection_with_plugins(
            &payer.pubkey(),
            &FAKE_PROGRAM_ID,
            &[(
                Plugin::PermanentTransferDelegate(PermanentTransferDelegate {}),
                Authority::UpdateAuthority,
            )],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_with_collection_instruction(
            asset,
            collection,
            payer.pubkey(),
            new_owner,
        )],
        &[&payer],
    );
    assert!(tx.error.is_some(), "expected rejection of foreign-owned collection");
}

// Section 13: Valid frozen asset sanity checks

// Report: ../report/transfer-rejects-valid-frozen-asset.md
#[test]
fn transfer_rejects_valid_frozen_asset() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &MPL_CORE_ID,
            &[(
                Plugin::FreezeDelegate(FreezeDelegate { frozen: true }),
                Authority::Owner,
            )],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    // A valid frozen asset rejects transfer due to the freeze plugin.
    assert!(tx.error.is_some(), "expected rejection of frozen asset");
}

// Report: ../report/transfer-succeeds-valid-unfrozen-asset-with-freeze-plugin.md
#[test]
fn transfer_succeeds_valid_unfrozen_asset_with_freeze_plugin() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = Pubkey::new_unique();
    let new_owner = Pubkey::new_unique();

    engine.set_account(
        &asset,
        build_asset_with_plugins(
            &payer.pubkey(),
            &MPL_CORE_ID,
            &[(
                Plugin::FreezeDelegate(FreezeDelegate { frozen: false }),
                Authority::Owner,
            )],
        ),
    );
    engine.set_account(&new_owner, Account::default());

    let tx = engine.send(
        &[transfer_v1_instruction(asset, payer.pubkey(), new_owner)],
        &[&payer],
    );
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Report: one Markdown page per scenario (completeness: all 25).

#[test]
fn generate_account_ownership_report() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/report");
    std::fs::create_dir_all(dir).unwrap();

    // Each entry: (test fn name, human title, one-line intent, scenario fn).
    // The page is named after its test fn in kebab-case (`_` -> `-`), and the
    // scenario fn replays that test's setup on a fresh engine, returning the
    // Transaction model. This keeps the report 1:1 with the `#[test]`s above.
    type Scenario = fn(&mut MolluskBackend) -> testsvm::model::Transaction;
    let scenarios: &[(&str, &str, &str, Scenario)] = &[
        (
            "transfer_rejects_fake_asset_owned_by_different_program",
            "Transfer rejects a foreign-owned fake asset",
            "An account carries valid AssetV1 bytes but is owned by an attacker's program; transfer must reject it.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, fake_asset_account(&payer.pubkey(), &FAKE_PROGRAM_ID));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "burn_rejects_fake_asset_owned_by_different_program",
            "Burn rejects a foreign-owned fake asset",
            "A foreign-owned account with valid AssetV1 bytes is presented to burn; it is rejected.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, fake_asset_account(&payer.pubkey(), &FAKE_PROGRAM_ID));
                e.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer])
            },
        ),
        (
            "update_rejects_fake_asset_owned_by_different_program",
            "Update rejects a foreign-owned fake asset",
            "A foreign-owned account with valid AssetV1 bytes is presented to update; it is rejected.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, fake_asset_account(&payer.pubkey(), &FAKE_PROGRAM_ID));
                e.send(&[update_v1_instruction(asset, payer.pubkey())], &[&payer])
            },
        ),
        (
            "update_collection_rejects_fake_collection_owned_by_different_program",
            "Update-collection rejects a foreign-owned fake collection",
            "A foreign-owned account with valid CollectionV1 bytes is presented to update-collection; it is rejected.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let collection = e.actor("Collection", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&collection, fake_collection_account(&payer.pubkey(), &FAKE_PROGRAM_ID));
                e.send(&[update_collection_v1_instruction(collection, payer.pubkey())], &[&payer])
            },
        ),
        (
            "transfer_rejects_account_with_wrong_discriminator",
            "Transfer rejects the wrong discriminator",
            "An mpl-core-owned account carries CollectionV1 (disc 5) bytes where AssetV1 (disc 1) is expected; transfer rejects it.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                let collection = CollectionV1::new(payer.pubkey(), "Wrong".to_string(), "https://wrong.com".to_string(), 0, 0);
                e.set_account(&asset, Account { lamports: ACCOUNT_LAMPORTS, data: borsh::to_vec(&collection).unwrap(), owner: MPL_CORE_ID, executable: false, rent_epoch: 0 });
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "burn_rejects_account_with_wrong_discriminator",
            "Burn rejects the wrong discriminator",
            "An mpl-core-owned account starts with Key::Uninitialized (0); burn rejects it.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let mut data = vec![0u8; 100];
                data[0] = Key::Uninitialized as u8;
                e.set_account(&asset, Account { lamports: ACCOUNT_LAMPORTS, data, owner: MPL_CORE_ID, executable: false, rent_epoch: 0 });
                e.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer])
            },
        ),
        (
            "transfer_rejects_random_data_account",
            "Transfer rejects garbage data",
            "An mpl-core-owned account is filled with 0xFF; transfer fails at deserialization.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.set_account(&asset, Account { lamports: ACCOUNT_LAMPORTS, data: vec![0xFF; 200], owner: MPL_CORE_ID, executable: false, rent_epoch: 0 });
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "transfer_rejects_empty_account",
            "Transfer rejects an empty account",
            "An mpl-core-owned account with no data is presented to transfer; it fails immediately.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.set_account(&asset, Account { lamports: ACCOUNT_LAMPORTS, data: vec![], owner: MPL_CORE_ID, executable: false, rent_epoch: 0 });
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "transfer_succeeds_with_valid_asset",
            "Transfer succeeds on a valid asset",
            "A valid mpl-core-owned asset is handed to a new owner by its owner; the transfer succeeds.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.set_account(&asset, valid_asset_account(&payer.pubkey()));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "transfer_rejects_system_owned_account_with_asset_discriminator",
            "Transfer rejects a system-owned account with an asset discriminator",
            "A System-owned account carries valid AssetV1 bytes; transfer rejects it on the owner check.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.set_account(&asset, fake_asset_account(&payer.pubkey(), &SYSTEM_PROGRAM));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "update_rejects_system_owned_account_with_asset_discriminator",
            "Update rejects a system-owned account with an asset discriminator",
            "A System-owned account carries valid AssetV1 bytes; update rejects it on the owner check.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                e.set_account(&asset, fake_asset_account(&payer.pubkey(), &SYSTEM_PROGRAM));
                e.send(&[update_v1_instruction(asset, payer.pubkey())], &[&payer])
            },
        ),
        (
            "transfer_rejects_unauthorized_caller_on_valid_asset",
            "Transfer rejects an unauthorized caller",
            "A valid asset is owned by someone else; an attacker signs the transfer and is rejected on the authority check.",
            |e| {
                let actual_owner = Pubkey::new_unique();
                let attacker = e.actor("Attacker", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.set_account(&asset, valid_asset_account(&actual_owner));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, attacker.pubkey(), new_owner)], &[&attacker])
            },
        ),
        (
            "burn_rejects_unauthorized_caller_on_valid_asset",
            "Burn rejects an unauthorized caller",
            "A valid asset is owned by someone else; an attacker signs the burn and is rejected on the authority check.",
            |e| {
                let actual_owner = Pubkey::new_unique();
                let attacker = e.actor("Attacker", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                e.set_account(&asset, valid_asset_account(&actual_owner));
                e.send(&[burn_v1_instruction(asset, attacker.pubkey())], &[&attacker])
            },
        ),
        (
            "transfer_rejects_fake_frozen_asset_owned_by_different_program",
            "Transfer rejects a foreign-owned frozen fake asset",
            "A foreign-owned asset embeds FreezeDelegate{frozen:true}; transfer rejects it on ownership before the freeze state matters.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }), Authority::Owner)]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "transfer_rejects_fake_unfrozen_asset_owned_by_different_program",
            "Transfer rejects a foreign-owned unfrozen fake asset",
            "Even an unfrozen foreign-owned asset (FreezeDelegate{frozen:false}) is rejected on ownership.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }), Authority::Owner)]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "burn_rejects_fake_frozen_asset_owned_by_different_program",
            "Burn rejects a foreign-owned frozen fake asset",
            "A foreign-owned asset embeds FreezeDelegate{frozen:true}; burn rejects it on ownership.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }), Authority::Owner)]));
                e.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer])
            },
        ),
        (
            "transfer_rejects_fake_asset_with_permanent_transfer_delegate",
            "Transfer rejects a foreign-owned asset with a permanent transfer delegate",
            "A PermanentTransferDelegate would normally force-approve any transfer; on a foreign-owned account it is rejected first.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[(Plugin::PermanentTransferDelegate(PermanentTransferDelegate {}), Authority::Owner)]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "burn_rejects_fake_asset_with_permanent_burn_delegate",
            "Burn rejects a foreign-owned asset with a permanent burn delegate",
            "A PermanentBurnDelegate would normally force-approve any burn; on a foreign-owned account it is rejected first.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[(Plugin::PermanentBurnDelegate(PermanentBurnDelegate {}), Authority::Owner)]));
                e.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer])
            },
        ),
        (
            "transfer_rejects_fake_asset_with_permanent_freeze_frozen",
            "Transfer rejects a foreign-owned asset with a frozen permanent-freeze delegate",
            "A PermanentFreezeDelegate{frozen:true} on a foreign-owned account is rejected on ownership.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[(Plugin::PermanentFreezeDelegate(PermanentFreezeDelegate { frozen: true }), Authority::Owner)]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "transfer_rejects_fake_asset_frozen_but_with_permanent_transfer",
            "Transfer rejects a foreign-owned frozen asset that also carries a permanent transfer delegate",
            "Conflicting favorable plugins (frozen + permanent transfer) on a foreign-owned account; ownership rejection wins.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[
                    (Plugin::FreezeDelegate(FreezeDelegate { frozen: true }), Authority::Owner),
                    (Plugin::PermanentTransferDelegate(PermanentTransferDelegate {}), Authority::Owner),
                ]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "burn_rejects_fake_asset_unfrozen_with_permanent_burn",
            "Burn rejects a foreign-owned unfrozen asset that also carries a permanent burn delegate",
            "All-favorable plugin state (unfrozen + permanent burn) on a foreign-owned account; ownership rejection wins.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[
                    (Plugin::FreezeDelegate(FreezeDelegate { frozen: false }), Authority::Owner),
                    (Plugin::PermanentBurnDelegate(PermanentBurnDelegate {}), Authority::Owner),
                ]));
                e.send(&[burn_v1_instruction(asset, payer.pubkey())], &[&payer])
            },
        ),
        (
            "transfer_rejects_when_fake_collection_has_permanent_freeze",
            "Transfer rejects when a referenced collection is foreign-owned (permanent freeze)",
            "A valid asset references a foreign-owned collection carrying a PermanentFreezeDelegate; the collection's owner check rejects the transfer.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let collection = e.actor("Collection", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                let real_asset = AssetV1::new(payer.pubkey(), UpdateAuthority::Collection(collection), "Real Asset".to_string(), "https://example.com/real".to_string());
                e.set_account(&asset, Account { lamports: ACCOUNT_LAMPORTS, data: borsh::to_vec(&real_asset).unwrap(), owner: MPL_CORE_ID, executable: false, rent_epoch: 0 });
                e.set_account(&collection, build_collection_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[(Plugin::PermanentFreezeDelegate(PermanentFreezeDelegate { frozen: false }), Authority::UpdateAuthority)]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_with_collection_instruction(asset, collection, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "transfer_rejects_when_fake_collection_has_permanent_transfer_delegate",
            "Transfer rejects when a referenced collection is foreign-owned (permanent transfer)",
            "A valid asset references a foreign-owned collection carrying a PermanentTransferDelegate; the collection's owner check rejects the transfer.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let collection = e.actor("Collection", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.register_alias(&FAKE_PROGRAM_ID, "ForeignProgram");
                let real_asset = AssetV1::new(payer.pubkey(), UpdateAuthority::Collection(collection), "Real Asset".to_string(), "https://example.com/real".to_string());
                e.set_account(&asset, Account { lamports: ACCOUNT_LAMPORTS, data: borsh::to_vec(&real_asset).unwrap(), owner: MPL_CORE_ID, executable: false, rent_epoch: 0 });
                e.set_account(&collection, build_collection_with_plugins(&payer.pubkey(), &FAKE_PROGRAM_ID, &[(Plugin::PermanentTransferDelegate(PermanentTransferDelegate {}), Authority::UpdateAuthority)]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_with_collection_instruction(asset, collection, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "transfer_rejects_valid_frozen_asset",
            "Transfer rejects a valid frozen asset",
            "A valid mpl-core-owned asset with FreezeDelegate{frozen:true}; the freeze plugin rejects the transfer.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &MPL_CORE_ID, &[(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }), Authority::Owner)]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
        (
            "transfer_succeeds_valid_unfrozen_asset_with_freeze_plugin",
            "Transfer succeeds on a valid unfrozen asset with a freeze plugin",
            "A valid mpl-core-owned asset with FreezeDelegate{frozen:false}; the owner's transfer succeeds.",
            |e| {
                let payer = e.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = e.actor("Asset", 0).pubkey();
                let new_owner = e.actor("NewOwner", 0).pubkey();
                e.set_account(&asset, build_asset_with_plugins(&payer.pubkey(), &MPL_CORE_ID, &[(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }), Authority::Owner)]));
                e.set_account(&new_owner, Account::default());
                e.send(&[transfer_v1_instruction(asset, payer.pubkey(), new_owner)], &[&payer])
            },
        ),
    ];

    for (test_fn, title, intent, run) in scenarios {
        let mut engine = engine_with_program();
        let tx = run(&mut engine);
        // The page is named after its test fn in kebab-case; `render_scenario`
        // links back to that fn (and the fn carries the reverse `Report:` link).
        let page = test_fn.replace('_', "-");
        std::fs::write(
            format!("{dir}/{page}.md"),
            render_scenario(title, intent, "tests/account_ownership.rs", test_fn, &tx),
        )
        .unwrap();
    }

    assert_eq!(scenarios.len(), 25, "all 25 scenarios must render a page");
    println!("wrote {} account-ownership scenario pages to {dir}", scenarios.len());
}
