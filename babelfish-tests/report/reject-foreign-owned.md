# Reject a foreign-owned asset

**Intent.** Transfer an account that carries valid AssetV1 bytes but is owned by another program.

**Outcome.** The transaction failed: `Invalid account owner`.

**Source.** [`tests/report.rs::reject_foreign_owned`](../tests/report.rs#L40)

## Structured execution log

```
CPI Tree (3,126 BPF CU / 1,400,000 budget):
└── Transfer FAILED: Invalid account owner (3,126 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Invalid account owner
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: Transfer (3126cu)
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
    ForeignProgram[ForeignProgram]:::owner
    Asset[(Asset)]:::account
    System[System]:::owner
    Payer[(Payer)]:::account
    ForeignProgram -->|owns| Asset
    System -->|owns| Payer
```
