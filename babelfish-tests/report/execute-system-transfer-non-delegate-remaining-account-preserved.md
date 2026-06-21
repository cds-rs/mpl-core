# Owner CPI passes non-delegate remaining accounts through

**Intent.** With no delegate record, every remaining account flows straight to the CPI: the System transfer's source and destination.

**Outcome.** The transaction succeeded.

**Source.** [`tests/execution_delegate.rs::execute_system_transfer_non_delegate_remaining_account_preserved`](../tests/execution_delegate.rs#L742)

## Structured execution log

```
CPI Tree (11,841 BPF CU / 1,400,000 budget):
└── Execute (11,841 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/state/asset.rs:335:Approve
    ├── Transfer System
    └── Transfer System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Owner
    participant MplCore
    participant System
    Owner ->> MplCore: Execute (11841cu)
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
    Owner([Owner]):::signer
    Dest[(Dest)]:::writable
    System[System]:::program
    Owner -->|signs| MplCore
    Owner -->|signs| System
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
    Owner[(Owner)]:::account
    Dest[(Dest)]:::account
    MplCore -->|owns| Asset
    System -->|owns| AssetSigner
    System -->|owns| Owner
    System -->|owns| Dest
```
