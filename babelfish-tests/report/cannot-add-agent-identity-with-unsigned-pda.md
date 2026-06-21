# Reject an unsigned AgentIdentity PDA

**Intent.** Attempt to add an AgentIdentity plugin where the correct PDA is present but not marked as a signer; the program rejects the missing signature.

**Outcome.** The transaction failed: `missing required signature for instruction`.

**Source.** [`tests/agent_identity.rs::cannot_add_agent_identity_with_unsigned_pda`](../tests/agent_identity.rs#L594)

## Structured execution log

```
CPI Tree (3,515 BPF CU / 1,400,000 budget):
└── AddExternalPluginAdapter FAILED: missing required signature for instruction (3,515 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: A signature was required but not found
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: AddExternalPluginAdapter (3515cu)
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
