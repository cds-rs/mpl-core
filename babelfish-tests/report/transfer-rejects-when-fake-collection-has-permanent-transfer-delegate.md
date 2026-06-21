# Transfer rejects when a referenced collection is foreign-owned (permanent transfer)

**Intent.** A valid asset references a foreign-owned collection carrying a PermanentTransferDelegate; the collection's owner check rejects the transfer.

**Outcome.** The transaction failed: `Invalid account owner`.

**Source.** [`tests/account_ownership.rs::transfer_rejects_when_fake_collection_has_permanent_transfer_delegate`](../tests/account_ownership.rs#L683)

## Structured execution log

```
CPI Tree (4,178 BPF CU / 1,400,000 budget):
└── Transfer FAILED: Invalid account owner (4,178 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Invalid account owner
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: Transfer (4178cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ Invalid account owner
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
