# Execute as owner

**Intent.** The asset owner invokes ExecuteV1. The AgentIdentity plugin approves on owner authority; the CPI to the (unloaded) Noop program then fails, so the run reaches the CPI frame but does not complete it.

**Outcome.** The transaction failed: `Unsupported program id`.

**Source.** [`tests/execution_delegate.rs::execute_as_owner`](../tests/execution_delegate.rs#L146)

## Structured execution log

```
CPI Tree (12,235 BPF CU / 1,400,000 budget):
└── Execute FAILED: Unsupported program id (12,235 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/state/asset.rs:335:Approve
    ├── Transfer System
    └── FAILED: Unsupported program id Noop
          >> log:  Program is not cached
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Owner
    participant MplCore
    participant System
    participant Noop
    Owner ->> MplCore: Execute (12235cu)
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
    Owner([Owner]):::signer
    System[System]:::program
    Noop[Noop]:::program
    Owner -->|signs| MplCore
    Owner -->|signs| System
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
    Owner[(Owner)]:::account
    MplCore -->|owns| Asset
    System -->|owns| Owner
```
