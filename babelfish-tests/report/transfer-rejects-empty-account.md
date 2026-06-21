# Transfer rejects an empty account

**Intent.** An mpl-core-owned account with no data is presented to transfer; it fails immediately.

**Outcome.** The transaction failed: `Program failed to complete`.

**Source.** [`tests/account_ownership.rs::transfer_rejects_empty_account`](../tests/account_ownership.rs#L289)

## Structured execution log

```
CPI Tree (2,339 BPF CU / 1,400,000 budget):
└── Transfer FAILED: SBF program Panicked in programs/mpl-core/src/utils/mod.rs at 31:22 (2,339 / 1,400,000 CU) MplCore (no CPIs)
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: Transfer (2339cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ SBF program Panicked in programs/mpl-core/src/utils/mod.rs at 31:22
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
