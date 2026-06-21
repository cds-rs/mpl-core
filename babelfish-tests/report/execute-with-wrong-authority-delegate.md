# Reject a delegate record with the wrong authority

**Intent.** The record's authority does not match the signer, so the plugin abstains: NoApprovals.

**Outcome.** The transaction failed: `custom program error: 0x1a`.

**Source.** [`tests/execution_delegate.rs::execute_with_wrong_authority_delegate`](../tests/execution_delegate.rs#L398)

## Structured execution log

```
CPI Tree (8,208 BPF CU / 1,400,000 budget):
└── Execute FAILED: custom program error: 0x1a (8,208 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x1a
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Signer
    participant MplCore
    Signer ->> MplCore: Execute (8208cu)
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
    Signer([Signer]):::signer
    Signer -->|signs| MplCore
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
    Signer[(Signer)]:::account
    MplCore -->|owns| Asset
    System -->|owns| Signer
```
