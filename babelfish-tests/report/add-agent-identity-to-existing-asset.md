# Add an AgentIdentity plugin to an existing asset

**Intent.** Attach an AgentIdentity external plugin to an already-initialized asset. The PDA signs as a remaining account.

**Outcome.** The transaction succeeded.

**Source.** [`tests/agent_identity.rs::add_agent_identity_to_existing_asset`](../tests/agent_identity.rs#L281)

## Structured execution log

```
CPI Tree (19,163 BPF CU / 1,400,000 budget):
└── AddExternalPluginAdapter (19,163 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/state/asset.rs:350:Approve
    ├── Transfer System
    └── Transfer System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    participant System
    Payer ->> MplCore: AddExternalPluginAdapter (19163cu)
    MplCore ->> System: Transfer
    MplCore ->> System: Transfer
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
    System[System]:::program
    Payer -->|signs| MplCore
    AgentIdentityPda -->|signs| MplCore
    Payer -->|signs| System
    MplCore -->|writes| Asset
    System -->|writes| Asset
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
