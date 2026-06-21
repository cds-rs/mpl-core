//! mpl-core's AgentIdentity external-plugin suite, ported from the project's
//! native mollusk tests (`programs/mpl-core/tests/agent_identity.rs`) onto the
//! TestSVM mollusk adapter. Every original `#[test]` is preserved by name; the
//! assertion shifts from mollusk's `process_instruction` + `matches!` to the
//! returned `Transaction` model (`tx.error.is_none()` / `is_some()`).
//!
//! The adapter, like mollusk's own harness, bypasses transaction-level signature
//! verification: it processes through `MolluskContext::process_instruction`, so a
//! PDA marked `signer: true` in an `AccountMeta` satisfies the program's
//! `assert_signer()` without a tx-level keypair. The PDA therefore stays a bare
//! pubkey in the instruction and is NOT handed to `send`; only the real signers
//! (the fee-paying payer) are actors with keypairs.

#![allow(deprecated)]

use {
    borsh::BorshSerialize,
    mpl_core_babelfish_tests::{world::*, *},
    mpl_core_program::{
        plugins::{
            AgentIdentityInitInfo, AgentIdentityUpdateInfo, ExternalCheckResult,
            ExternalPluginAdapterInitInfo, ExternalPluginAdapterKey,
            ExternalPluginAdapterUpdateInfo, HookableLifecycleEvent,
        },
        state::{Authority, CollectionV1},
        ID as MPL_CORE_ID,
    },
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    testsvm::{model::Transaction, TestSVM},
    testsvm_mollusk::MolluskBackend,
};

// The mpl-agent-identity program ID used for PDA derivation.
const AGENT_IDENTITY_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("1DREGFgysWYxLnRnKQnwrxnJQeSMk2HmGaC6whw2B2p");

// Account data builders

/// Derive the agent identity PDA for an asset.
fn agent_identity_pda(asset: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"agent_identity", asset.as_ref()],
        &AGENT_IDENTITY_PROGRAM_ID,
    )
}

/// A payer-style account funded with lamports, system-owned.
fn lamport_account() -> Account {
    system_owned_account(ACCOUNT_LAMPORTS)
}

// Instruction builders

/// Builds a CreateV2 instruction with an AgentIdentity external plugin.
///
/// Discriminator: 20. The agent-identity PDA is a `signer: true` remaining
/// account the program signs for: a bare pubkey here, never a tx-level signer.
fn create_v2_with_agent_identity(
    asset: Pubkey,
    payer: Pubkey,
    agent_identity_pda: Pubkey,
    init_info: &AgentIdentityInitInfo,
) -> Instruction {
    let mut data = vec![20u8];

    // DataState::AccountState = variant 0
    0u8.serialize(&mut data).unwrap();
    "Test Asset".to_string().serialize(&mut data).unwrap();
    "https://example.com/test"
        .to_string()
        .serialize(&mut data)
        .unwrap();
    // plugins: Option<Vec<PluginAuthorityPair>> = None
    0u8.serialize(&mut data).unwrap();
    let adapters = vec![ExternalPluginAdapterInitInfo::AgentIdentity(
        init_info.clone(),
    )];
    Some(adapters).serialize(&mut data).unwrap();

    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, true),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // collection (optional, MPL_CORE_ID placeholder)
            AccountMeta::new_readonly(MPL_CORE_ID, false), // authority (optional)
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // owner (optional)
            AccountMeta::new_readonly(MPL_CORE_ID, false), // update_authority (optional)
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // log_wrapper (optional)
            AccountMeta::new_readonly(agent_identity_pda, true), // PDA signs as remaining account
        ],
    )
}

/// Builds a CreateCollectionV2 instruction with an AgentIdentity external plugin.
///
/// Discriminator: 21.
fn create_collection_v2_with_agent_identity(
    collection: Pubkey,
    payer: Pubkey,
    init_info: &AgentIdentityInitInfo,
) -> Instruction {
    let mut data = vec![21u8];

    "Test Collection".to_string().serialize(&mut data).unwrap();
    "https://example.com/collection"
        .to_string()
        .serialize(&mut data)
        .unwrap();
    // plugins: Option<Vec<PluginAuthorityPair>> = None
    0u8.serialize(&mut data).unwrap();
    let adapters = vec![ExternalPluginAdapterInitInfo::AgentIdentity(
        init_info.clone(),
    )];
    Some(adapters).serialize(&mut data).unwrap();

    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(collection, true),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // update_authority (optional)
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
    )
}

/// Builds an AddExternalPluginAdapterV1 instruction for AgentIdentity.
///
/// Discriminator: 22. The PDA at index 6 is a `signer: true` remaining account.
fn add_agent_identity_instruction(
    asset: Pubkey,
    payer: Pubkey,
    agent_identity_pda: Pubkey,
    init_info: &AgentIdentityInitInfo,
) -> Instruction {
    let mut data = vec![22u8];

    ExternalPluginAdapterInitInfo::AgentIdentity(init_info.clone())
        .serialize(&mut data)
        .unwrap();

    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // collection (optional)
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(payer, true), // authority
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // log_wrapper (optional)
            AccountMeta::new_readonly(agent_identity_pda, true), // PDA signs as remaining account
        ],
    )
}

/// Builds an AddCollectionExternalPluginAdapterV1 instruction for AgentIdentity.
///
/// Discriminator: 23.
fn add_collection_agent_identity_instruction(
    collection: Pubkey,
    payer: Pubkey,
    init_info: &AgentIdentityInitInfo,
) -> Instruction {
    let mut data = vec![23u8];

    ExternalPluginAdapterInitInfo::AgentIdentity(init_info.clone())
        .serialize(&mut data)
        .unwrap();

    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(collection, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(payer, true), // authority
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // log_wrapper (optional)
        ],
    )
}

/// Builds an UpdateExternalPluginAdapterV1 instruction.
///
/// Discriminator: 26.
fn update_agent_identity_instruction(
    asset: Pubkey,
    payer: Pubkey,
    update_info: &AgentIdentityUpdateInfo,
) -> Instruction {
    let mut data = vec![26u8];

    ExternalPluginAdapterKey::AgentIdentity
        .serialize(&mut data)
        .unwrap();
    ExternalPluginAdapterUpdateInfo::AgentIdentity(update_info.clone())
        .serialize(&mut data)
        .unwrap();

    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // collection (optional)
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(payer, true), // authority
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // log_wrapper (optional)
        ],
    )
}

/// Builds a RemoveExternalPluginAdapterV1 instruction.
///
/// Discriminator: 24.
fn remove_agent_identity_instruction(asset: Pubkey, payer: Pubkey) -> Instruction {
    let mut data = vec![24u8];

    ExternalPluginAdapterKey::AgentIdentity
        .serialize(&mut data)
        .unwrap();

    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // collection (optional)
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(payer, true), // authority
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // log_wrapper (optional)
        ],
    )
}

fn default_agent_identity_init_info() -> AgentIdentityInitInfo {
    AgentIdentityInitInfo {
        uri: "https://example.com/agent.json".to_string(),
        init_plugin_authority: None, // defaults to UpdateAuthority
        lifecycle_checks: vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x1 }, // CAN_LISTEN
        )],
    }
}

// Happy-path tests

// Report: ../report/create-asset-with-agent-identity.md
#[test]
fn create_asset_with_agent_identity() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = engine.actor("Asset", 0);
    let (pda, _) = agent_identity_pda(&asset.pubkey());
    engine.set_account(&pda, lamport_account());

    let instruction = create_v2_with_agent_identity(
        asset.pubkey(),
        payer.pubkey(),
        pda,
        &default_agent_identity_init_info(),
    );

    let tx = engine.send(&[instruction], &[&payer, &asset]);
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Report: ../report/add-agent-identity-to-existing-asset.md
#[test]
fn add_agent_identity_to_existing_asset() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();
    let (pda, _) = agent_identity_pda(&asset_key);

    engine.set_account(&asset_key, valid_asset_account(&payer.pubkey()));
    engine.set_account(&pda, lamport_account());

    let instruction = add_agent_identity_instruction(
        asset_key,
        payer.pubkey(),
        pda,
        &default_agent_identity_init_info(),
    );

    let tx = engine.send(&[instruction], &[&payer]);
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Report: ../report/update-agent-identity-uri.md
#[test]
fn update_agent_identity_uri() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();

    let asset_account = build_asset_with_agent_identity(
        &payer.pubkey(),
        "https://example.com/agent.json",
        Authority::UpdateAuthority,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x1 },
        )],
    );
    engine.set_account(&asset_key, asset_account);

    let instruction = update_agent_identity_instruction(
        asset_key,
        payer.pubkey(),
        &AgentIdentityUpdateInfo {
            uri: Some("https://example.com/updated-agent.json".to_string()),
            lifecycle_checks: None,
        },
    );

    let tx = engine.send(&[instruction], &[&payer]);
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Report: ../report/update-agent-identity-lifecycle-checks.md
#[test]
fn update_agent_identity_lifecycle_checks() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();

    let asset_account = build_asset_with_agent_identity(
        &payer.pubkey(),
        "https://example.com/agent.json",
        Authority::UpdateAuthority,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x1 },
        )],
    );
    engine.set_account(&asset_key, asset_account);

    let instruction = update_agent_identity_instruction(
        asset_key,
        payer.pubkey(),
        &AgentIdentityUpdateInfo {
            uri: None,
            lifecycle_checks: Some(vec![(
                HookableLifecycleEvent::Execute,
                ExternalCheckResult { flags: 0x3 }, // CAN_LISTEN | CAN_APPROVE
            )]),
        },
    );

    let tx = engine.send(&[instruction], &[&payer]);
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Report: ../report/update-agent-identity-uri-and-lifecycle-checks.md
#[test]
fn update_agent_identity_uri_and_lifecycle_checks() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();

    let asset_account = build_asset_with_agent_identity(
        &payer.pubkey(),
        "https://example.com/agent.json",
        Authority::UpdateAuthority,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x1 },
        )],
    );
    engine.set_account(&asset_key, asset_account);

    let instruction = update_agent_identity_instruction(
        asset_key,
        payer.pubkey(),
        &AgentIdentityUpdateInfo {
            uri: Some("https://example.com/agent-v3.json".to_string()),
            lifecycle_checks: Some(vec![(
                HookableLifecycleEvent::Execute,
                ExternalCheckResult { flags: 0x7 }, // CAN_LISTEN | CAN_APPROVE | CAN_REJECT
            )]),
        },
    );

    let tx = engine.send(&[instruction], &[&payer]);
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Report: ../report/remove-agent-identity.md
#[test]
fn remove_agent_identity() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();

    let asset_account = build_asset_with_agent_identity(
        &payer.pubkey(),
        "https://example.com/agent.json",
        Authority::UpdateAuthority,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x1 },
        )],
    );
    engine.set_account(&asset_key, asset_account);

    let instruction = remove_agent_identity_instruction(asset_key, payer.pubkey());

    let tx = engine.send(&[instruction], &[&payer]);
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Report: ../report/create-asset-with-agent-identity-multiple-lifecycle-checks.md
#[test]
fn create_asset_with_agent_identity_multiple_lifecycle_checks() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = engine.actor("Asset", 0);
    let (pda, _) = agent_identity_pda(&asset.pubkey());
    engine.set_account(&pda, lamport_account());

    let init_info = AgentIdentityInitInfo {
        uri: "https://example.com/agent.json".to_string(),
        init_plugin_authority: None,
        lifecycle_checks: vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 }, // CAN_LISTEN | CAN_APPROVE
        )],
    };

    let instruction =
        create_v2_with_agent_identity(asset.pubkey(), payer.pubkey(), pda, &init_info);

    let tx = engine.send(&[instruction], &[&payer, &asset]);
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Report: ../report/create-asset-with-agent-identity-address-authority.md
#[test]
fn create_asset_with_agent_identity_address_authority() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset = engine.actor("Asset", 0);
    let (pda, _) = agent_identity_pda(&asset.pubkey());
    engine.set_account(&pda, lamport_account());
    let plugin_authority = Pubkey::new_unique();

    let init_info = AgentIdentityInitInfo {
        uri: "https://example.com/agent.json".to_string(),
        init_plugin_authority: Some(Authority::Address {
            address: plugin_authority,
        }),
        lifecycle_checks: vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x1 },
        )],
    };

    let instruction =
        create_v2_with_agent_identity(asset.pubkey(), payer.pubkey(), pda, &init_info);

    let tx = engine.send(&[instruction], &[&payer, &asset]);
    assert!(tx.error.is_none(), "{:?}", tx.error);
}

// Negative / security tests

// Report: ../report/cannot-create-collection-with-agent-identity.md
#[test]
fn cannot_create_collection_with_agent_identity() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let collection = engine.actor("Collection", 0);

    let instruction = create_collection_v2_with_agent_identity(
        collection.pubkey(),
        payer.pubkey(),
        &default_agent_identity_init_info(),
    );
    engine.set_account(&collection.pubkey(), empty_asset_account());

    let tx = engine.send(&[instruction], &[&payer, &collection]);
    // AgentIdentity rejects collections via validate_create → InvalidPluginAdapterTarget (46).
    assert!(tx.error.is_some(), "expected failure, got success");
}

// Report: ../report/cannot-add-agent-identity-to-collection.md
#[test]
fn cannot_add_agent_identity_to_collection() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let collection_key = engine.actor("Collection", 0).pubkey();

    // Build a valid collection account.
    let collection = CollectionV1::new(
        payer.pubkey(),
        "Test".to_string(),
        "https://example.com".to_string(),
        0,
        0,
    );
    let data = borsh::to_vec(&collection).unwrap();
    let collection_account = Account {
        lamports: ACCOUNT_LAMPORTS,
        data,
        owner: MPL_CORE_ID,
        executable: false,
        rent_epoch: 0,
    };
    engine.set_account(&collection_key, collection_account);

    let instruction = add_collection_agent_identity_instruction(
        collection_key,
        payer.pubkey(),
        &default_agent_identity_init_info(),
    );

    let tx = engine.send(&[instruction], &[&payer]);
    // AgentIdentity rejects collections via validate_add_external_plugin_adapter → InvalidPluginAdapterTarget (46).
    assert!(tx.error.is_some(), "expected failure, got success");
}

// Report: ../report/cannot-add-agent-identity-without-pda-remaining-account.md
#[test]
fn cannot_add_agent_identity_without_pda_remaining_account() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();
    engine.set_account(&asset_key, valid_asset_account(&payer.pubkey()));

    // Build the instruction WITHOUT the PDA remaining account.
    let mut data = vec![22u8];
    ExternalPluginAdapterInitInfo::AgentIdentity(default_agent_identity_init_info())
        .serialize(&mut data)
        .unwrap();

    let instruction = Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset_key, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // collection (optional)
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(payer.pubkey(), true), // authority
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false), // log_wrapper (optional)
                                                           // NO PDA remaining account!
        ],
    );

    let tx = engine.send(&[instruction], &[&payer]);
    // Last account is the log_wrapper placeholder (not a signer) → MissingRequiredSignature.
    assert!(tx.error.is_some(), "expected failure, got success");
}

// Report: ../report/cannot-add-agent-identity-with-wrong-pda.md
#[test]
fn cannot_add_agent_identity_with_wrong_pda() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();
    engine.set_account(&asset_key, valid_asset_account(&payer.pubkey()));

    // Use a completely wrong PDA (derived from a different asset).
    let wrong_asset = Pubkey::new_unique();
    let (wrong_pda, _) = agent_identity_pda(&wrong_asset);
    engine.set_account(&wrong_pda, lamport_account());

    let instruction = add_agent_identity_instruction(
        asset_key,
        payer.pubkey(),
        wrong_pda,
        &default_agent_identity_init_info(),
    );

    let tx = engine.send(&[instruction], &[&payer]);
    // PDA doesn't match derivation for asset_key → AgentIdentityMustSign (51).
    assert!(tx.error.is_some(), "expected failure, got success");
}

// Report: ../report/cannot-add-agent-identity-with-unsigned-pda.md
#[test]
fn cannot_add_agent_identity_with_unsigned_pda() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();
    let (pda, _) = agent_identity_pda(&asset_key);
    engine.set_account(&asset_key, valid_asset_account(&payer.pubkey()));
    engine.set_account(&pda, lamport_account());

    // Build instruction with PDA present but NOT marked as signer.
    let mut data = vec![22u8];
    ExternalPluginAdapterInitInfo::AgentIdentity(default_agent_identity_init_info())
        .serialize(&mut data)
        .unwrap();

    let instruction = Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset_key, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(payer.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(pda, false), // PDA present but NOT signer
        ],
    );

    let tx = engine.send(&[instruction], &[&payer]);
    // PDA is present but not a signer → MissingRequiredSignature.
    assert!(tx.error.is_some(), "expected failure, got success");
}

// Report: ../report/cannot-add-duplicate-agent-identity.md
#[test]
fn cannot_add_duplicate_agent_identity() {
    let mut engine = engine_with_program();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = engine.actor("Asset", 0).pubkey();
    let (pda, _) = agent_identity_pda(&asset_key);

    // Asset already has an agent identity plugin.
    let asset_account = build_asset_with_agent_identity(
        &payer.pubkey(),
        "https://example.com/agent.json",
        Authority::UpdateAuthority,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x1 },
        )],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&pda, lamport_account());

    let instruction = add_agent_identity_instruction(
        asset_key,
        payer.pubkey(),
        pda,
        &AgentIdentityInitInfo {
            uri: "https://example.com/agent2.json".to_string(),
            init_plugin_authority: None,
            lifecycle_checks: vec![(
                HookableLifecycleEvent::Execute,
                ExternalCheckResult { flags: 0x1 },
            )],
        },
    );

    let tx = engine.send(&[instruction], &[&payer]);
    // Plugin already exists → ExternalPluginAdapterAlreadyExists (32).
    assert!(tx.error.is_some(), "expected failure, got success");
}

// Execution report: one Markdown page per `#[test]` (completeness: all 14).

/// Each report scenario replays its `#[test]`'s setup on a fresh engine and
/// returns the `Transaction` model. Built from actors (and aliased PDAs) so the
/// rendered trees stay byte-reproducible.
type ReportScenario = fn(&mut MolluskBackend) -> Transaction;

#[test]
fn generate_agent_identity_report() {
    // Each entry: (test fn name, human title, one-line intent, scenario fn).
    // The page is named after its test fn in kebab-case (`_` -> `-`), and the
    // scenario fn replays that test's setup on a fresh engine. This keeps the
    // report 1:1 with the `#[test]`s above, and `render_scenario` links each
    // page back to its test via the `test_fn` argument.
    let scenarios: &[(&str, &str, &str, ReportScenario)] = &[
        (
            "create_asset_with_agent_identity",
            "Create an asset with an AgentIdentity plugin",
            "Allocate a fresh mpl-core asset carrying an AgentIdentity external plugin. The \
             agent-identity PDA is a `signer: true` remaining account the program signs for; \
             the asset is allocated through a System CPI.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = engine.actor("Asset", 0);
                let (pda, _) = agent_identity_pda(&asset.pubkey());
                engine.register_alias(&pda, "AgentIdentityPda");
                engine.set_account(&pda, lamport_account());
                let ix = create_v2_with_agent_identity(
                    asset.pubkey(),
                    payer.pubkey(),
                    pda,
                    &default_agent_identity_init_info(),
                );
                engine.send(&[ix], &[&payer, &asset])
            },
        ),
        (
            "add_agent_identity_to_existing_asset",
            "Add an AgentIdentity plugin to an existing asset",
            "Attach an AgentIdentity external plugin to an already-initialized asset. The PDA \
             signs as a remaining account.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                let (pda, _) = agent_identity_pda(&asset_key);
                engine.register_alias(&pda, "AgentIdentityPda");
                engine.set_account(&asset_key, valid_asset_account(&payer.pubkey()));
                engine.set_account(&pda, lamport_account());
                let ix = add_agent_identity_instruction(
                    asset_key,
                    payer.pubkey(),
                    pda,
                    &default_agent_identity_init_info(),
                );
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "update_agent_identity_uri",
            "Update an AgentIdentity plugin's URI",
            "Rewrite the AgentIdentity plugin's URI on an asset that already carries it. A flat \
             instruction: the update authority signs, no PDA needed.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                engine.set_account(
                    &asset_key,
                    build_asset_with_agent_identity(
                        &payer.pubkey(),
                        "https://example.com/agent.json",
                        Authority::UpdateAuthority,
                        vec![(
                            HookableLifecycleEvent::Execute,
                            ExternalCheckResult { flags: 0x1 },
                        )],
                    ),
                );
                let ix = update_agent_identity_instruction(
                    asset_key,
                    payer.pubkey(),
                    &AgentIdentityUpdateInfo {
                        uri: Some("https://example.com/updated-agent.json".to_string()),
                        lifecycle_checks: None,
                    },
                );
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "update_agent_identity_lifecycle_checks",
            "Update an AgentIdentity plugin's lifecycle checks",
            "Rewrite only the lifecycle-check flags on an asset's AgentIdentity plugin, widening \
             them to CAN_LISTEN | CAN_APPROVE. The update authority signs; no PDA needed.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                engine.set_account(
                    &asset_key,
                    build_asset_with_agent_identity(
                        &payer.pubkey(),
                        "https://example.com/agent.json",
                        Authority::UpdateAuthority,
                        vec![(
                            HookableLifecycleEvent::Execute,
                            ExternalCheckResult { flags: 0x1 },
                        )],
                    ),
                );
                let ix = update_agent_identity_instruction(
                    asset_key,
                    payer.pubkey(),
                    &AgentIdentityUpdateInfo {
                        uri: None,
                        lifecycle_checks: Some(vec![(
                            HookableLifecycleEvent::Execute,
                            ExternalCheckResult { flags: 0x3 },
                        )]),
                    },
                );
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "update_agent_identity_uri_and_lifecycle_checks",
            "Update an AgentIdentity plugin's URI and lifecycle checks",
            "Rewrite both the URI and the lifecycle-check flags (to CAN_LISTEN | CAN_APPROVE | \
             CAN_REJECT) in one update instruction. The update authority signs.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                engine.set_account(
                    &asset_key,
                    build_asset_with_agent_identity(
                        &payer.pubkey(),
                        "https://example.com/agent.json",
                        Authority::UpdateAuthority,
                        vec![(
                            HookableLifecycleEvent::Execute,
                            ExternalCheckResult { flags: 0x1 },
                        )],
                    ),
                );
                let ix = update_agent_identity_instruction(
                    asset_key,
                    payer.pubkey(),
                    &AgentIdentityUpdateInfo {
                        uri: Some("https://example.com/agent-v3.json".to_string()),
                        lifecycle_checks: Some(vec![(
                            HookableLifecycleEvent::Execute,
                            ExternalCheckResult { flags: 0x7 },
                        )]),
                    },
                );
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "remove_agent_identity",
            "Remove an AgentIdentity plugin",
            "Strip the AgentIdentity external plugin from an asset that carries it.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                engine.set_account(
                    &asset_key,
                    build_asset_with_agent_identity(
                        &payer.pubkey(),
                        "https://example.com/agent.json",
                        Authority::UpdateAuthority,
                        vec![(
                            HookableLifecycleEvent::Execute,
                            ExternalCheckResult { flags: 0x1 },
                        )],
                    ),
                );
                let ix = remove_agent_identity_instruction(asset_key, payer.pubkey());
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "create_asset_with_agent_identity_multiple_lifecycle_checks",
            "Create an asset with multiple lifecycle checks",
            "Allocate an asset whose AgentIdentity plugin registers a wider lifecycle check \
             (CAN_LISTEN | CAN_APPROVE) at creation time.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = engine.actor("Asset", 0);
                let (pda, _) = agent_identity_pda(&asset.pubkey());
                engine.register_alias(&pda, "AgentIdentityPda");
                engine.set_account(&pda, lamport_account());
                let init_info = AgentIdentityInitInfo {
                    uri: "https://example.com/agent.json".to_string(),
                    init_plugin_authority: None,
                    lifecycle_checks: vec![(
                        HookableLifecycleEvent::Execute,
                        ExternalCheckResult { flags: 0x3 },
                    )],
                };
                let ix = create_v2_with_agent_identity(
                    asset.pubkey(),
                    payer.pubkey(),
                    pda,
                    &init_info,
                );
                engine.send(&[ix], &[&payer, &asset])
            },
        ),
        (
            "create_asset_with_agent_identity_address_authority",
            "Create an asset with an address plugin authority",
            "Allocate an asset whose AgentIdentity plugin authority is an explicit address rather \
             than the default UpdateAuthority.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset = engine.actor("Asset", 0);
                let (pda, _) = agent_identity_pda(&asset.pubkey());
                engine.register_alias(&pda, "AgentIdentityPda");
                engine.set_account(&pda, lamport_account());
                let plugin_authority = Pubkey::new_unique();
                let init_info = AgentIdentityInitInfo {
                    uri: "https://example.com/agent.json".to_string(),
                    init_plugin_authority: Some(Authority::Address {
                        address: plugin_authority,
                    }),
                    lifecycle_checks: vec![(
                        HookableLifecycleEvent::Execute,
                        ExternalCheckResult { flags: 0x1 },
                    )],
                };
                let ix = create_v2_with_agent_identity(
                    asset.pubkey(),
                    payer.pubkey(),
                    pda,
                    &init_info,
                );
                engine.send(&[ix], &[&payer, &asset])
            },
        ),
        (
            "cannot_create_collection_with_agent_identity",
            "Reject an AgentIdentity plugin on a created collection",
            "Attempt to create a collection with an AgentIdentity plugin. AgentIdentity targets \
             assets only; the program rejects the collection target.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let collection = engine.actor("Collection", 0);
                let ix = create_collection_v2_with_agent_identity(
                    collection.pubkey(),
                    payer.pubkey(),
                    &default_agent_identity_init_info(),
                );
                engine.set_account(&collection.pubkey(), empty_asset_account());
                engine.send(&[ix], &[&payer, &collection])
            },
        ),
        (
            "cannot_add_agent_identity_to_collection",
            "Reject adding an AgentIdentity plugin to a collection",
            "Attempt to add an AgentIdentity plugin to an existing collection. The plugin targets \
             assets only; the add is rejected.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let collection_key = engine.actor("Collection", 0).pubkey();
                let collection = CollectionV1::new(
                    payer.pubkey(),
                    "Test".to_string(),
                    "https://example.com".to_string(),
                    0,
                    0,
                );
                let data = borsh::to_vec(&collection).unwrap();
                engine.set_account(
                    &collection_key,
                    Account {
                        lamports: ACCOUNT_LAMPORTS,
                        data,
                        owner: MPL_CORE_ID,
                        executable: false,
                        rent_epoch: 0,
                    },
                );
                let ix = add_collection_agent_identity_instruction(
                    collection_key,
                    payer.pubkey(),
                    &default_agent_identity_init_info(),
                );
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "cannot_add_agent_identity_without_pda_remaining_account",
            "Reject adding an AgentIdentity plugin without the PDA",
            "Attempt to add an AgentIdentity plugin with the signing PDA remaining account \
             omitted; the program cannot find a signer and rejects.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                engine.set_account(&asset_key, valid_asset_account(&payer.pubkey()));
                let mut data = vec![22u8];
                ExternalPluginAdapterInitInfo::AgentIdentity(default_agent_identity_init_info())
                    .serialize(&mut data)
                    .unwrap();
                let ix = Instruction::new_with_bytes(
                    MPL_CORE_ID,
                    &data,
                    vec![
                        AccountMeta::new(asset_key, false),
                        AccountMeta::new_readonly(MPL_CORE_ID, false),
                        AccountMeta::new(payer.pubkey(), true),
                        AccountMeta::new_readonly(payer.pubkey(), true),
                        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
                        AccountMeta::new_readonly(MPL_CORE_ID, false),
                    ],
                );
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "cannot_add_agent_identity_with_wrong_pda",
            "Reject a mismatched AgentIdentity PDA",
            "Attempt to add an AgentIdentity plugin signed by a PDA derived from a different \
             asset. The derivation check rejects the wrong PDA.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                engine.set_account(&asset_key, valid_asset_account(&payer.pubkey()));
                let wrong_asset = engine.actor("WrongAsset", 0).pubkey();
                let (wrong_pda, _) = agent_identity_pda(&wrong_asset);
                engine.register_alias(&wrong_pda, "WrongPda");
                engine.set_account(&wrong_pda, lamport_account());
                let ix = add_agent_identity_instruction(
                    asset_key,
                    payer.pubkey(),
                    wrong_pda,
                    &default_agent_identity_init_info(),
                );
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "cannot_add_agent_identity_with_unsigned_pda",
            "Reject an unsigned AgentIdentity PDA",
            "Attempt to add an AgentIdentity plugin where the correct PDA is present but not \
             marked as a signer; the program rejects the missing signature.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                let (pda, _) = agent_identity_pda(&asset_key);
                engine.register_alias(&pda, "AgentIdentityPda");
                engine.set_account(&asset_key, valid_asset_account(&payer.pubkey()));
                engine.set_account(&pda, lamport_account());
                let mut data = vec![22u8];
                ExternalPluginAdapterInitInfo::AgentIdentity(default_agent_identity_init_info())
                    .serialize(&mut data)
                    .unwrap();
                let ix = Instruction::new_with_bytes(
                    MPL_CORE_ID,
                    &data,
                    vec![
                        AccountMeta::new(asset_key, false),
                        AccountMeta::new_readonly(MPL_CORE_ID, false),
                        AccountMeta::new(payer.pubkey(), true),
                        AccountMeta::new_readonly(payer.pubkey(), true),
                        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
                        AccountMeta::new_readonly(MPL_CORE_ID, false),
                        AccountMeta::new_readonly(pda, false),
                    ],
                );
                engine.send(&[ix], &[&payer])
            },
        ),
        (
            "cannot_add_duplicate_agent_identity",
            "Reject a duplicate AgentIdentity plugin",
            "Attempt to add a second AgentIdentity plugin to an asset that already carries one; \
             the program rejects the duplicate.",
            |engine| {
                let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
                let asset_key = engine.actor("Asset", 0).pubkey();
                let (pda, _) = agent_identity_pda(&asset_key);
                engine.register_alias(&pda, "AgentIdentityPda");
                engine.set_account(
                    &asset_key,
                    build_asset_with_agent_identity(
                        &payer.pubkey(),
                        "https://example.com/agent.json",
                        Authority::UpdateAuthority,
                        vec![(
                            HookableLifecycleEvent::Execute,
                            ExternalCheckResult { flags: 0x1 },
                        )],
                    ),
                );
                engine.set_account(&pda, lamport_account());
                let ix = add_agent_identity_instruction(
                    asset_key,
                    payer.pubkey(),
                    pda,
                    &AgentIdentityInitInfo {
                        uri: "https://example.com/agent2.json".to_string(),
                        init_plugin_authority: None,
                        lifecycle_checks: vec![(
                            HookableLifecycleEvent::Execute,
                            ExternalCheckResult { flags: 0x1 },
                        )],
                    },
                );
                engine.send(&[ix], &[&payer])
            },
        ),
    ];

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/report");
    std::fs::create_dir_all(dir).unwrap();

    for (test_fn, title, intent, run) in scenarios {
        let mut engine = engine_with_program();
        let tx = run(&mut engine);
        let page = test_fn.replace('_', "-");
        std::fs::write(
            format!("{dir}/{page}.md"),
            render_scenario(title, intent, "tests/agent_identity.rs", test_fn, &tx),
        )
        .unwrap();
    }

    println!("wrote {} agent-identity report pages to {dir}", scenarios.len());
}
