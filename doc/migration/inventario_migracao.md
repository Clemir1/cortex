# Inventário de migração — KEEP / MERGE / EXTENSION / ARCHIVE

> Método (f1 §75): para cada componente legado —
> `old_component → responsibility → state_owner → input → output →
> actuator → new_component` — e só então reescrever. Este arquivo classifica
> os **componentes citados nas seis fontes**; o preenchimento campo a campo
> (owners, I/O, actuators) é a primeira tarefa da Fase 0 no repo legado —
> este inventário é o esqueleto de partida, não o levantamento final.
> Classificações marcadas ⚠ são ratificações pendentes (síntese D1–D8).

## 1. Definições

- **KEEP**: entra no core da nova estrutura (posição na árvore §49).
- **MERGE**: funde-se com outro(s) componente(s) (regra f1 §48 — mesmos
  dados/mesmo actuator/mesma variável).
- **EXTENSION**: sai do caminho crítico para `extensions/` (ADR-0006).
- **ARCHIVE**: sai do runtime para `legacy/research/` (sem consumidor vivo
  ou substituído por mecanismo transversal).

## 2. L1 — substrato

| Legado | Classe | Destino novo | Notas (evidência) |
|:--|:--|:--|:--|
| `cluster.py` / ClusterBio (grau 295) | KEEP | `l1_substrate/cluster` | entidade central; IDs+arrays no hot path (f4 §2) |
| `state_matrix.py` | KEEP | `l1_substrate` + `contracts/state` | cache derivado com `state_version`/hash — nunca 2ª autoridade (f3 §3.1) |
| `reservoir_readout.py` | KEEP | `l1_substrate/reservoir` | sinais derivados; `REAL_PROVIDER` vs synthetic carimbado (f3 §3.2) |
| Hebbian (HebbianNorm=0) | KEEP⚠ | `l1_substrate/plasticity` | ou declara BYPASSED — critério de conclusão L1 (f4 §3) |
| energy/survival/homeostasis | KEEP | `l1_substrate/energy` | mortalidade zero, conservação OK no legado (f4 §2) |
| gene_expression, meta_homeostasis (parte L1) | KEEP | `l1_substrate` | executa 160×/run; `update_tissue_params` vira parte de L2-feedback (f5 §3.5) |
| repair/dormancy (1999 clusters step 30; 939 dormancy) | MERGE | `l1_substrate/lifecycle` | histerese obrigatória (f4 §2); bajo DevelopmentGovernor STANDBY (f1 §4) |
| JL/SVD/RandLA/L2-sampling/spatial hashing/kernels | MERGE | `platform/math` + `platform/gpu` | sustentam cognição; não são subsistemas mentais (f1 §5) |

## 3. L2 — tecidos

| Legado | Classe | Destino novo | Notas |
|:--|:--|:--|:--|
| `adaptive_tissues.py` (Tissue/Registry/Controller) | KEEP | `l2_tissue/tissue` + `registry` | grau 64/43; periferia estrutural hoje (f5 §4) |
| `tissue_event.py` (TissueEvent/Ledger/CouplingArbiter) | KEEP | `l2_tissue` + `contracts/events` | ZERO aresta com adaptive_tissues — unificar por contrato (f5 §4) |
| Elo L3→L2 (`hml_tissue_signal`) | KEEP⚠ | `l2_tissue/feedback` (AdaptationRequest) | default ON na nova estrutura (síntese D2) |
| morphogenesis (topologia/arestas) | MERGE | `l2_tissue` + `development/regeneration` | STANDBY; TopologyController ÚNICO com histerese (f1 §4; f4 §7-9) |
| connectivity rebuild (ciclo avgDeg 17-18→12) | MERGE | idem | overshoot documentado (f4 §9) |
| bridges/conductance (`maintain_cross_tissue_bridges`) | KEEP | `l2_tissue/bridge` | tirar da fase de record (f5 achado 3); score 0.002-0.188 no legado (f6) |
| `tissue.py` + `issue_impl.py` (órfãos E0) | ARCHIVE | `legacy/research` | sem consumidor vivo (f5 §3.6) |
| TISSUE_COALESCE_EVERY duplicado | MERGE | `config` única | f5 achado 4 |

## 4. L3 — cognição local

| Legado | Classe | Destino novo | Notas |
|:--|:--|:--|:--|
| semantic emergence/coherence/binding/grounding (6 controladores) | MERGE | `l3_local/semantic/{representation,concept,binding,coherence,grounding,emergence}` | um domínio, não seis órgãos (f1 §9) |
| concepts/anchors (1059/784; reuso 74/85%) | KEEP | `l3_local/semantic` | sinais positivos; DAG com promoção por utilidade, depth>2 orgânico (f4 §13-15) |
| HOTM/recurrence local | KEEP | `l1_substrate/reservoir` + `l3_local/memory` | trace temporal em L1; memória cognitiva em L3/L4 (f1 §21) |
| WorkingMemory/TemporalTrace/ReservoirState/SemanticLTM/Episodic (sobrepostos) | MERGE | `l3_local/memory` + `l4_global/memory_integration` | separação obrigatória (f4 §11) |
| symbolic projection/abstraction/HierarchicalProcessor | KEEP | `l3_local/semantic` (projeção simbólica) | depth=1/266 abstratos no legado — reconstruir como DAG (f4 §13; f6 §3) |
| attention/salience local | KEEP | `l3_local/attention` | seleção local (f1 §8) |
| prediction local/novelty | KEEP | `l3_local/prediction` | f1 §8 |

## 5. L4 — cognição global (o gargalo)

| Legado | Classe | Destino novo | Notas |
|:--|:--|:--|:--|
| global_workspace (suspensa 2/3 da run; 1,26 s) | KEEP | `l4_global/workspace` | pequeno e central; NUNCA SHUTDOWN — modo degradado (f1 §12-13; f6) |
| attention_binding, semantic_router, cognitive_field, cross_module_resonance | MERGE | `l4_global/integration/{router,attention,competition,workspace,broadcast,synchronization}` | subsistemas do workspace, não 5 centros (f1 §14-16) |
| world_model (único E5 formal) | KEEP | `l4_global/world_model` | 3 camadas: Short-Term/Episodic/Cross-Run; uso de histórico = 0 no legado (f4 §18) |
| causal_graph + causal_cycle + causal_graph_cortex + causal_ecology + provenance | MERGE | `l4_global/causal/{graph,inference,episode,credit,validation}` + provenance transversal na telemetria | f1 §18 |
| unified_identity (Self 1,0 × Narrative 0,093) + self_model_ctrl | MERGE | `l5_meta/self_model/{state,identity,temporal,narrative,reflection}` | f1 §27; aresta A5 morta por construção no legado (f6 F-7) |
| reconsolidation, episodic_buffer, hotm_future_navigator | MERGE | `l4_global/memory_integration/{working,episodic,semantic,retrieval,reconsolidation,consolidation,provenance}` | f1 §21 |
| decisão (implicita no workspace; F2 6,7%) | KEEP (novo) | `l4_global/decision/{proposal,competition,selection,commit}` | domínio explícito — o alvo principal (f1 §19) |
| ação (escondida em decision) | KEEP (novo) | `l4_global/action/{request,execution,outcome}` | fronteira cognição→atuação→consequência (f1 §20) |
| emergent_language, inner_speech, linguistic_feedback | EXTENSION | `extensions/language` | f1 §11; ADR-0006 |
| symbol_ecology, deep_telemetry | MERGE | symbol_ecology→`governance/ecology` (adapter); deep_telemetry→`platform/telemetry` | f1 §38; infra não é camada (f6 §1) |

## 6. L5 — meta + governança (drástica redução)

| Legado | Classe | Destino novo | Notas |
|:--|:--|:--|:--|
| cognitive_economy + cognitive_energy_model + scarcity_profile (+ parte thalamus_governor) | MERGE | `l5_meta/resource_governor` | owner único de recursos (f1 §24) |
| auto_adaptive + meta_regulator + architecture_governor + architecture_cortex (+ parte self_engineering) | MERGE | `l5_meta/meta_controller` | observe→propose→validate→apply (f1 §25) |
| thalamic_network (routing/gating) | KEEP | `l4_global/integration` + `runtime/scheduler` | domínio routing L4; scheduler de software no runtime (f2 §16) |
| self_engineering (Proposer→Sandbox→Validator→Incorporator) | MERGE | `l5_meta/meta_controller` (experimento controlado) | loop legado não fecha: approved==incorporados (f6 §3) |
| governance_orchestrator | KEEP (reescrito) | `l5_meta` via contratos descendentes | cascata L5→…→L1 não existe (chave órfã config:2709) — vira contrato first-class (f6 F-1) |
| law_enforcer + LongitudinalEvidenceGate | KEEP | `governance/laws/{invariant,rule,violation,enforcement,validation}` | gate existe e rejeita; enforcement vira real (f6 §4) |
| federation_hub + PolicyArbitrator | KEEP | `governance/federation` | O3 runtime demonstrado (runs 776/777); protocolo do ResourceGovernor (f1 §34) |
| ecology/observer/ecologies_hub + symbol/binding/causal ecology | MERGE | `governance/ecology` (motor genérico + adapters) | um motor, não seis subsistemas (f1 §38) |
| forgetting_controller | MERGE | `learning/forgetting` | política em Lua (f1 §29) |
| allocortex | MERGE | `l4_global/memory_integration/{episodic,schema,context}` | f1 §22 |
| phase_detector, attractor_analyzer, sync_controller, identity_chaos, emergent_specialization, symbol_state_mapper, semantic_topology | ARCHIVE⚠ | `legacy/research` | suspensos/sem efeito medido na crise 1800 (f6 §3); revisitar via ADR-0006 |
| structural_noise, entropic_balance, cognitive_genome | MERGE | `cybernetics/o5_evolution` (baixa frequência) + genoma fora do loop principal | f1 §32-33 |
| nocturnal_consolidation, consolidation_dream | EXTENSION | `extensions/dream` | funciona só em janela longa (Run 1515) (f6 §4) |
| checkpointer | KEEP | `platform/persistence` | lineage: parent_run_id/hash/versão; `latest` deixa de ser verdade (f2 §32) |
| generative_decoder, language_predictor, linguistic_prediction | EXTENSION | `extensions/language` | f1 §11 |
| neural_processor, paleocortex, limbic_system | ARCHIVE⚠ | `extensions/full_brain` (após ADR) | estruturas de cérebro completo (f1 §50) |
| meta_goals, hierarchy, recursion_lifecycle, confidence_carryover, periodic_decay, chat_system_sync | MERGE/ARCHIVE⚠ | distribuir (telemetria/contracts) ou `legacy/research` | 26 módulos L5-de-fato default L3 (f6 §1) — decidir na Fase 0 |

## 7. Learning (transversal — hoje o elo rompido)

| Legado | Classe | Destino novo | Notas |
|:--|:--|:--|:--|
| learning (413 interrupções; learned=0) | KEEP (reescrito) | `learning/{prediction_error,credit,update,validation,transfer,forgetting}` | LearningEnvelope com `verified_future_effect` (f4 §33; f6 F-2) |
| HML/meta_learning (persistência 146→157) | KEEP | `learning` + `l5_meta/meta_controller` | contrato de 9 campos cross-run (f4 §25) |

## 8. Runtime / infraestrutura

| Legado | Classe | Destino novo | Notas |
|:--|:--|:--|:--|
| `system.py` (17.544 linhas, grau 308/455) | MERGE (desmontar) | `runtime/{scheduler,executor,event_bus,lifecycle,typed_context,tracing}` + `triad/` | orquestrador monolítico — a cola manual vira contratos (f5 §4) |
| frequency_scheduler + TissueAwareScheduler | KEEP (reescrito) | `runtime/scheduler` | scheduler ≠ prova de atividade; admission control novo (f4 §29) |
| cognitive_pipeline (PipelineRegistry) | MERGE | `runtime/executor` | dirty/effect provider "SEM_PROVEDOR" → reais (f6 §4) |
| event_driven.py (TissueEventBus) | KEEP | `runtime/event_bus` | eventos pequenos com dirty_mask (f4 §34) |
| l1_state_contract | KEEP | `contracts/state` + `runtime/tracing` | o padrão de proveniência que o core inteiro herda (f3 §12) |
| runtime_observability | KEEP | `platform/telemetry` | E0–E5 por módulo; derive_proof exige cadeia completa (f5 §12) |
| persistence/checkpoint (`latest`) | KEEP (reescrito) | `platform/persistence` | lineage/hash/schema_version (f2 §32) |
| GPU kernels (RX 580) | KEEP | `platform/gpu` | L1–L3 alimentam GPU; Lua nunca toca (f2 §31; f4 §37) |
| `PIPELINE_CASCADE_INTERVAL` (chave órfã) | ARCHIVE | substituída pelo contrato descendente | f6 F-1 |

## 9. Não-destinos explícitos (f1 §50)

Fora da primeira reescrita, sem classificação individual: language stack
completo, chat sync, dream/inner speech, nocturnal consolidation, cerebellum/
brainstem/basal ganglia/paleocortex, symbol/causal ecology separadas,
ecologies hub, architecture cortex, neural processor genérico, cognitive
field independente, cross-module resonance independente, cognitive genome
contínuo, structural noise contínuo, entropic injection contínua →
`extensions/` ou `legacy/research/` (ADR-0006).
