# Reject a duplicate AgentIdentity plugin

**Intent.** Attempt to add a second AgentIdentity plugin to an asset that already carries one; the program rejects the duplicate.

**Outcome.** The transaction failed: `custom program error: 0x20`.

**Source.** [`tests/agent_identity.rs::cannot_add_duplicate_agent_identity`](../tests/agent_identity.rs#L629)

## Structured execution log

```
CPI Tree (13,613 BPF CU / 1,400,000 budget):
└── AddExternalPluginAdapter FAILED: custom program error: 0x20 (13,613 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  programs/mpl-core/src/state/asset.rs:350:Approve
      >> log:  Error: Custom program error: 0x20
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: AddExternalPluginAdapter (13613cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ custom program error: 0x20
    end
```

## Authority graph

Who signed for what; an `invoke_signed` PDA appears as its own authority.

```mermaid
flowchart LR
    classDef signer fill:#d4edda,stroke:#28a745;
    classDef program fill:#cce5ff,stroke:#007bff;
    classDef writable fill:#fff3cd,stroke:#ffc107;
    MplCore[MplCore]:::program
    Asset[(Asset)]:::writable
    Payer([Payer]):::signer
    AgentIdentityPda([AgentIdentityPda]):::signer
    Payer -->|signs| MplCore
    AgentIdentityPda -->|signs| MplCore
    MplCore -->|writes| Asset
```

## Ownership graph

Which program owns each account the transaction wrote.

```mermaid
flowchart LR
    classDef owner fill:#cce5ff,stroke:#007bff;
    classDef account fill:#fff3cd,stroke:#ffc107;
    MplCore[MplCore]:::owner
    Asset[(Asset)]:::account
    System[System]:::owner
    Payer[(Payer)]:::account
    MplCore -->|owns| Asset
    System -->|owns| Payer
```
