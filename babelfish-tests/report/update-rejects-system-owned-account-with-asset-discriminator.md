# Update rejects a system-owned account with an asset discriminator

**Intent.** A System-owned account carries valid AssetV1 bytes; update rejects it on the owner check.

**Outcome.** The transaction failed: `Invalid account owner`.

**Source.** [`tests/account_ownership.rs::update_rejects_system_owned_account_with_asset_discriminator`](../tests/account_ownership.rs#L354)

## Structured execution log

```
CPI Tree (2,839 BPF CU / 1,400,000 budget):
└── Update FAILED: Invalid account owner (2,839 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Invalid account owner
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: Update (2839cu)
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
    System[System]:::owner
    Asset[(Asset)]:::account
    Payer[(Payer)]:::account
    System -->|owns| Asset
    System -->|owns| Payer
```
