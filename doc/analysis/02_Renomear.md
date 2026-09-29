# Análise — Renomear.txt.txt (fonte 2)

> **Fonte:** `D:\cortex\Renomear.txt.txt` · 2.016 linhas · 58 seções +
> conclusão, lidas integralmente em duas passadas.
> **Papel:** define **como organizar** a reescrita completa em Rust + Lua:
> organização por domínio (não por tecnologia), 12 divisões funcionais,
> estrutura de diretórios raiz, fronteira Rust↔Lua e regras de migração.
> **Tipo:** análise arquitetural (prescrição), não auditoria de código.

## 1. Tese central

Estruturar o novo Triad_AEE como **ORGANISMO COGNITIVO**, dentro do qual
existe uma arquitetura cerebral **opcional** — neutro entre "córtex",
"cérebro completo" e "sistema cognitivo abstrato". A decisão estruturante:
**domínio → módulo → implementação**, nunca `linguagem → arquivos → funções`
(§1). Rust e Lua são mecanismos de implementação; não definem a arquitetura.
O próprio documento adverte: as "12 partes" são **abstração computacional
funcional**, não afirmação anatômica literal (§0, §5).

## 2. Divisão de responsabilidades Rust × Lua

**Rust** (semanticamente inambíguo, §2): estado canônico, ClusterBio, tecidos,
grafos, HOTM/Reservoir, memória estrutural, scheduler, event bus, máquinas de
estado, World Model/Workspace estruturais, causalidade, persistência,
concorrência, GPU, telemetria, contratos, validação, FFI, segurança,
invariantes.

**Lua** (ajustável, §3): políticas, heurísticas, gates, thresholds, modos
cognitivos, políticas de crise/federativas/ecológicas, experimentos. Lua nunca
possui: memória canônica, clusters, tissue graph, persistência, estado
causal, buffers GPU, ownership de recursos, threads, locks.

Ciclo canônico da fronteira (§3, §36):

    Rust Snapshot → Lua analisa → Lua Proposal → Rust valida range
    → LawEnforcer valida invariantes → Governor arbitra → Rust aplica
    → Telemetry observa efeito → Learning avalia

Escrever direto (`cluster.energy = 0.4`) é proibido por construção.

**Regra de decisão por componente (§58):** mantém estado canônico / executa a
cada step / é pesado / concorrente / usa GPU / mexe em memória / precisa ser
determinístico / protege invariantes → **Rust**; define política / threshold /
estratégia / heurística substituível / experimental / mudança rápida →
**Lua**; se ambos: Rust = mecanismo, Lua = política.

## 3. As 12 divisões funcionais (§5)

| # | Divisão | Responsabilidade central |
|--:|:--|:--|
| 01 | brainstem | sobrevivência operacional: arousal, lifecycle, wake/sleep, emergency |
| 02 | hypothalamus | homeostase e necessidades internas: energia, escassez, stress, drives |
| 03 | thalamus | roteamento e gating: filas, admissão de sinais, prioridade, tráfego |
| 04 | cerebellum | coordenação temporal/preditiva: timing, correção de trajetória, sequências |
| 05 | basal_ganglia | seleção de ação: competição de propostas, gating, inibição, reward |
| 06 | limbic | valoração: saliência, valor, recompensa, aversão, motivação, novelty |
| 07 | hippocampal | memória histórica: episódica, allocortex, reconsolidação, replay, indexação |
| 08 | sensory | entrada e representação perceptiva: modality adapters, grounding, spatial |
| 09 | association | integração representacional: semântica, binding, símbolos, hierarquias |
| 10 | executive | cognição global: GW, World Model, planejamento, causal, decisão, atenção |
| 11 | action | saída e agência: planos, actuators, execution, outcomes, efference copy |
| 12 | metacognition | cognição sobre a cognição: Self Model, identidade, meta-learning, self-engineering |

**Governança não é a 13ª parte** (§6): leis, federações e ecologias são
propriedades organizacionais → ficam fora de `brain/`. Infraestrutura
(scheduler, GPU, telemetria, persistência, database) também não pertence a
região cerebral alguma.

## 4. Estrutura raiz e crates (§7–46)

Raiz: `Cargo.toml/lock`, `rust-toolchain.toml`, `README/LICENSE/CONTRIBUTING/
SECURITY/CHANGELOG`, `config/` (default/development/research/benchmark/
production + `modes/{embryogenesis, corticalization, maintenance, regeneration,
awake, sleep, dream, crisis, recovery}` + `hardware/{cpu, rx580, opencl, wgpu}`
+ `experiments/{ablation, deterministic, multi_seed}`), `crates/`, `lua/`,
`schemas/`, `assets/`, `data/`, `docs/`, `experiments/`, `tests/`, `benches/`,
`tools/`, `migrations/`, `var/` (runs/checkpoints/logs — estado mutável
separado do source), `target/`.

Crates-chave:
- **foundation** (§9): ids (RunId, StepId, ClusterId, TissueId, ConceptId,
  DecisionId, EpisodeId, TraceId), status de valor (VALUE, NO_DATA, STALE,
  INVALID, PENDING, DISABLED, ERROR — onde o problema do `None`/`_ctx`
  ambíguo é eliminado arquiteturalmente), clock, errors, result, evidence,
  provenance, version, units.
- **contracts** (§10): events (cognitive/memory/decision/learning/tissue/
  governance), messages (sensory/semantic/workspace/prediction/action/
  outcome), state (snapshot/delta/versioned), evidence (evidence_level,
  causal_proof). Nada implementa cognição aqui.
- **runtime** (§11): scheduler adaptativo, execution, event_bus, lifecycle,
  TypedContext (§51: cada valor com value/status/source/step/version/
  confidence/provenance — substitui o `_ctx` genérico), tracing. Substitui
  gradualmente o monólito `system.py`.
- **substrate** (§12): cluster (state/lifecycle/role/genome/plasticity),
  tissue, graph, reservoir (hotm/readout/recurrence/memory_trace/hebbian/
  spectral), morphogenesis (division/fusion/pruning/repair). Sem World Model,
  Workspace ou Meta Learning aqui.
- **brain/regions/** (§13–25): crate agregador pequeno (registry, region,
  brain_state) + 12 crates irmãos de região. Fluxos exemplares:
  Workspace→propostas→basal_ganglia competition→selected action (§18);
  DecisionProposal→SelectedAction→ActionExecution→Outcome→LearningSignal (§24);
  metacognição produz **propostas**, nunca modifica o organismo direto (§25);
  brainstem é domínio cognitivo — o scheduler de software fica em `runtime/`
  (§14).
- **governance** (§26): laws, federation, ecology, economy, arbitration.
- **development** (§27, §53): embryogenesis, corticalization, maturation,
  maintenance, regeneration, phase — embriogênese vira **regime explícito de
  desenvolvimento**, não região permanente; pode entrar em standby quando a
  corticalização assumir.
- **modes** (§28): awake/sleep/dream/crisis/stabilization/recovery/
  reintegration — o modo altera política/gain/permissions; sem duplicar
  Reservoir por modo.
- **learning** (§29) e **memory** (§30): transversais; interface comum de
  store/retrieve/trace/provenance/consolidation; implementações cognitivas
  específicas ficam no hippocampal.
- **platform** (§31–34): gpu (wgpu/opencl/cpu + kernels: energy, pairwise,
  fusion, morphogenesis, tissue_affinity, metrics — **Lua nunca toca
  buffers**), persistence (snapshot/journal/checkpoint/restore/migration/
  lineage com parent_run_id, checkpoint_hash, state_hash, schema_version —
  elimina `latest` como única verdade), telemetry (científica separada de
  logging operacional), storage (o domínio cognitivo não conhece SQL).
- **bindings/lua** + raiz `lua/` (§35): policies/governance/development/
  strategies/experiments; Lua recebe visão limitada; nunca
  `cluster_state.lua` com estado real.

## 5. Testes e garantias (§42–43)

`tests/cognitive_closure/` transforma objetivos científicos em testes de
software: concept_to_workspace, workspace_to_decision, decision_to_execution,
outcome_to_learning, learning_to_future_behavior.
`tests/architecture/` impede a regressão estrutural: no_layer_cycles,
no_direct_actuation_from_lua, no_infrastructure_in_brain, no_stale_as_value,
no_undefined_layers, no_context_key_collision, no_global_mutable_state.

## 6. Duas dimensões, não uma (§52)

As 12 regiões **não substituem** L1–L5. Cada `ModuleDescriptor` carrega
`region`, `layer`, `domain`, `entity_kind`, `execution_pipeline` — nunca
inferir um pelo outro (ex.: Thalamus tem substrato L1, circuito L2, routing
L3, integração L4, governor L5). Dependência unidirecional (§49):
foundation → contracts → substrate → regions → learning → governance →
runtime orchestration; infraestrutura entra por interface. Proibições:
foundation→World Model; substrate→Global Workspace; semantic→database;
Lua→GPU buffer. Regiões comunicam-se por mensagens tipadas (MemoryRetrieved,
SemanticRepresentation, WorkspaceCandidate, WorkspaceSelected, PredictionMade,
ActionProposed/Selected/Executed, OutcomeObserved, LearningUpdate) (§50) —
nunca chamadas arbitrárias entre regiões.

## 7. Migração (§48, §57)

Não portar `.py`→`.rs` um-a-um (apenas transportaria a arquitetura atual).
Sequência: inventariar componentes → atribuir entity_kind → atribuir L1–L5 →
atribuir região ou TRANSVERSE → definir input/output → definir owner de
estado → definir eventos → eliminar dependências circulares → criar
contratos → **somente então** reescrever em Rust/Lua. `legacy/python`
read-only temporário; nenhum código novo depende dele; arquivar fora do
runtime ao fim.

## 8. Veredicto e divergência com a fonte 1

Documento organizacional completo, compatível com a fonte 1 em quase tudo
(domínio-first, Rust canônico, Lua política, contratos tipados, learning
transversal, desenvolvimento como regime, TypedContext). **Divergência
central:** aqui as 12 regiões são crates-irmãos de primeira classe (§13), com
cortical/subcortical dentro de `brain/` (§54); a fonte 1 (§21 intro, §50)
exclui cerebelo/tronco/gânglios/paleocortex da primeira reescrita
(→ `extensions/full_brain/`). Resolução proposta (a ratificar em
`07_sintese_consolidada.md` e ADR-0005/0006): **núcleo mínimo L1–L5 da fonte
1 como core; as 12 regiões como mapa conceitual + extensão `full_brain`, com
promoção caso a caso pelo critério §52 da fonte 1.**
