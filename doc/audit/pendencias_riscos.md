# Pendências e riscos — consolidação de todas as frentes

> Consolidação das pendências mapeadas pelas fontes (f3 L1-01..08; f5 P1–P11;
> f6 F-1..F-12; síntese D1–D8), dívidas técnicas documentadas e riscos da
> migração. Itens ⚠ = ratificação/decisão humana; itens de reescrita migram
> para `migration/plano_marcos.md`.

## 1. Pendências L1 (f3 §16)

| Item | Estado | Encerramento |
|:--|:--|:--|
| L1-01 Graphify + L1StateLedger | CONCLUÍDO | — |
| L1-02 identidade temporal em contratos O1 | CONCLUÍDO (Run 765) | — |
| L1-03 fronteira L1→L2/L3 | CONCLUÍDO (Run 752) | — |
| L1-04 efeito O1 posterior (t+1/t+5) | PARCIAL | applied_value+delta+observed_effect pareados; multi-seed |
| L1-05 O2 | PENDENTE | recalibração + active/sham + efeito temporal |
| L1-06 O3 | RUNTIME_DEMONSTRATED | multi-seed + ganho cross-tissue |
| L1-07 O4 | MECHANISM_TESTED | seleção persistente reproduzida |
| L1-08 O5 | MECHANISM_TESTED | meta-regra aplicada em produção validada |

## 2. Pendências L2 (f5 §10 — status no legado)

P1 effect_id fisiologia→sobrevivência (→E5 formal) · P2 bridges fora da fase
de record · P3 unificar `TISSUE_COALESCE_EVERY` · **P4 política default do
`TRIAD_L2_FUNCTIONAL`** ⚠ (resolvido para a nova estrutura: default ON,
ADR-0005) · P5 rastreio per-tecido · P6 referências de linha defasadas ·
P7 aposentar órfãos `tissue.py`/`issue_impl.py` · P8 rótulo "cobertura
estrutural" · P9 status simétrico (IMPLEMENTADO) · P10 record_effect da
homeostase (IMPLEMENTADO; revalidado na 768) · **P11 contratos tipados
L2↔L3** (→ `contracts` da nova estrutura).

## 3. Pendências L4×L5 (f6 F-1..F-12 → destino na reescrita)

| F | Lacuna | Destino |
|:--|:--|:--|
| F-1 | cascata descendente inexistente | contratos_camadas §3; marco 4 |
| F-2 | LearningEnvelope + learning_status | `learning/validation`; marco 4 |
| F-3 | L4 viva no regime adulto | modo degradado obrigatório; lei; marcos 3–4 |
| F-4 | classificação única | descriptor (ADR-0005); fase 0 |
| F-5 | WM com histórico (3 camadas) | `l4_global/world_model`; marco 3 |
| F-6 | L5 incremental (acumuladores) | `runtime/typed_context` + MetricRegistry |
| F-7 | aresta A5 identidade | escritor fora do runner suspenso; marco 3 |
| F-8 | efeito regulatório de MetaAw | meta_controller com keep/revert |
| F-9 | self-engineering keep/revert + HML cross-run | meta_controller; learning |
| F-10 | admission control + L5 event-driven | runtime/scheduler; fase 0 |
| F-11 | baterias E5/ablação/adulta 200–300 | plano_marcos §4 |
| F-12 | razão tipada de perda | telemetria; fase 0 |

## 4. Ratificações pendentes ⚠ (síntese D1–D8)

D1 núcleo L1–L5 + regiões como extensão (ADR-0005/0006) · D2 elo L3→L2
default ON (ADR-0005) · D3 modes crise no core × dream/linguagem extensão
(ADR-0006) · D4 taxonomia única (ADR-0005) · D5 L4 nunca desligada · D6
`verified_future_effect` obrigatório · D7 observação nunca muta estado
(lei-contrato) · D8 custo do controle < economia (gate de aceite).

## 5. Dívidas técnicas documentadas (não migrar como estão)

- `system.py` 17.544 linhas, grau 308/455 — cola manual de L2 (f5 §4).
- Chave órfã `PIPELINE_CASCADE_INTERVAL` (1 match) (f6 §4).
- `TISSUE_COALESCE_EVERY` duplicado (f5 achado 4).
- `cognitive_economy.py:1059` or-fallback `0.0→0.5` (dívida NONE_CORRECAO)
  (f6 §4).
- `try/except` largos sem classificar NO_DATA/INVALID no caminho L2 (f5
  achado 12).
- Status assimétrico de `tissue_homeostasis` (corrigido; padrão a não
  reproduzir) (f5 achado 14).
- Mojibake em L1.md §13–14 (cosmético; conteúdo íntegro) (f5 achado 16).
- Índice Graphify divergente (skill 0.9.53 × pacote 0.9.33) — regenerar antes
  de qualquer refatoração no legado (f3 §6).

## 6. Riscos (da síntese §5 + operacionais)

1. Portar `.py`→`.rs` um-a-um e transportar a arquitetura doente (f2 §57).
2. Reproduzir a fragmentação: módulos lendo os mesmos dados sem fusão
   (f1 §48; 26 reguladores L5-de-fato como prova do risco).
3. Extensões promovidas cedo — superfície antes do circuito fechar
   (f1 §45; custo L5 96–113 s sem efeito como prova).
4. Calibração retroativa de política após veredito (f3 §17) — corruptor de
   prova; gate 5 do plano_marcos.
5. Escassez permanente (energia ~0.21) tratada como política de scheduling
   e não como lei de degradação (f6 §1 — desaceleração ×1.5 eterna no
   legado).
6. Janelas curtas de validação escondendo comportamento (nocturnal nunca
   executou em 5/5 runs curtas) (f6 §3) — adulta 200–300 steps obrigatória.
7. Perda de conhecimento: fases antigas e checklists paralelos não
   determinam a estrutura (f2 §56; f6 §1 — duas agendas paralelas) — os ADRs
   são o antídoto.
