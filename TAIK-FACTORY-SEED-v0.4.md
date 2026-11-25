# TAIK — Minimal Creative Factory Seed

**ID:** TAIK-SEED-001  
**Version:** 0.4  
**Status:** Final bootstrap baseline for the first experiment; not an implemented or validated system  
**Implementation:** Rust; reuse a supplied Clojure-like Rust language where useful  
**Audience:** Implementation agent  
**Precedence:** Supersedes TAIK-FACTORY-SEED v0.3, v0.2 and v0.1 for bootstrap. TAIK-RCK-001 v0.2 and earlier architectural proposals are not implementation backlogs.

## 0. Intent

Create the smallest executable Factory that can produce, inspect, retain and reuse results, and apply the same creative process to its own procedures.

Start from an empty TAIK project. Do not design its eventual architecture. Implement a working seed, exercise it, observe its behavior and let subsequent changes emerge from demonstrated needs. A justified next intent is an output of reflection, not an item in a predefined roadmap.

The seed is an experiment in cumulative, recursive creation and the possible emergence of interacting creative processes, not a claim that all natural or creative processes follow a universal law.

## 1. Four creative operations

| Operation | Responsibility |
|---|---|
| ADD | Expand the available possibilities by introducing, discovering, deriving or reusing inputs, elements or capabilities. |
| STRUCTURE | Establish or change relationships, organization and participation in space, time, meaning and function. |
| REDUCE | Select, remove, combine, compress, attenuate or deactivate according to criteria while preserving required properties. |
| REFLECT | Observe the actual artifact and procedure, compare observations with expectations, and derive evidence or possible changes. |

These are creative responsibilities, not mandatory services, agents, classes, language keywords or exclusive categories of computation.

- Structure includes time. An element may exist without participating continuously.
- Reduction is contextual. Deactivation is not necessarily deletion. Preserve required behavior and necessary redundancy. Lossy reduction is permissible only where the applicable constraints permit it.
- Addition need not import external information; it may derive possibilities from existing material.
- Reflection need not change anything. Observations, including failures, may justify retaining the current result, changing the procedure, or stopping.

### 1.1 Composition, not a state machine

A useful initial composition is:

```clojure
(-> state add structure reduce reflect)
```

This is an example, not a mandatory phase sequence. Operations may be repeated, omitted, interleaved or nested. Concurrent execution is permitted only when dependencies and effects allow it; concurrency is not a bootstrap requirement. Distinct compositions may also exchange results without executing concurrently.

```clojure
(-> state add add structure reflect)
(-> state reduce reflect reduce reflect)
```

A cycle is a bounded unit of work, observation and decision, not necessarily a traversal of all four operations. Acceptance evidence remains mandatory for promotion, irrespective of which creative operations were used. Do not invent work or no-op phase records to satisfy a prescribed sequence.

The notation is illustrative. No specific macros, syntax, interpreter extension or workflow DSL is required.

## 2. The Nautilus Principle

> The results of creation may become the instruments of subsequent creation.

The same creative vocabulary may operate on artifacts, their relationships, the procedures that produce them and the rules used to evaluate them.

Distinguish three mechanisms:

- **Iteration:** apply a procedure again to a state, potentially changed by previous work.
- **Recursion:** use a creative composition within another creative composition, or apply the creative process to its own representable procedures.
- **Cumulative development:** retain and reuse results and observations so later work can build on earlier work, potentially enabling new capabilities or abstraction levels.

The Nautilus spiral is a conceptual representation of their interaction: the same creative operations recur at different scales. Previous results need not be discarded when new levels of composition become possible. They may become reusable elements of subsequent work.

The spiral does not prescribe a geometric growth rate, a sequence of abstraction levels, monotonic improvement, or a mandatory increase in complexity. The Factory may grow, shrink, remain unchanged or stop. The existence of a loop is not evidence of progress.

### 2.1 Results as operators

An output may be ordinary data, an artifact, an observation, or a reusable executable transformation. An executable result must be representable, inspectable and invocable through the same available execution mechanism, subject to its scope and acceptance conditions.

A creative procedure may itself become an input, candidate result or reusable operator. The four operation roles remain the vocabulary of this experiment; their implementations and compositions are not frozen.

Do not introduce a universal object model, graph, type system or generic Factory hierarchy to achieve this. Start with whatever representation makes one genuine reuse of an executable result possible.

### 2.2 Artifact and procedure evolution

A conceptual model is:

```text
(S', P', O) = evolve(S, P, I, C, B)
```

`S` is the artifact state, `P` the current creative procedure, `I` the available inputs and observations, `C` the applicable constraints and acceptance criteria, `B` the finite resource bounds, and `O` the newly recorded observations. Primes denote proposed successors, not automatically accepted versions.

Either artifact or procedure can remain unchanged. Rejection preserves the accepted artifact and procedure while allowing useful observations to be retained. The environment and its changing conditions inform selection; they are not additional creative phases.

The model does not imply that operations are pure, deterministic or differentiable. It does not require a global optimization function. The trusted acceptance and permission boundary is not an ordinary mutable procedure.

### 2.3 Recursive trajectories and the Galaxy Principle

> The output of one creative process may become the input, operator, or procedure of another creative process. Creation may become a network of mutually enabling processes.

The Nautilus Principle describes cumulative development within and across compositions. The Galaxy Principle extends the same idea to a plurality of interacting compositions. It introduces no fifth creative operation.

The following geometric images distinguish possible patterns of development, not implementation stages:

| Image | Computational interpretation |
|---|---|
| Circle | Repeated application of a procedure. |
| Helix | Recurrent activity along a changing state trajectory, such as accumulated time or knowledge. |
| Nautilus | Retained results become reusable structures, potentially enabling new scales of composition. |
| Galaxy | Multiple creative trajectories coexist and exchange results, observations or executable procedures; their interaction may enable further trajectories. |

A creative trajectory may contain other creative trajectories. Different trajectories may operate at different scales, rates and with different locally applicable criteria. They need not share a phase, clock, hierarchy or abstraction level. They may be sequential, interleaved, concurrent, or temporarily inactive. None of these execution forms is mandatory.

A process may use a result produced by itself, a nested composition, or another composition. A resulting operator may in turn participate in several subsequent processes. Relationships between processes may therefore emerge from actual reuse rather than a predefined parent-child tree. Feedback may be cyclic, but actual execution must remain bounded.

A conceptual description is a changing collection of creative states and procedures, with interactions carrying results and observations between them. The collection, its interactions, and its procedures may themselves become subjects of creative work. No fixed number of processes, dimensions, coordinate system or global evaluation function is assumed.

The possible benefit is not necessarily a larger system. Reuse and cooperation may increase the useful results reachable within a given resource budget, or reduce the resources needed for an existing result. Redundant processes may be eliminated, consolidated or deactivated. Measure any claimed benefit against a relevant baseline and account for the cost of producing, maintaining and exchanging the results.

This is a computational hypothesis and a geometric analogy. A physical galaxy is not asserted to be a recursive helix or an instance of the four creative operations. Periodicity, self-similarity, geometric spiral growth, increasing dimensionality and monotonic improvement do not follow from the four operations.

### 2.4 Interaction without prescribed infrastructure

The first seed may remain a single process. If interaction between compositions becomes useful, start with the smallest actual exchange of a retained result. Ordinary values, function calls or files may suffice. Do not introduce a broker, shared graph, orchestrator, agent hierarchy, distributed runtime or concurrency merely to express the Galaxy Principle.

An exchanged result does not confer the producer's authority, permissions or remaining budget on its consumer. A consumer must establish that the result is suitable for its own constraints, and accepted work must remain distinguishable from speculative work. If shared or concurrent mutation is introduced, resolve conflicting assumptions or updates explicitly; do not silently overwrite accepted results. Keep enough provenance to identify the exchanged result and its actual contribution. Bound cyclic feedback and account for shared resource consumption when such interactions occur.

### 2.5 The Intelligence Hypothesis (non-normative)

> Intelligence may emerge from the recursive transformation of experiences into reusable knowledge, executable capabilities and adaptive procedures.

Creation changes an artifact. Learning changes the knowledge or procedure used to create it. Meta-learning changes the procedure by which such changes are discovered and evaluated. These are possible subjects of the same creative operations, not prescribed layers, roles or implementation stages.

In this interpretation, ADD introduces or derives experiences and hypotheses; STRUCTURE organizes relationships and patterns; REDUCE selects, compresses, abstracts or generalizes; REFLECT compares expectations with observations and may justify adaptation. A retained insight may become an executable operator or a new learning procedure. Learning procedures may themselves be transformed and reused, within the same process or across processes. No fixed order, number of levels or particular learning mechanism follows from this hypothesis.

Investigate intelligence growth through generalizable capability and resource efficiency rather than accumulated information, code volume, number of agents or structural complexity. A conceptual comparison is:

```text
R(S, P, B) = useful outcomes reachable under a specified task environment,
             acceptance criteria and finite resource budget B
```

A strict expansion of this set at a fixed budget, or achievement of an equivalent accepted outcome at lower cost, would support the capability-growth hypothesis. In practice, evaluate finite task samples with comparable conditions; do not claim to have measured the entire reachable set. Gains may be local, trade-offs may occur, and regressions are possible. Account for learning, tool construction, verification and reuse costs separately and in aggregate. Distinguish transfer to new tasks from memorization or optimization against a known evaluator.

No particular reinforcement learning algorithm, reward engine, model architecture, weight update, meta-agent, goal-generation mechanism or learning hierarchy is required. If a learning mechanism emerges from observed necessity, it is a candidate procedure subject to the existing acceptance and permission boundary. The hypothesis does not authorize changes to independent evaluation criteria, resource limits or permissions. It is not a definition of all intelligence, a proof of AGI, or a guarantee of self-improvement.

## 3. Minimum executable seed

Implement only enough machinery to:

1. Represent an intent, the current accepted result, and the properties that must remain true.
2. Execute a bounded composition of creative operations on a real subject, including a procedure when appropriate.
3. Isolate speculative changes, observe actual results, and accept or reject candidates against explicit criteria.
4. Retain the accepted state and sufficient observations to resume and reuse them after restart.
5. Invoke a further composition when justified, or return, wait or stop.

These responsibilities are not subsystem boundaries. Use the simplest working representation. Persistence, execution, composition and acceptance are enabling infrastructure, not extra creative operations.

The coding agent may be the initial creative worker. No autonomous LLM integration, additional agent, or persistent network service is required. A real operation must be executed or a real result evaluated; printing operation names is not an implementation.

Reuse a usable Clojure-like Rust implementation if supplied and relevant. If it is not available, start with ordinary Rust functions or simple callable procedures. Do not implement a language merely to bootstrap the first loop. Do not assume any particular feature of the supplied interpreter without checking it.

## 4. Minimal invariants

| ID | Requirement |
|---|---|
| S-01 | Execute a real, bounded creative composition. Support the four creative responsibilities without imposing a universal phase order. |
| S-02 | Each iteration has an explicit intent, scope, acceptance conditions and finite limits. Recursive or interacting work, if used, remains within the applicable delegated limits and permissions; exchanging an output does not transfer authority. |
| S-03 | Speculative artifacts and procedure changes cannot silently replace the accepted version. Preserve a recoverable known-good state. |
| S-04 | Acceptance requires actual evidence appropriate to the declared conditions. Unperformed tests, model judgments and assumptions must not be presented as verified facts. |
| S-05 | The accepted result and sufficient observations survive restart; incomplete work is distinguishable from accepted work. |
| S-06 | A later composition can consume a retained result or observation. Executable results may be reused as operators without a special-purpose Factory hierarchy. |
| S-07 | The seed builds and runs without Bevy and without a global periodic tick. |
| S-08 | Additional abstractions, rules and dependencies require observed need, demonstrated benefit or a necessary invariant. Prefer reuse and reduction. |
| S-09 | Candidate self-modifications cannot silently weaken the acceptance, resource or permission boundary used to approve them. Changes to that boundary require separate authorization and validation. |
| S-10 | Do not silently expand permissions or perform unapproved irreversible external actions. Isolate or defer effects where needed. |
| S-11 | Each completed iteration leaves a concise, traceable record of what happened, the evidence, the decision and a justified continuation or stop. |

These invariants are the initial experimental and trust boundary. No domain-specific goals, evaluator, hierarchy or optimization strategy is prescribed by them.

## 5. Execution and time

Start with on-demand execution. A direct function call or a command invocation may be sufficient. A simple event loop is appropriate only if required by actual work. Do not busy-poll when idle.

Bevy is optional, including its standalone ECS. It may later serve games, simulation, or suitable music-control workloads. High-frequency audio DSP may require a separate real-time-safe execution path. Desktop and backend applications do not inherently require ECS. Neither a global scheduler nor an async runtime is mandatory.

Introduce timers, event dispatch, sampling, parallelism or alternate execution mechanisms only when justified. No speculative backend abstraction is required to keep the seed independent of Bevy.

## 6. Bootstrap and first acceptance gate

Choose one small, local subject whose required properties can be checked with available tools. The choice is not part of this specification. It must permit genuine creative work and an observable result.

Build and run the smallest vertical slice. Exercise all four creative responsibilities across the demonstration, but not necessarily in every composition or in the same order. Make at least one further invocation using retained work. Exercise a failing candidate in isolation. The initial worker may be the coding agent; the reusable seed must actually execute or evaluate work and preserve continuity.

| Test | Observable result |
|---|---|
| T-01 | Clean build and execution without Bevy or mandatory sampling. |
| T-02 | Real artifact or procedure work is performed; the four creative responsibilities are demonstrated without a forced phase sequence. Declared acceptance checks actually run. |
| T-03 | A deliberately invalid candidate is rejected; the accepted result remains intact. |
| T-04 | After restart, a later composition consumes a retained result or observation. Interrupted work cannot be mistaken for accepted work. |
| T-05 | A finite bound or stop condition ends execution with recoverable state; idle execution does not busy-poll. |
| T-06 | A report distinguishes actual observations, verification evidence, assumptions and unperformed checks. |

This is the first executable gate, not a production architecture or a commitment to particular storage, syntax or scheduling machinery.

## 7. The first Nautilus experiment

After the initial gate, use the observed results to test the central hypothesis: can a result become an instrument of subsequent creation?

The following are experimental questions, not a fixed feature roadmap or an imposed sequence:

| ID | Question and evidence |
|---|---|
| N-01 | Can a creative composition itself become the subject of another creative composition? Record the original, candidate and actual operation. |
| N-02 | Can creative work produce a reusable executable operator? Identify the result and demonstrate that it can be invoked. |
| N-03 | Can a later composition actually consume that operator? Record its invocation and observable contribution. |
| N-04 | Can reflection justify a change to the creative procedure itself while preserving its required properties? Compare accepted baseline and candidate with independent acceptance checks. |

One experiment may satisfy several questions. If an existing representation suffices, reuse it. Do not construct generic infrastructure merely to satisfy the vocabulary of the experiment.

A change to the Factory's source or executable procedure is a valid initial form of self-application. Automatic self-replacement is not required. Keep candidate changes isolated, preserve the runnable baseline, and require the independently controlled promotion boundary. If a hypothesis fails, record the failure rather than fabricate improvement or force a new architecture.

Successful demonstrations support the computational hypothesis only; they do not prove universal creativity or guaranteed convergence.

### 7.1 Optional Galaxy experiments

These questions are not additional bootstrap gates, required features or a prescribed next generation. Investigate them only when earlier observations justify doing so. A negative result is valid evidence.

| ID | Question and evidence |
|---|---|
| G-01 | Can two distinct creative compositions exchange and actually use a retained result or executable operator? Record producer, result, consumer and observable contribution. Sequential execution is sufficient. |
| G-02 | Can a result of such an interaction feed a subsequent creative composition, including back into a previous participant, within finite limits? Record the causal chain and any stopping condition. |
| G-03 | Does reuse across compositions deliver a measured benefit over an appropriate baseline, including the costs of generating, verifying and exchanging the result? Record regressions and limitations as well as improvements. |

Use the existing seed wherever possible. No dedicated multi-Factory type or communication system is implied. An exchange of a file or callable value can be enough to test the hypothesis. Do not force interaction, hierarchy or expansion to make the experiment pass.

### 7.2 Optional intelligence experiments

These are research questions, not additional bootstrap gates, a backlog or instructions to reproduce published learning architectures. Investigate them only when justified by evidence from the running seed. Negative and inconclusive results are valid.

| ID | Question and evidence |
|---|---|
| I-01 | Can a creative process derive and retain its own reusable learning procedure from actual experience, rather than being given that procedure in advance? Compare with the initial procedure and demonstrate actual execution or use. |
| I-02 | Can a retained capability improve performance on previously unseen tasks? Use independent, held-out tasks and unchanged acceptance criteria; compare against an appropriate baseline and record failures. |
| I-03 | Can the evolved procedure achieve more accepted outcomes under a comparable total resource budget, or equivalent outcomes at lower cost? Include the costs of discovery, adaptation, validation and application; distinguish one-time costs from recurring costs and report uncertainty. |

Where an external coding agent supplies the learning or modifications, attribute its contribution explicitly. Do not claim autonomous learning by the seed merely because an external agent improved its code. Preserve the known-good baseline and independent evaluation boundary. Do not modify tests or selection criteria to manufacture improvement.

## 8. Continuation and evidence

After each completed iteration record:

- identity, intent, baseline and candidate;
- creative operations actually performed, their composition and material effects;
- observations of artifact and procedure, executed checks and results, with evidence paths;
- acceptance or rejection, unresolved limitations and relevant resource consumption;
- retained results, their actual reuse if any (including cross-composition provenance where applicable), and the next justified intent or stop reason.

Use the simplest readable representation. Reports are human-facing projections of actual work, not replacements for evidence. Avoid fabricated metrics or claims of autonomous execution.

Continue only within granted execution and resource bounds. If no justified next intent exists, stop or wait. If a candidate requires new permissions, an external resource or a human decision, retain continuity and stop at that boundary.

## 9. Non-prescription

Do not prebuild domain knowledge, goal hierarchies, fixed workflows, optimization strategies, generic evaluators, a semantic graph, ECS, custom IR, compiler, new DSL, multi-agent hierarchy, generalized plugin system, capability registry, distributed scheduler, database, UI, audio or rendering engine.

These are neither a blacklist nor a backlog. They may emerge from creative work if the observations justify them. The seed must not depend on a predetermined final TAIK architecture.

In particular, do not hard-code the examples, abstraction levels, helices, Nautilus spiral or galaxy diagrams as runtime entities. They describe possible patterns of development and interaction, not implementation data structures. A single-process seed satisfies the bootstrap scope; no multi-Factory runtime is required.

## 10. Agent mandate

Begin from scratch. Implement and run the minimum seed. Observe the actual results. Retain verified work and useful failure observations. Use the next justified intent to exercise the same creative vocabulary again, including on the Factory's own procedures.

Do not invent work for an operation. Do not grow the system merely to demonstrate growth. Do not weaken the independent acceptance boundary to make an experiment pass.

The output of creation may become the input, the operator, or the procedure of subsequent creation, within the same composition or across compositions.

**ADD · STRUCTURE · REDUCE · REFLECT**

Compose freely. Retain what matters. Recur when justified.


## Appendix A. Research context (non-normative)

These works motivate individual mechanisms of the Intelligence Hypothesis. They are reading references, not architecture templates, dependencies or implementation tasks. The bootstrap specification is self-contained and does not require access to these publications. Their results do not establish the completeness or minimality of the four creative operations, the Nautilus or Galaxy geometry as a mathematical law, or unbounded recursive improvement.

| Reference | Relevance |
|---|---|
| Sundaram, Sancaktar & Isola (2026), [Do LLM Agents Still Need to Be Told How to Self-Improve?](https://arxiv.org/abs/2611.23219) | Agents with fixed model weights investigate their own context-based learning procedures within supplied tools, goals and budgets. |
| Zhang et al. (2026), [Hyperagents](https://arxiv.org/abs/2603.19461) | Experiments with an editable agent program encompassing both task solving and the mechanism for modifying the agent. |
| Zhang et al. (2025), [Darwin Gödel Machine: Open-Ended Evolution of Self-Improving Agents](https://arxiv.org/abs/2505.22954) | Empirically evaluated agent modifications and retention of alternative development branches. |
| [Discovering state-of-the-art reinforcement learning algorithms](https://www.nature.com/articles/s41586-025-09761-x) (2025) | DiscoRL investigates automated discovery of reinforcement learning rules and their performance on benchmarks. |
| Schmidhuber (1987), [Evolutionary Principles in Self-Referential Learning](https://people.idsia.ch/~juergen/diploma.html) | Historical treatment of recursive learning and learning how to learn. |

The baseline is intentionally frozen at v0.4 for the initial implementation experiment. Further design changes should be justified by actual observations from the seed, not by a desire to pre-implement these research mechanisms. This freeze concerns the initial experimental specification, not the future creative procedures produced by the Factory.
