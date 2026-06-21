# Transfer rejects a valid frozen asset

**Intent.** A valid mpl-core-owned asset with FreezeDelegate{frozen:true}; the freeze plugin rejects the transfer.

**Outcome.** The transaction failed: `custom program error: 0x9`.

**Source.** [`tests/account_ownership.rs::transfer_rejects_valid_frozen_asset`](../tests/account_ownership.rs#L735)

## Structured execution log

```
CPI Tree (5,902 BPF CU / 1,400,000 budget):
└── Transfer FAILED: custom program error: 0x9 (5,902 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  programs/mpl-core/src/state/asset.rs:293:Approve
      >> log:  programs/mpl-core/src/plugins/internal/owner_managed/freeze_delegate.rs:59:Reject
      >> log:  programs/mpl-core/src/plugins/lifecycle.rs:739:Reject
      >> log:  Error: Custom program error: 0x9
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: Transfer (5902cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ custom program error: 0x9
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
    Payer -->|signs| MplCore
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
