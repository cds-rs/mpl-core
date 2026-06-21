# Update-collection rejects a foreign-owned fake collection

**Intent.** A foreign-owned account with valid CollectionV1 bytes is presented to update-collection; it is rejected.

**Outcome.** The transaction failed: `Invalid account owner`.

**Source.** [`tests/account_ownership.rs::update_collection_rejects_fake_collection_owned_by_different_program`](../tests/account_ownership.rs#L186)

## Structured execution log

```
CPI Tree (2,705 BPF CU / 1,400,000 budget):
└── UpdateCollection FAILED: Invalid account owner (2,705 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Invalid account owner
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: UpdateCollection (2705cu)
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
    Collection[(Collection)]:::writable
    Payer([Payer]):::signer
    Payer -->|signs| MplCore
    MplCore -->|writes| Collection
```

## Ownership graph

Which program owns each account the transaction wrote.

```mermaid
flowchart LR
    classDef owner fill:#cce5ff,stroke:#007bff;
    classDef account fill:#fff3cd,stroke:#ffc107;
    ForeignProgram[ForeignProgram]:::owner
    Collection[(Collection)]:::account
    System[System]:::owner
    Payer[(Payer)]:::account
    ForeignProgram -->|owns| Collection
    System -->|owns| Payer
```
