# Transfer rejects an unauthorized caller

**Intent.** A valid asset is owned by someone else; an attacker signs the transfer and is rejected on the authority check.

**Outcome.** The transaction failed: `custom program error: 0x1a`.

**Source.** [`tests/account_ownership.rs::transfer_rejects_unauthorized_caller_on_valid_asset`](../tests/account_ownership.rs#L369)

## Structured execution log

```
CPI Tree (4,687 BPF CU / 1,400,000 budget):
└── Transfer FAILED: custom program error: 0x1a (4,687 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x1a
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Attacker
    participant MplCore
    Attacker ->> MplCore: Transfer (4687cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ custom program error: 0x1a
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
    Attacker([Attacker]):::signer
    Attacker -->|signs| MplCore
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
    Attacker[(Attacker)]:::account
    MplCore -->|owns| Asset
    System -->|owns| Attacker
```
