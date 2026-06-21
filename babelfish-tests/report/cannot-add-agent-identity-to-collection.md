# Reject adding an AgentIdentity plugin to a collection

**Intent.** Attempt to add an AgentIdentity plugin to an existing collection. The plugin targets assets only; the add is rejected.

**Outcome.** The transaction failed: `custom program error: 0x2e`.

**Source.** [`tests/agent_identity.rs::cannot_add_agent_identity_to_collection`](../tests/agent_identity.rs#L500)

## Structured execution log

```
CPI Tree (3,268 BPF CU / 1,400,000 budget):
└── AddCollectionExternalPluginAdapter FAILED: custom program error: 0x2e (3,268 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x2e
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: AddCollectionExternalPluginAdapter (3268cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ custom program error: 0x2e
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
    MplCore[MplCore]:::owner
    Collection[(Collection)]:::account
    System[System]:::owner
    Payer[(Payer)]:::account
    MplCore -->|owns| Collection
    System -->|owns| Payer
```
