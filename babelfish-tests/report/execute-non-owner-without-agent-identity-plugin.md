# Reject a non-owner on an asset with no AgentIdentity plugin

**Intent.** The asset carries no AgentIdentity plugin, so there is nothing to approve a non-owner: NoApprovals.

**Outcome.** The transaction failed: `custom program error: 0x1a`.

**Source.** [`tests/execution_delegate.rs::execute_non_owner_without_agent_identity_plugin`](../tests/execution_delegate.rs#L360)

## Structured execution log

```
CPI Tree (6,536 BPF CU / 1,400,000 budget):
└── Execute FAILED: custom program error: 0x1a (6,536 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x1a
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant NonOwner
    participant MplCore
    NonOwner ->> MplCore: Execute (6536cu)
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
