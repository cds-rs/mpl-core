# Create an asset

**Intent.** Allocate a fresh asset; the program creates the account through a System CPI.

**Outcome.** The transaction succeeded.

**Source.** [`tests/report.rs::create`](../tests/report.rs#L16)

## Structured execution log

```
CPI Tree (8,232 BPF CU / 1,400,000 budget):
└── Create (8,232 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/state/asset.rs:155:Approve
    └── CreateAccount System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Asset
    participant MplCore
    participant System
    Asset ->> MplCore: Create (8232cu)
    MplCore ->> System: CreateAccount
```

## Authority graph

Who signed for what; an `invoke_signed` PDA appears as its own authority.

```mermaid
flowchart LR
    classDef signer fill:#d4edda,stroke:#28a745;
    classDef program fill:#cce5ff,stroke:#007bff;
    classDef writable fill:#fff3cd,stroke:#ffc107;
    MplCore[MplCore]:::program
    Asset([Asset]):::signer
    Payer([Payer]):::signer
    System[System]:::program
    Asset -->|signs| MplCore
    Payer -->|signs| MplCore
    Payer -->|signs| System
    Asset -->|signs| System
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
