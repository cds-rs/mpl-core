# Reject adding an AgentIdentity plugin without the PDA

**Intent.** Attempt to add an AgentIdentity plugin with the signing PDA remaining account omitted; the program cannot find a signer and rejects.

**Outcome.** The transaction failed: `missing required signature for instruction`.

**Source.** [`tests/agent_identity.rs::cannot_add_agent_identity_without_pda_remaining_account`](../tests/agent_identity.rs#L536)

## Structured execution log

```
CPI Tree (3,200 BPF CU / 1,400,000 budget):
└── AddExternalPluginAdapter FAILED: missing required signature for instruction (3,200 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: A signature was required but not found
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: AddExternalPluginAdapter (3200cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ missing required signature for instruction
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
    MplCore[MplCore]:::owner
    Asset[(Asset)]:::account
    System[System]:::owner
    Payer[(Payer)]:::account
    MplCore -->|owns| Asset
    System -->|owns| Payer
```
