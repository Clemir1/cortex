# Contratos de camadas L1→L5 e cadeias canônicas

> Normativo para a nova estrutura. Deriva de: f1 §19–22/§26–43/§53–62
> (cadeias e contratos), f2 §10/§50–52 (contratos tipados, mensagens),
> f4 §§1–35 (contrato comum, LearningEnvelope, ownership), f5 P11 (contratos
> L2↔L3), f6 R3 (contrato de governança). Implementação: `crates/contracts/`.

## 1. Contrato comum de camada

Cada camada possui 5 elementos obrigatórios (f4 §1): **entrada; estado
próprio; processamento; saída; feedback recebido posteriormente**. Nenhuma
camada acessa estruturas internas de outra — comunicação só por eventos e
mensagens tipadas do `crates/contracts`.

**Ownership de estado (f4 §35):** L1 fisiologia dos clusters; L2 tecidos/
topologia; L3 representações e memórias locais; L4 estado global, World
Model, decisões; L5 políticas e meta-história. Outras camadas têm leitura ou
comando — nunca escrita direta. Atuação cross-layer é **declarativa**
(`actuation_targets`, f5 §11 contra o CORRECAO §53).

**Envelope de mensagem (f2 §50, f4 §34):** eventos pequenos com referências —
`event_id, parent_event_id, entity_id, entity_version, event_type, priority,
deadline, dirty_mask`; o estado permanece no owner. Todo valor tipado carrega
`value, status, source, step, version, confidence, provenance`. Cascata =
rastreio desses campos no EventBus — não um sistema (f1 §41).

## 2. Fronteiras ascendentes

| Fronteira | Contrato (campos mínimos) | Fonte |
|:--|:--|:--|
| L1→L2 | `ClusterStateRef`: producer_id, step, state_version, state_phase (`POST_PHYSICAL/POST_TISSUE/PRE_COGNITIVE/POST_COGNITIVE`), population_total, state_dimension, sample_count, sample_policy, coverage, mean_state_hash, cluster_order_hash, provider_status, no_data_reason | f3 §5/§12 |
| L2→L3 | `TissueEvent` versionado: tissue_id (rastreio per-tecido opcional, f5 P5), event_id, state_hash, valid_until, parent_event_id, feedback_id; + `TissueState` (id, specialization, members, energy, coherence, capacity, connections) | f5 §3.2; f1 §7 |
| L3→L4 | `CognitiveCandidate → WorkspaceCandidate`: conceitos/binding/predição com proveniência da versão L1 consumida | f4 §31; f1 §56 |
| L4 interno (decisão) | `WorkspaceContent → DecisionProposal → DecisionSelected → DecisionCommitted` — competição com **perdas tipadas por fronteira** (candidate→eligibility→attention→competition→coalition→broadcast→arbitration) | f1 §19; f4 §16; f6 F-12 |
| L4→ação | `ActionRequest → ActionExecuted → OutcomeObserved` | f1 §20; f2 §24 |
| Ação→learning | `LearningEnvelope`: event_id, decision_id, prediction, action, expected_outcome, actual_outcome, error, credit_assignment, updated_modules, memory_update, policy_update, **verified_future_effect** — sem este campo o ciclo NÃO é contado como fechado | f4 §33; f6 F-2 |

**Decisão com identidade (f4 §17):** decision_id, origin_event, source_
concepts, workspace_contents, world_model_prediction, identity_context,
limbic_modulation, expected_outcome, confidence, action, actual_outcome,
prediction_error, learning_status.

## 3. Fronteiras descendentes (a cascata que não existe no legado)

    L5 PolicyProposal
    → L4 GlobalConstraint (attention/decision constraints)
    → L3 LocalGoal (salience/learning targets)
    → L2 ResourceAllocation (AdaptationRequest)
    → L1 ParameterAdjustment (plasticity/structural)

**Contrato de governança (f4 §28; f6 R3):** cada intervenção carrega
`governance_id, target_layer, target_parameter, old_value, new_value, reason,
expected_effect, ttl, confidence, observed_effect, status`. MetaController
nunca aplica direto: observe→propose→validate→apply→observe_effect→keep/revert
(f1 §25). Lei de terminação: intervenção sem `observed_effect` dentro do ttl
é revertida automaticamente.

**AdaptationRequest L3→L2 (f5 P11):** tipo first-class (requested_delta,
feedback_id, source_concepts, step, state_version_consumed); no legado só
existia `hml_tissue_signal` não-tipado com arbitragem opt-in. **Default ON
na nova estrutura** (decisão D2 da síntese), com CouplingArbiter como
aplicador de banda/histerese — não como gate de existência.

## 4. Cadeias canônicas (toda mensagem deve citar em qual cadeia entra)

1. **Cognitiva:** Input→L1State→TissueState→Representation→WorkspaceCandidate→
   WorkspaceSelection→WorldPrediction→DecisionProposal→DecisionCommit→Action→
   Outcome→PredictionError→CreditAssignment→LearningUpdate→FutureBehavior
   (f1 §56). Módulo que não souber sua posição precisa justificar a existência.
2. **Memória:** Experience→Encode→Store→Retrieve→**Consume**→Decision/
   PredictionEffect→Reconsolidate (sem Consume não é memória útil — f1 §57).
3. **Metacognição:** ObserveSystem→DetectProblem→GenerateProposal→
   ValidateProposal→Apply→ObserveEffect→Keep/Revert (f1 §58).
4. **Desenvolvimento:** CapacityProblem→DevelopmentGovernor→
   RegenerationProposal→LawValidation→Morphogenesis→StructuralEffect→
   CognitiveEffect→StopGrowth (f1 §59).
5. **Federação:** Need→Proposal→Agreement→ResourceTransfer→**Effect**→
   Settlement (f1 §60).
6. **Lei:** Invariant→Observation→Violation→Enforcement→**Postcondition**→
   EffectValidated (f1 §61).
7. **Ecológica:** Variation→Competition→Selection→Survival/Extinction→
   Retention→FutureEffect (f1 §62).

## 5. Leis-contrato (invariantes de fronteira, f1 §36 + lições das auditorias)

- Ausência ≠ zero: `NO_DATA`/`STALE`/`INVALID` nunca viram `VALUE`
  (f3 P0; f6 no_stale_as_value).
- Fallback nunca é publicado como valor medido (f3 P0).
- `FALLBACK` estrutural exige `fallback_reason` e `provider_id` (f3 §6).
- Observação **nunca muta** estado observado (f5 achado 3 — bridges na fase
  de record; f3 §17 cadência de records).
- Learning só recebe E5 após consequência observada (f1 §36; f6 R4).
- Nenhum controlador altera actuator que não possui (f1 §36) — atuação só
  via contratos descendentes com `observed_effect`.
- Módulo crítico (L4) nunca fica SUSPENDED sem razão explícita — modo
  degradado obrigatório (f1 §12; f6 F-3).
- Checkpoint exige lineage válida: parent_run_id, checkpoint_hash,
  state_hash, schema_version (f2 §32; f1 §36).
- Toda taxa publicada com denominador (razão tipada — f6 R1).

## 6. Escada de evidência por contrato

Cada fronteira publica o quinteto `input_events/accepted/acted/effect/
learned` + recibo no ledger `producer→key→version→write_step→consumer→
read_step→valid→stale→fallback→output_version→effect_id→effect_step`
(f3 §7.2; f6 §2). Classificação: `declared→initialized→eligible→executed→
productive→consumed→effect_validated` (E0–E5). Promoção de qualquer elo exige
efeito identificado em t+1/t+5 com braço sham pareado quando houver mudança
comportamental (f3 §8; gêmeas bit-idênticas para mudanças de display).

## 7. Regras de degradação (crise)

Máquina de estados transversal: NORMAL→PRESSURE→CRISIS→STABILIZATION→
RECOVERY→REINTEGRATION altera **políticas** (gains, budgets, períodos), nunca
desliga sistemas (f1 §44; síntese D5). Durante crise: morphogenesis↓,
division↓, telemetry pesada↓, persistência pesada↓, O5↓, linguagem OFF,
experimentos OFF. Preservar sempre: L1 homeostase, L2 tecidos, L3 memória/
semântica, L4 Workspace MINIMAL + World Model DEGRADED, decision, outcome,
learning. O legado provou o anti-padrão: energia <0.3 desligava GW e 13/18
módulos L5 (f6 §3) — na nova estrutura isso é violação de lei, não política.
