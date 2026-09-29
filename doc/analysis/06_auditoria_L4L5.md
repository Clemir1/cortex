# Análise — L4L5.md (fonte 6): auditoria funcional L4 × L5

> **Fonte:** `C:\Pictures\TB\tb\LAB\Triad_AEE\L4L5.md` · 316 linhas · 7 seções.
> **Papel:** auditoria funcional completa de L4 (cognição global) × L5
> (metacognição) e suas dependências de L1–L3 — código lido integralmente,
> records de runs 1789–1791/1799/1800 sondados, zero edição de core, zero
> runs novas. É a fonte mais dura da base: **prova empírica, por records, de
> que a cascata descendente e o learned não existem no legado.**

## 1. As três verdades paralelas (taxonomia fragmentada)

1. **Oficial:** L4 = 20 módulos declarados; L5 = 18 declarados. Pipelines:
   bio="L1-L2", cog="L3-L4", meta="L5".
2. **Runtime:** **26 módulos executam no meta_pipeline (L5-de-fato) sem entrada
   no HIERARCHY_LEVELS** (self_engineering, cross_run_persistence, paleocortex,
   limbic_system, causal_cycle, federation_hub, thalamus_governor,
   meta_regulator, architecture_governor, governance_orchestrator etc.) —
   contados como **L3 por default** nos resumos, fora do acelerador MetaAw,
   exibidos como "L5*" INFERRED.
3. **ecology_observer usa outra taxonomia** (6 níveis: L1_Bio, L2_Semantic,
   L3_Attentional, L4_Identity, L5_Predictive, L6_MetaCog) com semânticas
   diferentes. Dois mapas coexistem sem campo de proveniência. E o checklist
   EX- nunca cita a spec L1AL5: agendas paralelas.

Mecânica do escalonamento: energia <0.3 **desacelera tudo ×1.5**; com o atrator
adulto ~0.21, todo intervalo L4/L5 fica permanentemente ×1.5 (world_model 2→3,
nocturnal 30→45, checkpointer 100→150). Orçamento: quotas L1–L2 25% / L3–L4
35% / L5 25% / overhead 15%.

## 2. Prova empírica pelos records (runs 1790/36 e 1800/80 steps)

- **L3toL4 = 0 eventos em todos os steps** no tracker de fronteiras — com 0
  reportado é indistinguível "sem fluxo tipado" de "não instrumentado".
- **L5toL4 = 0 eventos em todos os steps** (mesmo com L4toL5 recebendo 6.821
  inputs) — **a cascata descendente não existe**.
- **learned = 0 nas 6 fronteiras, em todos os steps** — o quinteto
  input→accepted→acted→effect fecha, o 5º elo (learned) nunca.
- `cross_level_consumption`: razão global de consumo = **0,0%** (3 de 544
  outputs lidos) — a dependência L1–L3→L4/L5 corre pelo `_ctx`, não pelo canal
  tipado; a má classificação dos 26 módulos distorce a métrica.
- `recursion_closure` nativo: L5 `{down=applied 36–78, return=0, closed=0,
  status=HIPÓTESE, "efeito_para_baixo_sem_retorno"}`; L1–L4
  `{down=0, status=LACUNA}` — o instrumento confirma a lacuna nº 1.
- **L4 desligada no regime adulto:** GW suspensa 2/3 da run (25 exec × 53
  SHUTDOWN), unified_identity 11 exec × 53 SHUT; reativação exige energia
  >0.60 com piso real 0.21 (gap 0.39). "O organismo sacrifica o barato-crítico"
  (GW 1,26 s × morphogenesis 231 s). world_model e attention_binding 78/78.
- **L5 quase inteira desligada:** 13/18 suspensos na crise 1800; 6 L5 sem
  contadores de execução (invisíveis); nocturnal_consolidation NOT_EXECUTED
  em 5/5 runs (intervalo ×1.5 > janela); cognitive_economy nunca suspensa.
- **F2 é o gargalo e piorou:** 22,9% (spec) → ~8–9,7% (runs 1533–39) →
  **6,7%** (1800). Pipeline de decisão sem instrumento de perdas por
  fronteira (sem razão tipada).
- WM sem uso de história (`wm_history_consulted` 80×0); hierarquia rasa
  (depth 1; 20.593 nodes → 266 abstratos); promoções semânticas congeladas
  (12) com proteção acumulando (9.289) — fossilização.
- Aresta A5 identidade→WM **morta por construção** (escritor dentro do
  runner suspenso → `ui_memories_retrieved` sempre 0); Self=1,0 ×
  Narrative=0,093.
- Custo: meta_pipeline 33–34 s (36 steps) e 96–113 s CPU (80 steps) —
  problema da spec §23 persiste; depósito L5 é o pipeline mais chamado.
- Positivo: decisão com identidade REAL (decision_id, carimbo causal,
  effect_observed, anti-stale; 4 estágios provados, 8 trajetórias na 1529);
  World Model único E5 da matriz (E4=64/E5=1); F1/F2/F3 medidas por step;
  GW barato; longitudinal_gate existe e **rejeita** (83 julgamentos) — o
  veredito existe, o enforcement não.

## 3. L5 — censo funcional

**Funciona com prova:** cognitive_economy com efeito físico real (taxas,
mortes/predadores de conceitos, 6 controladores de homeostase); nocturnal_
consolidation funciona em janela longa (Run 1515: 8 episódios APPLIED 1.0);
governança com ttl/acceptance/cooldown; law_enforcer com sanções;
self_engineering tem todos os componentes (Proposer→Sandbox→Validator→
Incorporator).

**Falta (priorizado):**
1. **Feedback descendente inexistente como mecanismo** — `PIPELINE_CASCADE_
   INTERVAL` é **chave órfã** (1 match no core: só a declaração); "FASE 51
   L5→L1" é apenas zombie_warning; L5 só desce via 2 throttles e sanções.
2. Enforcement "fase futura": o longitudinal_gate nunca bloqueia aplicação;
   do critério detectar→formular→aplicar→medir→**aprender→persistir→reverter**
   faltam os 3 últimos elos.
3. Metacognição sem efeito regulatório: MetaAw produtor ~0.88 congelado ×
   consumidor 0.000 (EX-6.2 aberto).
4. L5 não incremental: cognitive_economy 20,7 s × GW 1,26 s; acumuladores
   L1–L4 da spec §24 não existem; `dirty_provider`/`effect_provider`
   "SEM_PROVEDOR" no próprio PipelineRegistry; custo do controlador nunca
   medido contra a economia poupada.
5. HML persistência incompleta (146→157 vs anterior maior; 9 campos não são
   contrato vigente).
6. Self-engineering não fecha o loop: approved == incorporados (proposta
   nunca incorporada em 36 steps).
7. L5 muda nas janelas de veredito: intervalos ×1.5 nunca disparam em baterias
   curtas; forgetting/periodic_decay suspensos sem retorno → fossilização
   (+461 conceitos / 0 mortes na 1800).
8. Admission control inexistente (sem urgência/valor esperado/custo antes de
   módulos caros).

## 4. Mapa do que está faltando (F-1..F-12)

| # | Lacuna | Classe |
|--:|:--|:--|
| F-1 | Feedback descendente L5→L4→L3→L2→L1 | COMPORTAMENTAL (LIBERADO) |
| F-2 | LearningEnvelope + learning_status por decisão (o E5) | COMPORTAMENTAL + OBS |
| F-3 | L4 viva no regime adulto (política de crise) | COMPORTAMENTAL |
| F-4 | Classificação única (26 runtime + 2 mapas paralelos) | RATIFICAÇÃO display + gate |
| F-5 | WM preditivo com histórico (3 camadas) | COMPORTAMENTAL |
| F-6 | L5 incremental (acumuladores + dirty provider) | COMPORTAMENTAL + OBS |
| F-7 | Aresta A5 identidade (escritor fora do runner suspenso) | COMPORTAMENTAL |
| F-8 | Efeito regulatório metacognitivo (O2 completo) | COMPORTAMENTAL |
| F-9 | Self-engineering keep/revert + HML cross-run | COMPORTAMENTAL |
| F-10 | Admission control + L5 event-driven (anomalia acorda L5) | DESENHO |
| F-11 | Baterias FASES 6–7: matriz E5, ablação L1…L1–L5, adulta 200–300 steps, loop L1→L5→L1 | DESENHO EXPERIMENTAL |
| F-12 | Perdas por fronteira com razão tipada (diagnóstico de F2) | OBS-PURA |

## 5. Receitas L45-R1..R8 (cada uma com predição falsificável)

- **R1 (OBS-PURA):** record publica por módulo o par nível declarado×efetivo +
  razão tipada da perda por fronteira + runs/skips por nível individual.
  Falsifica: onde F2 perde; se L3toL4=0 é "sem fluxo" ou "não instrumentado".
- **R2 (RATIFICAÇÃO + gate separados):** (a) display agrega pelos
  PIPELINE_MAP sem tocar `should_run`; (b) preencher HIERARCHY_LEVELS
  habilita o acelerador MetaAw — COMPORTAMENTAL, exige gêmea A/A.
- **R3 (COMPORTAMENTAL, LIBERADO):** feedback descendente mínimo viável —
  `governance_orchestrator` com o contrato completo da spec §28
  (governance_id, target_layer, target_parameter, old/new, reason,
  expected_effect, ttl, confidence, observed_effect, status), descendo **um
  degrau por fase** (L5→L4 primeiro). Predição: intervenção L5 muda 1
  parâmetro L4 com observed_effect e keep/revert.
- **R4 (DESENHO):** LearningEnvelope = fusão de eventos existentes
  (decision_id + wm_outcome + learning_update + effect_validation +
  learning_status por decisão).
- **R5 (DESENHO EXPERIMENTAL):** ablação L1/L1+L2/L1–L3/L1–L4/L1–L5 com
  steps ≥60, braço 150+; métricas pré-declaradas.
- **R6 (COMPORTAMENTAL + OBS):** L5 incremental — acumuladores L1–L4 (9
  chaves) por camada + dirty_provider real. Predição: custo de
  cognitive_economy cai de O(N) para O(1) sem mudar outputs.
- **R7 (OBS-PURA):** unificar taxonomias (scheduler 5 × ecology 6) com campo
  `source=REGISTRY/SPEC6` no display.
- **R8 (DESENHO EXPERIMENTAL — teste final):** par L5-ON × L5-OFF (mesma
  seed/ágenda, janela adulta ≥60): se L5 aumenta custo sem melhorar
  aprendizagem/adaptação, **não está funcional**. Veredito pré-registrado,
  sem calibração pós-observação.

## 6. Lições mapeadas para a nova estrutura

| Achado | Onde vive na nova estrutura |
|:--|:--|
| Taxonomia fragmentada (26 módulos "L3 por default") | `ModuleDescriptor` único: region/layer/domain/entity_kind/pipeline (`contracts`) |
| learned=0 em toda fronteira | cadeia de learning com `validation` como elo terminal obrigatório |
| L5toL4=0 / chave órfã de cascata | caminho descendente Policy→GlobalConstraint→LocalGoal→ResourceAllocation→ParameterAdjustment como contrato |
| L4 suspensa no regime adulto | modo degradado obrigatório + crise alterando política (nunca SHUTDOWN de módulos L4) |
| Escala ×1.5 permanente por escassez | admission control + scheduler event-driven (L5 acorda por anomalia) |
| L5 não incremental (O(N)) | acumuladores por camada + `MetricRegistry` versionado |
| GW barata sacrificada em favor de morphogenesis cara | ResourceGovernor com custo/valor esperado por módulo antes de rodar |
| Efeitos provados fora da obs | telemetria E0–E5 por módulo como critério de aceite desde o marco 1 |
| Graphify: governança ilhada dos módulos regulados | contratos tipados entre L5 e alvos (nunca import direto) |
