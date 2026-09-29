# Análise — L1AL5.txt (fonte 4): reconstrução L1–L5

> **Fonte:** `C:\Pictures\TB\tb\LAB\Triad_AEE\L1AL5.txt` · 1.104 linhas ·
> 50 seções + arquitetura-alvo.
> **Papel:** diagnóstico temporal + prescrição da reconstrução L1–L5 do
> legado. É a ponte entre as auditorias (estado) e a nova estrutura (alvo):
> define contratos por camada, ownership de estado, LearningEnvelope e a
> ordem de reconstrução (Fases 0–7).
> **Tipo:** análise arquitetural (prescrição), com métricas de run citadas.

## 1. Tese central

Reconstruir L1–L5 **agora**, antes de ampliar população, linguagem ou número
de módulos. O problema não é falta de componentes: é distribuição de
responsabilidades, contratos entre níveis e fechamento dos ciclos. L1–L5 são
níveis funcionais da hierarquia cognitiva; as 5 Ordens Cibernéticas são eixo
transversal distinto (relação matricial, §41 — ex.: a energia atravessa O1–O5).

## 2. Diagnóstico da run (evidências citadas)

- Scheduler: L1=354, L3=4505, L4=634, L5=153 execuções — **L2 ausente do
  resumo** da CognitiveHierarchy (a arquitetura pretendida sempre foi
  L1 Biology → L2 Tissue → L3 Local → L4 Global → L5 Meta).
- Transferência: F1 conceito→workspace 44%→**63,6%** (melhorou); F2
  workspace→decisão ~41%→**22,9%** (piorou); F3 decisão→execução 88,9%.
  **O gargalo migrou para L4.**
- Breakpoint causal: `learning` com 413 interrupções (vs ~zero em workspace e
  execução); 921 ciclos completos × 2.952 parciais × 413 interrupções no
  fechamento action→outcome→learning.
- L1: energia controlada (~0,21–0,22), mortalidade zero, conservação OK;
  mas HebbianNorm=0, JLdim=0, repair 1.999 clusters no step 30, dormancy 939,
  resiliência 1,00→0,84 (estado real misturado com métricas ambíguas).
- L2: 149 tecidos constantes (não prova lifecycle); TissueCoherence ~0,99 ×
  CohX caindo (ótima organização interna × integração insuficiente entre
  tecidos); ciclo topológico connectivity rebuild (avgDeg 17–18) →
  morphogenesis (avgDeg ~12) → overshoot.
- L3: mais ativa (~93,6% do scheduling); 1.059 conceitos, 784 anchors, binding
  ~0,768, 74%/85% de reuso; **regressão**: MemCap 0,0506→0,0389; Recurrence
  0,408→0,189→0,307; hierarquia presa em depth 2 (263 conceitos hierárquicos).
- L4: Self=1,0 × Narrative=0,093 (desequilíbrio de identidade); World Model
  com 111 links causais mas **0 steps de predição usando histórico**.
- L5: 153 execuções (~18,3%) não é defeito (meta-cognição deve ser lenta); o
  problema é custo (meta-pipeline >100 s; Cognitive Economy e cross-run
  persistence dominando) e persistência meta incompleta (acordou 146→157 vs
  run anterior maior).
- RAM 1,31→1,65 GB com população constante (ownership de memória ausente).

## 3. Prescrições por camada (contratos e critérios de conclusão)

**Contrato comum L1–L5 (§1):** cada nível tem 5 elementos obrigatórios —
entrada, estado próprio, processamento, saída, **feedback recebido
posteriormente**. Sem acesso indiscriminado a estruturas internas de outras
camadas.

- **L1** (§2–3): estados canônicos `BiologicalState/EnergyState/
  ActivationState/HomeostaticState/PlasticityState/StressState/LifecycleState`;
  data-oriented (arrays contíguos, batch, dirty clusters; custo O(active), não
  O(total)); eventos pequenos (`cluster_id, version, event_type, dirty_mask`)
  — nunca vetor 97D no evento. Concluída quando: homeostase fechada, Hebbian
  modifica pesos ou declara BYPASSED, dormancy/repair com histerese, sem onda
  global sem causa, processamento incremental.
- **L2** (§4–8): reunir tecidos, afinidade, morfogênese, conectividade, hubs,
  rotas, SpatialHash, repair estrutural, dormancy regional, especialização,
  recursos locais sob **um contrato L2** (hoje dissolvidos); estados
  `TissueState/TopologyState/RegionState/LocalRouteState/MorphogenicState/
  TissueResourceState`; lifecycle de tecido FORMING→ACTIVE→SPECIALIZED→
  CONSOLIDATED→DORMANT→MERGE/SPLIT→RETIRED; **bridges adaptativas** entre
  tecidos (`cross_tissue_bridge`, strength, reason, age, utility, decay) —
  coalizões temporárias sem fusão; **um único TopologyController** com
  contrato (target_degree_range, min/max_degree, utility semântica/causal,
  energy_cost, histerese). Concluída quando: aparece explicitamente no
  scheduler, lifecycle real, topologia convergida, CohX estável, repair
  incremental, nada força scan completo.
- **L3** (§9–14): transformar dinâmica em representação (HOTM, reservoir,
  recorrência, emergence, anchors, conceitos, composição, memórias locais,
  predição local, pattern completion, binding, abstração inicial); produzir
  candidatos a L4; separar `WorkingMemory/TemporalTrace/ReservoirState/
  SemanticLongTermMemory/EpisodicMemory`; recorrência por circuito
  (circuit_id, strength, causal_yield, memory_gain, prediction_gain,
  sleep_survival...); hierarquia como DAG feature→proto-concept→concept→
  composite→category→abstraction→meta-abstraction com promoção por reuso/
  persistência/utilidade causal — não por contador.
- **L4** (§15–21): exclusivamente GW, atenção global, competição, broadcast,
  World Model, reconsolidation global, unified identity, semantic routing,
  decisão global; pipeline explícito candidate→eligibility→attention score→
  competition→coalition→broadcast→decision candidate→arbitration→execution
  request **com perdas registradas por fronteira**; decisões com identidade
  (decision_id, origin_event, source_concepts, workspace_contents,
  wm_prediction, identity_context, limbic_modulation, expected_outcome,
  confidence, action, actual_outcome, prediction_error, learning_status);
  World Model em 3 camadas (Short-Term Predictive, Episodic Transition,
  Cross-Run Causal) com seleção contextual de evidência; identidade separada
  (SelfState, NarrativeIdentity, AutobiographicalMemory, IdentityPrediction,
  IdentityConstraints); linguagem tardia.
- **L5** (§22–30): observar e modificar regras, não pensar no lugar;
  consumir **resumos incrementais** (acumuladores L1–L4: active_count,
  energy_budget, memory_pressure, compute_pressure, attention_pressure,
  causal_yield, prediction_error, queue_pressure, dirty_ratio) em vez de
  percorrer clusters; meta-experiência com 9 campos persistidos cross-run;
  exploração orientada por prediction error/uncertainty/novelty/info gain/
  cost/risk; self-engineering como experimento controlado (baseline→
  hypothesis→patch→sandbox A-B→metrics→causal attribution→accept/reject→
  rollback→persistence); governança por contratos (governance_id, target_layer,
  target_parameter, old/new_value, reason, expected_effect, ttl, confidence,
  observed_effect, status); admission control antes de módulos caros
  (urgency, valor esperado, custo, deadline, dependência, pressão).

## 4. Transversais prescritos

- **Caminho ascendente:** L1 BiologicalEvent → L2 TissueEvent → L3
  CognitiveCandidate → L4 GlobalContent → L4 Decision → Action; `_ctx`
  substituído gradualmente por interfaces tipadas/versionadas.
- **Caminho descendente:** L5 Policy → L4 GlobalConstraint → L3 LocalGoal →
  L2 ResourceAllocation → L1 ParameterAdjustment (fecha governança sem
  manipulação direta de milhares de clusters).
- **LearningEnvelope (§33):** event_id, decision_id, prediction, action,
  expected_outcome, actual_outcome, error, credit_assignment, updated_modules,
  memory_update, policy_update, **verified_future_effect** — só com este o
  ciclo é contabilizado como fechado.
- **Event bus (§34):** eventos pequenos com referências (event_id, parent_id,
  entity_id, entity_version, event_type, priority, deadline, dirty_mask);
  estado permanece no owner.
- **Ownership (§35):** L1 fisiologia; L2 tecidos/topologia; L3 representações/
  memórias locais; L4 estado global/WM/decisões; L5 políticas/meta-história.
- **Concorrência (§36):** dividir por perfil — Tokio (eventos/filas/I/O),
  Rayon (CPU-bound/arrays), WGPU (hot paths numéricos), Python (controle/
  experimentação). GPU alimentada por L1–L3, não por governança L5.
- **Memória (§38):** bounded histories; cada camada publica resident_bytes,
  allocated_bytes, cache_bytes, history_bytes, growth_rate.
- **Métricas (§39):** `MetricRegistry` incremental — producer calcula uma
  vez, consumidores leem snapshot versionado (hoje energia/coerência/entropia/
  diversidade/yield/scarcity são recalculados por várias camadas).
- **Temporalidade (§40):** L1 rápida → L5 muito lenta/condicional; scheduler
  responde a eventos (L5 acorda em anomalia crítica), não apenas intervalos.

## 5. Ordem de reconstrução e testes (§42–48)

Fases: 0 contratos/IDs/ownership/versionamento/observabilidade → 1 L1 →
**2 L2 (prioridade estrutural: está invisível)** → 3 L3 → **4 L4 (prioridade
cognitiva: F2=22,9%)** → 5 L5 → 6 loop completo L1→L5→L1 → 7 benchmark/
ablation/cross-run. Testes de ablação cumulativos (L1; L1+L2; L1–L3; L1–L4;
L1–L5) com mesma seed/estímulos medindo Causal Yield, RCM, learning closure,
memory retention, generalization, adaptation, runtime, RAM, VRAM. Teste final
de L5 (§47): par L5-ON × L5-OFF — se L5 aumenta custo sem melhorar
aprendizagem/adaptação, **não está funcional**.

## 6. Previsão e arquitetura-alvo (§49–50)

- Sem reconstrução: semântica↑ bindings↑ histórico↑ RAM↑ estabilidade↑
  throughput↓ MemCap↓ plasticidade↓ — sistema mais organizado, porém mais
  pesado e rígido.
- Com reconstrução (ownership, eventos pequenos, dirty processing, hierarquia
  temporal, feedback fechado): mais cognição por cluster, mais reuso, maior
  profundidade, menos redundância, maior causal yield, menor custo por
  unidade cognitiva.
- Alvo: L5 META-COG / L4 GLOBAL / L3 LOCAL / L2 TISSUE / L1 BIOLOGICAL, com
  transversais O1–O5 + Event Bus + Canonical State + Telemetry + Persistence
  + CPU/GPU Scheduler. Prioridade: **reconstruir L2** (identidade diluída),
  depois **L4** (F2 22,9%), e fechar `outcome→learning` — só depois ampliar
  população/linguagem/profundidade.

## 7. Lições mapeadas para a nova estrutura

| Prescrição | Onde vive |
|:--|:--|
| Contrato comum de 5 elementos + ownership por camada | `architecture/contratos_camadas.md` + `crates/contracts/` |
| Estados canônicos L1/L2, lifecycle de tecido, TopologyController | `crates/l1_substrate/`, `crates/l2_tissue/` |
| Pipeline L4 com perdas por fronteira + decisão com identidade | `crates/l4_global/{integration,decision}` + telemetria E0–E5 |
| LearningEnvelope com `verified_future_effect` | `crates/learning/validation` (elo terminal obrigatório) |
| Acumuladores incrementais L1–L4 | `runtime/typed_context` + `MetricRegistry` em `platform/telemetry` |
| Governança declarativa por contratos (governance_id, ttl, observed_effect) | `crates/l5_meta/meta_controller` + `crates/governance/` |
| Eventos pequenos com dirty_mask; estado fica no owner | `crates/runtime/event_bus` + `contracts/events` |
| Bounded histories + bytes por camada | `platform/persistence` + leis (invariantes) |
| Ablação L1…L1–L5 e L5-ON×L5-OFF | `migration/plano_marcos.md` (critérios de aceite por marco) |
