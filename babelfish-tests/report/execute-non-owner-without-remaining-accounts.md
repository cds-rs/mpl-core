# Reject a non-owner with no delegate record

**Intent.** A non-owner authority with no remaining accounts: the plugin abstains and the transaction fails with NoApprovals.

**Outcome.** The transaction failed: `custom program error: 0x1a`.

**Source.** [`tests/execution_delegate.rs::execute_non_owner_without_remaining_accounts`](../tests/execution_delegate.rs#L283)

## Structured execution log

```
CPI Tree (7,670 BPF CU / 1,400,000 budget):
└── Execute FAILED: custom program error: 0x1a (7,670 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x1a
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant NonOwner
    participant MplCore
    NonOwner ->> MplCore: Execute (7670cu)
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
    NonOwner([NonOwner]):::signer
    NonOwner -->|signs| MplCore
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
    NonOwner[(NonOwner)]:::account
    MplCore -->|owns| Asset
    System -->|owns| NonOwner
```
