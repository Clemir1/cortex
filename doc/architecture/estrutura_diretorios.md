# Nova estrutura de diretórios — núcleo mínimo (referência normativa)

> Fonte primária: NUCLEO_MINIMO.txt.txt §49 (árvore minimalista). Complementos
> de Renomear.txt.txt §7–46 (config, schemas, tests, tools, migrations) e
> L1AL5.txt §§31–39 (eventos pequenos, ownership de estado, MetricRegistry).
> Este arquivo é normativo para a reescrita; divergências exigem ADR.

## 1. Árvore do núcleo

    Triad_AEE/
    ├── Cargo.toml  Cargo.lock  rust-toolchain.toml  README.md
    ├── config/
    │   ├── default.toml  research.toml  crisis.toml  corticalization.toml
    ├── crates/
    │   ├── foundation/         # ids, status, time, errors, provenance
    │   ├── contracts/           # events, state, messages, evidence
    │   ├── runtime/             # scheduler, executor, event_bus, lifecycle,
    │   │                        # typed_context, tracing
    │   ├── l1_substrate/        # cluster, graph, energy, lifecycle,
    │   │                        # plasticity, reservoir
    │   ├── l2_tissue/           # tissue, registry, affinity, bridge,
    │   │                        # adaptation, feedback
    │   ├── l3_local/            # semantic, memory, attention, prediction
    │   ├── l4_global/           # integration, workspace, world_model,
    │   │                        # causal, memory_integration, decision, action
    │   ├── l5_meta/             # self_model, identity, meta_controller,
    │   │                        # resource_governor, development_governor
    │   ├── learning/            # prediction_error, credit, update,
    │   │                        # validation, forgetting
    │   ├── cybernetics/         # o1_control, o2_adaptation, o3_coordination,
    │   │                        # o4_self_observation, o5_evolution
    │   ├── governance/          # laws, federation, ecology, arbitration
    │   ├── development/         # embryogenesis, corticalization,
    │   │                        # maintenance, regeneration
    │   ├── platform/            # gpu, persistence, telemetry, storage, math
    │   └── triad/               # app, boot, simulation
    ├── lua/
    │   ├── policies/            # energy, attention, learning, crisis, recovery
    │   ├── governance/          # federation, ecology
    │   ├── development/         # corticalization, regeneration
    │   └── experiments/
    ├── schemas/
    ├── tests/
    ├── benches/
    ├── docs/
    ├── extensions/
    └── var/                     # estado mutável de execução (runs, logs, tmp)

## 2. Anotações por pasta (o que entra / o que não entra)

- **foundation/** — tipos universais: RunId, StepId, ClusterId, TissueId,
  ConceptId, DecisionId, EpisodeId, TraceId; status de valor VALUE/NO_DATA/
  STALE/INVALID/PENDING/DISABLED/ERROR. É aqui que o problema do `None`/`_ctx`
  ambíguo é eliminado arquiteturalmente (fonte 2 §9).
- **contracts/** — eventos tipados das cadeias canônicas, mensagens, state
  (snapshot/delta/versioned), evidence. Nenhuma implementação de cognição.
- **runtime/** — substitui o monólito `system.py` gradualmente. TypedContext:
  cada valor com value/status/source/step/version/confidence/provenance.
- **l1_substrate/** — sem linguagem, identidade, World Model, metacognição,
  ecologia, federação, leis. Divisão/fusão/regeneração **sob demanda**.
- **l2_tissue/** — tecido genérico (id, specialization, members, energy,
  coherence, capacity, connections, state); especialização emerge como dado.
  `feedback/` é a ponteira estrutural L3→L2 (AdaptationRequest).
- **l3_local/** — 4 domínios apenas: semantic, memory, attention, prediction.
- **l4_global/** — integration (router/attention/competition/workspace/
  broadcast/synchronization como subsistemas do workspace), world_model
  (state/prediction/simulation/error/update), causal (graph/inference/
  episode/credit/validation), memory_integration (working/episodic/semantic/
  retrieval/reconsolidation/consolidation/provenance), decision (proposal/
  competition/selection/commit), action (request/execution/outcome).
- **l5_meta/** — self_model (state/identity/temporal/narrative/reflection),
  meta_controller (observe→propose→validate→apply→observe), resource_governor,
  development_governor. Nada de dezenas de reguladores.
- **learning/** — transversal; prediction_error, credit, update, validation,
  transfer, forgetting. Nunca em standby.
- **cybernetics/** — protocolos e mecanismos O1–O5 **sobre** componentes
  existentes; sem cópias de energia/Workspace/memória/tecidos; O5 lento.
- **governance/** — laws (invariant/rule/violation/enforcement/validation),
  federation (member/proposal/agreement/allocation/trade), ecology (um motor
  genérico: population/fitness/competition/selection/birth/decay/extinction),
  arbitration.
- **development/** — embryogenesis/corticalization/maintenance/regeneration;
  morfogênese STANDBY por default.
- **platform/** — gpu (backends wgpu/opencl/cpu + kernels), persistence
  (snapshot/journal/checkpoint/restore/migration/lineage), telemetry, storage,
  math. Cognição nunca conhece SQL; Lua nunca toca buffers GPU.
- **triad/** — app, boot, simulation (composição final).
- **lua/** — recebe PolicySnapshot, produz PolicyProposal; visão limitada;
  nunca estado real de clusters em scripts.
- **schemas/** — protege Rust, Lua, telemetria e persistência contra
  divergência semântica (events, state, telemetry, persistence, version).
- **tests/** — `architecture/` (no_layer_cycles, no_direct_actuation_from_lua,
  no_infrastructure_in_brain, no_stale_as_value, no_undefined_layers,
  no_context_key_collision, no_global_mutable_state) + `cognitive_closure/`
  (concept_to_workspace, workspace_to_decision, decision_to_execution,
  outcome_to_learning, learning_to_future_behavior).
- **var/** — runs/<run_id>/checkpoints, logs, telemetry; nunca misturado com
  source.

## 3. Extensões (fora do core)

    extensions/
    ├── language/              # linguistic decoder/predictor/feedback,
    │                          # inner speech, emergent language, generativos
    ├── dream/
    ├── full_brain/            # cerebellum, brainstem, basal ganglia,
    │                          # paleocortex — as 12 regiões da fonte 2 vivem
    │                          # aqui até critério de promoção
    ├── experimental_ecologies/
    ├── experimental_evolution/
    └── advanced_symbolics/

Promoção extensão→core somente com: necessidade demonstrada + efeito
reproduzível + consumidor real + benefício multi-seed + sem duplicação
(fonte 1 §52). Nada é apagado: pesquisa vai para `extensions/` ou
`legacy/research/`.

## 4. Legado

`legacy/python/` read-only com `read_only.md` — temporário; nenhum código novo
depende dele; arquivar fora do runtime ao fim da migração (fonte 2 §48).

## 5. Complementos da fonte 2 aplicáveis sem custo estrutural

- Metadados de raiz: LICENSE, CONTRIBUTING.md, SECURITY.md, CHANGELOG.md.
- `config/modes/` (awake, sleep, dream, crisis, recovery, embryogenesis,
  corticalization, maintenance, regeneration), `config/hardware/` (cpu, rx580,
  opencl, wgpu), `config/experiments/` (ablation, deterministic, multi_seed).
- `experiments/` com manifest.toml + hypothesis.md + config.toml +
  expected_metrics.toml; resultados sempre em `var/runs`, nunca no diretório
  do experimento.
- `assets/` (corpus, seeds, test_vectors) e `data/` (datasets, benchmarks)
  separados de `var/`.
- `tools/` (xtask, migration, graph, profiling, validation) como binários do
  workspace; `migrations/` versionadas (state, database, config, schema,
  legacy_python) — obrigatório por causa da memória cross-run.

## 6. Decisão pendente registrada

As 12 divisões da fonte 2 (brain/regions/) **não** entram como crates do core
nesta versão: ficam como mapa conceitual (dimensão `region` do
ModuleDescriptor) e como `extensions/full_brain/`. Ratificação formal em
`adr/ADR-0005-nucleo-minimo-L1-L5.md` e `adr/ADR-0006-extensoes-fora-do-core.md`
(pendentes).
