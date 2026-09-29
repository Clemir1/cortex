---
name: triad-aee-engineering
description: >-
  Skill profissional para investigar, modificar, validar e evoluir o Triad_AEE,
  um sistema cognitivo distribuído, adaptativo, evolutivo e emergente. Coordena
  análise arquitetural, causal, cognitiva, cibernética, temporal, de performance,
  GPU/WGPU/OpenCL, persistência, observabilidade, aprendizagem e experimentos A/B
  determinísticos. Use em correções, profiling, refactors, novos controladores,
  métricas, loops cognitivos, validação de hipóteses, evolução de arquitetura e
  preparação de baterias longas.
disable-model-invocation: false
---

# Triad_AEE Engineering

## O que é

Skill de engenharia, ciência de sistemas cognitivos e coordenação para o **Triad_AEE**.

O objetivo não é apenas fazer o código funcionar. O objetivo é preservar e aumentar simultaneamente:

- correção computacional;
- coerência arquitetural;
- fechamento causal e cibernético;
- evidência cognitiva;
- determinismo experimental;
- desempenho e escalabilidade;
- auditabilidade científica;
- compatibilidade com persistência e runs anteriores.

O agente deve tratar o Triad_AEE como **um sistema cognitivo distribuído experimental**, não como uma coleção independente de scripts.

## Entrada

A skill pode receber, isoladamente ou em conjunto:

- código-fonte;
- logs;
- profiling;
- relatórios de agentes/subagentes;
- commits/diffs;
- resultados de gêmeas;
- configs/flags;
- artefatos de persistência;
- métricas cognitivas;
- traces causais;
- relatórios GPU;
- checklist/board;
- hipótese técnica ou científica.

## Saída

A execução deve produzir, conforme a tarefa:

- diagnóstico epistemicamente classificado;
- mapa causal/arquitetural;
- hipótese testável;
- plano de instrumentação;
- patch mínimo;
- desenho A/B;
- critérios de aceitação;
- relatório de regressão;
- verdict final;
- atualização de board/checklist;
- próximos gargalos priorizados.

---

# Princípios centrais

1. Não confundir execução com cognição.
2. Não confundir estrutura com função.
3. Não confundir camadas cognitivas com pipelines.
4. Não confundir as 5 Ordens da Cibernética com L1..L5.
5. Não promover hipótese a fato sem isolamento causal.
6. Não misturar correção epistemológica, observabilidade, performance e comportamento no mesmo experimento.
7. Ausência de dado não é zero.
8. Toda mudança comportamental exige contrafactual.
9. Toda otimização deve declarar seu contrato de equivalência.
10. Linguagem permanece tardia.
11. Não criar módulo novo para compensar loop existente que ainda não fecha.
12. Não usar uma métrica isolada como prova de maturidade global.
13. Não usar contagem de clusters como medida de inteligência.
14. Preservar lineage experimental e artefatos inválidos como evidência metodológica.

---

# Modelo arquitetural canônico

## Camadas funcionais

- `L1` — Biologia / substrato computacional
- `L2` — Tecido / organização mesoscópica
- `L3` — Cognição local
- `L4` — Cognição global
- `L5` — Metacognição

Regras:

- `meta_pipeline` NÃO implica automaticamente `L5`;
- `generative_pipeline` NÃO implica automaticamente camada cognitiva;
- pipelines transversais podem usar `cognitive_layer = NONE`;
- a camada deve refletir função cognitiva, não localização de arquivo.

## Maturidade desenvolvimental

Escala separada:

- `S0` — pré-cortical
- `S1` — circuitos locais
- `S2` — integração distribuída
- `S3` — pré-neocórtex avançado
- `S4` — neocórtex funcional inicial
- `S5` — neocórtex funcional consolidado
- `S6` — sistema adaptativo maduro
- `S7` — generalista/autônomo

Nunca promover estágio por:

- número de clusters;
- SemCoh isolada;
- concepts/anchors isolados;
- uma run curta;
- um score agregado único.

---

# Evidência funcional

Usar obrigatoriamente:

- `E0 DECLARED`
- `E1 INITIALIZED`
- `E2 EXECUTED`
- `E3 PRODUCTIVE`
- `E4 CONSUMED`
- `E5 EFFECT_VALIDATED`

Definições:

`E0` — existe no registro/configuração.  
`E1` — inicializou corretamente.  
`E2` — executou.  
`E3` — produziu mudança válida ou output novo.  
`E4` — outro componente consumiu a saída.  
`E5` — efeito downstream observado e validado.

`run_count > 0` nunca é prova suficiente de função cognitiva.

---

# Fechamento causal/cognitivo

Todo loop relevante deve ser rastreável como:

`producer → signal → consumer → decision → actuator → effect → observer → learning → persistence`

Classificar:

- `COMPLETE_CHAIN`
- `PARTIAL_CHAIN`
- `BROKEN_CHAIN`
- `UNPROVEN_CHAIN`

Registrar sempre `broken_at` quando não completo.

---

# Pipeline da skill

```text
1. Reconciliação epistemológica        → [gate]
2. Baseline e reprodutibilidade        → [gate]
3. Observabilidade / mapa causal       → [gate]
4. Hipótese isolada                    → [gate]
5. Patch mínimo                        → [gate]
6. Prova curta determinística          → [gate]
7. Contrafactual A/B                   → [gate]
8. Validação adulta                    → [gate]
9. Consolidação / documentação         → [gate]
```

Cada gate exige evidência explícita antes de avançar.

---

# Gate 1 — Reconciliação epistemológica

Antes de editar:

1. identificar o claim;
2. identificar runs/commits/flags que o suportam;
3. verificar se os braços realmente diferiam;
4. verificar mudança de HEAD/config entre braços;
5. distinguir causalidade de correlação;
6. registrar limitações.

Estados:

- `CONFIRMED`
- `SUPPORTED`
- `STRONG_SUSPECT`
- `UNRESOLVED`
- `UNTESTED`
- `INVALID_COUNTERFACTUAL`
- `QUARANTINED`
- `REJECTED`

Nunca usar “inocentado” sem teste isolado.
Nunca usar “culpado” sem isolamento causal.

---

# Gate 2 — Baseline e reprodutibilidade

Registrar:

- `git_commit`
- `parent_commit`
- `working_tree_clean`
- `config_hash`
- `feature_flags`
- `seed`
- `run_id`
- `checkpoint`
- `python_version`
- `rust_build_hash`
- backend GPU real
- driver/device
- dataset/input hash
- steps
- cluster start/target

Para gêmeas:

- mesma seed;
- mesmo checkpoint;
- mesmo config;
- mesmo código;
- mesmos inputs;
- diferença apenas na variável experimental.

Abortar como `INVALID_COUNTERFACTUAL` se braços que deveriam diferir não diferirem de fato.

Baseline aceito somente quando:

- boot válido;
- run completa;
- hashes registrados;
- comparador funcionando;
- duas gêmeas do mesmo braço demonstram o determinismo esperado.

---

# Gate 3 — Observabilidade antes de correção

Instrumentar antes de alterar comportamento quando o gargalo não estiver localizado.

Observar:

- entrada;
- saída;
- contadores;
- latência;
- filas;
- skips;
- gates;
- fallback;
- drop reason;
- efeito downstream.

Telemetria pura deve:

- não consumir RNG;
- não alterar ordem de iteração;
- não materializar iterador destrutivo;
- não chamar getter com side effect;
- não escrever no estado cognitivo;
- não mudar scheduling.

Classificação de leitura:

- `DIRECT_FIELD`
- `PURE_COMPUTED`
- `CACHE_READ`
- `LAZY_COMPUTED`
- `SIDE_EFFECT_POSSIBLE`

Telemetria só é considerada inerte se ON/OFF preservar o estado cognitivo conforme o contrato experimental.

---

# Gate 4 — Formulação da hipótese

Toda hipótese:

`Se X é a causa, então ao alterar somente X devemos observar Y, enquanto Z permanece invariável.`

Definir antecipadamente:

- variável independente;
- variáveis dependentes;
- invariantes;
- critério de sucesso;
- critério de falha;
- confounders;
- primeira divergência esperada.

---

# Gate 5 — Patch mínimo

Preferir:

- menor diff possível;
- feature flag;
- sem refactor paralelo;
- sem métrica nova junto com alteração comportamental;
- sem otimização junto com correção causal.

Classificar a mudança:

- `OBSERVABILITY_ONLY`
- `BIT_EXACT_PERFORMANCE`
- `NUMERIC_PERFORMANCE`
- `BEHAVIORAL_FIX`
- `ARCHITECTURAL_CHANGE`
- `MIGRATION`
- `EXPERIMENTAL`

---

# Gate 6 — Prova curta

Antes da bateria adulta:

- compile/import;
- testes direcionados;
- boot;
- run curta;
- determinismo;
- contador esperado;
- efeito esperado;
- ausência de regressão imediata.

Uma run de 10–20 steps não prova estabilidade adulta.

---

# Gate 7 — Contrafactual A/B

Estrutura mínima:

- A1/A2 = baseline twins
- B1/B2 = treatment twins

Exigir:

1. A1 ≡ A2
2. B1 ≡ B2
3. A ≠ B apenas no efeito pretendido
4. invariantes críticos preservados

Registrar:

- primeira divergência;
- delta por métrica;
- delta temporal;
- delta de estado;
- trace causal da diferença.

---

# Gate 8 — Validação adulta

Depois da prova curta:

- janela longa;
- boundedness;
- estabilidade;
- recuperação;
- persistência;
- efeitos retardados;
- saturação;
- RAM/VRAM;
- filas;
- throughput;
- aprendizagem;
- recurrence.

Mudanças em governança, scheduling, learning, memória, energia, morfogênese e causalidade não viram default com prova curta apenas.

---

# Gate 9 — Consolidação

Atualizar:

- `AGENTS.md` — canal de coordenação entre sessões (recibo/declaração de cada frente);
- `CHECKLIST_MASTER_20092026.txt` — checklist mestre / agenda executiva das frentes;
- `CHECKLIST_25092026.txt` — checklist cognitivo reestruturado (investigação + frentes C0-C6);
- `CHECKLIST_19092026.txt` — histórico, permanece preservado como fonte;
- docs;
- status da tarefa;
- commit;
- runs;
- limitações;
- próximos experimentos.

Não apagar artefatos de experimentos inválidos.

---

# Governança experimental

Separar quatro categorias:

## A — Epistemologia
- errata;
- reclassificação de evidência;
- reconciliação de claims.

## B — Observabilidade
- métricas;
- traces;
- timers;
- contadores.

## C — Performance bit-exata
- hoist seguro;
- cache puro;
- redução de passadas;
- pré-cálculo sem mudar ordem/aritimética.

## D — Comportamento
- thresholds;
- prioridades;
- gates;
- controladores;
- learning;
- morphology;
- scheduling.

Não combinar categorias em um commit experimental.

---

# Instrumentação cognitiva

Não criar um único `cognition_score` como fonte de verdade.

Cada medida deve conter:

- `scope`
- `layer`
- `domain`
- `value`
- `raw_value`
- `valid`
- `evidence`
- `confidence`
- `coverage`
- `sample_count`
- `window`
- `source`
- `provenance`
- `metric_version`
- `no_data_reason`

## CognitiveScope

- `CLUSTER`
- `TISSUE`
- `RESERVOIR`
- `MODULE`
- `DOMAIN`
- `SYSTEM`

## CognitiveDomain

- `PERCEPTION`
- `REPRESENTATION`
- `SEMANTICS`
- `ATTENTION`
- `WORKSPACE`
- `MEMORY`
- `RECURRENCE`
- `PREDICTION`
- `WORLD_MODEL`
- `CAUSALITY`
- `DECISION`
- `EXECUTION`
- `LEARNING`
- `ADAPTATION`
- `IDENTITY`
- `SELF_MODEL`
- `METACOGNITION`
- `AGENCY`

## Contagens

- `registered_count`
- `available_count`
- `active_count`
- `productive_count`
- `consumed_count`
- `effective_count`
- `closed_loop_count`

Nunca usar quantidade de clusters como quantidade de cognição.

---

# Cluster e taxonomia

Interpretar:

- `cognitive_value` como utilidade funcional local;
- `cognitive_contribution` como contribuição estimada;
- `cognitive_density_score` como densidade/eficiência estrutural;
- `taxonomy` como classificação estrutural/funcional.

Nenhuma isoladamente prova cognição global.

Regras:

- default não conta como evidência;
- incluir `valid`;
- ausência de tissue/reservoir não vira zero silencioso;
- evitar circularidade entre `DevelopmentStage` e taxonomia;
- confidence heurística não é probabilidade calibrada.

---

# Reservoir + HOTM

Reservoir/HOTM é substrato dinâmico recorrente, não “a cognição inteira”.

Modos globais:

- `AWAKE`
- `SLEEP`
- `DREAM`
- `QUIESCENT`
- `RECOVERY`

Não confundir `ClusterLifecycleState.DORMANT` com sono global.

## AWAKE

Entrada dominante:
- ambiente;
- sensores;
- estado interno.

Permite:
- workspace;
- decisão;
- ação externa;
- learning online.

## SLEEP

Entrada dominante:
- replay;
- memória;
- consolidação.

Restringir:
- ação externa;
- estímulo externo;
- morfogênese agressiva.

Aumentar:
- replay;
- reconsolidação;
- pruning;
- causal review;
- identity integration.

## DREAM

Entrada dominante:
- memória;
- World Model;
- causalidade;
- contrafactuais.

Toda geração deve carregar provenance:

- `EXTERNAL_OBSERVED`
- `INTERNAL_REPLAY`
- `DREAM_SYNTHETIC`
- `COUNTERFACTUAL`
- `PREDICTED`

Nunca promover sonho/simulação a experiência observada.

Preferir `shadow_plasticity`.

---

# Learning

Loop:

`outcome → prediction_error → credit_assignment → update → future_behavior_change → effect_validation`

Separar:

- `learning_update_applied`
- `learning_effect_validated`

Não usar `weight_delta_norm`, `update_count` ou `hebbian_change` como prova suficiente.

---

# World Model

Distinguir:

- histórico disponível;
- histórico consultado;
- previsão gerada;
- previsão consumida;
- decisão influenciada;
- efeito validado.

Campos mínimos:

- `wm_history_available`
- `wm_history_consulted`
- `wm_prediction_generated`
- `wm_prediction_consumed`
- `wm_decision_influenced`
- `wm_effect_validated`

---

# Global Workspace

Instrumentar:

`posted → eligible → competed → selected → proposed → accepted → applied`

Registrar:

- queue depth;
- queue age;
- TTL;
- drop reason;
- starvation;
- latency por prioridade;
- scheduler share;
- thalamic gate result.

Se selected sobe e proposed não:
`selection→proposal` é o gargalo.

Se proposed sobe e accepted não:
`proposal→acceptance` é o gargalo.

Não aumentar prioridade repetidamente para mascarar downstream.

---

# L2 Tissue

Medir:

- `L3_to_L2_emitted`
- `L3_to_L2_received`
- `L3_to_L2_admitted`
- `L2_actuation_applied`
- `L2_effect_observed`
- `L2_feedback_to_L3`

Link OFF:
`DISABLED_BY_CONFIG`

Link ON sem efeito:
investigar cadeia funcional.

---

# Cinco Ordens da Cibernética

Não mapear para L1–L5.

## O1 — Regulação

Exigir:

- variável controlada;
- referência;
- sensor;
- comparator;
- actuator;
- plant;
- erro.

## O2 — Adaptação do controlador

Exigir:

- erro;
- ajuste;
- aplicação;
- validação do efeito.

Observador passivo não basta.

## O3 — Governança de controladores

Exigir:

- conflict graph;
- objetivos;
- constraints;
- arbitragem;
- ownership de atuadores;
- validação pós-efeito.

O4/O5 somente quando houver mecanismos operacionais explícitos, não por nomenclatura.

---

# Performance

Objetivo:
reduzir wall-clock preservando o contrato de equivalência.

Por step registrar:

- `step_wall_ms`
- `cpu_prepare_ms`
- `gpu_kernel_sum_ms`
- `gpu_timestamp_span_ms`
- `gpu_submit_cpu_ms`
- `gpu_wait_cpu_ms`
- `h2d_ms`
- `d2h_ms`
- `persistence_ms`
- `telemetry_ms`
- `unaccounted_ms`
- `unaccounted_pct`

Meta de profiler:
idealmente explicar >90–95% do wall time antes de grandes otimizações.

## BIT_EXACT_PERFORMANCE

Pode:
- hoist comprovadamente invariável;
- pré-set imutável;
- reduzir passadas sem mudar ordem;
- cache puro;
- evitar conversões repetidas.

Não pode:
- alterar ordem de soma;
- mudar RNG;
- mudar iteração;
- trocar redução;
- introduzir GEMM/BLAS se muda bits.

Qualquer divergência cognitiva reclassifica a mudança como behavioral.

---

# GPU / WGPU / OpenCL / Numba

Nunca inferir backend pelo boot.

Por operação:

- `requested_backend`
- `selected_backend`
- `actual_backend`
- `fallback_reason`

Contadores:

- `kernel_calls_wgpu`
- `kernel_calls_opencl`
- `kernel_calls_numba`
- `kernel_calls_cpu`

WGPU/Vulkan só é `PROVEN` após:

1. adapter
2. device
3. buffers
4. upload/download
5. pipeline mínima
6. dispatch
7. repetição
8. kernels reais
9. long-run
10. shutdown limpo

Falha de driver deve ficar isolada em processo separado.

## Prioridades para profiling WGPU

- morphogenesis/candidates;
- reservoir/matvec;
- energy/survival;
- tissue_affinity;
- fusion scoring;
- edges/connect;
- neural metrics;
- semantic vector ops.

Não atribuir `wgpu_ms` a logging, persistence, HML, identity, Workspace, governance ou EventBus sem dispatch real.

---

# Memória e escalabilidade

Evitar O(N²) global.

Preferir:

- candidate O(NK);
- estruturas sparse;
- CSR + delta overlay;
- top-K;
- tiling somente para all-pairs necessário.

Para pairwise registrar:

- rows;
- cols;
- tile size;
- bytes;
- peak VRAM;
- tiles;
- kernel time.

Manter estado quente residente na GPU quando possível.

---

# Persistência

Separar:

- estado canônico;
- cache;
- scratch;
- telemetria;
- checkpoints.

Preferir:

- dirty delta;
- journal;
- snapshots menos frequentes;
- restore determinístico.

Validar:

`save → shutdown → restore → run → equivalência`

---

# Event-driven architecture

Eventos devem carregar:

- `event_id`
- `trace_id`
- `parent_event_id`
- `type`
- `entity_id`
- `generation`
- `version`
- `priority`
- `deadline`
- `dirty_mask`
- `provenance`

Classes:

- `LOSSLESS`
- `COALESCIBLE`
- `BEST_EFFORT`
- `DEADLINE`
- `CAUSAL_ORDERED`

Filas bounded.
Registrar backpressure, supersession, drops e TTL.

---

# Tipagem e contexto

Evitar expansão de `Any`.

Preferir:

`ContextKey[T]`

com:

- nome;
- tipo;
- missing policy;
- versão quando necessário.

Estados:

- `VALUE`
- `NO_DATA`
- `STALE`
- `INVALID`

Não converter ausência em fallback válido silencioso.

---

# Undefined / fallback safety

Para símbolos indefinidos ou suspeitos:

1. localizar owner;
2. verificar import;
3. verificar path executado;
4. verificar broad exception;
5. verificar singleton duplicado;
6. verificar fallback silencioso.

Não corrigir com import arbitrário que crie segunda fonte de verdade.

## None semântico — regra arquitetural (26/09, auditoria None)

Referência completa da auditoria (censo, top riscos com âncoras, fronteiras,
cadeia causal, itens de correção NC-1..NC-8): `CHECKLIST_NONE_CORRECAO.txt`.

`None` pode existir internamente em Python. O que NUNCA pode acontecer é
`None`/ausência cruzar uma **fronteira cognitiva** (L1→L5, `_ctx`→record,
módulo→módulo, save→restore) **sem semântica explícita**. Cada valor
importante numa fronteira carrega:

- `value`
- `status` (`VALUE` / `NO_DATA` / `NOT_COMPUTED` / `DISABLED` / `STALE` /
  `INVALID` / `PENDING` / `ERROR` / `NOT_APPLICABLE`)
- e, quando houver decisão: `step`, `source`, `reason`

Padrões PROIBIDOS (todos confirmados na auditoria de 26/09):

1. **Trap `or`-fallback em valor medido** — `x = medido or default` engole
   `0.0`/`False`/vazio LEGÍTIMO e injeta o default como se fosse medida.
   Caso real: `system.py:6862` `float(ctx.get("meta_awareness", 0.5) or 0.5)`
   — MetaAw=0.0 real vira 0.5 fictício na tabela `system_state` (idem
   `gen_div` :6856 e `forecast_connectivity` :6865): o regulador decide
   sobre ficção. Correto: `x = v if v is not None else default` — e
   carregar status.
2. **Defaults divergentes para a MESMA chave entre camadas** — caso:
   `meta_awareness` lida com default 0.5 em 7 sites e 0.0 em outros 7 do
   `system.py`; o consumidor não sabe qual "ausência" recebeu.
3. **Fronteira temporal com defaults diferentes** — caso:
   `cross_run_persistence.py:1543` salva com default 0.5 e `:2002` restaura
   com default 0: a ausência MUDA de valor num ciclo save→restore.
4. **Coerção None→0.0 no record/relatório** — zero científico (o caso D2
   MetaAw; o caminho correto já existe no `reporting.py:730/:734/:1369`:
   `STALE`/`NO_DATA` — "ausência de risco permanece NO_DATA; não é um zero
   científico").
5. **Chave órfã / troca de nome** — leitor usa nome que nenhum escritor
   publica. Caso real: `system.py:11046` lê `energy_mean` e `:11047` lê
   `stress_mean` — chaves SEM writer no `_ctx` (os escritores publicam
   `mean_energy` :15571 e `mean_stress` :14857) → o health check L1 recebe
   zero crônico como se medido. Regra: antes de ler com default, confirmar
   por grep que a chave TEM escritor; renomeou a chave, migrou TODOS os
   leitores.
6. **Ausência fabricando semântica positiva** — o default INVENTA fato.
   Casos reais: `system.py:3689` `get("decision", "maintain")` (ausência
   vira decisão concreta), `:4297` `get("outcome", "observed")` (ações
   jamais observadas registram outcome "observed"), `:4299`
   `bool(get(..., True))` vs `:5561` `bool(get(..., False))` na MESMA chave.
   Regra: default de campo semântico é estado de AUSÊNCIA
   (`NO_DECISION`/`NO_OUTCOME`/`NO_DATA`), nunca estado positivo.
7. **Reinjeção do default via setdefault** — o default vira valor persistido
   e apaga a distinção ausente/real para todo o downstream. Caso real:
   `loop_helpers.py:1201-1216` (forecast/survival defaults 0.5/0.05
   re-publicados como dado) e `:1023-1024` (cognitive_value default 1.0
   vira `cl.cognitive_value` real). Regra: default injetado por leitura
   nunca é escrito de volta no objeto.
8. **Companion produzido sem consumidor** — value+status inútil: os
   companions existem no `_ctx` (`system.py:17416-17420`:
   self_model_output_status/reason/fresh/step/signal_age) mas têm ZERO
   leitores. Regra: todo status produzido deve ter o consumidor declarado
   no mesmo PR; sem consumidor é decoração, não semântica.
9. **Valor congelado sem carimbo no pulo de módulo** — módulo suspenso faz
   `continue` SEM escrever (`cognitive_pipeline.py:290-305`) e módulo que
   executa e retorna None também congela o anterior (`:329`): o downstream
   lê stale/ausente/default SEM nenhuma marca no valor em si. Regra: todo
   consumidor de chave produzida por módulo suspendível checa frescor
   (signal_age/output_status) NO PONTO DE USO — não confia que "o valor
   estar lá" significa "foi computado neste step".
10. **Fallback-plausível mascarando ausência** — o default devolve um valor
    fisicamente PLAUSÍVEL que cientificamente equivale a convicção. Caso
    real: `system.py:5375-5379` — sem transition_model, o fallback devolve
    o ESTADO ATUAL como "previsão" (`predicted_next_state, ms`) e o carimbo
    `_prediction_source` não separa "previu igual por convicção" de
    "previu igual porque não havia modelo". Regra: fallback que fabrica
    valor plausível carrega flag própria
    (`PERSISTENCE_FALLBACK`/`NO_MODEL`), nunca imita o caminho real.
11. **Estatística com filtro que exclui zero legítimo** — a média só conta
    o que passa num filtro de sinal. Caso real: `world_model.py:1486`
    `errors=[s.prediction_error for s in states if s.prediction_error > 0]`
    — erro 0.0 REAL nunca entra na média; "nenhum erro observado" produz
    0.0 idêntico ao default do campo — e é esse avg que governa o gate de
    learning (`system.py:7404→:7515`): "sem erros observados"→"erro 0"→
    "não aprende". Regra: estatística agregada leva junto o N observado e
    o filtro declarado; média sem N é conflatável por construção.
12. **Bool de elo causal sem razão** — o `False` tem N causas. Caso real:
    `wm_history_prediction_applied` (`system.py:5448-5452`): False =
    não consultado / sem links elegíveis / sham / erro de query /
    reassess hold / desalinhamento interno (≥6); `query_status/query_reason`
    decompõem 4, mas o "aplicado" em si não — é o que mata o degrau E5 sem
    causa nomeada (runs 781/782). Regra: bool de elo causal carrega
    `reason`; quando o motivo dominante muda, o reason muda com ele.

Exceção tolerada: CONTADORES (`or 0` = "nada aconteceu" — semântica
única). Métricas, percentuais, níveis e estados NUNCA.

Ao corrigir: mudança de default/fallback em valor medido é COMPORTAMENTAL —
exige par gêmeo A/A bit-idêntico. Vocabulário tipado já existente na casa:
`ModuleRuntimeState` (runtime_observability.py:104 — BOOTING/READY/ACTIVE/
IDLE/STANDBY/DEGRADED/FALLBACK/DISABLED/SUSPENDED/ERROR/SHUTDOWN/UNVERIFIED),
`STALE`/`FRESH` no consumo (:926), `NO_DATA`/`STALE` no record, provas E0-E5.
Template in vivo: `system.py:17404-17414` (status `SKIPPED` + reason
`NO_INPUT`/`SKIPPED_BY_GATE` + valor separado). Princípio correspondente:
central nº 7 — "Ausência de dado não é zero".

---

# Runtime observability

Separar `ModuleRuntimeState` de `EvidenceLevel`.

Estados:

- `BOOTING`
- `READY`
- `ACTIVE`
- `IDLE`
- `STANDBY`
- `DEGRADED`
- `FALLBACK`
- `DISABLED`
- `ERROR`
- `SHUTDOWN`
- `UNVERIFIED`

Não derivar ACTIVE apenas do registro estático.

---

# Designer System

Visões mínimas:

1. `Layer × Domain × Evidence`
2. `Domain × Score × Confidence × Coverage × N`
3. trace causal por `trace_id`
4. backend real por operação
5. filas/scheduler
6. memória/persistência
7. performance por step
8. regressões A/B
9. cognitive closure
10. estágio/maturidade

---

# Métricas temporais

Janelas:

- 1 step
- 5
- 20
- 50
- 100
- cross-run

Reportar:

- média;
- mediana;
- p5;
- p95;
- desvio;
- tendência;
- volatilidade.

Não usar apenas valor final.

---

# Baterias obrigatórias

## Determinismo
- twin runs;
- save/restore;
- replay.

## Robustez
- múltiplas seeds;
- perturbação;
- carga;
- falha de backend;
- fila saturada.

## Cognição
- memória;
- previsão;
- causalidade;
- decisão;
- learning;
- transferência;
- mudança de regra;
- contradição;
- generalização.

## Ablação
- no-memory;
- no-workspace;
- no-learning;
- no-world-model;
- shuffled causal links.

---

# Critérios para neocórtex funcional consolidado

Não declarar S5 enquanto faltarem:

- L2 funcional fechado;
- Global Workspace adulto estável;
- F2 sem gargalo dominante;
- learning effect validated crescente;
- World Model histórico realmente utilizado;
- recurrence estável;
- memória sem deterioração persistente;
- CCO com boost e decay;
- fusion com utilidade retardada;
- E3/E4/E5 nos loops críticos;
- multi-seed;
- save/restore;
- perturbation;
- ablation.

---

# Linguagem

Linguagem permanece tardia.

Classificação sugerida:

- decoder local → L3
- inner speech / integração linguística → L4
- self-monitoring linguístico → L5
- generative pipeline transversal → `cognitive_layer=NONE`, `domain=LANGUAGE`

Não acelerar linguagem antes de learning, recurrence, workspace, memória, world model e causal closure.

---

# Estrutura recomendada de artefatos

```text
runs/
  <run_id>/
    manifest.json
    config.json
    flags.json
    system.log
    runtime_events.jsonl
    cognition.jsonl
    performance.jsonl
    causal_traces.jsonl
    summary.json
    checksums.json

experiments/
  <task_id>/
    hypothesis.md
    baseline/
    treatment/
    compare/
    verdict.md

docs/
  architecture/
  cognition/
  cybernetics/
  performance/
  experiments/
```

---

# Formato obrigatório de verdict

## STATUS

`VALIDATED | SUPPORTED | PARTIAL | UNRESOLVED | INVALID | BLOCKED`

## O QUE FOI PROVADO

Somente fatos sustentados diretamente.

## O QUE NÃO FOI PROVADO

Lacunas, limitações e hipóteses.

## EVIDÊNCIA

- commit;
- runs;
- hashes;
- logs;
- métricas;
- comparadores.

## REGRESSÕES

- correctness;
- cognition;
- performance;
- memory;
- persistence.

## PRÓXIMO GARGALO

Um único gargalo prioritário.

---

# Regras de commit experimental

Prefixos:

- `obs:` observabilidade
- `perf:` performance bit-exata
- `fix:` comportamento/correção
- `arch:` arquitetura
- `exp:` experimento/flag
- `docs:` documentação/errata

Nunca esconder mudança comportamental em `perf:`.

---

# Checklist antes de editar

- [ ] Qual claim estou testando?
- [ ] Qual evidência atual?
- [ ] Há contrafactual válido?
- [ ] Baseline é reproduzível?
- [ ] Mudança é observabilidade, performance ou comportamento?
- [ ] Ausência/None cruza fronteira ou vira default/`or`-fallback? (regra do None semântico — ver "Undefined / fallback safety")
- [ ] Existe feature flag?
- [ ] Quais invariantes devem permanecer?
- [ ] Qual primeira métrica deve mudar?
- [ ] Qual métrica não deve mudar?
- [ ] Como detectar regressão?
- [ ] Como reverter?
- [ ] Há impacto em persistência?
- [ ] Há impacto em determinismo?
- [ ] Há impacto em GPU/backend?
- [ ] Há impacto cognitivo?
- [ ] Há evidência suficiente para alterar o board?

---

# Checklist depois de editar

- [ ] compile/import ok
- [ ] testes direcionados
- [ ] boot ok
- [ ] run curta
- [ ] twin determinism
- [ ] A/B se behavioral
- [ ] persistência
- [ ] RAM/VRAM
- [ ] profiling
- [ ] causal trace
- [ ] evidence E0–E5
- [ ] board atualizado
- [ ] limitações registradas
- [ ] artefatos preservados

---

# Não fazer

- Não criar módulo novo para resolver falta de fechamento.
- Não promover módulo por estar registrado.
- Não tratar ausência como zero.
- Não usar `dir()` como arquitetura de controle.
- Não esconder NameError em broad exception.
- Não importar singleton privado arbitrariamente.
- Não otimizar com GEMM/BLAS em caminho bit-exato sem prova.
- Não misturar telemetria e comportamento.
- Não mudar threshold para melhorar métrica sem hipótese causal.
- Não aumentar prioridade repetidamente para inflar F2.
- Não usar cluster count como inteligência.
- Não usar stage como prova independente se ele participa da fórmula.
- Não usar confidence heurística como probabilidade calibrada.
- Não chamar replay/sonho de experiência real.
- Não declarar aprendizagem por update de peso apenas.
- Não declarar neocórtex consolidado por uma única run.
- Não acelerar linguagem com loops críticos abertos.
- Não apagar runs inválidas; marque-as.
- Não fazer push sem autorização explícita quando o fluxo for local/experimental.

---

# Estratégia de decisão

Quando houver vários problemas, priorizar:

1. validade epistemológica;
2. observabilidade;
3. fechamento de loops críticos;
4. bug funcional;
5. determinismo;
6. performance;
7. escala/baterias longas;
8. expansão de capacidades;
9. linguagem;
10. novos módulos.

---

# Regra final

O agente deve sempre conseguir responder:

1. O que mudou?
2. Por que mudou?
3. Qual evidência prova isso?
4. Qual efeito downstream ocorreu?
5. O que continua não provado?

Se qualquer resposta estiver ausente, o trabalho ainda não está concluído.
