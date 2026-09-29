# Estado atual do legado — evidências consolidadas

> Espelho documental do sistema Python (`C:\Pictures\TB\tb\LAB\Triad_AEE`),
> consolidado das fontes 3–6 (auditorias) + f4 (diagnóstico). Números citados
> das runs documentadas nas próprias fontes. Regra: execução, contador ou
> leitura `VALUE` não promovem veredito (E0–E5).

## 1. Classificação por camada

| Camada | Classificação | Base |
|:--|:--|:--|
| L1 substrato | `IMPLEMENTADO, EXECUTÁVEL, INSTRUMENTADO, RUNTIME_VALIDATED` (runs 741–752) | f3 §10/§15 |
| L1 representação matricial | implementada; versão/fase fortes | f3 §10 |
| L1→L2 / L1→L3 | operacional (80/80 `VALUE`, zero STALE/fallback) | f3 §15 |
| O1-efeito | `NOT_VALID_YET` (772/773: erros active/sham 0,777/0,738 em t+1) | f3 §17 |
| O2–O5 | `NOT PROMOTED`; O3 `RUNTIME_DEMONSTRATED/ROBUSTNESS_PENDING` (776/777) | f3 §16/§18 |
| L2 tecidos | `PRESENT / PARTIALLY_FUNCTIONAL / EVIDENCE_BOUNDED` | f5 §13 |
| Elo L3→L2 | estruturalmente completo, INERTE por default (admitido=0 em 732–748; única admissão: Run 599) | f5 §6/§7 |
| L3 cognição local | ativa (≈93,6% do scheduling), com regressões de memória dinâmica | f4 §12–13 |
| L4 cognição global | executável; **gargalo F2 22,9%→~8,9%→6,7%**; suspensa no regime adulto | f4 §16; f6 §3 |
| L5 metacognição | 13/18 suspensos na crise; 6 sem contadores; sem cascata; custo alto | f6 §1/§3–4 |
| Ciclo organismo | `NÃO FECHADO` — learned=0 nas 6 fronteiras | f6 §2.1 |

## 2. Evidência E0–E5 global (by_proof das manifests)

- Congelado entre runs 588/590 e 732–748: **E0=14, E2=3, E3=7, E4=64,
  E5=1** — único E5 = world_model (f5 §7).
- Run 768 (gêmeas A/B, flags ON): braço A **E5=2** (`l2_tissue_homeostasis`
  com efeito identificado — segunda entidade E5 da história); braço B
  baseline puro (f5 §12).
- Fisiologia L2: E5-local real (muta energia/Phi → veredito de morte L1)
  sem E5 formal na obs (f5 §3.1) — o padrão "efeito provado fora da obs".
- Escada E0–E5 exige cadeia completa execução→output→consumo→efeito
  (lição da Run 768; f5 §12).

## 3. Números-chave observados (fontes)

| Métrica | Valor | Fonte |
|:--|:--|:--|
| Execuções scheduler | L1=354, L3=4505, L4=634, L5=153, **L2=0 no resumo** | f4 §3 |
| F1 conceito→workspace | 44%→**63,6%** | f4 §7 |
| F2 workspace→decisão | ~41%→**22,9%**→~8,9%→**6,7%** (piora contínua) | f4; f6 §3 |
| F3 decisão→execução | 88,9% | f4 §7 |
| Ciclos learning | 921 completos × 2952 parciais × **413 interrupções** | f4 §9 |
| Fronteiras tipadas | L3toL4=0, L5toL4=0, learned=0 (6/6, todos os steps) | f6 §2.1 |
| Consumo global tipado | 0,0% (3 de 544 outputs) | f6 §3 |
| Energia adulta | ~0,21–0,22 (piso <0.3 ativa desaceleração ×1.5 permanente) | f4 §2; f6 §1 |
| MemCap / Recurrence | 0,0506→0,0389 / 0,408→0,189→0,307 | f4 §13 |
| Hierarquia semântica | depth=1; 20.593 nodes→266 abstratos; promoções congeladas (12) × proteção 9.289 | f6 §3 |
| Identidade | Self=1,0 × Narrative=0,093; aresta A5 sempre 0 | f4 §19; f6 §3 |
| Tecidos | 132–149 tecidos; coverage=1.0 estrutural; affinity 0,5743; bridges 1017 (score transfer 0,002–0,188) | f5 §7; f6 |
| Coerência | TissueCoherence ~0,99 × CohX em queda | f4 §6 |
| Custo | meta-pipeline 33–34 s (36 steps), 96–113 s CPU (80 steps); cognitive_economy 20,7 s × GW 1,26 s | f4 §23; f6 §3 |
| RAM | 1,31→1,65 GB com população constante | f4 §38 |
| L4 suspensa | GW 25 exec × 53 SHUTDOWN; unified_identity 11 × 53+14; reativação exige energia >0,60 (gap 0,39) | f6 §3 |

## 4. O que funciona com prova (não jogar fora)

- Contrato temporal L1 (state_version/phase/hashes, ledger, 4 fases) — o
  padrão a generalizar (f3 §12).
- Decisão com identidade real: decision_id, carimbo causal, effect_observed,
  anti-stale; 4 estágios; 8 trajetórias completas (Run 1529) (f6 §3).
- World Model: único E5 formal da matriz (f6 §3).
- Fisiologia→sobrevivência L2 (E5-local) e homeostase tecidual com efeito
  provado por gêmeas 738/739 e 768 (f5).
- cognitive_economy com efeito físico real; nocturnal funcional em janela
  longa (Run 1515: 8 episódios APPLIED 1.0) (f6 §4).
- longitudinal_gate: existe, rejeita (83 julgamentos) — veredito sem
  enforcement (f6 §3).
- O3 federativo no runtime (776/777: CNP aplicado, conservação 0,0, reward
  causal positivo) (f3 §18).
- Filtro de staleness, escada derive_proof, gêmeas como método (f5 §5/§12).

## 5. O que NÃO está provado (não herdar como feito)

- Efeito causal O1 (NOT_VALID_YET); promoções O2–O5 (f3 §16–17).
- Qualquer learned em fronteira; aprendizagem consolidada; E5 generalizado
  (f6 §2.1).
- Cascata descendente L5→L4 (chave órfã; L5toL4=0) (f6 §4).
- WM usando história (80×0) (f6 §3).
- Lifecycle real de tecidos; efeito adaptativo do elo L3→L2 em produção
  (f5 §3.3/§6).
- Custo justificado de L5 (nunca medido contra economia poupada) (f6 §4).
- Uso efetivo de GPU (apenas boot do backend) (f3 §10).

## 6. Lacunas de instrumentação (herdar como requisitos)

- runs/skips por nível individual (`level_run_count` ausente; L2 aglutinada
  em "L1-L2"; 6 módulos L5 sem contadores) (f6 §3).
- Tracker de fronteiras mudo em L3toL4 (0 reportado = sem fluxo ou não
  instrumentado — indistinguível) (f6 §2.1).
- Razão tipada de perda por fronteira (taxas agregadas sem reason) (f6 F-12).
- Campo `source` de taxonomia (REGISTRY × SPEC6) no display (f6 R7).
