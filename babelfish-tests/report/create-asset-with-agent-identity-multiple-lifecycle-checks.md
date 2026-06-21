# Create an asset with multiple lifecycle checks

**Intent.** Allocate an asset whose AgentIdentity plugin registers a wider lifecycle check (CAN_LISTEN | CAN_APPROVE) at creation time.

**Outcome.** The transaction succeeded.

**Source.** [`tests/agent_identity.rs::create_asset_with_agent_identity_multiple_lifecycle_checks`](../tests/agent_identity.rs#L426)

## Structured execution log

```
CPI Tree (17,788 BPF CU / 1,400,000 budget):
└── CreateV2 (17,788 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/state/asset.rs:155:Approve
    ├── CreateAccount System
    ├── Transfer System
    └── Transfer System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Asset
    participant MplCore
    participant System
    Asset ->> MplCore: CreateV2 (17788cu)
    MplCore ->> System: CreateAccount
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
    Asset([Asset]):::signer
    Payer([Payer]):::signer
    AgentIdentityPda([AgentIdentityPda]):::signer
    System[System]:::program
    Asset -->|signs| MplCore
    Payer -->|signs| MplCore
    AgentIdentityPda -->|signs| MplCore
    Payer -->|signs| System
    Asset -->|signs| System
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
