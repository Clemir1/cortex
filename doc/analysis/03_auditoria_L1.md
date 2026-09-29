# Análise — L1.md (fonte 3): auditoria da camada L1

> **Fonte:** `C:\Pictures\TB\tb\LAB\Triad_AEE\L1.md` · 663 linhas · 18 seções.
> **Papel:** auditoria epistêmica da camada L1 (substrato) do sistema legado
> (Python), com evidências de runs 741–777. Define o padrão de disciplina que
> a nova estrutura herda (versionamento, ledger, gêmeas, gates de evidência).
> **Veredicto do documento:** `L1 OPERATIONAL / O1 INSTRUMENTED /
> O1 EFFECT PENDING / O2–O5 NOT PROMOTED`.

## 1. Escopo auditado

L1 = substrato biológico-computacional: clusters vivos → matriz de estados →
sinais (energia, sobrevivência, reservatório) → tecidos/topologia → contexto
temporal → L2 → L3. Arquivos cobertos: `core/cluster.py` (ClusterBio),
`core/state_matrix.py`, `core/reservoir_readout.py`, `core/adaptive_tissues.py`,
`core/system.py` (orquestração), `core/cognitive_pipeline.py`,
`core/frequency_scheduler.py`. Fora de escopo: linguagem, novo controlador
global, promoção a neocórtex.

Hubs Graphify (grau): `run_simulation()` 308 (orquestrador/ponto de acoplamento
global), `ClusterBio` 295, `RuntimeObservability` 72, `TissueRegistry` 64,
`ReservoirReadout` 61, `AdaptiveTissueController` 29, `TissueAwareScheduler` 13,
`ClusterStateMatrix` 11 (grau baixo ≠ baixa importância — possível lacuna de
observabilidade), `CognitivePipeline` 8. Aviso de divergência skill 0.9.53 ×
pacote 0.9.33: índice é evidência estrutural de navegação, não código atual.

## 2. Inconsistências encontradas (antes da instrumentação)

- **P0 — múltiplas autoridades de estado:** clusters, matriz e `_ctx`
  representam o mesmo estado em momentos diferentes, sem versão monotônica
  comum. Correção: `state_version` + `state_phase` (`PRE_PHYSICAL`,
  `POST_PHYSICAL`, `POST_TISSUE`, `PRE_COGNITIVE`, `POST_COGNITIVE`) + hashes.
- **P0 — fallback silencioso:** `ctx.get("mean_state", np.zeros(...))` mantém
  o runtime vivo transformando ausência em vetor zero. Correção: registrar
  `fallback_reason`, `provider_id`, `valid=False`.
- **P1 — amostragem sem contrato uniforme:** 50/100/todos os clusters sem
  `sample_count`, `population_count`, `sample_policy`, `coverage`.
- **P1 — scheduler confundido com atividade:** `should_run`/`run_count` provam
  agenda, não produtividade nem consumo. Escada correta:
  declared→initialized→eligible→executed→productive→consumed→effect.
- **P1 — contexto heterogêneo sem schema central:** priorizar getters/schema
  para `mean_state`, `cluster_states`, `reservoir_signals`, `tissue_effects`,
  `state_version`, `wm_input` (tipar chaves críticas, preservar o resto).
- **P2 — índice Graphify desatualizado** (regenerar antes de refatoração).

## 3. Instrumentação implementada e validada

- **Lote 1** (`core/l1_state_contract.py` integrado ao `system.py`): marcos
  versionados por step (`POST_PHYSICAL`, `PRE_COGNITIVE`, depois 4 fases
  `POST_PHYSICAL→POST_TISSUE→PRE_COGNITIVE→POST_COGNITIVE`, versões 1–4) com
  população, dimensão, amostra/cobertura, política, hash do estado médio, hash
  da ordem dos clusters, `provider_status`, motivo de ausência, linhagem
  (`trace_id`, `parent_event_id`). Persistência em `l1_state`/
  `l1_state_history` por record; World Model registra `l1_input_*`.
  Validações: 9 testes + py_compile + Ruff; smokes 741–748 (Run 748: 4 fases,
  reservatório `REAL_PROVIDER`, WM e L2 consumindo versão 3, zero `STALE`);
  **Run adulta 749**: 80/80 steps, exit 0, consumos WM e L2 80/80 `VALUE`,
  zero fallback/STALE; Run 750: `reservoir_source` persistido no histórico.
- **Lote 2** (consumidores secundários): `meta_homeostasis`,
  `deep_telemetry`, `generative_pipeline`, `symbol_ecology`,
  `attractor_analyzer`, `semantic_topology`, `thalamic_network`, `scarcity`
  lendo via `_read_l1_value` com recibo no ledger; contadores agregados
  `l1_consumption_*`. Run 751 (3 steps): 10 consumidores, 21 leituras `VALUE`,
  zero fallback/STALE/INVALID.
- **Revalidação adulta (Run 752):** seed 19, 80 steps, exit 0, shutdown
  completo; WM e L2 80/80 `VALUE`; snapshot final com 256 leituras `VALUE`,
  zero fallback/STALE/INVALID; cobertura de amostra 1.0; fontes do reservatório
  `NO_DATA` só antes da primeira produção, depois `REAL_PROVIDER`/`CACHED`.
  Classificação: `IMPLEMENTED, RUNTIME_SMOKE_VALIDATED, ADULT_REVALIDATED`.

## 4. Pendências (tabela de encerramento da auditoria)

| Item | Estado | Critério de encerramento |
|:--|:--|:--|
| L1-01 índice Graphify + cadeia `L1StateLedger` | CONCLUÍDO | `l1_state_contract.py` no grafo com produção/consumo/testes |
| L1-02 identidade temporal em contratos O1 | CONCLUÍDO | recibos com `state_version/phase/hash`; Run 765: 12 recibos |
| L1-03 fronteira L1→L2/L3 operacional | CONCLUÍDO | Run 752: 80/80 `VALUE`, zero STALE/fallback |
| L1-04 efeito O1 posterior (t+1/t+5) | PARCIAL | `applied_value` + delta + `observed_effect` pareados; instrumentação presente, prova longitudinal pendente |
| L1-05 promoção O2 | PENDENTE / outra frente | recalibração de regulador + active/sham válido + efeito temporal |
| L1-06 promoção O3 | PENDENTE / outra frente | ganho cross-tissue reproduzível |
| L1-07 promoção O4 | PENDENTE / outra frente | mudança arquitetural reversível persistida e reproduzida |
| L1-08 promoção O5 | PENDENTE / outra frente | meta-regra segura, auditável, vantagem reproduzível |

## 5. Prova causal e ordens cibernéticas (o que NÃO está provado)

- **O1-efeito:** Runs 772/773 (22 steps, seed 11, braços serial, hash de
  origem igual, protocolo t+1/t+5 válido): erro active/sham t+1
  `0.777105`/`0.737984`; t+5 `0.786068`/`0.784881` — veredito `NOT_VALID_YET`.
  Não autoriza calibrar política retroativamente nem marcar O1-Efeito/O2.
- **O3:** `FederationHub` + `PolicyArbitrator` mecanismo testado; Runs 776/777
  (CNP no step 5, braço sham pareado): Run 776 aplicou `federation_cnp` com
  conservação 0.0 e reward causal positivo; Run 777 sem CNP. O3 passa a
  `RUNTIME_DEMONSTRATED / ROBUSTNESS_PENDING` (faltam multi-seed e ganho).
- **O4:** `SelfEngineering`/`ArchitectureGovernor` com sandbox/A/B/rollback
  testados; falta seleção persistente em runs independentes →
  `MECHANISM_TESTED / INTEGRATED_PROOF_PENDING`.
- **O5:** `LawEnforcer`/`MetaRegulator`/`LongitudinalEvidenceGate` rejeitam
  evidência curta e aprovam após 3 runs comparáveis; falta meta-regra aplicada
  em produção e validada longitudinalmente.
- A auditoria NÃO prova: cognição global, agência, aprendizagem consolidada,
  uso histórico pelo World Model, fechamento CFI, neocórtex consolidado.
  Seções 13–14 do original chegaram com mojibake (conteúdo técnico íntegro).

## 6. Lições que a nova estrutura herda (mapeamento)

| Achado da auditoria | Onde vive na nova estrutura |
|:--|:--|
| `state_version`/fase/hashes por passagem | `foundation/` (ids, status, time) + `contracts/state/versioned.rs` + TypedContext |
| Ledger append-only de passagem (produtor→versão→consumidor→efeito) | `runtime/tracing/` + `platform/telemetry/` (E0–E5 por módulo) |
| Ausência ≠ zero (`VALUE/NO_DATA/STALE/INVALID/FALLBACK`) | `foundation/status` — eliminado o `None` ambíguo |
| Amostragem com `sample_policy`/`coverage` | contrato L1 (`contracts/state`) |
| Scheduler ≠ atividade (escada E0–E5) | `runtime/lifecycle` + telemetria `effect_validated` |
| Gêmeas ON/OFF + protocolo t+1/t+5 + sham pareado | `tests/` (cognitive_closure + ablação multi-seed) |
| Orquestrador monolítico (`system.py`, grau 308) | desmontado em `runtime/` (scheduler/executor/event_bus) |
| Efeito O1 `NOT_VALID_YET` sem retrocalibração | política de evidência: veredito pré-registrado, sem ajuste pós-observação |
