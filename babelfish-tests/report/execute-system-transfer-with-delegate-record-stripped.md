# Delegate executes a System transfer (record stripped)

**Intent.** A delegate drives a System transfer; the delegate record at the head of the remaining accounts is stripped before the CPI so the System program sees only source and destination.

**Outcome.** The transaction succeeded.

**Source.** [`tests/execution_delegate.rs::execute_system_transfer_with_delegate_record_stripped`](../tests/execution_delegate.rs#L632)

## Structured execution log

```
CPI Tree (18,544 BPF CU / 1,400,000 budget):
└── Execute (18,544 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/plugins/external/agent_identity.rs:67:Approve
    │ >> log:  programs/mpl-core/src/plugins/lifecycle.rs:822:Approve
    ├── Transfer System
    └── Transfer System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Delegate
    participant MplCore
    participant System
    Delegate ->> MplCore: Execute (18544cu)
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
    Delegate([Delegate]):::signer
    Dest[(Dest)]:::writable
    System[System]:::program
    Delegate -->|signs| MplCore
    Delegate -->|signs| System
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
    Delegate[(Delegate)]:::account
    Dest[(Dest)]:::account
    MplCore -->|owns| Asset
    System -->|owns| AssetSigner
    System -->|owns| Delegate
    System -->|owns| Dest
```
