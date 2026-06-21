//! Shared scaffolding for running mpl-core's instructions through the TestSVM
//! mollusk adapter: the hand-encoded instruction builders, the AssetV1
//! fabricators, and a per-scenario Markdown report renderer (structured log,
//! plain sequence diagram, authority + ownership graphs).

use {
    mpl_core_program::{
        state::{AssetV1, UpdateAuthority},
        ID as MPL_CORE_ID, SPL_NOOP_ID,
    },
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    testsvm::{model::Transaction, TestSVM},
    testsvm_mollusk::MolluskBackend,
};

/// The project's own built program binary.
pub const MPL_CORE_SO: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../target/deploy/mpl_core_program.so");

pub const ACCOUNT_LAMPORTS: u64 = 1_000_000_000;

/// The System Program id is 32 zero bytes (base58 `1111...`).
pub const SYSTEM_PROGRAM: Pubkey = Pubkey::new_from_array([0u8; 32]);

/// A fresh adapter with the program deployed and the Noop log-wrapper present.
pub fn engine_with_program() -> MolluskBackend {
    let mut engine = MolluskBackend::new();
    engine.deploy_from_file(&MPL_CORE_ID, MPL_CORE_SO, "MplCore");
    engine.register_alias(&SPL_NOOP_ID, "Noop");
    engine.set_account(&SPL_NOOP_ID, Account::default());
    engine
}

/// An `AssetV1` owned by `program_owner` (mpl-core for a valid asset, a foreign
/// program to model a fake), with `owner` as the asset's owner field.
pub fn asset_account(owner: &Pubkey, program_owner: &Pubkey) -> Account {
    let asset = AssetV1::new(
        *owner,
        UpdateAuthority::Address(*owner),
        "Asset".to_string(),
        "https://example.com/asset".to_string(),
    );
    Account {
        lamports: ACCOUNT_LAMPORTS,
        data: borsh::to_vec(&asset).unwrap(),
        owner: *program_owner,
        executable: false,
        rent_epoch: 0,
    }
}

/// CreateV1 (`[0, dataState, name, uri, plugins]`): the asset is a new signer
/// the program allocates via a System CPI. Optional accounts use the program id
/// as the None sentinel.
pub fn create_v1(asset: Pubkey, payer: Pubkey, name: &str, uri: &str) -> Instruction {
    let mut data = vec![0u8, 0u8]; // discriminator, DataState::AccountState
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(&(uri.len() as u32).to_le_bytes());
    data.extend_from_slice(uri.as_bytes());
    data.push(0u8); // plugins: None
    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &data,
        vec![
            AccountMeta::new(asset, true),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
    )
}

/// TransferV1 (`[14, 0]`): hand off the asset to `new_owner`.
pub fn transfer_v1(asset: Pubkey, payer: Pubkey, new_owner: Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &[14u8, 0u8],
        vec![
            AccountMeta::new(asset, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(new_owner, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(SPL_NOOP_ID, false),
        ],
    )
}

/// BurnV1 (`[12, 0]`): close the asset, lamports to the payer.
pub fn burn_v1(asset: Pubkey, payer: Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        MPL_CORE_ID,
        &[12u8, 0u8],
        vec![
            AccountMeta::new(asset, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
    )
}

/// Deserialize an asset's `AssetV1` from the committed account.
pub fn read_asset(engine: &MolluskBackend, asset: &Pubkey) -> Option<AssetV1> {
    borsh::from_slice(&engine.get_account(asset)?.data).ok()
}

// ---------------------------------------------------------------------------
// Report rendering
// ---------------------------------------------------------------------------

fn fenced(lang: &str, body: &str) -> String {
    format!("```{lang}\n{}\n```\n", body.trim_end())
}

/// The 1-based line of `fn <test_fn>(` in the test file, for a `#L<n>` anchor on
/// the source link. Read at render time, so the line matches the file in the
/// commit the report is generated from; regenerate after editing tests.
fn test_fn_line(test_file: &str, test_fn: &str) -> Option<usize> {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), test_file);
    let needle = format!("fn {test_fn}(");
    std::fs::read_to_string(path)
        .ok()?
        .lines()
        .position(|line| line.contains(&needle))
        .map(|i| i + 1)
}

/// One scenario's Markdown page: intent, outcome, a link back to the test that
/// produced it, then the four trace-sourced renders. The sequence diagram is
/// plain (no activation lifelines). `test_file` is repo-relative (e.g.
/// `tests/agent_identity.rs`); `test_fn` is the test's function name.
pub fn render_scenario(
    title: &str,
    intent: &str,
    test_file: &str,
    test_fn: &str,
    tx: &Transaction,
) -> String {
    let outcome = match &tx.error {
        None => "succeeded".to_string(),
        Some(e) => format!("failed: `{e}`"),
    };
    let mut md = String::new();
    md.push_str(&format!("# {title}\n\n"));
    md.push_str(&format!("**Intent.** {intent}\n\n"));
    md.push_str(&format!("**Outcome.** The transaction {outcome}.\n\n"));
    let anchor = test_fn_line(test_file, test_fn)
        .map(|n| format!("#L{n}"))
        .unwrap_or_default();
    md.push_str(&format!(
        "**Source.** [`{test_file}::{test_fn}`](../{test_file}{anchor})\n\n"
    ));
    md.push_str("## Structured execution log\n\n");
    md.push_str(&fenced("", &tx.pretty_cpi_tree()));
    md.push_str("\n## Sequence diagram\n\n");
    md.push_str(tx.mermaid_string().trim_end());
    md.push_str("\n\n## Authority graph\n\n");
    md.push_str("Who signed for what; an `invoke_signed` PDA appears as its own authority.\n\n");
    md.push_str(tx.authority_graph_string().trim_end());
    md.push_str("\n\n## Ownership graph\n\n");
    md.push_str("Which program owns each account the transaction wrote.\n\n");
    md.push_str(tx.ownership_graph_string().trim_end());
    md.push('\n');
    md
}

// ---------------------------------------------------------------------------
// Shared cast and props
// ---------------------------------------------------------------------------

/// The shared cast and props the ported suites stand up.
///
/// # Dramatis personae
///
/// The acting roles (real keypairs the suites mint with `engine.actor`):
///
/// - **Payer / Owner**: the protagonist. The fee payer, and (in the ownership
///   and execution suites) the asset's owner and update authority, the one key
///   that is allowed to act on the asset.
/// - **NewOwner**: the recipient of a transfer.
/// - **Delegate**: the execution / transfer / freeze delegate; a non-owner the
///   plugin machinery may still authorize for a specific lifecycle event.
/// - **NonOwner / Attacker**: unauthorized callers who sign and are expected to
///   be rejected on the owner / authority check.
///
/// The standing cast (programs and well-known ids, present in every scene):
///
/// - **MplCore**: the program under test (`mpl_core_program::ID`).
/// - **System**: the System program ([`SYSTEM_PROGRAM`], 32 zero bytes), the
///   None-sentinel and the allocator behind `CreateV1`.
/// - **Noop**: the SPL Noop log wrapper (`SPL_NOOP_ID`), registered by
///   [`engine_with_program`].
/// - **ForeignProgram**: the forged / foreign program ([`FAKE_PROGRAM_ID`]) an
///   attacker uses to own a look-alike asset.
/// - **MplAgentTools**: the agent-tools program ([`MPL_AGENT_TOOLS_ID`]) that
///   owns `ExecutionDelegateRecordV1` accounts.
/// - **BpfLoader**: BPF loader v2 ([`BPF_LOADER_ID`]), the owner of an
///   executable CPI-target account so `invoke` resolves a program.
pub mod world {
    use {
        super::{asset_account, ACCOUNT_LAMPORTS, SYSTEM_PROGRAM},
        mpl_core_program::{
            plugins::{
                AgentIdentity, ExternalCheckResult, ExternalPluginAdapter,
                ExternalPluginAdapterType, ExternalRegistryRecord, HookableLifecycleEvent, Plugin,
                PluginHeaderV1, PluginRegistryV1, PluginType, RegistryRecord,
            },
            state::{AssetV1, Authority, CollectionV1, DataBlob, Key, UpdateAuthority},
            ID as MPL_CORE_ID,
        },
        solana_account::Account,
        solana_pubkey::Pubkey,
    };

    // -- Standing cast: the well-known program ids the suites name. ----------

    /// A foreign program id standing in for an attacker's program (`[0xAA; 32]`).
    pub const FAKE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0xAA; 32]);

    /// The mpl-agent-tools program id: owner of `ExecutionDelegateRecordV1`.
    pub const MPL_AGENT_TOOLS_ID: Pubkey =
        Pubkey::from_str_const("TLREGni9ZEyGC3vnPZtqUh95xQ8oPqJSvNjvB7FGK8S");

    /// BPF loader v2 id: owner of the (executable) CPI-target account, so the
    /// program's `invoke` resolves a program rather than a data account.
    pub const BPF_LOADER_ID: Pubkey =
        Pubkey::from_str_const("BPFLoader2111111111111111111111111111111111");

    // -- Props: account fabricators. -----------------------------------------

    /// A valid `AssetV1` owned by mpl-core, with `owner` as the asset's owner.
    /// A thin wrapper over [`asset_account`](super::asset_account).
    pub fn valid_asset_account(owner: &Pubkey) -> Account {
        asset_account(owner, &MPL_CORE_ID)
    }

    /// An `AssetV1` owned by `program_owner` (a foreign program to model a
    /// forged asset, mpl-core for a valid one). A thin wrapper over
    /// [`asset_account`](super::asset_account).
    pub fn fake_asset_account(owner: &Pubkey, program_owner: &Pubkey) -> Account {
        asset_account(owner, program_owner)
    }

    /// A `CollectionV1` serialized into an Account owned by `program_owner`.
    pub fn fake_collection_account(update_authority: &Pubkey, program_owner: &Pubkey) -> Account {
        let collection = CollectionV1::new(
            *update_authority,
            "Fake".to_string(),
            "https://example.com".to_string(),
            0,
            0,
        );
        Account {
            lamports: ACCOUNT_LAMPORTS,
            data: borsh::to_vec(&collection).unwrap(),
            owner: *program_owner,
            executable: false,
            rent_epoch: 0,
        }
    }

    /// An empty (zero-data, zero-lamport) system-owned account for a fresh key.
    pub fn empty_asset_account() -> Account {
        Account {
            lamports: 0,
            data: vec![],
            owner: SYSTEM_PROGRAM,
            executable: false,
            rent_epoch: 0,
        }
    }

    /// A funded, system-owned account with no data (a passive non-signer, or a
    /// payer-style account). The `lamports` are caller-chosen.
    pub fn system_owned_account(lamports: u64) -> Account {
        Account {
            lamports,
            data: vec![],
            owner: SYSTEM_PROGRAM,
            executable: false,
            rent_epoch: 0,
        }
    }

    /// An executable, BPF-loader-owned account, the CPI target. The binary need
    /// not be loaded; an unloaded target makes the inner CPI fail (tolerated by
    /// the execution suite).
    pub fn executable_program_account() -> Account {
        Account {
            lamports: ACCOUNT_LAMPORTS,
            data: vec![],
            owner: BPF_LOADER_ID,
            executable: true,
            rent_epoch: 0,
        }
    }

    /// Serializes the plugin pair list into the on-account layout
    /// `[header][plugin data...][registry]` and returns the bytes appended after
    /// the core data, along with the registry offset (the header's
    /// `plugin_registry_offset`).
    fn serialize_plugins(core_len: usize, plugins: &[(Plugin, Authority)]) -> (Vec<u8>, usize) {
        // PluginHeaderV1 is 9 bytes: 1 (Key) + 8 (usize).
        let plugins_start = core_len + 9;

        let mut plugin_data = Vec::new();
        let mut registry_records = Vec::new();
        for (plugin, authority) in plugins {
            let offset = plugins_start + plugin_data.len();
            plugin_data.extend_from_slice(&borsh::to_vec(plugin).unwrap());

            let plugin_type = match plugin {
                Plugin::FreezeDelegate(_) => PluginType::FreezeDelegate,
                Plugin::PermanentFreezeDelegate(_) => PluginType::PermanentFreezeDelegate,
                Plugin::PermanentTransferDelegate(_) => PluginType::PermanentTransferDelegate,
                Plugin::PermanentBurnDelegate(_) => PluginType::PermanentBurnDelegate,
                _ => panic!("Unsupported plugin type in test helper"),
            };
            registry_records.push(RegistryRecord {
                plugin_type,
                authority: *authority,
                offset,
            });
        }

        let registry_offset = plugins_start + plugin_data.len();
        let header = PluginHeaderV1 {
            key: Key::PluginHeaderV1,
            plugin_registry_offset: registry_offset,
        };
        let registry = PluginRegistryV1 {
            key: Key::PluginRegistryV1,
            registry: registry_records,
            external_registry: vec![],
        };

        let mut bytes = borsh::to_vec(&header).unwrap();
        bytes.extend_from_slice(&plugin_data);
        bytes.extend_from_slice(&borsh::to_vec(&registry).unwrap());
        (bytes, registry_offset)
    }

    /// An Account with valid `AssetV1` data plus serialized plugins, owned by
    /// `program_owner`. Layout: `[AssetV1][PluginHeaderV1][plugin data][registry]`.
    pub fn build_asset_with_plugins(
        owner: &Pubkey,
        program_owner: &Pubkey,
        plugins: &[(Plugin, Authority)],
    ) -> Account {
        let asset = AssetV1::new(
            *owner,
            UpdateAuthority::Address(*owner),
            "Fake Asset".to_string(),
            "https://example.com/fake".to_string(),
        );
        let mut data = borsh::to_vec(&asset).unwrap();
        let (plugin_bytes, _) = serialize_plugins(asset.len(), plugins);
        data.extend_from_slice(&plugin_bytes);

        Account {
            lamports: ACCOUNT_LAMPORTS,
            data,
            owner: *program_owner,
            executable: false,
            rent_epoch: 0,
        }
    }

    /// An Account with valid `CollectionV1` data plus serialized plugins, owned
    /// by `program_owner`.
    pub fn build_collection_with_plugins(
        update_authority: &Pubkey,
        program_owner: &Pubkey,
        plugins: &[(Plugin, Authority)],
    ) -> Account {
        let collection = CollectionV1::new(
            *update_authority,
            "Fake Collection".to_string(),
            "https://example.com".to_string(),
            0,
            0,
        );
        let mut data = borsh::to_vec(&collection).unwrap();
        let (plugin_bytes, _) = serialize_plugins(collection.len(), plugins);
        data.extend_from_slice(&plugin_bytes);

        Account {
            lamports: ACCOUNT_LAMPORTS,
            data,
            owner: *program_owner,
            executable: false,
            rent_epoch: 0,
        }
    }

    /// An Account holding an `AssetV1` with an `AgentIdentity` external plugin
    /// already initialized, owned by mpl-core.
    #[allow(deprecated)]
    pub fn build_asset_with_agent_identity(
        owner: &Pubkey,
        uri: &str,
        plugin_authority: Authority,
        lifecycle_checks: Vec<(HookableLifecycleEvent, ExternalCheckResult)>,
    ) -> Account {
        let asset = AssetV1::new(
            *owner,
            UpdateAuthority::Address(*owner),
            "Test Asset".to_string(),
            "https://example.com/test".to_string(),
        );
        let asset_data = borsh::to_vec(&asset).unwrap();

        // Plugin header sits immediately after core data; PluginHeaderV1 is 9
        // bytes (1 Key + 8 usize), then the plugin data, then the registry.
        let plugin_data_start = asset.len() + 9;
        let plugin = ExternalPluginAdapter::AgentIdentity(AgentIdentity {
            uri: uri.to_string(),
        });
        let plugin_bytes = borsh::to_vec(&plugin).unwrap();
        let registry_offset = plugin_data_start + plugin_bytes.len();

        let header = PluginHeaderV1 {
            key: Key::PluginHeaderV1,
            plugin_registry_offset: registry_offset,
        };
        let external_record = ExternalRegistryRecord {
            plugin_type: ExternalPluginAdapterType::AgentIdentity,
            authority: plugin_authority,
            lifecycle_checks: Some(lifecycle_checks),
            offset: plugin_data_start,
            data_offset: None,
            data_len: None,
        };
        let registry = PluginRegistryV1 {
            key: Key::PluginRegistryV1,
            registry: vec![],
            external_registry: vec![external_record],
        };

        let mut data = asset_data;
        data.extend_from_slice(&borsh::to_vec(&header).unwrap());
        data.extend_from_slice(&plugin_bytes);
        data.extend_from_slice(&borsh::to_vec(&registry).unwrap());

        Account {
            lamports: ACCOUNT_LAMPORTS,
            data,
            owner: MPL_CORE_ID,
            executable: false,
            rent_epoch: 0,
        }
    }

    /// A manually constructed 104-byte `ExecutionDelegateRecordV1`, owned by
    /// [`MPL_AGENT_TOOLS_ID`].
    ///
    /// Layout (104 bytes):
    ///   `[0]` = `0x02` (`Key::ExecutionDelegateRecordV1`);
    ///   `[1]` = bump; `[2..8]` = padding; `[8..40]` = executive_profile;
    ///   `[40..72]` = authority; `[72..104]` = agent_asset.
    pub fn build_execution_delegate_record(
        executive_profile: &Pubkey,
        authority: &Pubkey,
        agent_asset: &Pubkey,
    ) -> Account {
        let mut data = vec![0u8; 104];
        data[0] = 0x02; // Key::ExecutionDelegateRecordV1
        data[1] = 255; // bump (arbitrary)
        data[8..40].copy_from_slice(executive_profile.as_ref());
        data[40..72].copy_from_slice(authority.as_ref());
        data[72..104].copy_from_slice(agent_asset.as_ref());

        Account {
            lamports: ACCOUNT_LAMPORTS,
            data,
            owner: MPL_AGENT_TOOLS_ID,
            executable: false,
            rent_epoch: 0,
        }
    }

    /// Derive the asset-signer PDA for the `ExecuteV1` instruction.
    pub fn asset_signer_pda(asset: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"mpl-core-execute", asset.as_ref()], &MPL_CORE_ID)
    }
}

/// The index page: one row per scenario, linking to its page.
pub fn render_index(entries: &[(String, String, String)]) -> String {
    let mut md = String::new();
    md.push_str("# mpl-core through TestSVM: an execution report\n\n");
    md.push_str(
        "Each scenario below runs an mpl-core instruction through the TestSVM mollusk \
         adapter against the project's own `mpl_core_program.so`, and renders what the \
         engine witnessed: the structured execution log, a plain sequence diagram, and the \
         authority and ownership graphs. The project's own suite asserts a boolean \
         `matches!(result, Success)`; this report shows the execution those checks cannot.\n\n",
    );
    md.push_str("| Scenario | Outcome | Report |\n|---|---|---|\n");
    for (file, title, outcome) in entries {
        md.push_str(&format!("| {title} | {outcome} | [page]({file}) |\n"));
    }
    md
}
