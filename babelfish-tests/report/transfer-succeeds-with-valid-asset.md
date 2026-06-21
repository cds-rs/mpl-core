# Transfer succeeds on a valid asset

**Intent.** A valid mpl-core-owned asset is handed to a new owner by its owner; the transfer succeeds.

**Outcome.** The transaction succeeded.

**Source.** [`tests/account_ownership.rs::transfer_succeeds_with_valid_asset`](../tests/account_ownership.rs#L316)

## Structured execution log

```
CPI Tree (4,445 BPF CU / 1,400,000 budget):
└── Transfer (4,445 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  programs/mpl-core/src/state/asset.rs:293:Approve
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: Transfer (4445cu)
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
