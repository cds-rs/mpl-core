# Reject a mismatched AgentIdentity PDA

**Intent.** Attempt to add an AgentIdentity plugin signed by a PDA derived from a different asset. The derivation check rejects the wrong PDA.

**Outcome.** The transaction failed: `custom program error: 0x33`.

**Source.** [`tests/agent_identity.rs::cannot_add_agent_identity_with_wrong_pda`](../tests/agent_identity.rs#L569)

## Structured execution log

```
CPI Tree (9,915 BPF CU / 1,400,000 budget):
└── AddExternalPluginAdapter FAILED: custom program error: 0x33 (9,915 / 1,400,000 CU) MplCore (no CPIs)
      >> log:  Error: Custom program error: 0x33
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Payer
    participant MplCore
    Payer ->> MplCore: AddExternalPluginAdapter (9915cu)
    rect rgb(255, 220, 220)
    note over MplCore: ✗ custom program error: 0x33
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
    WrongPda([WrongPda]):::signer
    Payer -->|signs| MplCore
    WrongPda -->|signs| MplCore
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
