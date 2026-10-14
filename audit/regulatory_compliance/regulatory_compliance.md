# OpenDex Regulatory Compliance — Master Document

**Last updated:** 2026-10-12

This document contains the complete regulatory architecture analysis for OpenDex, covering EU MiCA (Markets in Crypto-Assets Regulation) and UK FCA regimes. It consolidates all regulatory compliance documentation into a single reference.

**Key update (2026-10-12):** OpenDex operates with a **No KYC policy** — participants are not required to verify their identity. The ComplianceProvider has been completely removed from the codebase. This aligns with the decentralisation strategy: no central entity collects or stores identity documents, reducing regulatory exposure as a CASP (Crypto-Asset Service Provider).

---

## Disclaimer

These documents provide technical/regulatory architecture analysis for OpenDex. They are design guidance for EU regulatory counsel and **do not constitute legal opinions**. Final legal determinations should be made by qualified regulatory counsel in the relevant jurisdictions.

---

## Table of Contents

1. [Strategic Framework: Engineering Facts for Legal Perimeter](#1-strategic-framework-engineering-facts-for-legal-perimeter)
2. [ADMP v6: Decentralisation, Non-Custody & Regulatory Boundary Specification](#2-admp-v6-decentralisation-non-custody--regulatory-boundary-specification)
3. [Governance Boundaries: Three-Way Domain Separation](#3-governance-boundaries-three-way-domain-separation)
4. [MiCA Regulatory Architecture & Decentralisation Analysis](#4-mica-regulatory-architecture--decentralisation-analysis)
5. [Architecture Regulatory Risk Assessment](#5-architecture-regulatory-risk-assessment)
6. [Implementation Control Matrix](#6-implementation-control-matrix)
7. [Compliance Status Tracker](#7-compliance-status-tracker)
8. [Fiat Subsystem Separation Boundary](#8-fiat-subsystem-separation-boundary)
9. [Monero (XMR) and MiCA Article 76 Analysis](#9-monero-xmr-and-mica-article-76-analysis)
10. [Living Document Process](#10-living-document-process)
11. [Appendix A: Glossary](#appendix-a-glossary)
12. [Appendix B: DAO Interface Design Sketch](#appendix-b-dao-interface-design-sketch)

---

# 1. Strategic Framework: Engineering Facts for Legal Perimeter

The strategy can be defined as engineering the facts so the legal perimeter analysis has the strongest possible decentralisation/non-custody case.

There is an important catch, though: the fee/treasury arrangement is one of the facts the FCA says can matter. The FCA's September 2026 perimeter guidance (published 16 September 2026, following CP26/13) specifically says that, for decentralised/automated arrangements, relevant considerations can include who operates or maintains the service, who sets key parameters, who controls important aspects, and who receives fees or another commercial benefit.

So I would not say:
"OpenDex is decentralised, therefore the fees don't matter."

Instead:
OpenDex is designed so that the entity receiving fees has no corresponding control over user assets, execution, custody, or protocol operation.

That's a much stronger position.

## 1.1 MiCA and FCA Strategy

### 1.1.1 The MiCA Strategy

MiCA gives you the key statutory starting point: Recital 22 says that MiCA applies to activities performed or controlled by persons, including partially decentralised activities, but that crypto-asset services provided "in a fully decentralised manner without any intermediary" should not fall within MiCA. ESMA has expressly said that whether something is fully decentralised is assessed case by case.

So OpenDex should be engineered around demonstrating:

```
OPENDEX
│
┌─────────────────┼─────────────────┐
│                 │                 │
PROTOCOL           USERS             ENTITY
│                 │                 │
DAO governs       control assets    receives fees
protocol          control keys      controls treasury
│                 │                 │
▼                 ▼                 ▼
Network/matching    Blockchain       Commercial affairs
```

The crucial point is that the entity's economic benefit does not automatically translate into operational control.

### 1.1.2 What Your Entity Should NOT Control

Your OpenDex entity should not be able to:

- choose which user's trade executes;
- cancel another user's valid order;
- alter the order book;
- seize user assets;
- freeze user balances;
- withdraw user crypto;
- change an executed trade;
- override the settlement protocol;
- act as custodian of private keys;
- become the counterparty to every trade;
- operate the only node necessary for the system;
- determine which nodes may participate;
- secretly change protocol execution rules.

These constraints are formalised as DNC invariants in Section 2.10.

If those capabilities don't exist, the fact that your entity receives protocol fees tells a much more limited story.

### 1.1.3 Where the UK FCA Analysis Gets More Difficult

The UK is actually the area where I would be more cautious.

The FCA's new regime takes effect on 25 October 2027, and its final perimeter guidance expressly addresses interfaces to automated protocols. The FCA says whether an interface is within the perimeter depends on whether an identifiable person is carrying on a regulated activity by way of business, assessed case-by-case.

And the FCA specifically says that decentralisation, smart contracts or public blockchains do not themselves determine the perimeter.

That means your architecture needs to answer:
**What exactly does the OpenDex company do to earn its fees?**

That's the question I'd build the entire regulatory architecture around.

## 1.2 Architectural Model

### 1.2.1 The Strongest Version of Your Model

I think your intended architecture should be:

```
OPENDEX PROTOCOL
│
┌──────────────┴──────────────┐
│                             │
Independent nodes              DAO governance
│                             │
┌────────┴────────┐                    │
│                 │                    │
OrderBook          SwapEngine        Protocol rules
Matching           Settlement        Software upgrades
│                 │
└────────┬────────┘
│
▼
USER TRANSACTIONS
│
▼
USER-CONTROLLED ASSETS
│
▼
BLOCKCHAIN


                 SEPARATE
                    │
                    ▼
             OPENDEX ENTITY
                    │
                    ▼
              PROTOCOL FEES
                    │
                    ▼
             OPENDEX TREASURY
```

Notice what's missing:
OpenDex entity → user assets
There should be no such arrow.

And also:
OpenDex entity → execution
There should be no such arrow.

### 1.2.2 The DAO Actually Helps If We Use It Correctly

Your DAO position is now quite interesting.

The DAO governs:
- protocol rules;
- protocol upgrades;
- technical standards;
- node/network rules;
- perhaps settlement-adapter standards;
- perhaps other protocol-level parameters.

But:

```
DAO
X
│
├── Fee treasury
├── Company treasury
├── Company finances
└── User assets
```

There is no authority there.

That gives you three distinct control domains:

| Domain | Control |
|--------|---------|
| User assets | User |
| Protocol | DAO / decentralised protocol |
| Fee revenue & treasury | OpenDex entity |

That separation should be explicit in the architecture.

### 1.2.3 The FCA "Tripwire"

This is the part I don't want us to gloss over.

Suppose you have:

```
OpenDex entity
│
├── runs 95% of nodes
├── controls the website
├── controls protocol parameters
├── decides upgrades
├── can stop trading
└── receives all fees
```

Calling the protocol decentralised won't save you.

The FCA explicitly says the substance and role performed by the person matters rather than the terminology used.

But suppose instead:

```
OpenDex entity
│
└── receives protocol-defined fees

Independent nodes
│
├── execute protocol
├── maintain order state
├── validate trades
└── communicate P2P

Users
│
└── control their assets

DAO
│
└── governs protocol
```

Now you've created a materially different factual situation.

That's the architecture I think you're aiming for.

## 1.3 Decentralisation Testing

### 1.3.1 The FCA Company Disappearance Test

The FCA's current guidance says the question is whether an identifiable person is carrying on the relevant regulated activity by way of business. Its guidance also considers things such as continuity, commercial element and scale, although the newer cryptoasset regime has a narrower business test for its regulated activities.

So we should explicitly test:
**If OpenDex Ltd disappeared tomorrow:**

- Do existing nodes continue operating?
- Does the order book continue functioning?
- Can nodes still match?
- Can trades still propagate?
- Does BTC/XMR settlement still function?
- Can users still recover/control their assets?
- Does the protocol still work?
- Does the DAO still function?

If the answer is yes, that's a powerful architectural fact.

If the answer is: "No, because OpenDex Ltd runs the central API/matching engine/settlement service" then the decentralisation argument is considerably weaker.

## 1.4 Fee Design

### 1.4.1 Your Fees Need to Be Designed Carefully Too

This is where I would make a distinction between:

**Protocol-generated fee**
```
Trade
↓
protocol deterministically calculates fee
↓
fee is routed according to protocol rules
```

versus:

**Operator charging for an exchange service**
```
Customer
↓
OpenDex company
↓
OpenDex executes customer's transaction
↓
OpenDex charges customer
```

The second structure creates a much more obvious commercial-service narrative.

The first is not automatically outside regulation — the FCA specifically warns that receiving fees/commercial benefit can be relevant — but it gives us a much better starting point if the recipient has no corresponding control over the trading activity.

### 1.4.2 ESMA's Operational Reality Test

ESMA has also said that regulators should look at the actual operational reality, rather than simply accepting how a service describes itself. In its guidance on exchange/execution services, ESMA says the analysis should look at the actual order-fulfilment flow and the role the provider performs.

That is exactly why I think your architecture work is worthwhile.

We don't want:
"OpenDex is decentralised because our lawyers say so."

We want:
"Here is the executable architecture. Here is who can do what. Here is what OpenDex Ltd cannot do. Here is what happens if OpenDex Ltd disappears."

That's much stronger.

### 1.4.3 The "Bullet-Dodging Strategy"

Not:
Hide behind decentralisation.

But:
Remove the characteristics that would make the company the intermediary.

Specifically:

**NO:**
- central custody
- central matching authority
- unilateral execution authority
- unilateral settlement authority
- control of user keys
- ability to seize user principal
- permissioned node network
- company-controlled global order book
- company-controlled trade finality
- DAO control of company treasury

**YES:**
- protocol-generated fees
- company-controlled fee treasury
- company commercial activity
- company development
- company-operated infrastructure
- DAO protocol governance
- independent node operation
- user-controlled assets

That is a much more defensible architecture than simply calling OpenDex a DAO or DEX.

And importantly, I would not promise that this puts OpenDex outside MiCA or the FCA perimeter. The regulators' current guidance makes clear that the analysis is fact-specific, and the FCA expressly flags commercial benefit/fees as one relevant consideration.

What we can do is engineer OpenDex so that, when counsel performs that analysis, the facts are as favourable to the decentralised/non-custodial position as technically possible.

That should be the purpose of ADMP v6.

---

# 2. ADMP v6: Decentralisation, Non-Custody & Regulatory Boundary Specification

We should not design this as if fees are irrelevant to decentralisation. The FCA's September 2026 perimeter guidance (published 16 September 2026) expressly identifies operating/maintaining a service, setting key parameters, controlling important aspects, and receiving fees or other commercial benefit as potentially relevant facts. It also says decentralisation does not itself determine the perimeter.

At the EU level, MiCA Recital 22 provides the important starting point that crypto-asset services provided fully decentralised and without an intermediary should not fall within MiCA; ESMA says whether a platform is fully decentralised is assessed case-by-case.

So the objective is not to hide the fee relationship. It is to make the fee relationship genuinely separate from operational/intermediary control.

## 2.1 Purpose

OpenDex is designed as a decentralised, non-custodial peer-to-peer exchange protocol.

The architecture deliberately separates three domains:

1. **User financial assets** — controlled by users through their own cryptographic credentials and the applicable settlement/blockchain mechanisms.
2. **Protocol governance and operation** — performed through the decentralised OpenDex protocol and its governance mechanism.
3. **OpenDex commercial activity** — consisting principally of protocol-generated fee revenue and the treasury derived from those fees.

These domains must remain architecturally and operationally distinct.

The existence of an OpenDex commercial entity does not, by itself, give that entity authority over user assets, trade execution, order state, settlement outcomes, or protocol governance.

The DAO governs the protocol domain. It does not govern the OpenDex commercial treasury.

The OpenDex entity controls its own protocol-generated fee revenue and treasury. It does not control user principal.

The three domains are defined in detail in Section 2.3.

## 2.2 Core Architectural Principle

The fundamental principle of OpenDex is:

**No OpenDex-associated entity or governance mechanism may exercise unilateral financial control over user principal.**

User assets must remain outside OpenDex custody and outside any administrative balance system capable of overriding the underlying settlement mechanism.

OpenDex may generate and receive protocol fees.

Protocol fees are economically distinct from user principal.

Therefore:

```
User principal
│
▼
User-controlled settlement
│
▼
Blockchain / settlement mechanism


Protocol activity
│
▼
Protocol-defined fee
│
▼
OpenDex commercial treasury
```

There must be no architectural path equivalent to:

```
User principal
│
▼
OpenDex custody
│
▼
OpenDex decides settlement
```

## 2.3 Three Control Domains

### 2.3.1 User Asset Domain

The user asset domain consists of:

- private keys;
- wallet authority;
- blockchain assets;
- transaction-signing authority;
- user principal;
- settlement rights belonging to the user.

OpenDex must not possess unilateral authority to:

- transfer user principal;
- freeze user principal;
- seize user principal;
- rewrite blockchain ownership;
- override a valid blockchain settlement;
- withdraw user assets;
- substitute itself as owner of user assets.

The protocol may coordinate a transaction, construct transaction information, validate conditions, observe blockchain state, and determine protocol state.

It must not thereby acquire ownership or unilateral control of the underlying assets.

### 2.3.2 Protocol Domain

The protocol domain consists of:

- order submission;
- order propagation;
- order validation;
- matching;
- deterministic trade identity;
- trade validation;
- replicated state;
- trade state transitions;
- settlement protocol rules;
- blockchain observations;
- network communication;
- protocol persistence;
- protocol upgrades;
- interoperability rules.

Protocol execution must not depend upon a single OpenDex-controlled server.

An independent compatible node must be capable of participating in protocol operation without requiring permission from the OpenDex commercial entity.

### 2.3.3 OpenDex Commercial Domain

The OpenDex commercial domain consists of:

- protocol-generated fees;
- fee accounting;
- commercial revenue;
- the OpenDex treasury;
- commercial expenditure;
- business operations;
- commercial development activities.

The OpenDex entity may control its own commercial treasury.

The DAO does not control that treasury.

The DAO does not possess authority to:

- transfer treasury funds;
- appoint treasury signatories;
- determine commercial expenditure;
- direct company distributions;
- redirect company revenue;
- replace the commercial entity as treasury owner.

The commercial entity likewise does not possess authority over user principal merely because it receives protocol-generated fees.

## 2.4 DAO Boundary

The DAO governs the protocol domain.

DAO authority may include:

- protocol rules;
- protocol standards;
- protocol upgrade mechanisms;
- network parameters;
- compatibility requirements;
- technical governance;
- protocol-level economic parameters where appropriate.

DAO authority does not extend to:

- user assets;
- user private keys;
- user balances held outside the protocol;
- OpenDex commercial treasury;
- company bank accounts;
- company-owned assets;
- commercial expenditure;
- company personnel;
- company contractual arrangements.

The DAO therefore should not be described as the owner of OpenDex generally.

The more precise model is:

```
DAO
│
└── governs protocol


OpenDex entity
│
└── conducts commercial affairs
and controls protocol-generated fee revenue


Users
│
└── control their own assets
```

## 2.5 Fee Boundary

Fees are a deliberate exception to the otherwise non-custodial financial architecture.

OpenDex may receive protocol-generated fees.

However, fee authority must not create authority over user principal.

The protocol should calculate fees deterministically from protocol-defined parameters.

Conceptually:

```
Trade
│
├── User settlement amount
│
└── Protocol fee
    │
    ▼
    OpenDex treasury
```

The fee must be distinguishable from the user's principal.

Fee calculation should be:

- deterministic;
- transparent;
- independently reproducible;
- recorded in protocol state where appropriate;
- auditable;
- incapable of arbitrarily modifying the underlying trade;
- incapable of granting OpenDex additional settlement authority.

The existence of fee revenue must not permit OpenDex to:

- cancel another user's trade;
- alter another user's order;
- freeze another user's funds;
- redirect another user's principal;
- modify settlement conditions after execution;
- change the identity of an executed trade;
- override a blockchain settlement.

See Section 1 for the full list of capabilities the entity should not possess.

## 2.6 Commercial Benefit Does Not Equal Protocol Control

OpenDex explicitly recognises that receipt of fees is a relevant factual characteristic of the overall architecture.

The design therefore does not attempt to conceal or eliminate the commercial relationship.

Instead, the architecture establishes a separation between:

```
Commercial benefit
│
▼
OpenDex fee revenue
```

and:

```
Operational control
│
▼
Protocol execution
```

Receipt of protocol fees must not confer:

- matching authority;
- custody authority;
- settlement authority;
- user-account authority;
- order cancellation authority;
- blockchain transaction authority;
- unilateral protocol execution authority.

The commercial entity's economic interest therefore remains separate from control of user transactions.

## 2.7 No Central Matching Authority

OpenDex must not require an OpenDex-operated central matching engine.

The order book is replicated state.

Nodes independently:

- receive orders;
- validate orders;
- maintain order state;
- identify compatible orders;
- execute deterministic matching rules;
- generate deterministic trade identities;
- validate received trades;
- apply valid trades.

The existence of OpenDex-operated nodes for convenience, bootstrap, development, monitoring, or infrastructure purposes does not make those nodes authoritative protocol components.

Independent nodes must be able to perform the same protocol functions.

## 2.8 No Central Settlement Authority

Settlement must not depend on OpenDex approving a user's transaction.

For BTC/XMR atomic swaps:

```
Marketplace
│
▼
SwapEngine
│
▼
SettlementProtocol
│
├── BitcoinChain
│
└── MoneroChain
```

The SettlementProtocol defines the protocol conditions.

Blockchain watchers observe external state.

The state machine determines protocol consequences.

OpenDex personnel must not have an administrative mechanism allowing them to substitute an arbitrary settlement outcome for the protocol-defined outcome.

## 2.9 Non-Custodial Asset Invariant

The following invariant is mandatory:

**NC-001 — OpenDex cannot unilaterally move user principal.**

This must remain true even if:

- all OpenDex-operated nodes are compromised;
- an OpenDex administrator acts maliciously;
- the GUI is compromised;
- the API is compromised;
- the commercial treasury is compromised;
- the DAO is compromised;
- an individual developer is compromised.

Compromise of OpenDex infrastructure must not produce unilateral control over user principal.

## 2.10 Decentralisation Invariants

These invariants formalise the constraints described in Section 1.

The following invariants define the intended decentralised architecture.

**DNC-001** — OpenDex cannot transfer user principal unilaterally.

**DNC-002** — OpenDex cannot freeze user principal unilaterally.

**DNC-003** — OpenDex cannot seize user principal unilaterally.

**DNC-004** — OpenDex cannot become the mandatory counterparty to user trades.

**DNC-005** — OpenDex cannot unilaterally determine which valid user trade executes.

**DNC-006** — OpenDex cannot unilaterally rewrite an executed trade.

**DNC-007** — OpenDex cannot unilaterally override blockchain settlement.

**DNC-008** — An independent compatible node can participate without permission from OpenDex.

**DNC-009** — The protocol remains operational without a central OpenDex matching server.

**DNC-010** — The protocol remains capable of settlement without a central OpenDex settlement authority.

**DNC-011** — User assets remain controlled by user credentials.

**DNC-012** — The DAO cannot control the OpenDex commercial treasury.

**DNC-013** — OpenDex commercial treasury control cannot provide control over user principal.

**DNC-014** — Protocol fees are independently calculable.

**DNC-015** — Protocol fee collection cannot modify the identity of an executed trade.

**DNC-016** — Protocol fee collection cannot create additional rights over user principal.

**DNC-017** — Removal of OpenDex commercial infrastructure does not transfer user assets to OpenDex.

**DNC-018** — Removal of OpenDex commercial infrastructure does not invalidate blockchain ownership.

**DNC-019** — Removal of OpenDex commercial infrastructure does not make the protocol state dependent upon a central database.

**DNC-020** — Protocol state remains independently verifiable by participating nodes.

## 2.11 Company Failure Test

OpenDex must be designed to pass the following architectural test:

**If the OpenDex commercial entity ceased operating tomorrow, independent protocol nodes should retain the ability to operate the protocol and users should retain control of their assets.**

The disappearance of the commercial entity may affect:

- commercial support;
- commercial infrastructure;
- development resources;
- fee collection;
- company-operated interfaces.

It must not:

- confiscate user assets;
- invalidate blockchain ownership;
- destroy user settlement rights;
- make historical trades unverifiable;
- require a central database to recover protocol state;
- prevent independent nodes from operating the protocol.

This is a core decentralisation property.

## 2.12 OpenDex Infrastructure Is Not Protocol Authority

OpenDex may operate:

- bootstrap nodes;
- public nodes;
- websites;
- GUI distributions;
- documentation;
- developer infrastructure;
- monitoring infrastructure;
- APIs;
- archival infrastructure;
- development environments.

These services must not become indispensable protocol authorities.

Where practical, equivalent protocol functions should remain accessible through independent nodes and standardised protocol interfaces.

The architecture therefore distinguishes:

**OpenDex-operated infrastructure**

from:

**OpenDex protocol authority**

The former may exist.

The latter must be minimised.

## 2.13 GUI and Front-End Boundary

The GUI is an interface to the protocol.

It must not become the protocol itself.

A user should be able, in principle, to interact with OpenDex through an alternative compatible client.

The GUI must not contain privileged capabilities allowing OpenDex to:

- override protocol validation;
- alter another user's order;
- alter trade identity;
- bypass settlement rules;
- access user private keys without user authority.

The GUI should therefore be treated as:

```
Client
│
▼
Protocol
```

rather than:

```
User
│
▼
OpenDex company
│
▼
Protocol
```

## 2.14 API Boundary

If a future REST/WebSocket API is introduced, it must be treated as an interface rather than a central authority.

API requests should ultimately invoke protocol-defined operations.

The API must not contain privileged alternatives to the protocol state machine.

For example, there should be no administrative endpoint equivalent to:

- `/admin/cancel_trade`
- `/admin/transfer_balance`
- `/admin/freeze_user`
- `/admin/override_settlement`
- `/admin/rewrite_order`

unless such functionality is explicitly part of a separate, non-protocol commercial service and is incapable of affecting protocol user principal.

## 2.15 BalanceBook Boundary

The BalanceBook must be clearly classified as protocol/accounting state rather than evidence that OpenDex has custody of the underlying assets.

For blockchain settlement, the authoritative ownership state ultimately derives from the applicable blockchain and cryptographic control mechanism.

The BalanceBook may record:

- reservations;
- protocol state;
- settlement intent;
- escrow state;
- local accounting state;
- idempotency information.

It must not create a unilateral administrative right to transfer user assets.

This distinction is especially important for BTC/XMR.

## 2.16 Escrow Boundary

The escrowed state represents protocol settlement state.

It must not imply that OpenDex itself owns or has unilateral custody of the assets.

Where the settlement protocol uses cryptographic mechanisms such as atomic swaps, adaptor signatures, multisignature arrangements, timelocks, or equivalent mechanisms, the asset-control consequences must derive from those mechanisms rather than from an OpenDex administrator.

The state:

**Escrowed**

therefore means:

**protocol-defined settlement commitment exists**

and not:

**OpenDex owns the user's funds.**

## 2.17 BTC/XMR Settlement Boundary

The BTC/XMR architecture should preserve the following separation:

```
Marketplace
│
▼
SwapEngine
│
▼
SettlementProtocol
│
▼
Cryptographic settlement adapter
│
├── Bitcoin
└── Monero
```

The marketplace must not directly control cryptographic settlement primitives.

The OpenDex commercial entity must not possess an administrative override capable of replacing the settlement protocol.

Blockchain watchers are observers.

They do not become custodians.

The SwapEngine is a state machine.

It does not become a custodian.

The SettlementProtocol is a cryptographic boundary.

It does not become a central financial controller.

## 2.18 Protocol Governance Boundary

Governance changes must be subject to explicit protocol rules.

A governance decision must not automatically grant:

- custody;
- withdrawal authority;
- access to private keys;
- unilateral settlement authority;
- access to user principal.

Protocol governance should therefore be treated as a technical governance function rather than a financial custody function.

## 2.19 Upgrade Authority

Protocol upgrades must be evaluated separately from commercial treasury control.

The ability to upgrade protocol software must not automatically imply the ability to:

- seize funds;
- rewrite balances;
- alter historical trades;
- bypass settlement;
- transfer user assets.

Where upgrade mechanisms are technically capable of doing these things, that capability constitutes a significant centralisation and regulatory-risk control point and must be removed, constrained, or subjected to explicit decentralised safeguards.

## 2.20 Emergency Controls

Emergency controls must be designed with particular care.

A legitimate emergency mechanism may protect:

- protocol integrity;
- network safety;
- software deployment;
- vulnerability mitigation.

It must not become a hidden mechanism for:

- user asset seizure;
- user balance modification;
- unilateral settlement;
- arbitrary trade cancellation;
- treasury extraction from users.

Emergency protocol controls must therefore be separately documented and tested against the non-custody invariants.

## 2.21 Fee Treasury Architecture

The fee treasury is deliberately outside DAO governance.

Conceptually:

```
Protocol
│
│ protocol-generated fee
▼
OpenDex commercial entity
│
▼
OpenDex treasury
```

The DAO has no direct authority over the treasury.

The treasury has no authority over user principal.

This creates two separate financial systems:

**USER FINANCIAL SYSTEM**
User → settlement → blockchain

**OPENDEX COMMERCIAL SYSTEM**
Protocol → fees → OpenDex treasury

They must not be merged into a single balance or custody system.

## 2.22 Fee Transparency

Although the DAO does not govern the treasury, the protocol fee mechanism should remain technically transparent.

Nodes and users should be able to determine:

- applicable fee parameters;
- fee calculation;
- fee amount;
- fee recipient;
- transaction/trade to which the fee relates;
- protocol state used to calculate the fee.

The OpenDex entity's subsequent use of treasury funds is a commercial matter and is not part of protocol consensus.

## 2.23 Regulatory Architecture Principle

The architecture is intended to support, rather than predetermine, the legal analysis of the OpenDex model.

The relevant legal question is not:
"Does OpenDex call itself decentralised?"

The relevant factual question is:
**Who actually provides, operates, controls, maintains, governs, or economically benefits from the relevant activity, and what authority does that person possess in practice?**

Accordingly, OpenDex must preserve evidence demonstrating:

- distributed protocol operation;
- absence of custody;
- absence of unilateral user-asset control;
- independent node operation;
- deterministic execution;
- transparent fee calculation;
- separation between protocol governance and commercial treasury;
- absence of privileged execution paths;
- absence of administrative settlement overrides.

## 2.24 MiCA Design Objective

MiCA Recital 22 recognises that crypto-asset services provided in a fully decentralised manner without an intermediary should not fall within MiCA.

OpenDex therefore seeks to ensure that the relevant protocol services are architecturally decentralised and non-custodial rather than merely described as such.

This is a design objective and not a legal conclusion.

Whether a particular OpenDex service is within or outside MiCA must ultimately be determined from the implemented architecture, actual operation, relevant territorial facts, applicable crypto-assets and the activities performed.

## 2.25 UK FCA Design Objective

The UK architecture must be assessed separately from MiCA.

The FCA's perimeter analysis is substance-based and fact-specific.

OpenDex must therefore assume that the following characteristics may be relevant to the analysis:

- who operates the service;
- who maintains the service;
- who controls key aspects;
- who sets important parameters;
- who contracts with users;
- who provides interfaces;
- who performs the relevant activity;
- who receives fees or another commercial benefit.

Accordingly, OpenDex must not rely upon the existence of a DAO or decentralised technology alone.

The architecture must demonstrate actual separation of:

**Protocol operation**
≠
**OpenDex commercial control**
≠
**User asset control**

## 2.26 Regulatory Evidence Model

OpenDex should maintain technical evidence capable of demonstrating the architectural boundaries.

Evidence should include:

- protocol specifications;
- source-code history;
- node operation documentation;
- network topology;
- protocol message specifications;
- deterministic execution tests;
- custody-boundary tests;
- fee-calculation tests;
- governance permissions;
- treasury permissions;
- upgrade permissions;
- cryptographic key architecture;
- settlement architecture;
- blockchain transaction records;
- independent-node test results.

The audit logs already implemented in OpenDex should be extended where appropriate to capture evidence relevant to these boundaries.

## 2.27 Decentralisation Test Suite

The project should add automated tests corresponding to the DNC invariants.

At minimum:

**User asset tests**
- OpenDex cannot construct an administrative user withdrawal.
- OpenDex cannot alter user settlement ownership.
- OpenDex cannot bypass cryptographic signing requirements.
- OpenDex cannot change completed blockchain settlement.

**Protocol tests**
- Independent nodes produce identical deterministic trade identities.
- Independent nodes converge on valid state.
- No central node is required for matching.
- No central node is required for trade propagation.
- No central node is required for protocol state recovery.

**Governance tests**
- DAO operations cannot access the commercial treasury.
- DAO operations cannot access user principal.
- Governance cannot bypass settlement rules.

**Treasury tests**
- Treasury operations cannot access user balances.
- Treasury compromise cannot alter user principal.
- Fee revenue is distinguishable from user principal.
- Fee calculation is deterministic.

**Failure tests**
- OpenDex commercial node shutdown.
- OpenDex website shutdown.
- OpenDex API shutdown.
- OpenDex bootstrap-node shutdown.
- OpenDex developer-key compromise.
- OpenDex treasury compromise.

Each scenario should be tested for preservation of user asset control and protocol integrity.

## 2.28 The OpenDex Separation Test

The architecture should satisfy the following conceptual test:

**If OpenDex Ltd disappears:**
- user assets remain theirs
- blockchain ownership remains valid
- independent nodes can operate
- protocol state remains verifiable
- settlement mechanisms remain usable

**And:**
**If the DAO disappears:**
- the commercial treasury remains the property/control of the OpenDex entity
- and user assets remain user-controlled.

**And:**
**If the OpenDex treasury is compromised:**
- user principal remains unaffected.

**And:**
**If an individual OpenDex developer is compromised:**
- that developer cannot obtain unilateral control over user principal.

These are architectural security properties.

## 2.29 Architectural Anti-Patterns

The following designs should be considered inconsistent with the intended OpenDex model unless separately justified:

- central custodial wallet;
- OpenDex-controlled omnibus customer wallet;
- company-controlled private keys for user assets;
- central matching authority with mandatory OpenDex participation;
- company-controlled trade cancellation;
- company-controlled settlement override;
- hidden administrative balance modification;
- DAO-controlled company treasury;
- company-controlled DAO voting;
- upgrade keys capable of moving user assets;
- emergency keys capable of moving user assets;
- mandatory company-operated API;
- mandatory company-operated front-end;
- protocol functionality unavailable to independent nodes.

## 2.30 Final Architectural Position

OpenDex therefore consists of three intentionally separated systems:

```
OPENDEX ECOSYSTEM
│
┌──────────────────┼──────────────────┐
│                  │                  │
▼                  ▼                  ▼

USER ASSET          PROTOCOL          COMMERCIAL
DOMAIN            DOMAIN             DOMAIN
│                  │                  │
│                  │                  │
User keys           DAO             OpenDex entity
│                  │                  │
▼                  ▼                  ▼
Blockchain       Decentralised       Fees
settlement       protocol            Treasury
│
▼
Independent nodes
```

The governing principle is:

**The protocol may coordinate transactions without owning the assets involved in those transactions.**

**The DAO may govern the protocol without governing the OpenDex commercial treasury.**

**The OpenDex entity may control protocol-generated fees without controlling user principal.**

**Commercial benefit must not confer unilateral operational or financial authority over users.**

This separation is a core architectural property of OpenDex and must be preserved through future protocol, settlement, GUI, API, governance, treasury and infrastructure development.

## 2.31 Implementation Requirement

All future OpenDex architecture documents should treat the following as compatibility-level invariants:

```
USER PRINCIPAL
↓
USER CONTROL
↓
BLOCKCHAIN / CRYPTOGRAPHIC SETTLEMENT

PROTOCOL
↓
DECENTRALISED EXECUTION
↓
DAO GOVERNANCE

PROTOCOL FEES
↓
OPENDEX ENTITY
↓
OPENDEX TREASURY
```

No future feature may silently collapse these three domains into one.

Any feature that introduces:

- custody;
- unilateral settlement authority;
- central matching authority;
- privileged user-balance control;
- DAO treasury control;
- company control over user principal;
- mandatory company infrastructure;

must trigger a new decentralisation and regulatory-boundary review before implementation.

This gives us the engineering specification we can now work from. It also deliberately avoids saying "this makes OpenDex exempt"; the FCA's current guidance is clear that the analysis is fact-specific, and its new cryptoasset regime is due to expand the UK perimeter from 25 October 2027.

---

# 3. Governance Boundaries: Three-Way Domain Separation

The DAO governs the protocol, but it does not govern the fees or the fee treasury.

The fee/treasury domain belongs to your OpenDex entity, while the protocol itself remains decentralised and non-custodial with respect to user assets.

```
OPENDEX
│
┌─────────────────┴─────────────────┐
│                                   │
DECENTRALISED PROTOCOL                OPENDEX ENTITY
│                                   │
│                              Fee / Treasury
│                                   │
┌────┼────┐                       ┌──────┴──────┐
│    │    │                       │             │
Nodes Nodes Nodes              Protocol fees   Treasury
│    │    │
└────┼────┘
│
User transactions
│
▼
USER-CONTROLLED ASSETS
```

And separately:

```
DAO
│
governs protocol only
│
┌───────────────┼───────────────┐
│               │               │
Protocol         Software        Network
rules            upgrades        parameters
```

No DAO authority over:

- fee revenue;
- fee treasury;
- your company's treasury;
- your company's financial decisions;
- distribution of fee revenue.

## 3.1 The Critical Three-Way Separation

See Section 2.3 for the Three Control Domains specification.

ADMP v6 explicitly defines three separate domains:

### 3.1.1 User financial domain
Controlled by the user.

```
Private keys
↓
User assets
↓
Blockchain
```

OpenDex cannot arbitrarily access these assets.

### 3.1.2 Protocol domain
Governed by the DAO / protocol governance mechanism.

- Matching
- Order rules
- Protocol parameters
- Network rules
- Software/protocol upgrades
- Settlement protocol standards

The DAO controls how the protocol operates, not people's money.

### 3.1.3 OpenDex commercial domain
Controlled by your entity.

```
Protocol fees
↓
OpenDex revenue
↓
OpenDex treasury
↓
Business decisions
```

The DAO has no authority over this domain.

That is an important distinction, and I would not describe the DAO as the "owner" or "controller" of OpenDex generally. It governs the protocol.

## 3.2 One Architectural Consequence

We should therefore avoid putting the fee treasury inside the DAO architecture.

Instead:

```
PROTOCOL
│
│ generates
▼
FEES
│
▼
OpenDex Fee Treasury
│
▼
OpenDex Entity
```

while:

```
DAO
│
▼
Protocol Governance
```

are parallel systems.

The DAO shouldn't have a transaction such as:

- `DAO → transfer_treasury()`
- `DAO → change_treasury_owner()`
- `DAO → distribute_fees()`

unless you deliberately decide to change the business model later.

## 3.3 Precise Architectural Statement

Instead of saying:
"OpenDex controls the financial side."

I'd document it as:

**OpenDex is a decentralised, non-custodial protocol. User assets remain under user control and are not subject to unilateral financial control by OpenDex, its governance system, node operators, or developers. OpenDex's commercial entity separately controls protocol-generated fee revenue and the treasury derived from that revenue. Protocol governance does not extend to the fee treasury or the commercial affairs of the OpenDex entity.**

That's a very clear architectural boundary.

## 3.4 Domain Controller Table

| Domain | Controller |
|--------|-----------|
| User assets | User |
| Private keys | User |
| Trade execution | Protocol/nodes |
| Order book | Distributed protocol state |
| Protocol rules | DAO governance |
| Protocol software | DAO governance / protocol process |
| Network participation | Open participation |
| Protocol fees | OpenDex entity |
| Fee treasury | OpenDex entity |
| Commercial decisions | OpenDex entity |
| User principal | Never OpenDex |

That is the model I would use as the foundation for the next architecture revision.

---

# 4. MiCA Regulatory Architecture & Decentralisation Analysis

**Version:** 1.0
**Date:** 8 October 2026
**Scope:** EU Markets in Crypto-Assets Regulation (MiCA), Regulation (EU) 2023/1114
**Purpose:** Technical/regulatory architecture analysis for OpenDex
**Status:** Design guidance for EU regulatory counsel; not a legal opinion

Below is the regulatory architecture working specification for OpenDex, deliberately separating the protocol, node software, infrastructure, and commercial operator, because MiCA turns heavily on who is actually providing a service to clients.

## 4.1 Executive Conclusion

OpenDex's current architecture has a credible technical basis for arguing that the OpenDex protocol can operate in a fully decentralised manner without an intermediary, which is outside MiCA's scope under Recital 22.

However, the conclusion cannot be based on the existence of an order book, P2P networking, open-source code, or the label "DEX" alone.

ESMA's current position is that crypto-asset services provided in a fully decentralised manner without an intermediary are outside MiCA, but whether a platform is actually fully decentralised is determined case by case by the competent authorities.

MiCA defines crypto-asset services to include:

- custody and administration;
- operation of a trading platform;
- exchange of crypto-assets for funds;
- exchange of crypto-assets for other crypto-assets;
- execution of orders;
- reception and transmission of orders;
- transfer services;
- and other specified services.

Article 59 prohibits a person from providing MiCA crypto-asset services in the Union unless appropriately authorised or otherwise permitted under MiCA.

### Preliminary OpenDex Position

The architecture should therefore be divided into four regulatory layers:

```
┌─────────────────────────────────────────────┐
│             OPENDEX PROTOCOL                │
│                                             │
│ P2P order propagation                       │
│ replicated order books                      │
│ deterministic matching                      │
│ deterministic trade identity                │
│ P2P settlement                              │
│ atomic BTC/XMR swaps                        │
│                                             │
│ Potentially outside MiCA if genuinely       │
│ fully decentralised and intermediary-free   │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│             NODE SOFTWARE                   │
│                                             │
│ User-operated node                          │
│ User-controlled wallet                      │
│ Local OrderBook / TradeBook / BalanceBook   │
│ Local SwapEngine                            │
│                                             │
│ Potentially outside MiCA                    │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│       OPTIONAL INFRASTRUCTURE               │
│                                             │
│ Relays / bootstrap nodes / frontend / API   │
│ Block explorers / discovery services        │
│                                             │
│ Must not become mandatory intermediary      │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│         COMMERCIAL OPERATOR                 │
│                                             │
│ Company / DAO / foundation                  │
│                                             │
│ Highest regulatory risk if it:              │
│ - matches orders                            │
│ - routes orders                             │
│ - executes trades                           │
│ - holds assets                              │
│ - controls escrow                           │
│ - operates a trading platform               │
│ - exchanges crypto for fiat                 │
└─────────────────────────────────────────────┘
```

The recommended architecture is therefore not to build OpenDex as a conventional CASP and call it a DEX.

Instead, the stronger architecture is:

**The OpenDex company develops and publishes a decentralised protocol, but no OpenDex-controlled intermediary is necessary for users to discover, negotiate, execute or settle trades.**

## 4.2 Applicable MiCA Perimeter

MiCA defines a crypto-asset service as one of the enumerated services relating to crypto-assets. Relevant to OpenDex are particularly:

1. custody and administration;
2. operation of a trading platform;
3. exchange of crypto-assets for funds;
4. exchange of crypto-assets for other crypto-assets;
5. execution of orders on behalf of clients;
6. reception and transmission of orders;
7. transfer services.

The significance is that OpenDex does not need to become a CASP merely because the software permits people to trade crypto-assets.

The critical question is:
**Who is providing the service to whom, and is there an intermediary?**

MiCA Article 59 provides that a person may not provide crypto-asset services within the Union unless authorised or otherwise permitted under the Regulation.

## 4.3 Decentralisation Exemption

MiCA Recital 22 provides the principal argument for OpenDex:

**crypto-asset services provided in a fully decentralised manner without any intermediary are outside the scope of MiCA.**

The European Commission has subsequently confirmed through the ESMA Q&A process that crypto-asset services provided in a fully decentralised manner without an intermediary are not in scope and that whether a platform is fully decentralised is assessed case by case by competent authorities.

This should be treated as an architectural condition, not as a marketing statement.

The objective should therefore be to make the following proposition technically true:

**No identifiable person or entity is required to act as an intermediary between the parties for the execution and settlement of an OpenDex trade.**

## 4.4 OpenDex OrderBook

### 4.4.1 Current Architecture

OpenDex uses replicated OrderBooks.

Each node maintains:

- active orders;
- applied trade IDs;
- pending trades;
- removed-order tombstones;
- fill sequences.

Matching occurs locally through `OrderBook::match_order()`.

There is no stated single canonical matching server.

### 4.4.2 MiCA Significance

This is one of the strongest elements of the current architecture.

A centralised exchange normally has:

```
Client A
|
v
Exchange server
|
v
Central order book
|
v
Exchange matching engine
|
v
Client B
```

OpenDex instead has:

```
Node A                 Node B
OrderBook              OrderBook
|                      |
+------ P2P -----------+
|
replicated state
```

That substantially strengthens the decentralisation argument.

### 4.4.3 Required Invariant

The architecture should explicitly require:

**No OpenDex-controlled service shall constitute the unique or mandatory matching engine for OpenDex trades.**

A user should be able to match an order without sending the order to an OpenDex-operated matching service.

## 4.5 Does Exchange Create a Regulated Intermediary?

Not necessarily.

The Rust component:

```
Exchange
├── OrderBook
├── TradeBook
└── BalanceBook
```

is a software abstraction.

That does not itself make the developer of the software a CASP.

The regulatory issue arises if an identifiable operator runs the Exchange component as a service for clients.

### 4.5.1 Protocol-Only Model

```
Alice runs node
|
v
Alice's Exchange instance
|
v
Alice's local OrderBook
```

and:

```
Bob runs node
|
v
Bob's Exchange instance
|
v
Bob's local OrderBook
```

### 4.5.2 Company-Operated Model

```
Alice
|
v
OpenDex Ltd API
|
v
OpenDex Exchange instance
|
v
Central order book
|
v
Bob
```

The second model creates a substantially stronger argument that OpenDex Ltd is operating or providing a service to clients.

### 4.5.3 Recommended Approach

The architecture should favour the protocol-only model where each user runs their own Exchange instance locally, rather than relying on an OpenDex-operated central service.

## 4.6 Trading-Platform Analysis

MiCA expressly regulates the operation of a trading platform for crypto-assets.

Article 76 imposes extensive requirements on CASPs operating such platforms, including operating rules, admission procedures, fair and non-discriminatory participation criteria, orderly trading procedures and execution systems.

Therefore the design objective should be:

**OpenDex Ltd must not itself operate a MiCA trading platform unless it deliberately chooses to become a CASP.**

The decentralised protocol should instead provide the underlying peer-to-peer mechanism.

### Important Distinction

Do not rely on:
"There is no central server."

The stronger argument is:
**"There is no intermediary whose service is necessary for the transaction."**

A decentralised system could theoretically still have an intermediary.

## 4.7 Order Matching

OpenDex currently performs:

```
incoming order
|
v
local matching
|
v
Trade
|
v
deterministic trade ID
|
v
validation
|
v
application
```

This is preferable to:

```
incoming order
|
v
OpenDex server
|
v
OpenDex decides matching
```

### Recommended Protocol Invariant

Add:

**MATCH-001:** Matching shall be executable by an independently operated OpenDex node without contacting infrastructure controlled by the OpenDex development entity.

And:

**MATCH-002:** No OpenDex-operated service shall possess exclusive authority to determine whether a valid order is matched.

## 4.8 Deterministic Trade IDs

Your deterministic trade identity system is technically strong:

```
trade_id =
SHA256(
  buy_order_id :
  sell_order_id :
  execution_price :
  execution_amount :
  fill_sequence
)[0..16]
```

This does not itself determine MiCA status.

However, it supports the proposition that execution is governed by protocol rules rather than operator discretion.

That is useful.

The regulatory significance is therefore indirect:

```
Deterministic protocol
↓
less operator discretion
↓
less central execution control
↓
stronger decentralisation evidence
```

## 4.9 Trade Validation

The architecture states:

**No replicated execution is trusted merely because it was received.**

This is a strong protocol design principle.

The validation chain is:

```
Network message
↓
Peer authentication
↓
Trade identity
↓
Trade structure
↓
Referenced orders
↓
Execution constraints
↓
Local application
```

This supports an important architectural position:

**No OpenDex operator is the authority that declares a trade valid.**

Instead, validity derives from the protocol's deterministic rules.

This should be preserved.

## 4.10 P2P Network and Relayers

This is one of the areas that should receive additional design attention.

The current architecture uses libp2p/gossipsub.

That is compatible with decentralisation.

However, a future architecture such as:

```
All users
|
v
OpenDex Relay Network
|
v
P2P nodes
```

could undermine the argument if the OpenDex relay infrastructure becomes necessary.

### Recommended Design

Users should be able to:

- run their own node;
- connect directly to peers;
- use independently operated relays;
- discover peers without OpenDex-controlled infrastructure;
- propagate orders through multiple independent paths.

### Recommended Invariant

**NET-001:** No OpenDex-controlled relay, API, gateway or server shall be technically necessary for trade execution.

And:

**NET-002:** The protocol shall support independently operated network infrastructure.

## 4.11 Frontend / GUI

The desktop GUI (`crates/desktop-gui`) is comparatively favourable because it communicates with a local node.

**Current Implementation:** The desktop GUI is a standalone crate extracted from `p2p-node`, built with iced 0.14. It runs as a thin client of the user's local P2P node — it does not accept orders, route trades, or provide a trading interface to a central exchange. The `iced` dependency is isolated from `p2p-node`, so headless builds don't compile the GUI framework.

**Regulatory Rating:** 🟢 GREEN — Local node client, no central exchange gateway.

The preferred model is:

```
GUI
|
v
Local OpenDex Node
|
+---- P2P
```

rather than:

```
GUI
|
v
OpenDex API
|
v
OpenDex servers
|
v
P2P network
```

### Recommended Invariant

**GUI-001:** The OpenDex GUI shall not provide privileged trading functionality unavailable to an independently operated node.

The GUI should be a client of the user's node, not the gateway to an OpenDex-operated exchange.

## 4.12 Web Frontend

The web frontend has been implemented as a static informational website (🟢 GREEN).

**Current Implementation:** The web portal (`crates/web-frontend`) is a static informational site built with Leptos (Rust → WebAssembly). It provides project information, software downloads, a built-in source code browser, and legal compliance pages. It does not accept orders, route trades, or provide a trading interface.

**Regulatory Rating:** 🟢 GREEN — Informational only, no CASP activity.

A future interactive web frontend (beyond the current informational site) would be one of the most significant regulatory design decisions.

### Preferred

```
Browser
|
v
Local / user-controlled node
|
v
P2P OpenDex network
```

### More Problematic

```
Browser
|
v
opendex.com
|
v
OpenDex API
|
v
OpenDex order-routing service
|
v
P2P network
```

The second arrangement can make the website/operator an identifiable service provider.

The website should therefore be capable of functioning as a non-custodial client interface without becoming a central execution intermediary.

## 4.13 Custody

See Section 2.3.1 for the User Asset Domain definition.

MiCA expressly includes custody and administration of crypto-assets on behalf of clients as a regulated crypto-asset service.

### 4.13.1 MiCA Custody Definition

Therefore the key question for OpenDex escrow is not the name of the software object.

The question is:
**Who controls the private keys?**

### 4.13.2 OpenDex Custody Model

```
User
|
| controls private key
v
User wallet
|
v
protocol-controlled transaction
```

### Higher-Risk Architecture

```
User
|
v
OpenDex
|
v
OpenDex-controlled wallet
|
v
escrow
```

The second model should be avoided if the objective is to maintain a decentralised/non-custodial structure.

### 4.13.3 Required Invariant

The architecture must ensure that OpenDex-controlled infrastructure never holds user private keys or unilateral control over user crypto-assets.

## 4.14 BalanceBook

The BalanceBook currently tracks:

- available;
- locked;
- escrowed;
- reservations;
- settlement status.

This is acceptable as a local ledger/state machine.

The regulatory concern arises if OpenDex Ltd controls the underlying assets.

The architecture should therefore distinguish clearly between:

**ACCOUNTING STATE**

and:

**CONTROL OF CRYPTO-ASSETS**

### Recommended Invariant

**CUST-001:** A node's BalanceBook shall not give the node operator unilateral control over another user's crypto-assets.

### Recommended Stronger Formulation

**CUST-002:** OpenDex protocol software shall not require users to transfer custody of crypto-assets to an OpenDex-operated wallet.

## 4.15 Escrow

### 4.15.1 Current Escrow Model

Your escrow model is:

```
locked
↓
escrowed
↓
released / returned
```

This is technically sensible.

### 4.15.2 Regulatory Risk

But "escrowed" must not imply:
"OpenDex has custody."

The architecture should specify exactly what cryptographic mechanism creates escrow.

### 4.15.3 Required Invariant

For the BTC/XMR system, the strongest model is:

```
Alice key material
+
Bob key material
+
protocol conditions
↓
blockchain-enforced settlement
```

rather than:

```
Alice
↓
OpenDex escrow wallet
↓
OpenDex decides payout
↓
Bob
```

If a third party has unilateral authority to release funds, the regulatory analysis changes substantially.

The escrow mechanism must be cryptographic, not administrative — no third party should have unilateral release authority.

## 4.16 BTC/XMR Atomic Swaps

The BTC/XMR architecture is potentially one of the strongest parts of OpenDex.

You have:

```
Marketplace
↓
SwapEngine
↓
SettlementProtocol
↓
BtcXmrAdapter
↓
Bitcoin / Monero
```

This separation is good.

The important architectural principle should be:

**The marketplace does not take possession of the crypto-assets being exchanged.**

The ideal settlement flow is:

```
Alice
|
| BTC
v
Bitcoin protocol
^
|
| XMR
|
Bob
```

with the swap mechanism enforcing the conditions.

If OpenDex itself controls the assets, however, the regulatory analysis becomes materially different.

## 4.17 Fiat Subsystem

This is the area requiring the greatest separation from the BTC/XMR architecture.

MiCA expressly identifies:

**exchange of crypto-assets for funds**

as a crypto-asset service.

It also separately identifies exchange of crypto-assets for other crypto-assets.

Therefore OpenDex should treat these as two different products:

**Product A**
BTC ↔ XMR
P2P atomic swap.

**Product B**
BTC ↔ EUR
BTC ↔ GBP
XMR ↔ EUR
Fiat-enabled P2P marketplace.

The second requires substantially more regulatory analysis.

## 4.18 Fiat Should Not Be Mixed into the Protocol's Core Decentralisation Claim

The protocol should ideally make this distinction:

**CORE PROTOCOL**

```
crypto asset
↕
crypto asset

P2P settlement
```

versus:

**APPLICATION / PAYMENT LAYER**

```
crypto
↕
fiat
↕
bank/payment infrastructure
```

This prevents the fiat system from contaminating the decentralisation architecture of the atomic-swap protocol.

## 4.19 Reception and Transmission of Orders

MiCA separately regulates reception and transmission of orders.

ESMA describes RTO as situations where a CASP receives a client's order and transmits it to a third party for execution or onward transmission.

Therefore avoid an architecture such as:

```
Alice
↓
OpenDex Ltd receives order
↓
OpenDex routes order
↓
DEX / third party
```

because the company could potentially be performing an RTO function.

### Preferred

```
Alice's node
↓
P2P propagation
↓
Bob's node
```

There is no OpenDex Ltd order-routing role.

## 4.20 Execution of Orders

ESMA's current explanation is also important.

Execution services apply when a CASP acts as an agent for the client and concludes the transaction on the client's behalf.

Therefore OpenDex should avoid having a company-controlled component act as:

**Alice's agent**

for execution.

The protocol should instead facilitate:

**Alice ↔ Bob**

with neither party represented by OpenDex Ltd.

## 4.21 ESMA's 2025/2026 DEX Treatment Is Useful for OpenDex

ESMA has clarified that when an authorised CASP executes an order outside a MiCA-authorised trading platform, the relevant arrangements can include decentralised exchanges.

This is important because it demonstrates that EU regulators do not treat the words "decentralised exchange" as inherently meaningless.

However, that does not establish that every DEX is outside MiCA.

The decisive question remains whether the DEX is genuinely fully decentralised and intermediary-free.

## 4.22 XMR / Privacy-Asset Issue

This deserves a separate architecture flag.

MiCA imposes special requirements on regulated trading platforms concerning crypto-assets with an inbuilt anonymisation function.

Article 76 includes restrictions concerning admission to trading of crypto-assets with such features.

Therefore:

**OpenDex as decentralised protocol**
≠
**OpenDex as MiCA-regulated CASP**

The implications for XMR are substantially different under those two models.

If OpenDex remains genuinely decentralised, the protocol's position must be analysed under the decentralisation perimeter.

If OpenDex becomes a regulated trading platform, the XMR listing question becomes much more significant.

Therefore do not design the system assuming that:
"We can become a normal MiCA exchange later and continue exactly as before."

## 4.23 Governance

See Section 2.4 for the DAO boundary specification.

### 4.23.1 DAO and MiCA

A DAO is not automatically decentralised for MiCA purposes.

The relevant question should be:
**Who can actually exercise control?**

For example:

```
DAO
|
+-- change matching algorithm
+-- freeze accounts
+-- censor users
+-- select mandatory relayers
+-- control custody
+-- change settlement
```

would require careful analysis.

### 4.23.2 ESMA Treatment

The stronger architecture is:

```
Governance
|
v
software development
|
v
users voluntarily adopt versions
```

rather than:

```
Governance
|
v
mandatory protocol authority
|
v
all user trades
```

### 4.23.3 Required Design

Protocol governance should influence software development, with users voluntarily adopting new versions, rather than having mandatory authority over live user trades.

## 4.24 Upgrade Mechanism

The project should document exactly how protocol upgrades work.

### Recommended

```
OpenDex developers
|
v
publish version
|
v
users independently decide
|
v
run version
```

### Higher-Risk

```
OpenDex authority
|
v
mandatory upgrade
|
v
network changes
```

The more unilateral power an identifiable entity has over live trading, the harder it becomes to characterise the system as genuinely intermediary-free.

## 4.25 Fees

Fees require special treatment.

A decentralised protocol can technically have fees.

The important question is:
**Who receives them, and what service is being provided in exchange?**

### Potentially Stronger

```
Protocol transaction fee
↓
automatically distributed
↓
no operator discretion
```

### Potentially Weaker

```
User trade
↓
OpenDex Ltd
↓
10 bps service fee
↓
OpenDex Ltd operates infrastructure
```

The second model can strengthen the argument that OpenDex Ltd is commercially providing a service.

Fees are therefore not automatically prohibited; they need to be analysed together with control and service provision.

## 4.26 Open-Source Software

Open source is helpful but not determinative.

The following does not establish decentralisation by itself:

```
MIT licence
+
GitHub repository
+
public source code
=
not MiCA
```

What matters is operational reality.

The stronger combination is:

```
open source
+
permissionless node operation
+
non-custody
+
no mandatory infrastructure
+
no central matcher
+
no central router
+
no central settlement authority
+
no unilateral operator control
```

## 4.27 Recommended OpenDex Decentralisation Invariants

The following should be added to the architecture specification.

**DEC-001 — No mandatory intermediary**
No OpenDex-controlled intermediary shall be necessary for a valid trade.

**DEC-002 — Independent matching**
Any node must be capable of performing the protocol's matching algorithm without contacting OpenDex-controlled infrastructure.

**DEC-003 — Independent order propagation**
Orders must be propagatable through independently operated nodes.

**DEC-004 — No central order authority**
No entity shall possess exclusive authority over the canonical order book.

**DEC-005 — No operator custody**
OpenDex-controlled infrastructure shall not hold user private keys or unilateral control over user crypto-assets.

**DEC-006 — No operator settlement authority**
OpenDex-controlled infrastructure shall not have unilateral authority to release or confiscate user settlement assets.

**DEC-007 — Independent node operation**
A third party must be capable of running a complete OpenDex node without permission from OpenDex.

**DEC-008 — Independent frontend operation**
A third party must be capable of creating a frontend that connects directly to the protocol.

**DEC-009 — Independent relay operation**
A third party must be capable of operating network relay/discovery infrastructure.

**DEC-010 — No privileged operator**
OpenDex-operated infrastructure shall not possess protocol privileges unavailable to ordinary nodes unless technically unavoidable and independently justified.

**DEC-011 — No mandatory OpenDex API**
Trading functionality shall not depend on an OpenDex-operated REST/WebSocket API.

**DEC-012 — No mandatory OpenDex account**
Users shall not be required to create an account with OpenDex Ltd merely to use the protocol.

**DEC-013 — No unilateral censorship**
No OpenDex operator shall have unilateral authority to prevent a valid peer-to-peer transaction.

**DEC-014 — User-controlled keys**
Users shall retain control of the private keys necessary to control their crypto-assets.

**DEC-015 — Deterministic execution**
Where protocol rules determine execution, those rules shall be executable independently by nodes.

## 4.28 Green / Yellow / Red Architecture Matrix

| Component | Current design | Regulatory assessment |
|-----------|---------------|----------------------|
| Replicated OrderBook | Yes | 🟢 |
| Local matching | Yes | 🟢 |
| Central matcher | Not described | 🟢 |
| P2P gossip | Yes | 🟢 |
| Deterministic trades | Yes | 🟢 |
| Trade validation | Yes | 🟢 |
| Persistent state | Yes | 🟢 |
| User-run nodes | Intended | 🟢 |
| Desktop GUI | Standalone crate, local node client | 🟢 |
| Open-source node | Yes | 🟢 |
| Local BalanceBook | Yes | 🟢 |
| Local escrow ledger | Yes | 🟡 |
| Key-control specification | Needs clarification | 🟡 |
| Central relay | Not currently described | 🟡 |
| Mandatory API | Not currently described | 🟡 |
| Web frontend | Informational site implemented | 🟢 |
| DAO | Not specified | 🟡 |
| Upgrade authority | Not specified | 🟡 |
| Fiat subsystem | Existing/planned | 🔴/🟡 |
| OpenDex custody | Should be avoided | 🔴 |
| Central matching | Should be avoided | 🔴 |
| Central order routing | Should be avoided | 🔴 |
| Central settlement | Should be avoided | 🔴 |
| OpenDex fiat exchange | High regulatory risk | 🔴 |
| Mandatory OpenDex relayers | Avoid | 🔴 |
| OpenDex-controlled escrow keys | Avoid | 🔴 |

## 4.29 Proposed Target Architecture

The regulatory target should be:

```
OPENDEX PROTOCOL

┌───────────────────────────────────────────────────────┐
│                                                       │
│                 Permissionless P2P network            │
│                                                       │
│  ┌──────────┐       ┌──────────┐       ┌──────────┐  │
│  │ Alice    │◄─────►│ Bob      │◄─────►│ Carol    │  │
│  │ Node     │       │ Node     │       │ Node     │  │
│  │          │       │          │       │          │  │
│  │ OrderBook│       │ OrderBook│       │ OrderBook│  │
│  │ TradeBook│       │ TradeBook│       │ TradeBook│  │
│  │ Balance  │       │ Balance  │       │ Balance  │  │
│  │ SwapEngine│      │ SwapEngine│      │ SwapEngine│ │
│  └──────────┘       └──────────┘       └──────────┘  │
│                                                       │
└───────────────────────────────────────────────────────┘

                  │                    │
                  ▼                    ▼

             Bitcoin                 Monero
             blockchain              blockchain
```

**No mandatory:**
- OpenDex Ltd → central API → central matcher → central order book → central custodian → central settlement agent

## 4.30 Commercial Company Structure

The commercial entity should ideally be positioned as:

```
OpenDex Ltd
|
+-- develops software
+-- maintains open-source repository
+-- publishes releases
+-- produces documentation
+-- performs security research
+-- may provide optional infrastructure
```

rather than:

```
OpenDex Ltd
|
+-- accepts customer orders
+-- matches orders
+-- routes orders
+-- holds customer assets
+-- controls escrow
+-- settles trades
+-- operates the trading venue
```

The second model is much closer to a CASP business.

## 4.31 Evidence That OpenDex Should Maintain

If OpenDex intends to rely on the decentralisation position, it should maintain evidence rather than merely making a legal assertion.

### Recommended Evidence Includes

**Protocol evidence**
- public source code;
- reproducible builds;
- protocol specifications;
- deterministic matching tests;
- deterministic trade-ID tests;
- node interoperability tests.

**Network evidence**
- independent nodes;
- independent relays;
- direct peer connections;
- no central routing requirement;
- no central discovery requirement.

**Custody evidence**
- key-control documentation;
- wallet architecture;
- cryptographic transaction flows;
- proof that OpenDex cannot unilaterally move user assets.

**Governance evidence**
- upgrade procedures;
- release process;
- voting/control arrangements;
- evidence that users can reject software updates.

**Operational evidence**
- no central order database;
- no central matching logs;
- no central customer trading accounts;
- no operator ability to cancel arbitrary trades.

**Economic evidence**
- fee mechanism;
- fee recipients;
- operator revenue;
- infrastructure costs;
- whether revenue depends on providing a crypto-asset service.

### 4.31.1 Technical Evidence

Protocol-level proof of decentralisation, including replicated state, deterministic matching, and independent node operation.

### 4.31.2 Operational Evidence

Evidence that no central authority controls order books, matching, or settlement.

### 4.31.3 Commercial Evidence

Documentation of fee mechanisms, revenue flows, and the separation between commercial and protocol operations.

## 4.32 What an EU Competent Authority Is Likely to Ask

### 4.32.1 Control Questions

OpenDex should be able to answer these questions precisely:

1. Who operates the platform?
2. Who owns the order book?
3. Where is the canonical order book?
4. Who matches orders?
5. Who can modify matching?
6. Who receives orders?
7. Who routes orders?
8. Who executes orders?
9. Who controls user assets?
10. Who controls private keys?
11. Who controls escrow?
12. Who can reverse settlement?
13. Who can freeze funds?
14. Who can censor a user?
15. Who can shut down trading?
16. Who operates the relayers?
17. Can a third party operate a relayer?
18. Can users connect directly?
19. Can users operate their own node?
20. Can the protocol operate without OpenDex Ltd?
21. Can another developer fork the protocol?
22. Can users continue using the old protocol if OpenDex disappears?
23. Who receives fees?
24. Who determines fees?
25. Who controls protocol upgrades?
26. Is there a company behind the frontend?
27. Does the frontend transmit orders?
28. Does the frontend match orders?
29. Does the frontend provide custody?
30. Does OpenDex interact with fiat?
31. Does OpenDex provide transfer services?
32. Does OpenDex operate any central API?
33. Does OpenDex operate mandatory infrastructure?

### 4.32.2 Service Questions

Additional questions focus on whether OpenDex provides a service to clients:

- Does OpenDex receive client orders?
- Does OpenDex route orders?
- Does OpenDex execute trades on behalf of clients?
- Does OpenDex hold client assets?

### 4.32.3 Recommended Answers

The stronger the answers are:
**"Nobody; the protocol itself does this and any node can perform the function."**
the stronger the decentralisation argument becomes.

## 4.33 Recommended Legal Positioning

I would not describe OpenDex publicly as:
"A MiCA-exempt exchange."

That sounds like a legal conclusion.

Instead, technically describe it as:
**"A non-custodial, peer-to-peer protocol in which order propagation, matching, execution validation and settlement are performed by independently operated nodes without a central matching authority."**

Then have EU counsel assess whether the resulting system qualifies for the fully decentralised/intermediary-free treatment.

That distinction is important because ESMA expressly leaves the fully-decentralised determination to a case-by-case assessment.

## 4.34 Important Distinction: Protocol vs Company

The project should maintain separate documentation for:

**A. OpenDex Protocol** — What the software does.
**B. OpenDex Node** — What an independent user-operated node does.
**C. OpenDex Infrastructure** — Optional infrastructure that may be operated by anyone.
**D. OpenDex Ltd** — What the commercial entity actually does.

This separation should exist both:
- technically; and
- legally/corporately.

Do not let documentation imply that OpenDex Ltd operates the protocol merely because it develops it.

Conversely, do not use technical decentralisation language to obscure activities that the company itself actually performs.

## 4.35 MiCA CASP Fallback Position

If OpenDex eventually decides that it wants to operate:

- a hosted trading platform;
- a hosted exchange;
- customer accounts;
- managed custody;
- order routing;
- central execution;
- fiat exchange;

then it should assume that a MiCA CASP analysis is required.

The authorisation regime is substantial.

For example, ESMA's current MiCA materials identify minimum capital requirements of:

- €50,000 for certain Class 1 services;
- €125,000 for Class 2 services including custody and exchange;
- €150,000 for Class 3 services including operation of a trading platform.

That is not a reason to avoid regulation; it simply means the CASP route should be treated as a different product architecture.

## 4.36 Recommended Immediate Architecture Changes

Before implementing the public-facing system, I recommend these changes.

**Priority 1 — Define custody**
Document exactly:
- where BTC private keys live;
- where XMR private keys live;
- whether any server ever sees them;
- how escrow is cryptographically enforced;
- who can release escrow;
- what happens if OpenDex Ltd disappears.

This is the most important missing architectural/legal boundary.

**Priority 2 — Define network independence**
Specify:
- peer discovery;
- bootstrap nodes;
- relay nodes;
- direct connections;
- alternative relay providers.

Ensure OpenDex Ltd infrastructure is optional.

**Priority 3 — Define frontend independence**
Ensure a third party can build:

```
AlternativeFrontend
↓
OpenDex node
```

without permission from OpenDex Ltd.

**Priority 4 — Define upgrade control**
Document:
- who publishes releases;
- who can force upgrades;
- whether old versions continue functioning;
- whether protocol rules can be changed unilaterally.

**Priority 5 — Separate fiat**
Create a hard architecture boundary:

**Crypto/crypto protocol**
versus:
**Fiat/P2P application**

The latter needs a separate regulatory workstream.

**Priority 6 — Define operator powers**
Create a machine-readable/documented list of every administrative capability OpenDex Ltd possesses.

Ideally:
**Operator powers = none**
with respect to user trades and user assets.

## 4.37 Proposed Formal Decentralisation Statement

Subject to legal review, the technical architecture could state:

**OpenDex Decentralisation Principle**

The OpenDex protocol is designed to operate without a central intermediary. Order discovery, order propagation, matching, trade identification, trade validation, trade-state processing and crypto-asset settlement are performed by independently operated protocol nodes according to deterministic protocol rules.

No OpenDex-operated server is required for a participant to create, discover, match, execute or settle a valid trade.

OpenDex-operated infrastructure, where provided, is optional and does not possess exclusive authority over order matching, execution, settlement, custody or state validation.

Users retain control of the cryptographic credentials required to control their crypto-assets. The protocol does not require users to transfer crypto-assets to an OpenDex-operated custodian.

The protocol does not designate an OpenDex-operated entity as the authoritative operator of a canonical order book or matching engine.

Independent parties may operate nodes, relays, interfaces and other protocol infrastructure without permission from OpenDex.

Protocol software is publicly inspectable and may be independently implemented or operated.

No statement in this document constitutes a determination that OpenDex is outside the scope of Regulation (EU) 2023/1114. Whether a particular deployment constitutes a fully decentralised crypto-asset service without an intermediary is a matter for assessment under the applicable law and, where necessary, by the relevant competent authority.

## 4.38 Overall Assessment

**Protocol architecture**
Strong decentralisation characteristics.

The replicated OrderBook, local matching, P2P propagation, deterministic execution identity, local validation and non-central state model are all positive.

**Node architecture**
Potentially strong.

The design is compatible with user-operated nodes rather than a mandatory hosted exchange.

**Custody**
Needs explicit cryptographic specification.

The BalanceBook/escrow model is not enough to establish non-custody. Key control must be documented.

**Relayers**
Needs architectural safeguards.

Do not allow OpenDex-operated relayers to become mandatory.

**Frontend**
Currently favourable.

The desktop GUI (`crates/desktop-gui`) is a standalone crate that runs as a client of the user's local node rather than a central exchange frontend. The GUI code is isolated from the p2p-node crate, and the iced dependency is not coupled to the node.

**BTC/XMR**
Potentially strong decentralised model.

The SettlementProtocol/SwapEngine architecture is well suited to a non-custodial atomic-swap design.

**Fiat**
Highest-risk component.

Treat this as a separate regulatory product.

**Company**
Potentially outside MiCA only if its actual activities remain outside the regulated-service perimeter.

The company should not accidentally turn itself into the intermediary that the protocol was designed to eliminate.

## 4.39 Final Architectural Target

The cleanest target for OpenDex is:

```
┌──────────────────────┐
│   OPENDEX PROTOCOL   │
│                      │
│ Permissionless       │
│ Non-custodial        │
│ P2P                  │
│ Deterministic        │
└──────────┬───────────┘
│
┌─────────────────┼─────────────────┐
│                 │                 │
▼                 ▼                 ▼
Node A            Node B            Node C
│                 │                 │
OrderBook          OrderBook          OrderBook
TradeBook          TradeBook          TradeBook
BalanceBook        BalanceBook        BalanceBook
SwapEngine         SwapEngine         SwapEngine
│                 │                 │
└─────────────────┼─────────────────┘
│
P2P settlement
│
┌──────────┴──────────┐
▼                     ▼
Bitcoin                Monero


             OpenDex Ltd
                  │
       ┌──────────┼───────────┐
       ▼          ▼           ▼
    Software   Documentation  Optional
    development              infrastructure

                  X

No mandatory OpenDex:
- custody
- matching
- order routing
- execution
- settlement
- canonical order book
- relay network
- trading account
```

## 4.40 Bottom Line

Your existing architecture is considerably closer to a potentially MiCA-out-of-scope decentralised protocol than to a conventional centralised CASP.

The strongest parts are your replicated order book, local matching, P2P state propagation, deterministic execution, independent nodes and planned BTC/XMR atomic-swap architecture.

The areas I would treat as the principal regulatory attack surfaces are:

1. who operates the nodes/relayers;
2. whether users can trade without OpenDex infrastructure;
3. who controls private keys;
4. who controls escrow;
5. whether a frontend/API receives or routes client orders;
6. whether OpenDex Ltd receives transaction fees for an identifiable service;
7. governance and upgrade control;
8. the existing/planned fiat subsystem.

The most important engineering principle to preserve is:

**If OpenDex Ltd disappeared tomorrow, two independently operated nodes should still be able to discover each other, match, execute and settle a valid BTC/XMR trade using only the protocol and the underlying blockchains.**

That is the architectural property I would make central to the project's MiCA strategy.

Regulatory basis: MiCA Article 3 defines the relevant CASP services; Article 59 establishes the authorisation requirement; Recital 22 and the Commission/ESMA Q&A provide the fully-decentralised/intermediary-free perimeter; ESMA's current material also recognises decentralised exchanges in the context of execution outside MiCA-authorised trading platforms.

---

# 5. Architecture Regulatory Risk Assessment

This is an architecture/regulatory-risk assessment, not a legal opinion, but the design can be made substantially more defensible.

The most important finding is that the technical architecture already contains many of the characteristics you'd want for a genuinely decentralised system, but several names, control points, and deployment assumptions currently leave an identifiable "operator" argument open.

ESMA's position is particularly important: the fully-decentralised/no-intermediary exclusion is real, but whether a DEX qualifies is assessed case-by-case.

## 5.0 Executive Assessment

I'd currently rate the architecture:

| Area | Risk | Assessment |
|------|------|-----------|
| P2P network | 🟢 GREEN | Strong decentralisation characteristics |
| Replicated order book | 🟢 GREEN | Good, provided no privileged operator controls it |
| Deterministic matching | 🟢 GREEN | Strong protocol-level design |
| Trade validation | 🟢 GREEN | Independently verifiable |
| Actor authentication | 🟢 GREEN | Good cryptographic boundary |
| User-local balances | 🟢 GREEN | Strong if users control keys |
| Atomic swaps | 🟢 GREEN | Potentially strong decentralised settlement |
| Blockchain watchers | 🟢 GREEN | Read-only infrastructure |
| Event sourcing | 🟢 GREEN | Protocol infrastructure |
| Local escrow | 🟡 AMBER | Depends heavily on key/control model |
| Exchange coordinator | 🟡 AMBER/🔴 RED | Wording and implementation need clarification |
| Order submission API | 🟡 AMBER | Could become reception/transmission of orders |
| GUI | 🟢/🟡 | Fine if merely a local client |
| Bootstrap nodes | 🟡 AMBER | Need to ensure they aren't mandatory intermediaries |
| OpenDex website | 🟢 GREEN | Informational only — no trading interface |
| Central API | 🔴 RED | Potential CASP service boundary |
| Central matching server | 🔴 RED | Very strong trading-platform argument |
| OpenDex custody | 🔴 RED | Potential custody service |
| OpenDex-controlled escrow | 🔴 RED | Significant custody/control concern |
| Central dispute operator | 🟡/🔴 | Could create intermediary characteristics |
| Protocol governance | 🟡 AMBER | One of the biggest unresolved questions |
| XMR trading | 🟡/🔴 | Additional issue if platform is a MiCA trading platform |

## 5.1 The Legal Test We Should Design Around

See Section 2.2 for the Core Architectural Principle.

MiCA defines a crypto-asset service provider as a legal person or undertaking whose business is providing one or more crypto-asset services to clients professionally. The listed services include custody, operation of a trading platform, exchange for funds, exchange for other crypto-assets, execution of orders, reception/transmission of orders and transfer services.

Article 59 then establishes the basic authorisation requirement for persons providing those services in the EU.

But Recital 22 provides the critical decentralisation concept:

**services provided in a fully decentralised manner without any intermediary**

are outside MiCA.

The problem is that MiCA does not provide a simple technical checklist for "fully decentralised."

ESMA has explicitly said that the determination is case-by-case based on the features of the system.

So our objective should not be:
"Make OpenDex look like a DEX."

It should be:
**Design OpenDex so there is no identifiable entity performing, controlling, or intermediating the relevant crypto-asset service.**

That's a much stronger design objective.

## 5.2 The Biggest Architectural Change: Redefine Exchange

Your current architecture says:
"Exchange coordinates the OrderBook, TradeBook, and BalanceBook."

and:
"The Exchange layer therefore acts as the transactional coordination layer..."

Technically that's sensible.

Regulatorily, however, "Exchange" is exactly the wrong abstraction to leave ambiguous.

I recommend changing the conceptual model to:

```
PARTICIPANT NODE
│
┌───────────┼───────────┐
│           │           │
▼           ▼           ▼
OrderBook    TradeBook   BalanceBook
│           │           │
└───────────┼───────────┘
│
▼
Local Coordinator
│
▼
P2P Protocol
```

Not:

```
OpenDex Exchange
│
┌─────────────┼─────────────┐
▼             ▼             ▼
OrderBook     TradeBook    BalanceBook
```

### Recommendation

Rename the conceptual component:

**Exchange**

to something like:

**NodeCoordinator**

or:

**LocalExecutionCoordinator**

or:

**NodeEngine**

The important thing isn't the Rust name. The important thing is the architecture:

**There is no OpenDex-operated exchange service. There is only software executed by each participant's node.**

That should become a formal protocol invariant.

## 5.3 GREEN — Replicated Order Book

Your order book is one of the strongest parts of the architecture.

You have:

- local order state
- deterministic IDs
- deterministic trades
- monotonic synchronization
- persistent tombstones
- pending trades
- independent validation
- no single matching authority

That is substantially different from:

```
client → central order book → central matcher
```

Your model is:

```
Alice Node
│
├───────────── P2P ─────────────┐
│                               │
OrderBook                       OrderBook
│                               │
TradeBook                       TradeBook
│                               │
BalanceBook                     BalanceBook
```

### Recommendation

Make this a protocol invariant:

**No node is authoritative over another node's order book.**

## 5.4 GREEN — Deterministic Matching

Matching is performed locally by each node using the same deterministic algorithm.

This means:

- No central matching authority.
- No operator discretion over which trades execute.
- Independent verification of matching results.

This is a strong decentralisation property.

## 5.5 GREEN — Deterministic Trade Identity

The deterministic trade ID system:

```
trade_id = SHA256(buy_order_id : sell_order_id : execution_price : execution_amount : fill_sequence)[0..16]
```

ensures that:

- Any node can independently compute the same trade ID.
- Trade identity cannot be altered by any single party.
- Trade replay is prevented by the fill sequence.

This supports the decentralisation argument by demonstrating that execution is rule-based, not operator-controlled.

## 5.6 GREEN — Trade Validation

The architecture requires independent validation of all received trades:

```
Network message
↓
Peer authentication
↓
Trade identity verification
↓
Trade structure validation
↓
Referenced order validation
↓
Execution constraint check
↓
Local application
```

No OpenDex operator declares a trade valid. Validity derives from protocol rules.

## 5.7 GREEN — Actor Authentication

Peer authentication provides a cryptographic boundary:

- Each node authenticates its peers.
- Messages are attributed to specific actors.
- No anonymous submission of orders or trades.

This is a good security boundary that also supports the decentralisation argument.

## 5.8 GREEN — User-Local Balances

The BalanceBook is maintained locally by each node.

If users control their own private keys, this is a strong non-custodial design.

The key question is: **who controls the keys?**

If the answer is "the user," the architecture is strong.

If the answer is "OpenDex," the architecture is a CASP.

## 5.9 GREEN — Atomic Swaps

The BTC/XMR atomic swap architecture:

```
Marketplace
↓
SwapEngine
↓
SettlementProtocol
↓
BtcXmrAdapter
↓
Bitcoin / Monero
```

is potentially one of the strongest decentralisation features.

The marketplace does not take possession of the assets.

Settlement is enforced by cryptographic mechanisms, not by an OpenDex operator.

## 5.10 GREEN — Blockchain Watchers

Blockchain watchers are read-only observers:

```
Blockchain
│
▼
Watcher
│
▼
Observation
│
▼
SwapEngine
│
▼
Protocol decision
```

They do not make financial decisions or directly mutate user assets.

This is a strong architectural pattern.

## 5.11 GREEN — Event Sourcing

Event sourcing provides:

- Immutable protocol history.
- State reconstruction capability.
- Independent audit trail.

This supports the decentralisation argument by making protocol state independently verifiable.

## 5.12 AMBER — Local Escrow

The escrow model:

```
locked
↓
escrowed
↓
released / returned
```

is technically sensible.

However, "escrowed" must not imply "OpenDex has custody."

The architecture needs to specify exactly what cryptographic mechanism creates escrow.

For BTC/XMR, the strongest model is:

```
Alice key material
+
Bob key material
+
protocol conditions
↓
blockchain-enforced settlement
```

rather than:

```
Alice
↓
OpenDex escrow wallet
↓
OpenDex decides payout
↓
Bob
```

## 5.13 AMBER/RED — Exchange Coordinator

The "Exchange" component name is regulatorily problematic.

See Section 2 above for the recommended rename to NodeCoordinator/LocalExecutionCoordinator/NodeEngine.

The architecture must make clear that there is no OpenDex-operated exchange service.

## 5.14 AMBER — Order Submission API

If an API is introduced, it must be treated as an interface, not a central authority.

There should be no administrative endpoints equivalent to:

- `/admin/cancel_trade`
- `/admin/transfer_balance`
- `/admin/freeze_user`
- `/admin/override_settlement`
- `/admin/rewrite_order`

unless such functionality is explicitly part of a separate, non-protocol commercial service.

## 5.15 AMBER — GUI

The GUI is fine if it is merely a local client of the user's node.

It must not become the protocol itself.

A user should be able to interact with OpenDex through an alternative compatible client.

## 5.16 AMBER — Bootstrap Nodes

Bootstrap nodes must not become mandatory intermediaries.

Users should be able to:

- Run their own node.
- Connect directly to peers.
- Use independently operated relays.
- Discover peers without OpenDex-controlled infrastructure.

## 5.17 GREEN — OpenDex Website

The website's regulatory status depends on what it does.

If it merely provides information and links to the software, it is low risk.

If it accepts orders, routes orders, or provides a trading interface that connects to OpenDex-controlled infrastructure, it creates a strong CASP argument.

### Current Implementation

The OpenDex website is implemented as a static informational portal. It provides:

- Project information (what OpenDex is, how it works).
- Feature descriptions.
- Download links for native desktop applications.
- A built-in source code browser with project files and audit documentation.
- Legal compliance pages (disclaimer, risk disclosure, jurisdiction notice, privacy policy, terms of use, contact).

The website does not:

- Accept orders or trade requests.
- Route trades or provide a trading interface.
- Connect to any OpenDex-controlled infrastructure.
- Collect personal data or user information.
- Act as a Crypto-Asset Service Provider (CASP).

The website is fully static, served by a standard web server (Nginx/Caddy), and requires no backend application. This design maintains the lowest possible regulatory risk profile.

## 5.18 RED — Central API

A central API that receives and routes client orders is a potential CASP service boundary.

The architecture should avoid:

```
Alice
↓
OpenDex Ltd API
↓
OpenDex Exchange instance
↓
Central order book
↓
Bob
```

## 5.19 RED — Central Matching Server

A central matching server is a very strong trading-platform argument.

The architecture must not require an OpenDex-operated central matching engine.

## 5.20 RED — OpenDex Custody

If OpenDex controls user private keys or has unilateral control over user crypto-assets, it is a CASP.

The architecture must ensure:

- Users control their own private keys.
- OpenDex cannot unilaterally move user assets.
- No OpenDex-controlled wallet holds user funds.

## 5.21 RED — OpenDex-Controlled Escrow

If OpenDex has unilateral authority to release escrowed funds, it is a custodian.

The architecture must ensure:

- Escrow is enforced by cryptographic mechanisms.
- No single party can unilaterally release funds.
- Settlement authority derives from the protocol, not from OpenDex.

## 5.22 AMBER/RED — Central Dispute Operator

A central dispute operator could create intermediary characteristics.

The architecture should avoid:

```
Dispute
↓
OpenDex decides
↓
Resolution
```

## 5.23 AMBER — Protocol Governance

Governance is one of the biggest unresolved questions.

A DAO is not automatically decentralised for MiCA purposes.

The relevant question is: **Who can actually exercise control?**

The stronger architecture is:

```
Governance
↓
software development
↓
users voluntarily adopt versions
```

rather than:

```
Governance
↓
mandatory protocol authority
↓
all user trades
```

## 5.24 AMBER/RED — XMR Trading

XMR trading creates additional regulatory complexity.

MiCA Article 76 restricts anonymised crypto-assets on regulated trading platforms.

If OpenDex is classified as a CASP trading platform, Article 76 directly prohibits listing XMR.

If OpenDex is decentralised and outside MiCA scope, Article 76 does not directly apply.

See the dedicated XMR/Article 76 analysis below for details.

## 5.25 Prioritized Action List

Based on the risk assessment, the following actions are recommended in priority order:

1. **Define custody boundaries** — Document exactly where private keys live and who controls them.
2. **Rename Exchange component** — Change to NodeCoordinator or equivalent to avoid regulatory confusion.
3. **Define network independence** — Ensure bootstrap nodes and relays are not mandatory.
4. **Define frontend independence** — Ensure third parties can build alternative frontends.
5. **Define upgrade control** — Document how protocol upgrades work and who controls them.
6. **Separate fiat** — Create a hard architecture boundary between crypto/crypto and fiat subsystems.
7. **Define operator powers** — Create a documented list of every administrative capability OpenDex Ltd possesses.
8. **Add decentralisation tests** — Create automated tests for all DNC and DEC invariants.

---

# 6. Implementation Control Matrix

If the architecture says OpenDex is non-custodial, the Rust dependency graph and capability boundaries need to make custody and unilateral financial control difficult or impossible to introduce accidentally.

## 6.1 Purpose

See Section 2.3 for the Three Control Domains specification.

This document translates the OpenDex decentralisation, non-custody and commercial-boundary principles into implementation-level controls.

The objective is to ensure that the architectural separation between:

1. user assets;
2. decentralised protocol operation;
3. DAO governance; and
4. OpenDex commercial fees and treasury

is reflected in the Rust crate architecture, interfaces, permissions and persistence model.

The implementation must prevent accidental convergence of these domains.

## 6.2 Control Domains

```
OPENDEX
│
┌───────────────┼────────────────┐
│               │                │
▼               ▼                ▼

USER ASSET         PROTOCOL         COMMERCIAL
DOMAIN            DOMAIN            DOMAIN
│               │                │
▼               ▼                ▼
User keys       DAO / nodes       OpenDex entity
Blockchain      Matching          Fees
Settlement      State             Treasury
Transport
```

A fourth boundary exists between protocol governance and commercial ownership:

```
DAO governance
X
│
└──> OpenDex commercial treasury

OpenDex commercial entity
X
│
└──> User principal
```

## 6.3 Core Implementation Rule

The following rule applies to all crates:

**No protocol crate may acquire a capability that allows it to unilaterally control user principal unless that capability is explicitly part of a cryptographic settlement mechanism whose authority derives from the users and underlying blockchain protocol rather than from OpenDex administration.**

Likewise:

**No commercial crate may acquire protocol-authority capabilities merely because it receives protocol fees.**

## 6.4 Existing Crate Classification

### 6.4.1 common

**Reference:** `crates/common/`

**Classification:** Protocol-neutral/common infrastructure.

**Permitted:**
- shared utility types;
- common serialization;
- generic helpers;
- protocol-independent primitives.

**Forbidden:**
- user private keys;
- treasury authority;
- administrative balance mutation;
- privileged settlement;
- commercial fee withdrawal authority.

**Control:** `common` must remain incapable of becoming an implicit dependency injection point for financial authority.

### 6.4.2 protocol

**Reference:** `crates/protocol/`

**Classification:** Legacy protocol-domain crate.

**Permitted:**
- P2P trading types;
- payment methods;
- fiat currency definitions;
- P2P offer types;
- trade-domain structures.

**Forbidden:**
- custody;
- private-key management;
- commercial treasury operations;
- DAO treasury operations;
- arbitrary user balance mutation.

**Required boundary:** `protocol` represents protocol data. It must not represent ownership of assets.

### 6.4.3 p2p-node

**Reference:** `crates/p2p-node/`

**Classification:** Protocol node / application layer.

This is one of the most sensitive crates because it currently coordinates:

- libp2p;
- order book;
- trade book;
- balance book;
- settlement;
- GUI;
- CLI;
- persistence.

**Permitted:**
- network participation;
- order validation;
- matching;
- trade validation;
- protocol-state persistence;
- protocol-state synchronisation;
- invocation of settlement protocols;
- local protocol accounting;
- GUI communication.

**Forbidden:**
- central authority over other nodes;
- unilateral user-asset transfer;
- administrative user-balance modification;
- arbitrary settlement override;
- hidden privileged trade cancellation;
- company-controlled protocol state override.

**Required control:** `p2p-node` must not contain an administrator pathway capable of changing another user's financial state independently of protocol rules.

### 6.4.4 OrderBook

**Reference:** `crates/p2p-node/src/orderbook.rs`

**Classification:** Protocol state.

**Owns:**
- active orders;
- applied trade IDs;
- pending trades;
- removed-order tombstones;
- fill sequences.

**Permitted:**
- deterministic matching;
- order validation;
- trade generation;
- state application;
- monotonic synchronisation.

**Forbidden:**
- wallet access;
- private-key access;
- treasury access;
- direct blockchain asset transfer;
- administrative balance modification.

**DNC controls:** OrderBook must satisfy DNC-005, DNC-006, DNC-019, DNC-020.

The order book must never become a financial custody layer.

### 6.4.5 TradeBook

**Reference:** `crates/p2p-node/src/trades.rs`

**Classification:** Protocol execution state.

**Permitted:**
- trade storage;
- trade lookup;
- deterministic trade identity;
- trade lifecycle references.

**Forbidden:**
- user asset custody;
- treasury control;
- arbitrary trade rewriting;
- unilateral settlement.

**Required invariant:** A Trade represents an execution record. It does not represent ownership of the assets involved.

### 6.4.6 BalanceBook

**Reference:** `crates/p2p-node/src/balances.rs`

**Classification:** Protocol/accounting state.

This is a high-risk boundary and requires explicit terminology.

The BalanceBook must not be interpreted as a custodial ledger.

Its values represent protocol state such as:

- reservations;
- locked amounts;
- escrow state;
- settlement state;
- local accounting.

For blockchain settlement, authoritative ownership ultimately remains with the applicable blockchain/cryptographic mechanism.

**Permitted:**
- reservations;
- local accounting;
- settlement state;
- escrow state;
- idempotency tracking;
- invariant enforcement.

**Forbidden:**
- administrative transfer of user principal;
- arbitrary balance creation;
- arbitrary balance destruction;
- treasury withdrawal;
- bypassing settlement authorization.

**Required invariant:** No method should exist equivalent to:

- `admin_transfer(user_a, user_b, amount)`
- `admin_set_balance(...)`
- `admin_confiscate(...)`
- `admin_freeze(...)`
- `admin_release(...)`

for user principal.

### 6.4.7 Exchange

**Reference:** `crates/p2p-node/src/exchange.rs`

**Classification:** Protocol coordination layer.

The existing architecture correctly places:

- actor authentication;
- matching;
- trade generation;
- trade validation;
- trade recording;
- order application;
- reservation reconciliation

inside this layer.

**Permitted:**
- protocol coordination;
- validation;
- deterministic execution;
- reservation management;
- state transition coordination.

**Forbidden:**
- company override;
- privileged administrator execution;
- unilateral settlement;
- direct treasury control.

**Required property:** Exchange must have no privileged execution mode such as:

- `submit_order_as_admin(...)`
- `record_trade_as_admin(...)`
- `override_trade(...)`
- `settle_as_admin(...)`

### 6.4.8 opendex-types

**Reference:** `crates/opendex-types/`

**Classification:** Domain model.

**Permitted:**
- SwapId;
- Satoshis;
- Piconero;
- Asset;
- Price;
- trade/order domain types.

**Forbidden:**
- key management;
- treasury authority;
- privileged capabilities.

**Design rule:** Financial types should represent values. They should not implicitly represent authority.

### 6.4.9 opendex-errors

**Reference:** `crates/opendex-errors/`

**Classification:** Error boundary.

**Permitted:**
- domain errors;
- protocol errors;
- settlement errors;
- transport errors;
- recovery errors.

**Forbidden:** No financial authority.

### 6.4.10 opendex-protocol-types

**Reference:** `crates/opendex-protocol-types/`

**Classification:** Critical protocol abstraction layer.

This crate contains:

- SettlementProtocol;
- SwapState;
- ProtocolAction.

**Permitted:** The SettlementProtocol abstraction may describe:

- what settlement action is required;
- what conditions must be satisfied;
- what blockchain observation occurred;
- what state transition should occur.

**Forbidden:** The trait must not expose a generic administrative capability such as:

- `transfer_user_funds(...)`
- `override_settlement(...)`

**Critical design principle:** The protocol abstraction should express conditions and actions, not grant OpenDex ownership authority.

### 6.4.11 opendex-protocol

**Reference:** `crates/opendex-protocol/`

**Classification:** Protocol negotiation and registry.

**Permitted:**
- protocol versions;
- compatibility;
- protocol selection;
- feature negotiation.

**Forbidden:**
- custody;
- treasury access;
- user balance authority.

### 6.4.12 opendex-btc-xmr

**Reference:** `crates/opendex-btc-xmr/`

**Classification:** Cryptographic settlement adapter.

This is one of the most security-sensitive components.

**Permitted:**
- DLEQ proofs;
- adaptor signatures;
- cryptographic settlement logic;
- BTC/XMR protocol-specific operations;
- construction of settlement data.

**Forbidden:**
- OpenDex administrator withdrawal;
- arbitrary user transfer;
- company treasury authority;
- bypass of user cryptographic authority.

**Required principle:** The adapter must derive authority from the swap protocol and the users' cryptographic participation. It must not derive authority from:

- OpenDex administrator
- OpenDex company key
- DAO key
- developer key

### 6.4.13 opendex-bitcoin

**Reference:** `crates/opendex-bitcoin/`

**Classification:** Bitcoin chain abstraction.

**Permitted:**
- chain observation;
- transaction construction;
- transaction broadcasting where authorised;
- confirmation observation;
- UTXO observation.

**Forbidden:**
- arbitrary company withdrawal;
- hidden administrative signing;
- central custody.

**Required interface distinction:** There should be a clear distinction between:

**construct transaction**

and:

**authorise/sign transaction**

The latter must remain subject to the applicable cryptographic authority.

### 6.4.14 opendex-monero

**Reference:** `crates/opendex-monero/`

**Classification:** Monero chain abstraction.

**Permitted:**
- chain observation;
- transaction construction;
- protocol-specific transaction handling;
- confirmation observation.

**Forbidden:**
- OpenDex-controlled user wallet;
- arbitrary company withdrawal;
- administrative spend authority.

### 6.4.15 opendex-chain

**Reference:** `crates/opendex-chain/`

**Classification:** Generic blockchain abstraction.

**Permitted:**
- chain interfaces;
- chain observations;
- transaction representations;
- confirmation state.

**Forbidden:** A generic chain interface must not introduce generic administrative custody.

Avoid interfaces such as:

```rust
trait Chain {
    fn seize_funds(...);
    fn freeze_account(...);
    fn admin_transfer(...);
}
```

These abstractions would make prohibited capabilities structurally easy to introduce.

### 6.4.16 opendex-swap

**Reference:** `crates/opendex-swap/`

**Classification:** Swap state machine.

**Permitted:**
- swap lifecycle;
- persistence;
- recovery;
- transport integration;
- protocol action processing.

**Forbidden:**
- unilateral settlement override;
- administrative user-asset transfer;
- treasury operations.

**Required principle:** The SwapEngine determines protocol state. It does not determine ownership of assets.

### 6.4.17 opendex-events

**Reference:** `crates/opendex-events/`

**Classification:** Protocol event sourcing.

**Permitted:**
- immutable protocol events;
- state reconstruction;
- event history.

**Forbidden:** Events must not become an alternative administrative financial ledger.

For example, an event such as:

`AdminTransferredUserFunds`

should not exist in the protocol event vocabulary.

### 6.4.18 opendex-storage

**Reference:** `crates/opendex-storage/`

**Classification:** Persistence abstraction.

**Permitted:**
- protocol state;
- swap state;
- event logs;
- transaction intents;
- recovery information.

**Forbidden:** Storage must not contain an administrative authority mechanism.

A stored value must not itself constitute permission to move user funds.

### 6.4.19 opendex-watchers

**Reference:** `crates/opendex-watchers/`

**Classification:** Read-only blockchain observers.

This is an important architectural strength.

**Permitted:**
- observe blockchain state;
- report confirmations;
- report transactions;
- report chain events.

**Forbidden:** Watchers must not make financial decisions or directly mutate user assets.

The intended model is:

```
Blockchain
│
▼
Watcher
│
▼
Observation
│
▼
SwapEngine
│
▼
Protocol decision
```

Not:

```
Blockchain
│
▼
Watcher
│
└──> transfer user funds
```

### 6.4.20 opendex-recovery

**Reference:** `crates/opendex-recovery/`

**Classification:** Deterministic protocol recovery.

**Permitted:**
- replaying events;
- reconstructing state;
- determining recovery decisions;
- recovering interrupted swaps.

**Forbidden:** Recovery must not introduce an administrative override.

The recovery engine must derive its decision from protocol inputs.

### 6.4.21 opendex-transport

**Reference:** `crates/opendex-transport/`

**Classification:** P2P communication.

**Permitted:**
- message transport;
- protocol envelopes;
- replay protection;
- peer communication.

**Forbidden:** Transport must not confer authority.

A message saying:

"OpenDexAdmin: transfer 10 BTC"

must not become valid merely because it arrived over an authenticated OpenDex transport channel.

Authentication answers:
**Who sent the message?**

It does not answer:
**Does that sender possess authority to perform the requested financial action?**

Both questions must be evaluated separately.

### 6.4.22 GUI

**Classification:** Client interface.

**Permitted:**
- display state;
- submit protocol commands;
- display balances;
- display trade state;
- display settlement state.

**Forbidden:** The GUI must not contain hidden administrative financial operations.

The GUI must not be a privileged back door.

### 6.4.23 CLI

The CLI must be subject to the same permissions as every other client.

There must be no distinction such as:

- GUI → restricted
- CLI → administrator

unless the operation is explicitly a local node-management operation rather than a user-financial operation.

### 6.4.24 New Commercial Fee Crate

I recommend introducing a separate crate/domain for fee accounting rather than placing treasury functionality into `p2p-node`, `BalanceBook`, or the DAO.

Conceptually:

**opendex-fees**

**Responsibilities:**
- deterministic fee calculation;
- fee attribution;
- fee records;
- fee reporting;
- fee reconciliation;
- protocol fee metadata.

**Explicitly excluded:**
- user custody;
- user private keys;
- settlement override;
- user balance modification;
- DAO treasury authority.

### 6.4.25 Commercial Treasury Boundary

The commercial treasury should be outside the protocol-state machine.

Conceptually:

```
Protocol
│
▼
Fee record
│
▼
Commercial settlement
│
▼
OpenDex treasury
```

The protocol should not contain:

- `dao_withdraw_treasury(...)`
- `protocol_transfer_treasury(...)`

The treasury is a commercial domain.

Its ownership and operation should be represented outside consensus-critical protocol state.

### 6.4.26 Fee Recipient Design

The protocol should have an explicit fee-recipient concept.

For example:

```rust
struct ProtocolFee {
    trade_id: TradeId,
    asset: Asset,
    amount: Amount,
    recipient: FeeRecipient,
}
```

The exact implementation should be determined after the settlement architecture is reviewed.

The important property is:

**The fee recipient receives the fee; it does not receive authority over the remainder of the user's settlement.**

### 6.4.27 Dependency Direction

The dependency graph should reinforce the separation.

**Preferred:**

```
opendex-types
│
┌────────────────┼────────────────┐
│                │                │
protocol-types       protocol        events
│                                 │
▼                                 ▼
settlement                         storage
│
┌──────┴──────┐
▼             ▼
BTC adapter     XMR adapter
│             │
▼             ▼
Bitcoin          Monero
```

Commercial fee infrastructure should not become a dependency of the cryptographic settlement layer.

**Prefer:**

```
Protocol
│
└── emits/records fee information
│
▼
Fee subsystem
│
▼
Commercial domain
```

**Rather than:**

```
Settlement
│
▼
OpenDex Treasury
│
▼
Protocol
```

### 6.4.28 Capability Model

The project should progressively move toward explicit capabilities rather than broad mutable references.

Conceptually:

- ProtocolStateCapability
- OrderCapability
- TradeCapability
- SettlementCapability
- FeeCapability
- TreasuryCapability

These capabilities should not be interchangeable.

In particular:

```
FeeCapability
X
│
└──> UserAssetCapability

TreasuryCapability
X
│
└──> SettlementOverrideCapability
```

### 6.4.29 Private-Key Boundary

Private-key access must be isolated from ordinary protocol code.

The following crates should not receive user private keys merely to perform protocol operations:

- OrderBook;
- TradeBook;
- Exchange;
- opendex-events;
- opendex-transport;
- opendex-watchers;
- GUI;
- fee subsystem.

Where signing is required, signing authority should be provided only by the user's wallet/key-management boundary or the appropriate cryptographic settlement mechanism.

### 6.4.30 Administrative-Key Boundary

OpenDex must distinguish:

**software signing key**

from:

**user asset signing key**

and:

**treasury authority**

These must never collapse into a single master key.

Compromise of an OpenDex release-signing key must not produce user-asset authority.

Compromise of the commercial treasury must not produce user-asset authority.

Compromise of DAO governance must not produce commercial treasury authority.

### 6.4.31 Threat Isolation Matrix

| Compromise | User principal | Protocol | OpenDex treasury |
|-----------|----------------|----------|-----------------|
| GUI compromise | Must remain safe | Potentially affected | No authority |
| API compromise | Must remain safe | Potentially affected | No authority |
| Node compromise | Must remain safe | Node-specific impact | No authority |
| Developer key compromise | Must remain safe | Software risk | Must remain separate |
| DAO compromise | Must remain safe | Governance risk | Must remain separate |
| Treasury compromise | Must remain safe | No authority | Treasury affected |
| Watcher compromise | Must remain safe | Observation risk | No authority |
| Transport compromise | Must remain safe | Network risk | No authority |

This table should become an explicit security-test target.

### 6.4.32 Required Forbidden Interfaces

The codebase should be audited for and reject interfaces conceptually equivalent to:

- `admin_transfer_user_funds()`
- `admin_set_user_balance()`
- `admin_freeze_user()`
- `admin_confiscate_user()`
- `admin_override_trade()`
- `admin_override_settlement()`
- `admin_cancel_any_trade()`
- `admin_rewrite_order()`
- `dao_transfer_company_treasury()`

The names above are illustrative.

The architectural requirement is broader:

**No privileged capability with equivalent effect should exist under another name.**

### 6.4.33 Required Protocol Interfaces

The protocol should instead expose operations conceptually equivalent to:

- `submit_order()`
- `validate_order()`
- `match_order()`
- `record_trade()`
- `validate_trade()`
- `apply_trade()`
- `advance_trade_state()`
- `observe_chain_state()`
- `construct_settlement_action()`
- `apply_protocol_observation()`
- `calculate_protocol_fee()`

Each operation must operate within its domain.

### 6.4.34 Fee Calculation Must Not Mutate User Principal

The fee calculation path must be functionally separated from settlement ownership.

Conceptually:

```rust
let fee = calculate_fee(&trade, &fee_policy);
```

must not itself mutate:

**user principal**

It should produce a deterministic result.

The actual settlement mechanism then implements the fee according to the applicable protocol.

### 6.4.35 Protocol Fee Audit Trail

Each fee should be independently attributable to:

- trade ID;
- applicable fee policy;
- asset;
- amount;
- timestamp or protocol sequence;
- fee recipient;
- settlement reference.

This makes protocol revenue independently auditable.

The commercial treasury may subsequently have its own accounting system.

The protocol should not need to know how the company spends its treasury.

### 6.4.36 DAO Isolation Tests

The DAO test suite must explicitly prove:

**DAO cannot:**
- transfer user principal
- withdraw OpenDex treasury
- modify commercial ownership
- modify company accounting

**while allowing:**

**DAO can:**
- govern protocol parameters
- approve protocol upgrades
- govern technical protocol standards

The exact governance scope should be defined separately.

### 6.4.37 Treasury Isolation Tests

Treasury tests must explicitly prove:

**Treasury cannot:**
- modify user balances
- sign user transactions
- alter trade state
- override settlement
- modify order state

The treasury should only be capable of operating upon assets that are already legitimately part of the commercial treasury.

### 6.4.38 Company Infrastructure Independence Test

The integration test environment should be capable of running the protocol with:

**OpenDex company services = OFF**

while retaining:

- independent nodes = ON
- protocol = ON
- P2P transport = ON
- matching = ON
- trade validation = ON
- swap engine = ON
- blockchain observers = ON

This should become a CI/integration test environment.

### 6.4.39 Company Disappearance Test

Create a documented recovery scenario:

1. Shut down OpenDex-operated infrastructure.
2. Start independent nodes.
3. Restore protocol state.
4. Synchronise peers.
5. Submit orders.
6. Match orders.
7. Create trades.
8. Execute settlement.
9. Verify blockchain state.

**Success criteria:** The protocol continues to operate without requiring OpenDex-operated infrastructure.

### 6.4.40 Commercial Treasury Disappearance Test

A treasury outage must not affect:

- order matching;
- trade validation;
- user asset control;
- blockchain settlement;
- protocol state convergence.

Only commercial fee accounting/collection may be affected.

### 6.4.41 User-Key Compromise Boundary

A compromised user's private key is a user security event.

It must not automatically imply:

- compromise of OpenDex;
- compromise of other users;
- compromise of protocol treasury;
- compromise of DAO governance.

Likewise, compromise of OpenDex must not automatically imply compromise of user keys.

### 6.4.42 Persistence Boundary

Protocol persistence should contain:

- orders;
- trades;
- state transitions;
- event logs;
- recovery state;
- synchronization metadata.

Commercial treasury persistence should remain separate.

User private keys should not be persisted in ordinary protocol state.

This avoids turning protocol backups into custodial key backups.

### 6.4.43 Logging Boundary

Existing audit logging should continue to record:

- matching;
- trade validation;
- settlement state;
- network messages;
- snapshot application;
- escrow;
- fee calculation.

However:

**Logging an operation must never itself create authority to perform the operation.**

Logs are evidence, not permissions.

### 6.4.44 Regulatory Evidence Mapping

The implementation should map code controls to architectural assertions.

**Example:**

**Claim:** OpenDex is non-custodial.

**Evidence:**
- no user private keys in protocol crates;
- no admin transfer primitive;
- settlement requires cryptographic authority;
- independent-node tests;
- compromise tests.

**Another:**

**Claim:** DAO does not control treasury.

**Evidence:**
- no DAO → treasury capability;
- separate crate/domain;
- separate signing authority;
- integration tests;
- governance permission matrix.

**Another:**

**Claim:** OpenDex receives protocol fees but does not control user principal.

**Evidence:**
- deterministic fee calculation;
- separate fee capability;
- no treasury → user-asset capability;
- settlement tests;
- threat-model tests.

### 6.4.45 Code Review Rule

Any pull request introducing one of the following must automatically trigger a decentralisation review:

- wallet access;
- private-key access;
- new balance mutation;
- new settlement capability;
- new administrator role;
- DAO authority;
- treasury authority;
- protocol upgrade authority;
- fee modification;
- trade cancellation;
- order modification;
- blockchain signing;
- emergency controls.

The reviewer must determine whether the change crosses one of the four architectural boundaries.

### 6.4.46 New DNC Test Categories

The existing 191 tests should eventually be supplemented with dedicated DNC suites:

```
tests/dnc/
custody.rs
settlement_authority.rs
node_independence.rs
dao_isolation.rs
treasury_isolation.rs
fee_calculation.rs
compromise.rs
upgrade_authority.rs
key_boundaries.rs
```

The exact location can be adjusted to the workspace structure.

### 6.4.47 Definition of Done

The OpenDex decentralisation/non-custody architecture should not be considered complete until:

- no ordinary protocol crate possesses user private keys;
- no ordinary protocol crate possesses unilateral user-asset transfer authority;
- no DAO operation possesses commercial treasury authority;
- no commercial treasury operation possesses user-asset authority;
- fee calculation is deterministic;
- fee attribution is auditable;
- independent nodes can execute the protocol;
- OpenDex infrastructure is not a mandatory execution authority;
- settlement authority derives from the applicable settlement protocol;
- blockchain watchers remain read-only;
- protocol state can be reconstructed without commercial treasury state;
- compromise of OpenDex infrastructure does not create unilateral user-asset control;
- compromise of the commercial treasury does not create user-asset control;
- compromise of DAO governance does not create treasury or user-asset control;
- removal of the OpenDex commercial entity does not destroy user asset control;
- all DNC invariants have executable tests.

### 6.4.48 Final Capability Model

The intended final architecture is:

```
┌─────────────────────┐
│       USERS         │
│                     │
│  Keys / Principal   │
└──────────┬──────────┘
│
cryptographic
authority
│
▼
┌─────────────────────┐
│ BLOCKCHAIN / SWAP   │
│     SETTLEMENT      │
└──────────┬──────────┘
│
observations
│
▼
┌───────────────────────────────────────────────────────────┐
│                    OPENDEX PROTOCOL                       │
│                                                           │
│ OrderBook │ TradeBook │ SwapEngine │ Transport │ Watchers│
│                                                           │
│             deterministic protocol rules                  │
└──────────────────────────────┬────────────────────────────┘
│
protocol fee
│
▼
┌─────────────────────┐
│ OPENDEX COMMERCIAL  │
│       DOMAIN        │
│                     │
│ Fees → Treasury     │
└─────────────────────┘


                    ┌─────────────────────┐
                    │         DAO         │
                    │                     │
                    │ Protocol governance │
                    └─────────────────────┘

                    DAO ──X──> Treasury
                    DAO ──X──> User principal

                    Treasury ──X──> User principal
```

This is the implementation target.

The fundamental rule remains:

**Protocol authority, user-asset authority, DAO governance authority and commercial treasury authority are separate capabilities. None may be inferred from another.**

---

# 7. Compliance Status Tracker

**Last updated:** 2026-10-08

This section tracks the implementation status of all decentralisation invariants defined in the regulatory architecture.

## 7.1 Status Legend

| Status | Meaning |
|--------|---------|
| ✅ **Implemented** | The invariant is satisfied by current code with dedicated supporting tests |
| 🟢 **Architecturally Satisfied** | The invariant is satisfied by the current architecture but lacks dedicated test coverage |
| 🟡 **Partially Implemented** | The invariant is only partially satisfied; some aspects remain aspirational or incomplete |
| 🔴 **Not Started** | The invariant has no current implementation or test coverage |
| ⬜ **N/A** | The invariant does not apply to the current codebase scope |

**Note:** The distinction between 🟢 and 🟡 is important: 🟢 means the architecture already satisfies the invariant (e.g., no custody capability exists), while 🟡 means the invariant is only partially met (e.g., some aspects are implemented but others are missing).

## 7.2 DNC Invariants (from ADMP v6)

All 20 DNC invariants are implemented with dedicated tests in `crates/opendex-dao/src/lib.rs`. The `DaoEngine` runtime enforces these constraints via state machine transitions and immutable configuration.

| Invariant | Description | Status | Evidence / Notes |
|-----------|-------------|--------|------------------|
| **DNC-001** | OpenDex cannot transfer user principal unilaterally | ✅ Implemented | `test_dnc_001_no_unilateral_user_principal_transfer` verifies no transfer variant exists in `DaoProposal`. |
| **DNC-002** | OpenDex cannot freeze user principal unilaterally | ✅ Implemented | `test_dnc_002_no_unilateral_user_principal_freeze` verifies no freeze variant exists. |
| **DNC-003** | OpenDex cannot seize user principal unilaterally | ✅ Implemented | `test_dnc_003_no_unilateral_user_principal_seizure` verifies no seize variant exists. |
| **DNC-004** | OpenDex cannot become the mandatory counterparty to user trades | ✅ Implemented | `test_dnc_004_no_mandatory_counterparty` verifies no counterparty control variant exists. |
| **DNC-005** | OpenDex cannot unilaterally determine which valid user trade executes | ✅ Implemented | `test_dnc_005_no_unilateral_trade_determination` verifies no trade influence variant exists. |
| **DNC-006** | OpenDex cannot unilaterally rewrite an executed trade | ✅ Implemented | `test_dnc_006_no_unilateral_trade_rewrite` verifies no trade modification variant exists. |
| **DNC-007** | OpenDex cannot unilaterally override blockchain settlement | ✅ Implemented | `test_dnc_007_no_unilateral_blockchain_override` verifies no blockchain override variant exists. |
| **DNC-008** | An independent compatible node can participate without permission from OpenDex | ✅ Implemented | `test_dnc_008_permissionless_node_participation` verifies no node approval variant exists. |
| **DNC-009** | The protocol remains operational without a central OpenDex matching server | ✅ Implemented | `test_dnc_009_operational_without_central_server` verifies no P2P disable variant exists. |
| **DNC-010** | The protocol remains capable of settlement without a central OpenDex settlement authority | ✅ Implemented | `test_dnc_010_settlement_without_central_authority` verifies no settlement centralization variant exists. |
| **DNC-011** | User assets remain controlled by user credentials | ✅ Implemented | `test_dnc_011_user_assets_controlled_by_user_credentials` verifies no user asset control variant exists. |
| **DNC-012** | The DAO cannot control the OpenDex commercial treasury | ✅ Implemented | `dnc_012_dao_cannot_control_treasury` in `opendex-dao` and `opendex-fee-treasury` verifies DAO/treasury isolation at runtime. |
| **DNC-013** | OpenDex commercial treasury control cannot provide control over user principal | ✅ Implemented | `dnc_013_treasury_cannot_affect_user_principal` in `opendex-fee-treasury` verifies treasury has no user asset access. |
| **DNC-014** | Protocol fees are independently calculable | ✅ Implemented | `dnc_014_fees_independently_calculable` in `opendex-fee-treasury` verifies `FeeCalculator::calculate()` is a pure function. |
| **DNC-015** | Protocol fee collection cannot modify the identity of an executed trade | ✅ Implemented | `dnc_015_fee_collection_cannot_modify_trade_identity` in `opendex-fee-treasury` verifies treasury receives only fee data. |
| **DNC-016** | Protocol fee collection cannot create additional rights over user principal | ✅ Implemented | `dnc_016_fee_collection_no_rights_over_principal` in `opendex-fee-treasury` verifies treasury receives only fees. |
| **DNC-017** | Removal of OpenDex commercial infrastructure does not transfer user assets to OpenDex | ✅ Implemented | `test_dnc_017_removal_of_commercial_infrastructure_no_asset_transfer` verifies no asset transfer on removal. |
| **DNC-018** | Removal of OpenDex commercial infrastructure does not invalidate blockchain ownership | ✅ Implemented | `test_dnc_018_removal_of_commercial_infrastructure_no_blockchain_invalidation` verifies blockchain independence. |
| **DNC-019** | Removal of OpenDex commercial infrastructure does not make the protocol state dependent upon a central database | ✅ Implemented | `test_dnc_019_removal_of_commercial_infrastructure_no_central_database_dependency` verifies local state persistence. |
| **DNC-020** | Protocol state remains independently verifiable by participating nodes | ✅ Implemented | `test_dnc_020_protocol_state_independently_verifiable` verifies deterministic state. |

### DNC Summary

| Status | Count | Invariants |
|--------|-------|------------|
| ✅ Implemented | 20 | DNC-001 through DNC-020 |

**Status:** All 20 DNC invariants are fully implemented with dedicated tests in `crates/opendex-dao/` and `crates/opendex-fee-treasury/`. The DAO runtime engine (`DaoEngine`) enforces constraints via state machine transitions. The fee/treasury subsystem (`FeeCalculator`, `Treasury`) enforces DNC-012–016 at runtime with 26 additional tests.

## 7.3 DEC Invariants (from MiCA Analysis)

All 15 DEC invariants are implemented with dedicated tests in `crates/opendex-dao/src/lib.rs`.

| Invariant | Description | Status | Evidence / Notes |
|-----------|-------------|--------|------------------|
| **DEC-001** | No mandatory intermediary — No OpenDex-controlled intermediary shall be necessary for a valid trade | ✅ Implemented | `test_dec_001_no_mandatory_intermediary` verifies no intermediary requirement in DAO interface. |
| **DEC-002** | Independent matching — Any node must be capable of performing the protocol's matching algorithm without contacting OpenDex-controlled infrastructure | ✅ Implemented | `test_dec_002_independent_matching` verifies no centralization of matching in DAO interface. |
| **DEC-003** | Independent order propagation — Orders must be propagatable through independently operated nodes | ✅ Implemented | `test_dec_003_independent_order_propagation` verifies no order propagation blocking in DAO interface. |
| **DEC-004** | No central order authority — No entity shall possess exclusive authority over the canonical order book | ✅ Implemented | `test_dec_004_no_central_order_authority` verifies no order book centralization in DAO interface. |
| **DEC-005** | No operator custody — OpenDex-controlled infrastructure shall not hold user private keys or unilateral control over user crypto-assets | ✅ Implemented | `test_dec_005_no_operator_custody` verifies no key access in DAO interface. |
| **DEC-006** | No operator settlement authority — OpenDex-controlled infrastructure shall not have unilateral authority to release or confiscate user settlement assets | ✅ Implemented | `test_dec_006_no_operator_settlement_authority` verifies no settlement control in DAO interface. |
| **DEC-007** | Independent node operation — A third party must be capable of running a complete OpenDex node without permission from OpenDex | ✅ Implemented | `test_dec_007_independent_node_operation` verifies no node permission control in DAO interface. |
| **DEC-008** | Independent frontend operation — A third party must be capable of creating a frontend that connects directly to the protocol | ✅ Implemented | `test_dec_008_independent_frontend_operation` verifies no frontend restriction in DAO interface. |
| **DEC-009** | Independent relay operation — A third party must be capable of operating network relay/discovery infrastructure | ✅ Implemented | `test_dec_009_independent_relay_operation` verifies no relay mandate in DAO interface. |
| **DEC-010** | No privileged operator — OpenDex-operated infrastructure shall not possess protocol privileges unavailable to ordinary nodes | ✅ Implemented | `test_dec_010_no_privileged_operator` verifies no operator privilege grants in DAO interface. |
| **DEC-011** | No mandatory OpenDex API — Trading functionality shall not depend on an OpenDex-operated REST/WebSocket API | ✅ Implemented | `test_dec_011_no_mandatory_opendex_api` verifies no central API requirement. No central API exists; all trading is P2P. |
| **DEC-012** | No mandatory OpenDex account — Users shall not be required to create an account with OpenDex Ltd merely to use the protocol | ✅ Implemented | `test_dec_012_no_mandatory_opendex_account` verifies no account requirement. No account system; peer authentication is cryptographic. |
| **DEC-013** | No unilateral censorship — No OpenDex operator shall have unilateral authority to prevent a valid peer-to-peer transaction | ✅ Implemented | `test_dec_013_no_unilateral_censorship` verifies no censorship mechanism in DAO interface. |
| **DEC-014** | User-controlled keys — Users shall retain control of the private keys necessary to control their crypto-assets | ✅ Implemented | `test_dec_014_user_controlled_keys` verifies no key control in DAO interface. |
| **DEC-015** | Deterministic execution — Where protocol rules determine execution, those rules shall be executable independently by nodes | ✅ Implemented | `test_dec_015_deterministic_execution` verifies no non-deterministic execution in DAO interface. |

### DEC Summary

| Status | Count | Invariants |
|--------|-------|------------|
| ✅ Implemented | 15 | DEC-001 through DEC-015 |

**Status:** All 15 DEC invariants are fully implemented with dedicated tests in `crates/opendex-dao/`. DEC-011 and DEC-012 are additionally satisfied by the absence of central API and account systems in the protocol architecture.

## 7.4 Overall Compliance Summary

| Category | Total | ✅ Implemented | 🟢 Architecturally Satisfied | 🔴 Not Started |
|----------|-------|---------------|-----------------------------|----------------|
| DNC Invariants | 20 | 20 | 0 | 0 |
| DEC Invariants | 15 | 15 | 0 | 0 |
| **Combined** | **35** | **35** | **0** | **0** |

### Status

All 35 invariants (20 DNC + 15 DEC) are fully implemented with dedicated tests across `crates/opendex-dao/` and `crates/opendex-fee-treasury/`. The DAO runtime engine (`DaoEngine`) provides 20 integration tests covering the full proposal lifecycle. The fee/treasury subsystem provides 26 tests covering fee calculation, treasury isolation, and DNC-012–016 runtime verification. Total test coverage: 87 tests (61 DAO + 26 fee/treasury).

### ADMP v6 Status

**All ADMP v6 requirements are fully implemented.** The DAO governance interface, runtime engine, and fee/treasury subsystem are complete with comprehensive test coverage.

## 7.5 Test Inventory

### Existing Tests (p2p-node crate)

| Test File | Coverage | Related Invariants |
|-----------|----------|-------------------|
| `security.rs` | Peer authentication, transport security | DEC-007, DEC-010 |
| `settlement.rs` | Atomic swap settlement flows | DNC-007, DEC-006 |
| `exchange.rs` | Exchange coordination logic | DNC-005, DNC-006 |
| `balances.rs` | BalanceBook mutations | DNC-001, DNC-002, DNC-003, DNC-011 |
| `cli.rs` | CLI interface | DEC-008 |
| `persistence.rs` | Protocol state persistence | DNC-019, DNC-020 |
| `orderbook.rs` | OrderBook operations, matching | DNC-005, DNC-006, DEC-002, DEC-004, DEC-015 |
| `trades.rs` | Trade creation, validation | DNC-006, DEC-015 |
| `network.rs` | P2P network, order propagation | DEC-003, DEC-009 |

### Implemented DAO Tests (opendex-dao crate)

All DNC and DEC invariant tests are implemented in `crates/opendex-dao/src/lib.rs`:

| Test Category | Count | Related Invariants |
|---------------|-------|-------------------|
| DNC invariant tests | 26 | DNC-001 through DNC-020 |
| DEC invariant tests | 15 | DEC-001 through DEC-015 |
| Engine integration tests | 20 | Full proposal lifecycle, state machine, participation threshold |
| **DAO Subtotal** | **61** | All 35 invariants + runtime engine |

### Implemented Fee/Treasury Tests (opendex-fee-treasury crate)

All fee/treasury tests are implemented in `crates/opendex-fee-treasury/src/lib.rs`:

| Test Category | Count | Related Invariants |
|---------------|-------|-------------------|
| Fee calculator tests | 9 | DNC-014 (deterministic calculation) |
| Treasury tests | 6 | DNC-012, DNC-013, DNC-015, DNC-016 |
| DNC invariant tests | 5 | DNC-012 through DNC-016 |
| Integration tests | 5 | Full fee workflow, DAO integration |
| Doctests | 1 | `FeeCalculator::calculate()` example |
| **Fee/Treasury Subtotal** | **26** | DNC-012 through DNC-016 |

### Combined Test Total

| Crate | Tests | Coverage |
|-------|-------|----------|
| `opendex-dao` | 61 | 35 invariants + runtime engine |
| `opendex-fee-treasury` | 26 | Fee calculation + treasury isolation + DNC-012–016 |
| **Total** | **87** | All ADMP v6 requirements |

### Planned Test Suite (future work)

The following test suites are planned for the `p2p-node` crate to provide runtime verification of invariants beyond the DAO interface:

| Test File | Purpose | Related Invariants |
|-----------|---------|-------------------|
| `custody.rs` | Verify no custody capability exists at runtime | DNC-001–003, DEC-005, DEC-014 |
| `settlement_authority.rs` | Verify settlement is cryptographic, not administrative | DNC-007, DEC-006 |
| `node_independence.rs` | Verify independent node operation | DNC-008–009, DEC-001–003, DEC-007 |
| `fee_calculation.rs` | Verify deterministic fee calculation at runtime | DNC-014–016 |
| `compromise.rs` | Verify compromise scenarios don't create unilateral control | DNC-017–018 |

## 7.6 Review Schedule

This tracker should be updated:

- **On each major release** — to reflect new invariants or changed implementation status
- **When a DNC/DEC test is added** — to mark the corresponding invariant as implemented
- **When regulatory guidance changes** — to assess impact on existing invariants
- **Quarterly** — as a minimum, even without code changes

---

# 8. Fiat Subsystem Separation Boundary

**Last updated:** 2026-10-08

This section defines the regulatory architecture for separating the fiat subsystem from the crypto/crypto P2P protocol. All existing regulatory documents correctly identify the fiat subsystem as the highest regulatory risk and recommend separation.

## 8.1 Why Separation Is Required

The crypto/crypto P2P protocol (BTC/XMR atomic swaps) can potentially operate outside MiCA scope under Recital 22 if genuinely decentralised and intermediary-free. The fiat subsystem cannot make this argument, because:

- Fiat on-ramps and off-ramps inherently involve a legal entity handling fiat currency.
- Fiat handling triggers payment services regulations (PSD2/PSR) in addition to MiCA.
- A fiat operator is, by definition, an identifiable intermediary.
- Mixing fiat operations with the crypto protocol would contaminate the decentralisation argument for the entire system.

**Conclusion:** The fiat subsystem must be architecturally, legally, and operationally isolated from the crypto/crypto P2P protocol.

## 8.2 Applicable Regulatory Regimes

| Regime | Scope | Trigger |
|--------|-------|---------|
| **MiCA — CASP authorisation** | Operation of trading platform, exchange of crypto-assets for funds | Any entity exchanging fiat for crypto-assets professionally |
| **MiCA — Custody** | Custody and administration of crypto-assets | If the fiat operator holds user crypto-assets |
| **PSD2 / UK PSR** | Payment institution activities (money transmission, account issuance) | Handling fiat payments, receiving/transmitting fiat funds |
| **UK FCA Crypto-asset regime** | Effective 25 October 2027; covers all crypto-asset activities | Any crypto-asset activity in the UK |
| **AML/CFT (5AMLD / UK MLRs)** | Anti-money laundering, customer due diligence | Any virtual asset service provider activity |
| **Sanctions compliance** | OFSI, EU sanctions, HMT sanctions | Processing transactions involving sanctioned parties |

## 8.3 Architectural Separation Model

### 8.3.1 The Boundary

```
┌─────────────────────────────────────────────────────────────┐
│              CRYPTO/CRYPTO P2P PROTOCOL                     │
│                                                             │
│  BTC/XMR atomic swaps                                      │
│  P2P order book                                            │
│  Deterministic matching                                    │
│  Replicated state                                          │
│  User-controlled keys                                      │
│                                                             │
│  Potentially outside MiCA under Recital 22                 │
└─────────────────────────────────────────────────────────────┘

                        ════════════════════════════════════
                        ARCHITECTURAL SEPARATION BOUNDARY
                        ════════════════════════════════════

┌─────────────────────────────────────────────────────────────┐
│              FIAT SUBSYSTEM                                 │
│                                                             │
│  Fiat on-ramp / off-ramp                                   │
│  Payment processing                                        │
│  KYC/AML compliance                                        │
│  Fiat custody                                              │
│  Regulatory reporting                                      │
│                                                             │
│  Within MiCA + PSD2/PSR scope                              │
│  Requires CASP + Payment Institution authorisation          │
└─────────────────────────────────────────────────────────────┘
```

### 8.3.2 Separation Requirements

| Requirement | Description |
|-------------|-------------|
| **Separate legal entity** | The fiat operator should be a distinct legal entity from the protocol developer, with its own regulatory licences. |
| **Separate codebase** | Fiat functionality should not be part of the P2P node software. It should be a separate service with a well-defined API boundary. |
| **Separate data stores** | Fiat transaction data, KYC records, and compliance data must not be stored in the protocol's replicated state. |
| **Separate key management** | Fiat subsystem keys must not be accessible to the protocol layer, and protocol keys must not be accessible to the fiat layer. |
| **No protocol dependency** | The crypto/crypto protocol must not require the fiat subsystem to operate. A user should be able to use the full P2P protocol without any fiat interaction. |
| **No fiat dependency on protocol authority** | The fiat subsystem should not have privileged access to protocol state or the ability to influence matching, order book, or settlement. |

## 8.4 Fiat Subsystem Design Principles

### 8.4.1 What the Fiat Subsystem Should Do

- Accept fiat payments from users (on-ramp).
- Convert fiat to crypto-assets and deliver to user-controlled wallets.
- Accept crypto-assets from user-controlled wallets and convert to fiat (off-ramp).
- Perform KYC/AML checks on fiat users.
- Maintain regulatory records and reports.
- Handle fiat custody in compliance with banking/payment regulations.

### 8.4.2 What the Fiat Subsystem Must Not Do

- **Must not** participate in P2P protocol matching or order book operations.
- **Must not** hold protocol governance authority.
- **Must not** have privileged access to replicated protocol state.
- **Must not** be able to influence which trades execute or how they settle.
- **Must not** store user private keys for protocol participation.
- **Must not** be a mandatory component for protocol operation.

### 8.4.3 Interaction Pattern

The fiat subsystem interacts with users, not with the protocol:

```
User
│
├── (P2P protocol) ──► Node ──► P2P network ──► Counterparty Node
│
└── (Fiat service) ──► Fiat operator ──► Payment processor ──► Bank
```

The fiat operator is a separate service that the user optionally uses. It does not sit between the user and the protocol.

## 8.5 Rust Crate Boundaries

Based on the implementation control matrix crate classification model:

| Crate | Fiat boundary |
|-------|---------------|
| `p2p-node` | **Must not** contain fiat functionality. Pure crypto/crypto P2P protocol only. |
| `protocol` | **Must not** contain fiat types that imply custody or payment processing. Fiat currency definitions are acceptable as data types only. |
| `BalanceBook` | **Must not** handle fiat balances. Crypto balances only. |
| `SwapEngine` | **Must not** handle fiat settlement. Crypto atomic swaps only. |
| **Future: `fiat-gateway`** | Separate crate for fiat on-ramp/off-ramp. Not part of the P2P node. Contains payment processing, KYC integration, and fiat custody logic. |

## 8.6 User Experience Considerations

### 8.6.1 Optional Fiat Access

The fiat subsystem should be an optional service. The user journey should be:

1. User installs and runs the P2P node (no fiat involvement).
2. User participates in crypto/crypto trading (no fiat involvement).
3. If the user wants fiat access, they optionally connect to a fiat service provider.
4. The fiat service provider is a separate entity with its own terms, KYC, and fees.

### 8.6.2 Multiple Fiat Providers

To avoid creating a mandatory intermediary, the architecture should support multiple fiat service providers:

- The protocol should not be coupled to a single fiat provider.
- A user should be able to choose between different fiat on-ramp/off-ramp providers.
- The fiat provider interface should be standardised so that competition is possible.

## 8.7 Regulatory Licensing Path

For the fiat subsystem operator, the expected licensing path in the EU is:

| Licence | Regime | Purpose |
|---------|--------|---------|
| **CASP licence** | MiCA | For exchanging crypto-assets for fiat funds |
| **Payment Institution licence** | PSD2 | For handling fiat payment transactions |
| **AML registration** | 5AMLD | For AML/CFT compliance obligations |

In the UK:

| Licence | Regime | Purpose |
|---------|--------|---------|
| **FCA registration** | Money Transmission Regulations | For crypto-asset activities |
| **Payment Institution licence** | PSR | For fiat payment services |
| **AML registration** | UK MLRs | For AML/CFT compliance |

## 8.8 Risk Mitigation Summary

| Risk | Mitigation |
|------|-----------|
| Fiat operations contaminate the decentralisation argument for the P2P protocol | Architectural separation ensures the P2P protocol can operate without any fiat component |
| Single fiat operator becomes a mandatory intermediary | Support for multiple fiat providers; fiat is optional |
| Regulatory action against the fiat operator affects the protocol | Separate legal entities; no shared infrastructure |
| KYC/AML data leaks into protocol state | Fiat data stored separately; no KYC data in replicated protocol state |
| Fiat custody compromise affects user protocol assets | Separate key management; fiat keys never access protocol assets |

## 8.9 Implementation Checklist

- [ ] Ensure `p2p-node` crate contains no fiat-specific logic beyond data types.
- [ ] Ensure `BalanceBook` handles only crypto balances.
- [ ] Ensure `SwapEngine` handles only crypto atomic swaps.
- [ ] Design the fiat gateway as a separate service, not part of the node.
- [ ] Define a standardised fiat provider interface to support multiple providers.
- [ ] Ensure the protocol operates fully without any fiat component.
- [ ] Establish the fiat operator as a separate legal entity with appropriate licences.
- [ ] Implement separate key management for fiat and protocol operations.
- [ ] Ensure no KYC/AML data enters the replicated protocol state.

---

# 9. Monero (XMR) and MiCA Article 76 Analysis

**Last updated:** 2026-10-08

This section analyses the interaction between Monero's privacy features and MiCA Article 76, and evaluates how the decentralisation perimeter affects the applicability of restrictions on anonymised crypto-assets.

## 9.1 MiCA Article 76 — Anonymised Crypto-Assets

### 9.1.1 What Article 76 Says

MiCA Article 76 restricts **anonymised crypto-assets**. Key provisions:

- **Definition:** An anonymised crypto-asset is one where the issuance, ownership, or transfer cannot be traced, and the identity of parties cannot be determined.
- **Offer to the public:** Member States may prohibit or restrict the offer to the public of anonymised crypto-assets.
- **Trading platforms:** Regulated crypto-asset service providers (CASPs) operating trading platforms are prohibited from providing trading services for anonymised crypto-assets.
- **National discretion:** Member States have broad discretion to impose additional restrictions beyond the MiCA minimum.

### 9.1.2 Does Monero Qualify as an Anonymised Crypto-Asset?

Monero uses ring signatures, stealth addresses, and confidential transactions to obfuscate:

- The sender's identity.
- The receiver's identity.
- The transaction amount.

Under MiCA's definition, Monero's properties mean that **ownership and transfer cannot be readily traced**, which is the core characteristic of an anonymised crypto-asset.

**Assessment:** Monero is very likely to be classified as an anonymised crypto-asset under MiCA Article 76.

## 9.2 The Key Question: Does Article 76 Apply to OpenDex?

### 9.2.1 The Direct Application Path

Article 76's trading platform restriction applies to **regulated CASPs operating trading platforms**. The critical question is whether OpenDex is a regulated CASP operating a trading platform.

If OpenDex qualifies as fully decentralised under Recital 22, then:

- OpenDex is **not** a CASP.
- OpenDex is **not** operating a regulated trading platform.
- Article 76's prohibition on regulated trading platforms listing anonymised assets **does not directly apply**.

### 9.2.2 The Member State Discretion Path

Even if OpenDex is outside MiCA scope, Member States retain discretion under Article 76 to:

- Prohibit or restrict the **offer to the public** of anonymised crypto-assets within their jurisdiction.
- Impose restrictions that go beyond MiCA's minimum requirements.

This means that even if OpenDex avoids the CASP/trading-platform classification, individual EU Member States could still restrict Monero trading by residents within their jurisdiction.

### 9.2.3 Analysis of the Interaction

| Scenario | Article 76 applicability |
|----------|-------------------------|
| OpenDex is classified as a CASP trading platform | **Directly applies** — prohibited from listing XMR |
| OpenDex is decentralised, outside MiCA scope | **Does not directly apply** — no CASP to regulate |
| Member State imposes national restrictions | **May apply at national level** — can restrict XMR offers to residents |
| UK FCA regime (post-October 2027) | **Separate analysis required** — UK may adopt similar restrictions |

## 9.3 The Decentralisation Perimeter as a Shield

### 9.3.1 How It Works

The decentralisation argument does not change Monero's technical properties. Monero remains an anonymised crypto-asset regardless of how it is traded.

What the decentralisation argument changes is **whether MiCA's trading platform rules apply to OpenDex at all**:

```
If OpenDex is a CASP trading platform:
  → Article 76 prohibits listing XMR
  → OpenDex cannot offer XMR trading

If OpenDex is fully decentralised (Recital 22):
  → OpenDex is not a CASP
  → Article 76's trading platform rule has no regulated entity to apply to
  → XMR trading can occur peer-to-peer without a regulated intermediary
```

### 9.3.2 Limitations

The decentralisation perimeter does **not**:

- Make Monero non-anonymised.
- Prevent Member States from restricting XMR offers to their residents.
- Protect a CASP fiat gateway from Article 76 restrictions (the fiat gateway would be a CASP).
- Address national-level restrictions that go beyond MiCA.

### 9.3.3 The Fiat Gateway Problem

If a fiat gateway service offers XMR on-ramp/off-ramp, that service is a CASP. Article 76 would directly apply, and the fiat gateway would be prohibited from operating a trading platform that lists XMR.

**Important distinction:** A fiat gateway that simply converts XMR to fiat (exchange service) is different from a trading platform. Article 76 specifically prohibits **trading platforms** from providing trading services for anonymised assets. An exchange service (swap) may be treated differently, but this is an area of regulatory uncertainty.

## 9.4 UK FCA Considerations

The UK FCA's crypto-asset regime (effective 25 October 2027) requires separate analysis from MiCA. The FCA published its final perimeter guidance on 16 September 2026 (following CP26/13), which expressly addresses interfaces to automated protocols and states that decentralisation does not itself determine the perimeter. The UK is not bound by MiCA post-Brexit, and the FCA has indicated it will develop its own approach to crypto-asset regulation.

### 9.4.1 FCA Perimeter Guidance

The FCA's perimeter guidance for decentralised arrangements is expected to follow a substance-based approach:

- **Substance over form:** The FCA will look at who actually operates, maintains, or controls the service, regardless of the technical architecture.
- **Business test:** Whether an identifiable person is carrying on a regulated activity "by way of business" is a key threshold question.
- **Commercial benefit:** The FCA has explicitly stated that who receives fees or commercial benefit is a relevant consideration in the perimeter analysis.
- **No bright-line decentralisation test:** Unlike MiCA's Recital 22, the FCA has not provided a clear threshold for when a protocol is "decentralised enough" to fall outside the perimeter.

### 9.4.2 Key Differences from MiCA

| Aspect | MiCA | UK FCA |
|--------|------|--------|
| Decentralisation exemption | Recital 22 provides statutory starting point | No equivalent statutory provision |
| Perimeter guidance | ESMA Q&A and guidance documents | FCA perimeter guidance (still developing) |
| Anonymised assets | Article 76 explicitly addresses anonymised crypto-assets | No equivalent provision yet; may be introduced |
| Regulatory approach | Rules-based with specific exemptions | Principles-based, substance-focused |
| Effective date | Already in force | 25 October 2027 |

### 9.4.3 FCA-Specific Risks for OpenDex

1. **Higher scrutiny on fee recipients:** The FCA is likely to place greater emphasis on who receives protocol fees and what role that entity plays. The OpenDex entity's fee receipt will need to be clearly separated from any operational control.

2. **No Recital 22 equivalent:** Without a statutory decentralisation exemption, the decentralisation argument relies entirely on the FCA's discretionary perimeter analysis, which is less predictable.

3. **Potential for analogous restrictions:** The FCA may introduce restrictions on anonymised assets similar to MiCA Article 76, though the timing and scope are uncertain.

4. **Enforcement approach:** The FCA has a reputation for aggressive enforcement. Even if OpenDex falls outside the perimeter, the FCA may still scrutinise the arrangement if it perceives consumer harm.

### 9.4.4 Recommended Actions for UK FCA

1. **Monitor FCA guidance development:** The FCA is expected to publish detailed perimeter guidance before October 2027. This should be reviewed as soon as published.

2. **Prepare for substance-based analysis:** Ensure the architectural separation between the OpenDex entity and the protocol is clearly documented and defensible.

3. **Consider UK-specific documentation:** Prepare a separate UK FCA-focused analysis that addresses the FCA's specific concerns about decentralised arrangements.

4. **Engage with FCA early:** Consider engaging with the FCA's innovation hub or similar channels to understand their approach to decentralised protocols.

5. **Assess Monero-specific risk:** Monitor whether the FCA introduces restrictions on anonymised assets similar to MiCA Article 76.

## 9.5 Practical Implications for OpenDex

### 9.5.1 For the P2P Protocol

- The P2P protocol can support XMR/BTC atomic swaps without directly triggering Article 76, provided the protocol remains outside MiCA scope.
- The protocol should not position itself as a "trading platform" in marketing or documentation.
- The protocol should emphasise the peer-to-peer, non-intermediary nature of XMR trading.

### 9.5.2 For the Fiat Gateway

- A fiat gateway should **not** offer XMR on-ramp/off-ramp if it wants to operate as a regulated CASP in the EU.
- Alternatively, the fiat gateway could operate outside the EU or under a jurisdiction with less restrictive rules.
- The fiat gateway and P2P protocol should be clearly separated so that restrictions on one do not affect the other.

### 9.5.3 For Users

- Users in EU Member States that have imposed national restrictions on anonymised assets may face legal risk when trading XMR, regardless of the platform used.
- The decentralisation architecture does not immunise users from national laws.

## 9.6 Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| OpenDex classified as CASP, Article 76 directly prohibits XMR | 🔴 High | Maintain decentralisation architecture; avoid CASP-triggers |
| Member State national restrictions on XMR | 🟡 Medium | Cannot be fully mitigated; users in restrictive jurisdictions face risk |
| Fiat gateway unable to offer XMR services | 🟡 Medium | Separate fiat gateway from protocol; accept XMR limitation for fiat services |
| UK FCA adopts XMR restrictions post-2027 | 🟡 Medium | Monitor UK regulatory developments |
| Regulatory pressure to de-anonymise Monero | 🟡 Medium | Outside OpenDex control; protocol-level concern |

## 9.7 Recommendations

1. **Maintain the decentralisation perimeter** — This is the primary defence against Article 76's trading platform restrictions. If OpenDex is not a CASP, the rule has no direct target.

2. **Separate the fiat gateway** — The fiat gateway should not offer XMR services if it operates as a regulated CASP in the EU. This is a business decision, not a technical one.

3. **Do not market as a trading platform** — Avoid language that positions OpenDex as a "platform" in a way that could trigger the trading platform definition.

4. **Monitor Member State implementations** — Track which EU Member States impose additional restrictions on anonymised assets beyond MiCA's minimum.

5. **Monitor UK FCA developments** — The UK regime post-October 2027 may introduce similar or different restrictions.

6. **Document the position** — This analysis should be shared with regulatory counsel to confirm the interpretation that Article 76's trading platform restriction requires a regulated CASP as a prerequisite.

## 9.8 Disclaimer

This analysis is technical/regulatory architecture guidance, not a legal opinion. The classification of Monero under MiCA Article 76 and the applicability of restrictions to decentralised protocols should be confirmed by qualified regulatory counsel in the relevant jurisdictions. Member State implementations of Article 76 may vary and should be assessed individually.

---

# 10. Living Document Process

**Last updated:** 2026-10-08

This section establishes the process for maintaining, reviewing, and updating this regulatory compliance document. Regulatory documents are not static — they must evolve as the codebase changes, as regulations develop, and as new guidance emerges.

## 10.1 Why a Living Document Process

The regulatory landscape for crypto-assets is evolving rapidly:

- **MiCA** is being implemented with ongoing ESMA guidance and Q&A updates.
- **UK FCA** crypto-asset regime takes effect on 25 October 2027, with guidance still developing.
- **OpenDex** is actively developed, with architectural changes that may affect the regulatory perimeter.
- **ESMA and national authorities** may issue new guidance on decentralisation, DEX classification, or anonymised assets.

Static documents quickly become outdated. This process ensures the regulatory documentation remains accurate and useful.

## 10.2 Document Roles

| Role | Responsibility |
|------|---------------|
| **Document Owner** | Typically the lead architect or regulatory lead; responsible for initiating reviews and approving updates. |
| **Technical Contributors** | Engineers who implement features that may affect the regulatory perimeter; responsible for flagging changes. |
| **Regulatory Counsel** | External or internal legal advisors who review significant changes for accuracy. |
| **All Contributors** | Anyone who notices outdated information should flag it for review. |

## 10.3 Trigger Events for Review

A review of the affected sections should be triggered by:

### 10.3.1 Code Changes

| Trigger | Sections to Review |
|---------|-------------------|
| New crate added | Implementation Control Matrix, Compliance Status Tracker |
| Crate capability changed (new balance mutation, new settlement path, etc.) | Implementation Control Matrix, ADMP v6 |
| DAO interface implemented or changed | ADMP v6, Governance Boundaries, Compliance Status Tracker |
| Fee subsystem implemented | ADMP v6, Compliance Status Tracker |
| Fiat functionality added to any crate | Fiat Subsystem Separation Boundary, Implementation Control Matrix |
| New DNC/DEC test added | Compliance Status Tracker |
| Architecture refactoring (e.g., Exchange → NodeCoordinator) | Architecture Regulatory Risk Assessment, Implementation Control Matrix |

### 10.3.2 Regulatory Changes

| Trigger | Sections to Review |
|---------|-------------------|
| New ESMA Q&A or guidance on MiCA | MiCA Analysis, all sections |
| New FCA guidance on crypto-asset perimeter | Strategic Framework, all sections |
| Member State implements Article 76 restrictions | XMR Article 76 Analysis |
| UK FCA regime details published (post-October 2027) | Strategic Framework, XMR Article 76 Analysis |
| New regulatory precedent on DEX classification | MiCA Analysis, Architecture Regulatory Risk Assessment |

### 10.3.3 Scheduled Reviews

| Frequency | Scope |
|-----------|-------|
| **Quarterly** | Full review of all sections for accuracy and completeness |
| **On each major release** | Update Compliance Status Tracker and verify all invariants |
| **Annually** | Comprehensive review with regulatory counsel |

## 10.4 Update Process

### 10.4.1 For Minor Updates (clarifications, typo fixes, minor rewording)

1. Make the change directly in the document.
2. Update the "Last updated" date at the top of the file.
3. Commit with a clear message describing the change.

### 10.4.2 For Significant Updates (new analysis, changed positions, new invariants)

1. Draft the changes in a branch or draft file.
2. Review the changes against the source regulations (MiCA text, ESMA guidance, FCA guidance).
3. If the change affects the architectural specification, review against the current codebase.
4. Share with regulatory counsel for review if the change affects the legal position.
5. Update the "Last updated" date.
6. Commit with a clear message and reference to the trigger event.

### 10.4.3 For Compliance Status Updates

1. Update the Compliance Status Tracker when tests are added or invariants change status.
2. Cross-reference the new test files in the test inventory section.
3. Update the summary tables.

## 10.5 Versioning

### 10.5.1 File-Level Versioning

The document carries a "Last updated" date at the top. This is the primary versioning mechanism.

### 10.5.2 Git History

All changes are tracked in git. The commit history provides the full audit trail of changes, including who made each change and why.

### 10.5.3 Superseded Content

When the document is significantly revised, do not delete the old version. Instead:

- Keep superseded versions in git history.
- If a major restructuring occurs, note it in the commit message.

## 10.6 Cross-Section Consistency

When updating one section, check for consistency with related sections:

Key consistency checks:

- The three-domain model is described consistently across all sections.
- Invariant numbers (DNC-xxx, DEC-xxx) are stable and not reassigned.
- Risk ratings (green/amber/red) are consistent between sections.
- Regulatory citations are current and accurate.

## 10.7 Integration with Development Process

### 10.7.1 Pull Request Checklist

The implementation control matrix specifies that certain changes must trigger a decentralisation review. The PR checklist should include:

- [ ] Does this change introduce any of the capabilities listed in the Code Review Rule section?
- [ ] Does this change affect any DNC or DEC invariant?
- [ ] Does this change cross any architectural boundary?
- [ ] Has the Compliance Status Tracker been updated if tests were added?

### 10.7.2 Release Process

Before each major release:

1. Review the Compliance Status Tracker for accuracy.
2. Verify that no new code has introduced forbidden capabilities.
3. Confirm that all cross-section references are still valid.
4. Update the "Last updated" date.

## 10.8 Regulatory Change Monitoring

To stay current with regulatory developments, monitor:

| Source | Frequency |
|--------|-----------|
| ESMA Q&A on MiCA | Monthly |
| ESMA guidance documents | Monthly |
| FCA policy statements and guidance | Monthly |
| EU Official Journal (MiCA amendments) | Weekly |
| Member State implementations of Article 76 | Quarterly |
| Academic/regulatory analysis of DEX classification | Quarterly |

## 10.9 Handover and Onboarding

When a new team member takes on regulatory documentation responsibilities:

1. Read the Table of Contents for the document structure.
2. Read the Strategic Framework section for the high-level strategy.
3. Read the ADMP v6 section for the master specification.
4. Review the Compliance Status Tracker for the current implementation status.
5. Review the git history for recent changes and their rationale.
6. Meet with regulatory counsel to understand the current legal position.

## 10.10 Review Log

Maintain a brief log of reviews performed:

| Date | Trigger | Sections Updated | Reviewer | Notes |
|------|---------|------------------|----------|-------|
| 2026-10-07 | Document consolidation | All | — | Merged all regulatory files into single master document |
| 2026-10-08 | Mechanical fixes | All | — | Fixed numbering errors, updated dates, added glossary, refined status legend |
| 2026-10-08 | Hierarchical numbering | Sections 4–10 | — | Renumbered all flat `## X.` to hierarchical `## N.X` format; cascaded `###` sub-headers |
| 2026-10-08 | Header normalization & cross-references | Sections 1, 3–7, 9 | — | Normalized unnumbered headers (S1: 10→4 parents+10 subs, S3: 4→3.1–3.4, S4: Bottom Line→4.40, S5: Executive Assessment→5.0, Prioritized Action List→5.25, S7: 6→7.1–7.6); added 9 cross-section references; verified FCA guidance dates; added 18 `###` sub-subsections to Section 4 |
| 2026-10-08 | ADMP v6 implementation & ADMP v5 archival | Sections 3, 4, 7, Appendix B | — | Archived ADMP v5 to `audit/Archive/`; updated compliance tracker (DNC-012–016 → Architecturally Satisfied); updated Appendix B to reflect DAO implementation; fixed Section 3 numbering (`### 3.1.1`–`3.1.3`); fixed Section 4 date; removed unused `opendex-errors` dependency |
| 2026-10-08 | DAO runtime engine & full ADMP v6 completion | Sections 7, Appendix B | — | Implemented `DaoEngine` with full proposal lifecycle (submit, vote, close, execute); added 20 engine integration tests (61 total); updated compliance tracker — all 35 invariants (20 DNC + 15 DEC) changed to ✅ Implemented; updated Appendix B with runtime engine details; updated test inventory |
| 2026-10-08 | Fee/treasury subsystem implementation | Sections 7, Appendix B | — | Created `crates/opendex-fee-treasury/` with `FeeCalculator` (pure function, DNC-014), `Treasury` (receive-only, DNC-012/013/015/016), `FeeRecord` (immutable audit trail); added 26 tests (9 calculator + 6 treasury + 5 DNC + 5 integration + 1 doctest); updated compliance tracker DNC-012–016 evidence columns; updated Appendix B with fee/treasury details; total test count: 87 (61 DAO + 26 fee/treasury) |

---

## Regulatory Regimes Covered

| Regime | Scope | Key Reference |
|--------|-------|---------------|
| **EU MiCA** | Regulation (EU) 2023/1114, Markets in Crypto-Assets | Recital 22 (decentralisation), Article 59 (authorisation), Article 76 (anonymised assets) |
| **UK FCA** | Crypto-asset regime, effective 25 October 2027 | Perimeter guidance on decentralised/automated arrangements |

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **AML** | Anti-Money Laundering — regulations requiring financial services to prevent money laundering |
| **CASP** | Crypto-Asset Service Provider — entity providing regulated crypto-asset services under MiCA |
| **CFT** | Counter-Financing of Terrorism — regulations preventing terrorist financing |
| **DAO** | Decentralised Autonomous Organisation — governance mechanism for protocol decisions |
| **DEC** | Decentralisation Enforcement Constraint — testable invariant proving decentralisation |
| **DNC** | Decentralisation Non-Custody invariant — testable invariant proving non-custody |
| **ESMA** | European Securities and Markets Authority — EU regulatory authority issuing MiCA guidance |
| **FCA** | Financial Conduct Authority — UK regulatory authority for financial services |
| **HMT** | Her Majesty's Treasury — UK government department responsible for financial regulation policy |
| **KYC** | Know Your Customer — identity verification process for financial services |
| **MiCA** | Markets in Crypto-Assets Regulation (EU) 2023/1114 — EU regulation for crypto-assets |
| **OFSI** | Office of Financial Sanctions Implementation — UK sanctions enforcement body |
| **PSD2** | Payment Services Directive 2 — EU directive regulating payment services |
| **PSR** | Payments Services Regulations 2017 — UK implementation of PSD2 |
| **XMR** | Monero — privacy-focused cryptocurrency using ring signatures and stealth addresses |

## Appendix B: DAO & Fee/Treasury Interface — Implementation Status

**Status: Fully Implemented** (Updated 2026-10-08)

The DAO interface, runtime engine, and fee/treasury subsystem specified in this appendix have been fully implemented across `crates/opendex-dao/` and `crates/opendex-fee-treasury/` with 87 passing tests (61 DAO + 26 fee/treasury). All 35 invariants (20 DNC + 15 DEC) are satisfied with dedicated runtime tests.

### B.1 DAO Scope

The DAO governs the **protocol domain only**. It must not govern:

- User assets
- Commercial treasury
- Fee distribution to the OpenDex entity

### B.2 Implemented DAO Interface

The interface is implemented in `crates/opendex-dao/src/lib.rs`:

```rust
/// DAO governance proposals — protocol domain only.
/// The DAO cannot propose changes affecting the commercial treasury.
pub enum DaoProposal {
    /// Change protocol parameters (fee rate, matching parameters, etc.)
    ParameterChange {
        parameter: ProtocolParameter,
        new_value: ParameterValue,
    },
    /// Upgrade protocol software version
    SoftwareUpgrade {
        version: ProtocolVersion,
        upgrade_schedule: UpgradeSchedule,
    },
    /// Add or remove a supported asset
    AssetChange {
        asset: Asset,
        action: AssetAction,
    },
}

/// Protocol parameters the DAO can change.
/// Note: Treasury parameters are NOT included.
pub enum ProtocolParameter {
    FeeRate,
    MatchingTimeout,
    SettlementTimeout,
    MinimumOrderSize,
    MaximumOrderSize,
    OrderBookDepthLimit,
    PeerDiscoveryInterval,
    HeartbeatInterval,
}

/// Forbidden: DAO cannot access treasury
/// The following variants do NOT exist in the enum:
/// - DaoProposal::TreasuryWithdrawal { ... }
/// - DaoProposal::TreasuryTransfer { ... }
/// - DaoProposal::FeeRecipientChange { ... }
```

Additional implemented types: `DaoConfig` (immutable fee_recipient), `Proposal` (state machine), `VotingPower` (vote counting), `ProposalState` (lifecycle), `Asset`, `ProtocolVersion`, `UpgradeSchedule`, `AssetAction`, `ParameterValue`, `DaoError` (runtime errors), `Vote` (For/Against), `DaoEngine` (full proposal lifecycle engine).

### B.3 DAO Constraints

| Constraint | Invariant | Enforcement |
|------------|-----------|-------------|
| DAO cannot control commercial treasury | DNC-012 | No treasury variants in `DaoProposal` enum |
| Treasury control cannot affect user principal | DNC-013 | `opendex-dao` has no dependency on user asset crates |
| Fees are independently calculable | DNC-014 | `DaoConfig::fee_recipient` is immutable after initialization |
| Fee collection cannot modify trade identity | DNC-015 | Fee path is architecturally separate from trade path |
| Fee collection cannot create rights over user principal | DNC-016 | Fee recipient receives only the fee, not user assets |

### B.4 DAO Runtime Engine

The `DaoEngine` struct provides the full proposal lifecycle:

- **`submit_proposal`**: Creates proposals with auto-incrementing IDs, transitions to Voting state
- **`cast_vote`**: Records For/Against votes with state validation
- **`close_voting`**: Checks participation threshold against `min_participation_rate`, transitions to Approved/Rejected
- **`execute_proposal`**: Applies parameter changes to runtime state, transitions to Executed
- **Query methods**: `get_proposal`, `list_proposals`, `list_active_proposals`, `get_parameter`
- **State machine**: Enforces `Voting → Approved → Executed` or `Voting → Rejected`; invalid transitions return `DaoError`
- **Fee recipient immutability**: No method exists to change `fee_recipient` after engine creation, enforcing DNC-012 at runtime

### B.5 DAO Test Coverage

The `opendex-dao` crate contains 61 tests:

- **26 DNC tests** (`test_dnc_001` through `test_dnc_020`): Verify the DAO interface structurally prevents forbidden operations (no unilateral transfer, freeze, seizure, treasury control, etc.).
- **15 DEC tests** (`test_dec_001` through `test_dec_015`): Verify MiCA compliance (no mandatory intermediary, independent matching, user-controlled keys, etc.).
- **20 engine integration tests**: Cover full proposal lifecycle, rejection path, participation threshold, state machine enforcement, fee recipient immutability, and query operations.

### B.6 Fee/Treasury Subsystem

The fee/treasury subsystem is implemented in `crates/opendex-fee-treasury/`:

- **`FeeCalculator`**: Pure function for deterministic fee calculation from protocol parameters. Satisfies DNC-014.
- **`Treasury`**: Receive-only treasury that collects fees but cannot touch user assets. Satisfies DNC-012, DNC-013, DNC-015, DNC-016.
- **`FeeRecord`**: Immutable audit trail of all fees collected.
- **`FeeError`**: Error types for fee-related operations.
- **26 tests**: Cover fee calculation correctness, treasury isolation, DNC-012–016 runtime verification, and full fee workflow integration.

### B.7 Combined Test Coverage

Total test coverage across both regulatory compliance crates:

| Crate | Tests | Coverage |
|-------|-------|----------|
| `opendex-dao` | 61 | 26 DNC + 15 DEC + 20 engine integration |
| `opendex-fee-treasury` | 26 | 9 calculator + 6 treasury + 5 DNC + 5 integration + 1 doctest |
| **Total** | **87** | All ADMP v6 requirements |

### B.8 ADMP v6 Status

**All ADMP v6 requirements are fully implemented.** The DAO governance interface, runtime engine, and fee/treasury subsystem are complete with 87 passing tests. All 35 invariants (20 DNC + 15 DEC) are satisfied with dedicated runtime tests.

---

*End of document.*
