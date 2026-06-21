# Transfer rejects the wrong discriminator

**Intent.** An mpl-core-owned account carries CollectionV1 (disc 5) bytes where AssetV1 (disc 1) is expected; transfer rejects it.

**Outcome.** The transaction failed: `custom program error: 0x6`.

**Source.** [`tests/account_ownership.rs::transfer_rejects_account_with_wrong_discriminator`](../tests/account_ownership.rs#L204)

## Structured execution log

```
CPI Tree (3,336 BPF CU / 1,400,000 budget):
└── Transfer FAILED: custom program error: 0x6 (3,336 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x6
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: Transfer (3336cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ custom program error: 0x6
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
