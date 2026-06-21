# Transfer rejects garbage data

**Intent.** An mpl-core-owned account is filled with 0xFF; transfer fails at deserialization.

**Outcome.** The transaction failed: `custom program error: 0x1`.

**Source.** [`tests/account_ownership.rs::transfer_rejects_random_data_account`](../tests/account_ownership.rs#L262)

## Structured execution log

```
CPI Tree (3,332 BPF CU / 1,400,000 budget):
└── Transfer FAILED: custom program error: 0x1 (3,332 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x1
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: Transfer (3332cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ custom program error: 0x1
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
