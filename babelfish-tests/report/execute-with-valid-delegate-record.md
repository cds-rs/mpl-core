# Execute via a valid delegate record

**Intent.** A non-owner authority presents an ExecutionDelegateRecordV1 whose authority and agent_asset match. The plugin approves; the Noop CPI fails as above.

**Outcome.** The transaction failed: `Unsupported program id`.

**Source.** [`tests/execution_delegate.rs::execute_with_valid_delegate_record`](../tests/execution_delegate.rs#L188)

## Structured execution log

```
CPI Tree (11,419 BPF CU / 1,400,000 budget):
└── Execute FAILED: Unsupported program id (11,419 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/plugins/external/agent_identity.rs:67:Approve
    │ >> log:  programs/mpl-core/src/plugins/lifecycle.rs:822:Approve
    ├── Transfer System
    └── FAILED: Unsupported program id Noop
          >> log:  Program is not cached
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Delegate
    participant MplCore
    participant System
    participant Noop
    Delegate ->> MplCore: Execute (11419cu)
    MplCore ->> System: Transfer
    MplCore ->> Noop: unnamed
    rect rgb(255, 220, 220)
    note over Noop: ✗ Unsupported program id
    end
    rect rgb(255, 220, 220)
    note over MplCore: ✗ Unsupported program id
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
    Delegate([Delegate]):::signer
    System[System]:::program
    Noop[Noop]:::program
    Delegate -->|signs| MplCore
    Delegate -->|signs| System
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
    Delegate[(Delegate)]:::account
    MplCore -->|owns| Asset
    System -->|owns| Delegate
```
