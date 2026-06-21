//! mpl-core's execution-delegate suite, run through the TestSVM mollusk adapter
//! against the project's own built `mpl_core_program.so`. A port of
//! `programs/mpl-core/tests/execution_delegate.rs`: same scenarios, same builder
//! and fabricator bodies, but driven through `engine.send` and asserting on the
//! returned `model::Transaction` instead of mollusk's `process_instruction`.
//!
//! These tests exercise `ExecuteV1` (discriminator 31) and the
//! `ExecutionDelegateRecordV1` mechanism in the AgentIdentity plugin's
//! `validate_execute`. When the record account sits at the first remaining
//! account (index 7) and its `authority`/`agent_asset` match, the plugin
//! approves execution for a non-owner authority.
//!
//! On the adapter, the only real transaction signers are the fee payer and (when
//! it is a separate account) the authority; both are minted via `engine.actor`.
//! The `asset_signer` PDA is signed for by the program through `invoke_signed`,
//! so it stays a bare pubkey. The CPI target Noop is not a loaded program here,
//! so the inner CPI fails: the original tolerated that as `UnknownError`, and the
//! adapter surfaces it as a transaction error. The port distinguishes "validation
//! reached the CPI, which then failed" (the error is NOT NoApprovals) from
//! "validation rejected with NoApprovals" by inspecting the error string.

use {
    borsh::BorshSerialize,
    mpl_core_babelfish_tests::{world::*, *},
    mpl_core_program::{
        plugins::{ExternalCheckResult, HookableLifecycleEvent},
        state::Authority,
        ID as MPL_CORE_ID, SPL_NOOP_ID,
    },
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    testsvm::{model::Transaction, TestSVM},
    testsvm_mollusk::MolluskBackend,
};

// Account data builders

/// An asset carrying an AgentIdentity plugin with the given lifecycle checks.
/// The execution suite fixes the URI and the UpdateAuthority; this adapts the
/// shared [`world::build_asset_with_agent_identity`] to that two-argument shape.
fn asset_with_execute_checks(
    owner: &Pubkey,
    lifecycle_checks: Vec<(HookableLifecycleEvent, ExternalCheckResult)>,
) -> Account {
    build_asset_with_agent_identity(
        owner,
        "https://example.com/agent.json",
        Authority::UpdateAuthority,
        lifecycle_checks,
    )
}

/// A funded, system-owned account used for passive non-signer slots (the
/// asset_signer slot in the validation tests).
fn payer_like_account() -> Account {
    system_owned_account(ACCOUNT_LAMPORTS)
}

/// An executable, bpf-loader-owned Noop account, the CPI target. The Noop binary
/// is not loaded into the adapter, so the inner CPI fails (tolerated).
fn noop_program_account() -> Account {
    executable_program_account()
}

// Instruction builder

/// Builds an ExecuteV1 instruction (discriminator 31).
///
/// Account layout (7 fixed):
///   0: asset (writable)
///   1: collection (optional, writable) -- MPL_CORE_ID placeholder
///   2: asset_signer -- PDA from ["mpl-core-execute", asset]
///   3: payer (writable, signer)
///   4: authority (optional, signer) -- MPL_CORE_ID placeholder if same as payer
///   5: system_program
///   6: program_id -- target CPI program
///
/// Remaining accounts (index 7+): delegate record (if any), then CPI accounts.
fn execute_v1_instruction(
    asset: Pubkey,
    asset_signer: Pubkey,
    payer: Pubkey,
    authority: Option<Pubkey>,
    program_id: Pubkey,
    delegate_record: Option<(Pubkey, Account)>,
    cpi_remaining_accounts: Vec<AccountMeta>,
    instruction_data: Vec<u8>,
) -> Instruction {
    let mut data = vec![31u8]; // ExecuteV1 discriminator

    // ExecuteV1Args: { instruction_data: Vec<u8> }
    instruction_data.serialize(&mut data).unwrap();

    let authority_meta = match authority {
        Some(auth) => AccountMeta::new_readonly(auth, true),
        None => AccountMeta::new_readonly(MPL_CORE_ID, false), // placeholder
    };

    let mut accounts = vec![
        AccountMeta::new(asset, false),                 // 0: asset
        AccountMeta::new(MPL_CORE_ID, false),           // 1: collection (optional)
        AccountMeta::new_readonly(asset_signer, false), // 2: asset_signer
        AccountMeta::new(payer, true),                  // 3: payer
        authority_meta,                                 // 4: authority
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false), // 5: system_program
        AccountMeta::new_readonly(program_id, false),   // 6: program_id
    ];

    if let Some((delegate_key, _)) = &delegate_record {
        accounts.push(AccountMeta::new_readonly(*delegate_key, false)); // 7: delegate record
    }

    accounts.extend(cpi_remaining_accounts);

    Instruction::new_with_bytes(MPL_CORE_ID, &data, accounts)
}

/// True when the transaction failed with the AgentIdentity plugin's `NoApprovals`
/// error (Custom(26) == 0x1a). The negative tests assert this; the tolerant
/// happy-path tests assert its absence (validation reached the CPI).
fn is_no_approvals(tx: &Transaction) -> bool {
    tx.error
        .as_ref()
        .map(|e| e.contains("0x1a") || e.contains("custom program error: 0x1a"))
        .unwrap_or(false)
}

/// True when the transaction failed with `InvalidExecutePda` (Custom(49) == 0x31).
fn is_invalid_execute_pda(tx: &Transaction) -> bool {
    tx.error
        .as_ref()
        .map(|e| e.contains("0x31"))
        .unwrap_or(false)
}

// Happy-path tests

/// Owner calls execute -- no delegate needed, owner authority approves.
/// The CPI to the noop program fails (binary not loaded) but validation passes:
/// the failure must NOT be NoApprovals.
///
// Report: ../report/execute-as-owner.md
#[test]
fn execute_as_owner() {
    let mut engine = engine_with_program();
    let owner = engine.actor("Owner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);

    let asset_account = asset_with_execute_checks(
        &owner.pubkey(),
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 }, // CAN_LISTEN | CAN_APPROVE
        )],
    );

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        owner.pubkey(),
        None, // authority = payer (owner)
        SPL_NOOP_ID,
        None,
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());

    let tx = engine.send(&[instruction], &[&owner]);
    // Validation should pass (owner is authority); we must NOT see NoApprovals.
    assert!(!is_no_approvals(&tx), "owner execute must not be NoApprovals: {:?}", tx.error);
}

// Execution delegate happy-path tests

/// Non-owner authority with a valid ExecutionDelegateRecordV1 that matches
/// (authority + agent_asset). The plugin has Execute with CAN_APPROVE, so
/// validation approves; the failure (if any) is NOT NoApprovals.
///
// Report: ../report/execute-with-valid-delegate-record.md
#[test]
fn execute_with_valid_delegate_record() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let executive_profile = Pubkey::new_unique();

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let delegate_record_key = Pubkey::new_unique();
    let delegate_record_account =
        build_execution_delegate_record(&executive_profile, &delegate_authority.pubkey(), &asset_key);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        delegate_authority.pubkey(),
        None, // authority = payer
        SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&delegate_authority]);
    assert!(!is_no_approvals(&tx), "valid delegate must not be NoApprovals: {:?}", tx.error);
}

/// Delegate as a separate authority account (payer != authority).
///
// Report: ../report/execute-with-delegate-as-separate-authority.md
#[test]
fn execute_with_delegate_as_separate_authority() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let executive_profile = Pubkey::new_unique();
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let delegate_record_key = Pubkey::new_unique();
    let delegate_record_account =
        build_execution_delegate_record(&executive_profile, &delegate_authority.pubkey(), &asset_key);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        payer.pubkey(),
        Some(delegate_authority.pubkey()), // separate authority
        SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&payer, &delegate_authority]);
    assert!(
        !is_no_approvals(&tx),
        "valid delegate with separate authority must not be NoApprovals: {:?}",
        tx.error
    );
}

// Negative / security tests

/// Non-owner, no remaining accounts (<=7 total) -- plugin abstains -- NoApprovals.
///
// Report: ../report/execute-non-owner-without-remaining-accounts.md
#[test]
fn execute_non_owner_without_remaining_accounts() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let non_owner = engine.actor("NonOwner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        non_owner.pubkey(),
        None,
        SPL_NOOP_ID,
        None,
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());

    let tx = engine.send(&[instruction], &[&non_owner]);
    assert!(tx.error.is_some(), "non-owner without remaining accounts must fail");
    assert!(is_no_approvals(&tx), "expected NoApprovals, got: {:?}", tx.error);
}

/// Wrong PDA for asset_signer -- InvalidExecutePda (Custom(49)).
///
// Report: ../report/execute-with-invalid-asset-signer-pda.md
#[test]
fn execute_with_invalid_asset_signer_pda() {
    let mut engine = engine_with_program();
    let owner = engine.actor("Owner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let wrong_signer = Pubkey::new_unique(); // not a valid PDA

    let asset_account = asset_with_execute_checks(
        &owner.pubkey(),
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let instruction = execute_v1_instruction(
        asset_key,
        wrong_signer,
        owner.pubkey(),
        None,
        SPL_NOOP_ID,
        None,
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&wrong_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());

    let tx = engine.send(&[instruction], &[&owner]);
    assert!(tx.error.is_some(), "wrong asset_signer PDA must fail");
    assert!(is_invalid_execute_pda(&tx), "expected InvalidExecutePda, got: {:?}", tx.error);
}

/// Asset has no AgentIdentity plugin, non-owner tries execute -- NoApprovals.
///
// Report: ../report/execute-non-owner-without-agent-identity-plugin.md
#[test]
fn execute_non_owner_without_agent_identity_plugin() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let non_owner = engine.actor("NonOwner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);

    let asset_account = valid_asset_account(&owner);

    let delegate_record_key = Pubkey::new_unique();
    let delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &non_owner.pubkey(), &asset_key);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        non_owner.pubkey(),
        None,
        SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&non_owner]);
    assert!(tx.error.is_some(), "non-owner without plugin must fail");
    assert!(is_no_approvals(&tx), "expected NoApprovals, got: {:?}", tx.error);
}

/// Delegate record has different authority than signer -- Abstain -- NoApprovals.
///
// Report: ../report/execute-with-wrong-authority-delegate.md
#[test]
fn execute_with_wrong_authority_delegate() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let actual_signer = engine.actor("Signer", ACCOUNT_LAMPORTS);
    let different_authority = Pubkey::new_unique(); // doesn't match signer
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let delegate_record_key = Pubkey::new_unique();
    // Record authority is different_authority, but signer is actual_signer.
    let delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &different_authority, &asset_key);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        actual_signer.pubkey(),
        None,
        SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&actual_signer]);
    assert!(tx.error.is_some(), "wrong-authority delegate must fail");
    assert!(is_no_approvals(&tx), "expected NoApprovals, got: {:?}", tx.error);
}

/// Delegate record has different agent_asset -- Abstain -- NoApprovals.
///
// Report: ../report/execute-with-wrong-asset-delegate.md
#[test]
fn execute_with_wrong_asset_delegate() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let wrong_asset = Pubkey::new_unique(); // doesn't match asset_key
    let (asset_signer, _) = asset_signer_pda(&asset_key);

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let delegate_record_key = Pubkey::new_unique();
    // Record agent_asset is wrong_asset, not asset_key.
    let delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &delegate_authority.pubkey(), &wrong_asset);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        delegate_authority.pubkey(),
        None,
        SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&delegate_authority]);
    assert!(tx.error.is_some(), "wrong-asset delegate must fail");
    assert!(is_no_approvals(&tx), "expected NoApprovals, got: {:?}", tx.error);
}

/// Delegate record account NOT owned by mpl_agent_tools::ID -- Abstain -- NoApprovals.
///
// Report: ../report/execute-with-wrong-program-owner-delegate.md
#[test]
fn execute_with_wrong_program_owner_delegate() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let delegate_record_key = Pubkey::new_unique();
    let mut delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &delegate_authority.pubkey(), &asset_key);
    delegate_record_account.owner = SYSTEM_PROGRAM;

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        delegate_authority.pubkey(),
        None,
        SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&delegate_authority]);
    assert!(tx.error.is_some(), "wrong-owner delegate must fail");
    assert!(is_no_approvals(&tx), "expected NoApprovals, got: {:?}", tx.error);
}

/// Delegate record with Key byte != 0x02 -- Abstain -- NoApprovals.
///
// Report: ../report/execute-with-invalid-discriminator-delegate.md
#[test]
fn execute_with_invalid_discriminator_delegate() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let delegate_record_key = Pubkey::new_unique();
    let mut delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &delegate_authority.pubkey(), &asset_key);
    delegate_record_account.data[0] = 0xFF;

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        delegate_authority.pubkey(),
        None,
        SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![],
        vec![],
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&delegate_authority]);
    assert!(tx.error.is_some(), "invalid-discriminator delegate must fail");
    assert!(is_no_approvals(&tx), "expected NoApprovals, got: {:?}", tx.error);
}

// CPI account shift bug tests

/// Baseline: owner executes a system transfer via CPI with no delegate record.
/// The asset_signer PDA is the source. This should succeed end-to-end.
///
// Report: ../report/execute-system-transfer-owner-no-delegate.md
#[test]
fn execute_system_transfer_owner_no_delegate() {
    let mut engine = engine_with_program();
    let owner = engine.actor("Owner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let dest = Pubkey::new_unique();

    let asset_account = asset_with_execute_checks(
        &owner.pubkey(),
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let transfer_ix = solana_system_interface::instruction::transfer(&asset_signer, &dest, 1);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        owner.pubkey(),
        None,
        SYSTEM_PROGRAM,
        None,
        vec![
            AccountMeta::new(asset_signer, false),
            AccountMeta::new(dest, false),
        ],
        transfer_ix.data,
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, system_owned_account(ACCOUNT_LAMPORTS));
    engine.set_account(&dest, system_owned_account(0));

    let tx = engine.send(&[instruction], &[&owner]);
    assert!(
        tx.error.is_none(),
        "owner system transfer CPI should succeed, got: {:?}",
        tx.error
    );
}

/// Delegate execute with system transfer CPI. The delegate record at
/// remaining_accounts[0] must be stripped before CPI so the target program
/// receives only the real CPI accounts.
///
// Report: ../report/execute-system-transfer-with-delegate-record-stripped.md
#[test]
fn execute_system_transfer_with_delegate_record_stripped() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let executive_profile = Pubkey::new_unique();
    let dest = Pubkey::new_unique();

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let delegate_record_key = Pubkey::new_unique();
    let delegate_record_account =
        build_execution_delegate_record(&executive_profile, &delegate_authority.pubkey(), &asset_key);

    let transfer_ix = solana_system_interface::instruction::transfer(&asset_signer, &dest, 1);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        delegate_authority.pubkey(),
        None,
        SYSTEM_PROGRAM,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![
            AccountMeta::new(asset_signer, false),
            AccountMeta::new(dest, false),
        ],
        transfer_ix.data,
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, system_owned_account(ACCOUNT_LAMPORTS));
    engine.set_account(&dest, system_owned_account(0));
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&delegate_authority]);
    assert!(
        tx.error.is_none(),
        "delegate system transfer CPI should succeed after stripping delegate record, got: {:?}",
        tx.error
    );
}

/// Delegate execute with separate authority and system transfer CPI.
///
// Report: ../report/execute-system-transfer-delegate-separate-authority.md
#[test]
fn execute_system_transfer_delegate_separate_authority() {
    let mut engine = engine_with_program();
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let executive_profile = Pubkey::new_unique();
    let dest = Pubkey::new_unique();

    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let delegate_record_key = Pubkey::new_unique();
    let delegate_record_account =
        build_execution_delegate_record(&executive_profile, &delegate_authority.pubkey(), &asset_key);

    let transfer_ix = solana_system_interface::instruction::transfer(&asset_signer, &dest, 1);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        payer.pubkey(),
        Some(delegate_authority.pubkey()),
        SYSTEM_PROGRAM,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![
            AccountMeta::new(asset_signer, false),
            AccountMeta::new(dest, false),
        ],
        transfer_ix.data,
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, system_owned_account(ACCOUNT_LAMPORTS));
    engine.set_account(&dest, system_owned_account(0));
    engine.set_account(&delegate_record_key, delegate_record_account);

    let tx = engine.send(&[instruction], &[&payer, &delegate_authority]);
    assert!(
        tx.error.is_none(),
        "delegate (separate authority) system transfer should succeed, got: {:?}",
        tx.error
    );
}

/// Non-delegate remaining account (not owned by mpl_agent_tools) should NOT be
/// stripped -- it should be passed through to the CPI as-is.
///
// Report: ../report/execute-system-transfer-non-delegate-remaining-account-preserved.md
#[test]
fn execute_system_transfer_non_delegate_remaining_account_preserved() {
    let mut engine = engine_with_program();
    let owner = engine.actor("Owner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let dest = Pubkey::new_unique();

    let asset_account = asset_with_execute_checks(
        &owner.pubkey(),
        vec![(
            HookableLifecycleEvent::Execute,
            ExternalCheckResult { flags: 0x3 },
        )],
    );

    let transfer_ix = solana_system_interface::instruction::transfer(&asset_signer, &dest, 1);

    let instruction = execute_v1_instruction(
        asset_key,
        asset_signer,
        owner.pubkey(),
        None,
        SYSTEM_PROGRAM,
        None,
        vec![
            AccountMeta::new(asset_signer, false),
            AccountMeta::new(dest, false),
        ],
        transfer_ix.data,
    );

    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, system_owned_account(ACCOUNT_LAMPORTS));
    engine.set_account(&dest, system_owned_account(0));

    let tx = engine.send(&[instruction], &[&owner]);
    assert!(
        tx.error.is_none(),
        "owner CPI without delegate should pass all remaining accounts through, got: {:?}",
        tx.error
    );
}

// Report

type Scenario = fn(&mut MolluskBackend) -> Transaction;

/// Each scenario reconstructs its setup against a fresh engine and returns the
/// `Transaction`, so the report renders one page per ported test. Determinism:
/// actors are minted by name, asset/PDA keys are derived rather than random
/// where the page would otherwise churn; the random `Pubkey::new_unique`
/// witnesses are stable enough for a one-shot render.
#[test]
fn generate_execution_delegate_report() {
    // One row per `#[test]`. The `test_fn` is both the report's filename stem
    // (kebab-cased) and the back-link target, so the page and its test share a
    // name: `fn execute_as_owner` <-> `report/execute-as-owner.md`.
    let scenarios: &[(&str, &str, &str, Scenario)] = &[
        (
            "execute_as_owner",
            "Execute as owner",
            "The asset owner invokes ExecuteV1. The AgentIdentity plugin approves \
             on owner authority; the CPI to the (unloaded) Noop program then fails, \
             so the run reaches the CPI frame but does not complete it.",
            scenario_execute_as_owner,
        ),
        (
            "execute_with_valid_delegate_record",
            "Execute via a valid delegate record",
            "A non-owner authority presents an ExecutionDelegateRecordV1 whose \
             authority and agent_asset match. The plugin approves; the Noop CPI \
             fails as above.",
            scenario_valid_delegate,
        ),
        (
            "execute_with_delegate_as_separate_authority",
            "Execute via a delegate as a separate authority",
            "The payer and the delegate authority are distinct signers; the record \
             still matches, so the plugin approves.",
            scenario_delegate_separate_authority,
        ),
        (
            "execute_non_owner_without_remaining_accounts",
            "Reject a non-owner with no delegate record",
            "A non-owner authority with no remaining accounts: the plugin abstains \
             and the transaction fails with NoApprovals.",
            scenario_non_owner_no_remaining,
        ),
        (
            "execute_with_invalid_asset_signer_pda",
            "Reject a wrong asset_signer PDA",
            "The asset_signer account is not the program-derived address, so the \
             program rejects with InvalidExecutePda.",
            scenario_invalid_pda,
        ),
        (
            "execute_non_owner_without_agent_identity_plugin",
            "Reject a non-owner on an asset with no AgentIdentity plugin",
            "The asset carries no AgentIdentity plugin, so there is nothing to \
             approve a non-owner: NoApprovals.",
            scenario_no_plugin,
        ),
        (
            "execute_with_wrong_authority_delegate",
            "Reject a delegate record with the wrong authority",
            "The record's authority does not match the signer, so the plugin \
             abstains: NoApprovals.",
            scenario_wrong_authority,
        ),
        (
            "execute_with_wrong_asset_delegate",
            "Reject a delegate record bound to a different asset",
            "The record's agent_asset does not match the asset being executed: \
             NoApprovals.",
            scenario_wrong_asset,
        ),
        (
            "execute_with_wrong_program_owner_delegate",
            "Reject a delegate record owned by the wrong program",
            "The record account is not owned by mpl-agent-tools, so the plugin \
             does not trust it: NoApprovals.",
            scenario_wrong_owner,
        ),
        (
            "execute_with_invalid_discriminator_delegate",
            "Reject a delegate record with an invalid discriminator",
            "The record's first byte is not the ExecutionDelegateRecordV1 key, so \
             the plugin abstains: NoApprovals.",
            scenario_invalid_discriminator,
        ),
        (
            "execute_system_transfer_owner_no_delegate",
            "Owner executes a System transfer via CPI",
            "The asset owner drives a System transfer out of the asset_signer PDA. \
             The program signs the CPI for the PDA via invoke_signed; the transfer \
             completes end to end.",
            scenario_system_transfer_owner,
        ),
        (
            "execute_system_transfer_with_delegate_record_stripped",
            "Delegate executes a System transfer (record stripped)",
            "A delegate drives a System transfer; the delegate record at the head \
             of the remaining accounts is stripped before the CPI so the System \
             program sees only source and destination.",
            scenario_system_transfer_delegate_stripped,
        ),
        (
            "execute_system_transfer_delegate_separate_authority",
            "Delegate (separate authority) executes a System transfer",
            "As above, but the payer and the delegate authority are distinct \
             signers.",
            scenario_system_transfer_delegate_separate,
        ),
        (
            "execute_system_transfer_non_delegate_remaining_account_preserved",
            "Owner CPI passes non-delegate remaining accounts through",
            "With no delegate record, every remaining account flows straight to \
             the CPI: the System transfer's source and destination.",
            scenario_system_transfer_passthrough,
        ),
    ];

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/report");
    std::fs::create_dir_all(dir).unwrap();

    for (test_fn, title, intent, run) in scenarios {
        let mut engine = engine_with_program();
        let tx = run(&mut engine);
        let file = format!("{}.md", test_fn.replace('_', "-"));
        let md = render_scenario(title, intent, "tests/execution_delegate.rs", test_fn, &tx);
        std::fs::write(format!("{dir}/{file}"), md).unwrap();
    }
    println!("wrote {} execution-delegate report pages to {dir}", scenarios.len());
}

// The report scenario bodies mirror the test bodies, returning the Transaction.

fn scenario_execute_as_owner(engine: &mut MolluskBackend) -> Transaction {
    let owner = engine.actor("Owner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    engine.register_alias(&asset_key, "Asset");
    let asset_account = asset_with_execute_checks(
        &owner.pubkey(),
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, owner.pubkey(), None, SPL_NOOP_ID, None, vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.send(&[instruction], &[&owner])
}

fn scenario_valid_delegate(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let executive_profile = Pubkey::new_unique();
    engine.register_alias(&asset_key, "Asset");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let delegate_record_account =
        build_execution_delegate_record(&executive_profile, &delegate_authority.pubkey(), &asset_key);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, delegate_authority.pubkey(), None, SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())), vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&delegate_authority])
}

fn scenario_delegate_separate_authority(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let executive_profile = Pubkey::new_unique();
    engine.register_alias(&asset_key, "Asset");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let delegate_record_account =
        build_execution_delegate_record(&executive_profile, &delegate_authority.pubkey(), &asset_key);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, payer.pubkey(), Some(delegate_authority.pubkey()), SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())), vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&payer, &delegate_authority])
}

fn scenario_non_owner_no_remaining(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let non_owner = engine.actor("NonOwner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    engine.register_alias(&asset_key, "Asset");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, non_owner.pubkey(), None, SPL_NOOP_ID, None, vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.send(&[instruction], &[&non_owner])
}

fn scenario_invalid_pda(engine: &mut MolluskBackend) -> Transaction {
    let owner = engine.actor("Owner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let wrong_signer = Pubkey::new_unique();
    engine.register_alias(&asset_key, "Asset");
    engine.register_alias(&wrong_signer, "WrongSigner");
    let asset_account = asset_with_execute_checks(
        &owner.pubkey(),
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let instruction = execute_v1_instruction(
        asset_key, wrong_signer, owner.pubkey(), None, SPL_NOOP_ID, None, vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&wrong_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.send(&[instruction], &[&owner])
}

fn scenario_no_plugin(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let non_owner = engine.actor("NonOwner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    engine.register_alias(&asset_key, "Asset");
    let asset_account = valid_asset_account(&owner);
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &non_owner.pubkey(), &asset_key);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, non_owner.pubkey(), None, SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())), vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&non_owner])
}

fn scenario_wrong_authority(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let actual_signer = engine.actor("Signer", ACCOUNT_LAMPORTS);
    let different_authority = Pubkey::new_unique();
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    engine.register_alias(&asset_key, "Asset");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &different_authority, &asset_key);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, actual_signer.pubkey(), None, SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())), vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&actual_signer])
}

fn scenario_wrong_asset(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let wrong_asset = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    engine.register_alias(&asset_key, "Asset");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &delegate_authority.pubkey(), &wrong_asset);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, delegate_authority.pubkey(), None, SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())), vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&delegate_authority])
}

fn scenario_wrong_owner(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    engine.register_alias(&asset_key, "Asset");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let mut delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &delegate_authority.pubkey(), &asset_key);
    delegate_record_account.owner = SYSTEM_PROGRAM;
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, delegate_authority.pubkey(), None, SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())), vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&delegate_authority])
}

fn scenario_invalid_discriminator(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    engine.register_alias(&asset_key, "Asset");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let mut delegate_record_account =
        build_execution_delegate_record(&Pubkey::new_unique(), &delegate_authority.pubkey(), &asset_key);
    delegate_record_account.data[0] = 0xFF;
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, delegate_authority.pubkey(), None, SPL_NOOP_ID,
        Some((delegate_record_key, delegate_record_account.clone())), vec![], vec![],
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, payer_like_account());
    engine.set_account(&SPL_NOOP_ID, noop_program_account());
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&delegate_authority])
}

fn scenario_system_transfer_owner(engine: &mut MolluskBackend) -> Transaction {
    let owner = engine.actor("Owner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let dest = Pubkey::new_unique();
    engine.register_alias(&asset_key, "Asset");
    engine.register_alias(&asset_signer, "AssetSigner");
    engine.register_alias(&dest, "Dest");
    let asset_account = asset_with_execute_checks(
        &owner.pubkey(),
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let transfer_ix = solana_system_interface::instruction::transfer(&asset_signer, &dest, 1);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, owner.pubkey(), None, SYSTEM_PROGRAM, None,
        vec![AccountMeta::new(asset_signer, false), AccountMeta::new(dest, false)],
        transfer_ix.data,
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, system_owned_account(ACCOUNT_LAMPORTS));
    engine.set_account(&dest, system_owned_account(0));
    engine.send(&[instruction], &[&owner])
}

fn scenario_system_transfer_delegate_stripped(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let executive_profile = Pubkey::new_unique();
    let dest = Pubkey::new_unique();
    engine.register_alias(&asset_key, "Asset");
    engine.register_alias(&asset_signer, "AssetSigner");
    engine.register_alias(&dest, "Dest");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let delegate_record_account =
        build_execution_delegate_record(&executive_profile, &delegate_authority.pubkey(), &asset_key);
    let transfer_ix = solana_system_interface::instruction::transfer(&asset_signer, &dest, 1);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, delegate_authority.pubkey(), None, SYSTEM_PROGRAM,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![AccountMeta::new(asset_signer, false), AccountMeta::new(dest, false)],
        transfer_ix.data,
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, system_owned_account(ACCOUNT_LAMPORTS));
    engine.set_account(&dest, system_owned_account(0));
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&delegate_authority])
}

fn scenario_system_transfer_delegate_separate(engine: &mut MolluskBackend) -> Transaction {
    let owner = Pubkey::new_unique();
    let delegate_authority = engine.actor("Delegate", ACCOUNT_LAMPORTS);
    let payer = engine.actor("Payer", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let executive_profile = Pubkey::new_unique();
    let dest = Pubkey::new_unique();
    engine.register_alias(&asset_key, "Asset");
    engine.register_alias(&asset_signer, "AssetSigner");
    engine.register_alias(&dest, "Dest");
    let asset_account = asset_with_execute_checks(
        &owner,
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let delegate_record_key = Pubkey::new_unique();
    engine.register_alias(&delegate_record_key, "DelegateRecord");
    let delegate_record_account =
        build_execution_delegate_record(&executive_profile, &delegate_authority.pubkey(), &asset_key);
    let transfer_ix = solana_system_interface::instruction::transfer(&asset_signer, &dest, 1);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, payer.pubkey(), Some(delegate_authority.pubkey()), SYSTEM_PROGRAM,
        Some((delegate_record_key, delegate_record_account.clone())),
        vec![AccountMeta::new(asset_signer, false), AccountMeta::new(dest, false)],
        transfer_ix.data,
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, system_owned_account(ACCOUNT_LAMPORTS));
    engine.set_account(&dest, system_owned_account(0));
    engine.set_account(&delegate_record_key, delegate_record_account);
    engine.send(&[instruction], &[&payer, &delegate_authority])
}

fn scenario_system_transfer_passthrough(engine: &mut MolluskBackend) -> Transaction {
    let owner = engine.actor("Owner", ACCOUNT_LAMPORTS);
    let asset_key = Pubkey::new_unique();
    let (asset_signer, _) = asset_signer_pda(&asset_key);
    let dest = Pubkey::new_unique();
    engine.register_alias(&asset_key, "Asset");
    engine.register_alias(&asset_signer, "AssetSigner");
    engine.register_alias(&dest, "Dest");
    let asset_account = asset_with_execute_checks(
        &owner.pubkey(),
        vec![(HookableLifecycleEvent::Execute, ExternalCheckResult { flags: 0x3 })],
    );
    let transfer_ix = solana_system_interface::instruction::transfer(&asset_signer, &dest, 1);
    let instruction = execute_v1_instruction(
        asset_key, asset_signer, owner.pubkey(), None, SYSTEM_PROGRAM, None,
        vec![AccountMeta::new(asset_signer, false), AccountMeta::new(dest, false)],
        transfer_ix.data,
    );
    engine.set_account(&asset_key, asset_account);
    engine.set_account(&asset_signer, system_owned_account(ACCOUNT_LAMPORTS));
    engine.set_account(&dest, system_owned_account(0));
    engine.send(&[instruction], &[&owner])
}
