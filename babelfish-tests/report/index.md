# mpl-core through TestSVM: full execution report

Every test in mpl-core's mollusk suite, authored through the TestSVM mollusk adapter against the project's own `mpl_core_program.so`. Each page carries the structured execution log, a plain sequence diagram, and the authority + ownership graphs the project's boolean `matches!(result, Success)` cannot show, and links back to the test that produced it.

**57 scenarios.**

## Core (create / transfer / burn)

| Scenario | Outcome | Page |
|---|---|---|
| Burn an asset | succeeded | [burn.md](burn.md) |
| Create an asset | succeeded | [create.md](create.md) |
| Reject a foreign-owned asset | failed | [reject-foreign-owned.md](reject-foreign-owned.md) |
| Transfer an asset | succeeded | [transfer.md](transfer.md) |

## Account ownership

| Scenario | Outcome | Page |
|---|---|---|
| Burn rejects the wrong discriminator | failed | [burn-rejects-account-with-wrong-discriminator.md](burn-rejects-account-with-wrong-discriminator.md) |
| Burn rejects a foreign-owned fake asset | failed | [burn-rejects-fake-asset-owned-by-different-program.md](burn-rejects-fake-asset-owned-by-different-program.md) |
| Burn rejects a foreign-owned unfrozen asset that also carries a permanent burn delegate | failed | [burn-rejects-fake-asset-unfrozen-with-permanent-burn.md](burn-rejects-fake-asset-unfrozen-with-permanent-burn.md) |
| Burn rejects a foreign-owned asset with a permanent burn delegate | failed | [burn-rejects-fake-asset-with-permanent-burn-delegate.md](burn-rejects-fake-asset-with-permanent-burn-delegate.md) |
| Burn rejects a foreign-owned frozen fake asset | failed | [burn-rejects-fake-frozen-asset-owned-by-different-program.md](burn-rejects-fake-frozen-asset-owned-by-different-program.md) |
| Burn rejects an unauthorized caller | failed | [burn-rejects-unauthorized-caller-on-valid-asset.md](burn-rejects-unauthorized-caller-on-valid-asset.md) |
| Transfer rejects the wrong discriminator | failed | [transfer-rejects-account-with-wrong-discriminator.md](transfer-rejects-account-with-wrong-discriminator.md) |
| Transfer rejects an empty account | failed | [transfer-rejects-empty-account.md](transfer-rejects-empty-account.md) |
| Transfer rejects a foreign-owned frozen asset that also carries a permanent transfer delegate | failed | [transfer-rejects-fake-asset-frozen-but-with-permanent-transfer.md](transfer-rejects-fake-asset-frozen-but-with-permanent-transfer.md) |
| Transfer rejects a foreign-owned fake asset | failed | [transfer-rejects-fake-asset-owned-by-different-program.md](transfer-rejects-fake-asset-owned-by-different-program.md) |
| Transfer rejects a foreign-owned asset with a frozen permanent-freeze delegate | failed | [transfer-rejects-fake-asset-with-permanent-freeze-frozen.md](transfer-rejects-fake-asset-with-permanent-freeze-frozen.md) |
| Transfer rejects a foreign-owned asset with a permanent transfer delegate | failed | [transfer-rejects-fake-asset-with-permanent-transfer-delegate.md](transfer-rejects-fake-asset-with-permanent-transfer-delegate.md) |
| Transfer rejects a foreign-owned frozen fake asset | failed | [transfer-rejects-fake-frozen-asset-owned-by-different-program.md](transfer-rejects-fake-frozen-asset-owned-by-different-program.md) |
| Transfer rejects a foreign-owned unfrozen fake asset | failed | [transfer-rejects-fake-unfrozen-asset-owned-by-different-program.md](transfer-rejects-fake-unfrozen-asset-owned-by-different-program.md) |
| Transfer rejects garbage data | failed | [transfer-rejects-random-data-account.md](transfer-rejects-random-data-account.md) |
| Transfer rejects a system-owned account with an asset discriminator | failed | [transfer-rejects-system-owned-account-with-asset-discriminator.md](transfer-rejects-system-owned-account-with-asset-discriminator.md) |
| Transfer rejects an unauthorized caller | failed | [transfer-rejects-unauthorized-caller-on-valid-asset.md](transfer-rejects-unauthorized-caller-on-valid-asset.md) |
| Transfer rejects a valid frozen asset | failed | [transfer-rejects-valid-frozen-asset.md](transfer-rejects-valid-frozen-asset.md) |
| Transfer rejects when a referenced collection is foreign-owned (permanent freeze) | failed | [transfer-rejects-when-fake-collection-has-permanent-freeze.md](transfer-rejects-when-fake-collection-has-permanent-freeze.md) |
| Transfer rejects when a referenced collection is foreign-owned (permanent transfer) | failed | [transfer-rejects-when-fake-collection-has-permanent-transfer-delegate.md](transfer-rejects-when-fake-collection-has-permanent-transfer-delegate.md) |
| Transfer succeeds on a valid unfrozen asset with a freeze plugin | succeeded | [transfer-succeeds-valid-unfrozen-asset-with-freeze-plugin.md](transfer-succeeds-valid-unfrozen-asset-with-freeze-plugin.md) |
| Transfer succeeds on a valid asset | succeeded | [transfer-succeeds-with-valid-asset.md](transfer-succeeds-with-valid-asset.md) |
| Update-collection rejects a foreign-owned fake collection | failed | [update-collection-rejects-fake-collection-owned-by-different-program.md](update-collection-rejects-fake-collection-owned-by-different-program.md) |
| Update rejects a foreign-owned fake asset | failed | [update-rejects-fake-asset-owned-by-different-program.md](update-rejects-fake-asset-owned-by-different-program.md) |
| Update rejects a system-owned account with an asset discriminator | failed | [update-rejects-system-owned-account-with-asset-discriminator.md](update-rejects-system-owned-account-with-asset-discriminator.md) |

## Agent identity

| Scenario | Outcome | Page |
|---|---|---|
| Add an AgentIdentity plugin to an existing asset | succeeded | [add-agent-identity-to-existing-asset.md](add-agent-identity-to-existing-asset.md) |
| Reject adding an AgentIdentity plugin to a collection | failed | [cannot-add-agent-identity-to-collection.md](cannot-add-agent-identity-to-collection.md) |
| Reject an unsigned AgentIdentity PDA | failed | [cannot-add-agent-identity-with-unsigned-pda.md](cannot-add-agent-identity-with-unsigned-pda.md) |
| Reject a mismatched AgentIdentity PDA | failed | [cannot-add-agent-identity-with-wrong-pda.md](cannot-add-agent-identity-with-wrong-pda.md) |
| Reject adding an AgentIdentity plugin without the PDA | failed | [cannot-add-agent-identity-without-pda-remaining-account.md](cannot-add-agent-identity-without-pda-remaining-account.md) |
| Reject a duplicate AgentIdentity plugin | failed | [cannot-add-duplicate-agent-identity.md](cannot-add-duplicate-agent-identity.md) |
| Reject an AgentIdentity plugin on a created collection | failed | [cannot-create-collection-with-agent-identity.md](cannot-create-collection-with-agent-identity.md) |
| Create an asset with an address plugin authority | succeeded | [create-asset-with-agent-identity-address-authority.md](create-asset-with-agent-identity-address-authority.md) |
| Create an asset with multiple lifecycle checks | succeeded | [create-asset-with-agent-identity-multiple-lifecycle-checks.md](create-asset-with-agent-identity-multiple-lifecycle-checks.md) |
| Create an asset with an AgentIdentity plugin | succeeded | [create-asset-with-agent-identity.md](create-asset-with-agent-identity.md) |
| Remove an AgentIdentity plugin | succeeded | [remove-agent-identity.md](remove-agent-identity.md) |
| Update an AgentIdentity plugin's lifecycle checks | succeeded | [update-agent-identity-lifecycle-checks.md](update-agent-identity-lifecycle-checks.md) |
| Update an AgentIdentity plugin's URI and lifecycle checks | succeeded | [update-agent-identity-uri-and-lifecycle-checks.md](update-agent-identity-uri-and-lifecycle-checks.md) |
| Update an AgentIdentity plugin's URI | succeeded | [update-agent-identity-uri.md](update-agent-identity-uri.md) |

## Execution delegate

| Scenario | Outcome | Page |
|---|---|---|
| Execute as owner | failed | [execute-as-owner.md](execute-as-owner.md) |
| Reject a non-owner on an asset with no AgentIdentity plugin | failed | [execute-non-owner-without-agent-identity-plugin.md](execute-non-owner-without-agent-identity-plugin.md) |
| Reject a non-owner with no delegate record | failed | [execute-non-owner-without-remaining-accounts.md](execute-non-owner-without-remaining-accounts.md) |
| Delegate (separate authority) executes a System transfer | succeeded | [execute-system-transfer-delegate-separate-authority.md](execute-system-transfer-delegate-separate-authority.md) |
| Owner CPI passes non-delegate remaining accounts through | succeeded | [execute-system-transfer-non-delegate-remaining-account-preserved.md](execute-system-transfer-non-delegate-remaining-account-preserved.md) |
| Owner executes a System transfer via CPI | succeeded | [execute-system-transfer-owner-no-delegate.md](execute-system-transfer-owner-no-delegate.md) |
| Delegate executes a System transfer (record stripped) | succeeded | [execute-system-transfer-with-delegate-record-stripped.md](execute-system-transfer-with-delegate-record-stripped.md) |
| Execute via a delegate as a separate authority | failed | [execute-with-delegate-as-separate-authority.md](execute-with-delegate-as-separate-authority.md) |
| Reject a wrong asset_signer PDA | failed | [execute-with-invalid-asset-signer-pda.md](execute-with-invalid-asset-signer-pda.md) |
| Reject a delegate record with an invalid discriminator | failed | [execute-with-invalid-discriminator-delegate.md](execute-with-invalid-discriminator-delegate.md) |
| Execute via a valid delegate record | failed | [execute-with-valid-delegate-record.md](execute-with-valid-delegate-record.md) |
| Reject a delegate record bound to a different asset | failed | [execute-with-wrong-asset-delegate.md](execute-with-wrong-asset-delegate.md) |
| Reject a delegate record with the wrong authority | failed | [execute-with-wrong-authority-delegate.md](execute-with-wrong-authority-delegate.md) |
| Reject a delegate record owned by the wrong program | failed | [execute-with-wrong-program-owner-delegate.md](execute-with-wrong-program-owner-delegate.md) |
