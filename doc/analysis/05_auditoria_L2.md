# Análise — L2.md (fonte 5): auditoria da camada L2

> **Fonte:** `C:\Pictures\TB\tb\LAB\Triad_AEE\L2.md` · 446 linhas · 13 seções.
> **Papel:** auditoria da camada L2 (organização mesoscópica) do legado, por
> três frentes paralelas (código, grafo, runtime), com zero edições no núcleo
> na etapa de diagnóstico.
> **Veredicto do documento:** `PRESENT / PARTIALLY_FUNCTIONAL /
> EVIDENCE_BOUNDED` — o elo funcional L3→L2 existe em estrutura completa, mas
> é **inerte por design** (`TRIAD_L2_FUNCTIONAL`, default OFF).

## 1. Escopo e mapa estrutural

L2 agrupa clusters em tecidos, mantém fisiologia/energia per-tecido, topologia
e pontes inter-teciduais, e publica eventos com proveniência:

    L1 → tecidos (formação/assignment/affinity gate) → fisiologia por tecido
    (phi_vector, circulação de energia) → bridges (condutância)
    → TissueEvent versionado + ledger causal
    → L3 consome t2_signal; L3 propõe feedback (hml_tissue_signal)
    → L2 adjudica via CouplingArbiter (OPT-IN, default OFF)

Arquivos: `adaptive_tissues.py` (2.167 linhas — Tissue, TissueRegistry,
AdaptiveTissueController, tissue_survival_pressure), `tissue_event.py` (595 —
TissueEvent, TissueCausalLedger, CouplingArbiter, frontier_decomposition),
`morphogenesis.py` (2.182 — topologia/arestas, fronteira L1–L2),
`event_driven.py` (TissueEventBus), `state_matrix.py` (colunas tecido),
`meta_homeostasis.py:update_tissue_params` (flag própria, default OFF),
`tissue.py`/`issue_impl.py` (adapters órfãos), `system.py` (wiring/records).

## 2. Achados estruturais (código + Graphify 26.068 nós / 37.710 arestas)

- **Zero módulos L2 nos top-20 hubs:** TissueRegistry é o maior nó L2 (grau
  64), Tissue 43 — tráfego concentra em `system.py` (455) e ClusterBio (295).
  L2 é periferia estrutural, sem agência no grafo global.
- **`adaptive_tissues.py` ↔ `tissue_event.py`: ZERO arestas nas duas
  direções** — os dois pilares não se importam; `system.py` é a cola manual
  (~40 call sites). Risco: refactor de L2 quebra em runtime, não em import.
- `TissueEventBus` vive em `event_driven.py` (não em `tissue_event.py`) —
  drift documental; é o único módulo L2/SERVICE formal do boot.
- `tissue_organization` executa no `meta_pipeline` com `HIERARCHY_LEVELS=L2`
  — pipeline de execução ≠ camada cognitiva; o rótulo "L1-L2" do
  `bio_pipeline` é nome, não prova de camada.
- **E4/E5 parcial real:** fisiologia L2 muta variáveis de cluster que
  alimentam o veredito de morte/sobrevivência (`tissue_survival_pressure` →
  threshold por-tecido, critério de morte #6 por `Phi_tau`).
- TissueEvent é **agregado** (`tissue_id=None` por design) — sem rastreio
  per-tecido (sem E3 granular).
- DRIFT: `maintain_cross_tissue_bridges` muta o grafo **na fase de record**
  (mutação acoplada ao LOG_EVERY); `TISSUE_COALESCE_EVERY=50` duplicado no
  config (última vence); `tissue_homeostasis` omite `status` quando ATIVO
  (chave assimétrica — corrigido em §12); adapters `tissue.py`/`issue_impl.py`
  sem consumidor (E0 órfão); `try/except` largos silenciam falhas sem
  classificar NO_DATA/INVALID.

## 3. Fronteiras

- **L1→L2 INSTRUMENTED + RUNTIME_VALIDATED** (runs 748–751; herdado da
  auditoria L1): 4 marcos versionados/step, leituras classificadas com
  recibo no ledger (Run 748: L2 recebe versão 3 `PRE_COGNITIVE`, zero
  `STALE`; Run 749 adulta 80/80 `VALUE`). Prova entrega com proveniência —
  não promove E3/E4/E5 automaticamente.
- **L3→L2 — o ponto crítico:** o feedback (`hml_tissue_signal` →
  CouplingArbiter → modulation → TissueEvent v2) está **estruturalmente
  completo e inerte por default**. Runtime: `recebido=4 admitido=0` (Run 739);
  `admitido=0` em todas as runs 732–748; **única admissão da história: Run
  599**. Campos `t2_coupling_*` são placeholder determinístico quando OFF
  (bit-idêntico A/A por design). Narrativa correta: **"elo OFF auditado"**,
  não "cadeia quebrada" nem "loop fechado".
- O gargalo causativo do organismo **não** é L2 — é F2
  (workspace→decisão) em L4.

## 4. Classificação E0–E5 por subcomponente (resumo)

| Subcomponente | Prova máxima |
|:--|:--|
| Fisiologia por tecido (phi/energia/respond_to_environment) | **E5 local** (efeito no veredito de morte L1) |
| Homeostase tecidual (`update_tissue_params`) | E4–E5 factual (gêmeas 738 ON/739 OFF: 7 ajustes reais), E3–E4 formal na obs |
| `tissue_organization` | E4 (consumidor genérico, sem effect_id) |
| Affinity gate | E4 (gate real; isenção de neonatos) |
| Assignment (coverage=1.0) | E3 **estrutural** (cobertura ≠ efeito causal) |
| TissueCausalLedger + frontier | E3/E4 (fresh/stale operam de fato) |
| CouplingArbiter | E2–E3 (modulation_applied=0 na produção corrente) |
| TissueEventBus | E2/E3 (SERVICE L2; execução por evento observada; efeito fica no consumidor) |
| morphogenesis | E3/E4 (muta arestas; fora do TISSUE_MAP) |
| `tissue.py`/`issue_impl.py` | **E0 órfão** |

`by_proof` global congelado em `E0=14/E2=3/E3=7/E4=64/E5=1` entre as runs
588/590 e 732–748 — o único E5 é world_model. Seis gaps de E5 mapeados:
arbiter (record_effect na admissão), homeostase (entidade obs + record_effect),
frontier (efeito→decisão posterior), assignment (consumo de afinidade + efeito),
tissue_organization (effect_id), bus (saída/consumo por publish/subscribe).

## 5. Instrumentação da própria rodada (sessão 4, telemetria-pura)

1. `emit_tissue_effect()` fail-soft em `tissue_event.py` + gancho no ramo
   **admitido** do CouplingArbiter (`effect_id`, `affected_var=coupling_rate`).
2. `tissue_homeostasis` com `status="ACTIVE"` explícito + efeito identificado
   na obs quando `tissue_adjustments>0` (`l2_tissue_homeostasis`).
3. `tests/test_l2_effect_wiring.py` (3 testes) + suítes L2 verdes.
**Revalidação (Run 768, par gêmeo A/B):** braço A (flags ON) — E5 saltou
**1→2** e `l2_tissue_homeostasis` chegou à tabela/by_proof (segunda entidade
com E5 formal da história das runs); braço B (defaults OFF) — baseline
world_model, zero entidade fantasma. Lição de instrumentação: a escada
`derive_proof` exige cadeia completa execução→output→consumo→efeito — a ponte
teve de registrar os elos anteriores, senão o efeito ficava E0.
`l2_coupling_arbiter` permaneceu E0 (12 steps, elo ON, zero feedbacks
admitidos — histerese predomina). Ferramenta pronta:
`analysis/run_l2_gemea.py` (par A/B determinístico, guarda de janela).

## 6. Propostas de fechamento (P1–P11, sem execução nesta rodada)

P1 effect_id fisiologia→sobrevivência (promove a E5 formal); P2 mover
bridges para fora da fase de record; P3 unificar `TISSUE_COALESCE_EVERY`;
P4 decidir política padrão de `TRIAD_L2_FUNCTIONAL` com A/B por braço;
P5 rastreio per-tecido opcional; P6 atualizar referências de linha; P7
aposentar adapters órfãos; P8 rótulo "cobertura estrutural" no display;
P9 schema simétrico de status (**IMPLEMENTADO**); P10 record_effect da
homeostase (**IMPLEMENTADO**; revalidação runtime feita na 768); **P11 —
lacuna estrutural: tipar os contratos L2→L3/L3→L2 do CORRECAO.txt §9**
(hoje só existe `hml_tissue_signal`; faltam `TissueState`, `TissueContext`,
`AdaptationRequest`, `MemoryDemand`, `SpecializationFeedback` como tipos).
Contra o canônico: fisiologia L2 atua diretamente em L1 — precisa de
`actuation_targets=[L1]` declarativo, não eliminação.

## 7. Lições mapeadas para a nova estrutura

| Achado | Onde vive na nova estrutura |
|:--|:--|
| Dois pilares sem aresta + cola manual em `system.py` | `crates/l2_tissue/` com contrato único; integração via `contracts/events` |
| Elo L3→L2 completo porém inerte (flag OFF) | `l2_tissue/feedback/` com `AdaptationRequest` **first-class e ratificado** (P4 virando decisão de design, não default) |
| TissueEvent agregado sem tissue_id | eventos per-tecido opcionais no contrato (`contracts/events/tissue.rs`) |
| Efeitos provados mas invisíveis na obs (homeostase 738/739) | telemetria E0–E5 por módulo como requisito de aceite, não pós-facto |
| Mutações na fase de record (bridges) | lei/invariante: observação nunca muta estado observado |
| Status assimétrico / fallbacks | `foundation/status` — status sempre presente; ausência ≠ zero |
| Gêmeas A/B como padrão de prova | `tests/` + `migration/plano_marcos.md` (critérios por marco) |
