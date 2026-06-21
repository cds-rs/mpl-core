# Delegate (separate authority) executes a System transfer

**Intent.** As above, but the payer and the delegate authority are distinct signers.

**Outcome.** The transaction succeeded.

**Source.** [`tests/execution_delegate.rs::execute_system_transfer_delegate_separate_authority`](../tests/execution_delegate.rs#L686)

## Structured execution log

```
CPI Tree (12,761 BPF CU / 1,400,000 budget):
└── Execute (12,761 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/plugins/external/agent_identity.rs:67:Approve
    │ >> log:  programs/mpl-core/src/plugins/lifecycle.rs:822:Approve
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
    Payer ->> MplCore: Execute (12761cu)
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
    AssetSigner([AssetSigner]):::signer
    Payer([Payer]):::signer
    Delegate([Delegate]):::signer
    Dest[(Dest)]:::writable
    System[System]:::program
    Payer -->|signs| MplCore
    Delegate -->|signs| MplCore
    Payer -->|signs| System
    AssetSigner -->|signs| System
    MplCore -->|writes| Asset
    MplCore -->|writes| AssetSigner
    MplCore -->|writes| Dest
    System -->|writes| Asset
    System -->|writes| Dest
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
    AssetSigner[(AssetSigner)]:::account
    Payer[(Payer)]:::account
    Dest[(Dest)]:::account
    MplCore -->|owns| Asset
    System -->|owns| AssetSigner
    System -->|owns| Payer
    System -->|owns| Dest
```
