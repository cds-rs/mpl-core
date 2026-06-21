# Update an AgentIdentity plugin's URI

**Intent.** Rewrite the AgentIdentity plugin's URI on an asset that already carries it. A flat instruction: the update authority signs, no PDA needed.

**Outcome.** The transaction succeeded.

**Source.** [`tests/agent_identity.rs::update_agent_identity_uri`](../tests/agent_identity.rs#L303)

## Structured execution log

```
CPI Tree (10,758 BPF CU / 1,400,000 budget):
└── UpdateExternalPluginAdapter (10,758 / 1,400,000 CU) MplCore
    │ >> log:  programs/mpl-core/src/plugins/external_plugin_adapters.rs:381:Base:Approved
    └── Transfer System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    participant System
    Payer ->> MplCore: UpdateExternalPluginAdapter (10758cu)
    MplCore ->> System: Transfer
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
    System[System]:::program
    Payer -->|signs| MplCore
    Payer -->|signs| System
    MplCore -->|writes| Asset
    System -->|writes| Asset
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
