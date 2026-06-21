# Reject an AgentIdentity plugin on a created collection

**Intent.** Attempt to create a collection with an AgentIdentity plugin. AgentIdentity targets assets only; the program rejects the collection target.

**Outcome.** The transaction failed: `custom program error: 0x2e`.

**Source.** [`tests/agent_identity.rs::cannot_create_collection_with_agent_identity`](../tests/agent_identity.rs#L481)

## Structured execution log

```
CPI Tree (11,023 BPF CU / 1,400,000 budget):
└── CreateCollectionV2 FAILED: custom program error: 0x2e (11,023 / 1,400,000 CU) MplCore
    │ >> log:  Error: Custom program error: 0x2e
    ├── CreateAccount System
    └── Transfer System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Collection
    participant MplCore
    participant System
    Collection ->> MplCore: CreateCollectionV2 (11023cu)
    MplCore ->> System: CreateAccount
    MplCore ->> System: Transfer
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
    Collection([Collection]):::signer
    Payer([Payer]):::signer
    System[System]:::program
    Collection -->|signs| MplCore
    Payer -->|signs| MplCore
    Payer -->|signs| System
    Collection -->|signs| System
    System -->|writes| Collection
```

## Ownership graph

Which program owns each account the transaction wrote.

```mermaid
flowchart LR
    classDef owner fill:#cce5ff,stroke:#007bff;
    classDef account fill:#fff3cd,stroke:#ffc107;
    System[System]:::owner
    Collection[(Collection)]:::account
    Payer[(Payer)]:::account
    System -->|owns| Collection
    System -->|owns| Payer
```
