# Reject a wrong asset_signer PDA

**Intent.** The asset_signer account is not the program-derived address, so the program rejects with InvalidExecutePda.

**Outcome.** The transaction failed: `custom program error: 0x31`.

**Source.** [`tests/execution_delegate.rs::execute_with_invalid_asset_signer_pda`](../tests/execution_delegate.rs#L322)

## Structured execution log

```
CPI Tree (4,818 BPF CU / 1,400,000 budget):
└── Execute FAILED: custom program error: 0x31 (4,818 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x31
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Owner
    participant MplCore
    Owner ->> MplCore: Execute (4818cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ custom program error: 0x31
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
    Owner([Owner]):::signer
    Owner -->|signs| MplCore
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
    Owner[(Owner)]:::account
    MplCore -->|owns| Asset
    System -->|owns| Owner
```
