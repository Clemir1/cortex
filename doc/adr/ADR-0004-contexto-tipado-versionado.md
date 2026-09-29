# ADR-0004 — Contexto tipado e versionado (fim do `_ctx` genérico)

- **Status:** PROPOSTO (ratificação pendente)
- **Fontes:** f2 §51 (TypedContext); f3 §6 (P0 fallback silencioso, P1
  schema central); f5 P11; f6 R1.

## Contexto

O `_ctx` heterogêneo foi útil para o desenvolvimento, mas produziu os P0 da
auditoria L1: (a) fallback silencioso `ctx.get("mean_state", np.zeros(...))`
publicando ausência como valor; (b) múltiplas autoridades sem versão comum;
(c) consumidores sem como distinguir `VALUE` de `NO_DATA/STALE/INVALID/
FALLBACK`. As auditorias L2/L4L5 confirmaram o padrão em escala: contratos
L2↔L3 não tipados (só `hml_tissue_signal` cru), 26 módulos classificados por
default, fronteiras sem razão tipada de perda.

## Decisão

1. Substituir gradualmente o `_ctx` por **TypedContext** + eventos/estado
   versionado. Todo valor carrega `value, status, source, step, version,
   confidence, provenance` (f2 §51).
2. Status de valor na fundação: `VALUE, NO_DATA, STALE, INVALID, PENDING,
   DISABLED, ERROR` (+ `FALLBACK` estrutural) — **ausência ≠ zero** como
   lei-contrato (f1 §36; `foundation/status`).
3. Fallback sempre explícito: `fallback_reason, provider_id, valid=false`;
   nunca publicado como medido (f3 §6).
4. Chaves críticas com schema/getter próprio: `mean_state`, `cluster_states`,
   `reservoir_signals`, `tissue_effects`, `state_version`, `wm_input` (f3 P1)
   — tipar o crítico sem congelar a heterogeneidade restante.
5. Nomes por namespace (`l2.tissue.*`, `l2.bridge.*` — CORRECAO §54) e
   descriptor único por módulo (ver `dependency_rules.md` §4).
6. Fronteiras publicam o quinteto `input/accepted/acted/effect/learned` +
   razão tipada de perda (f6 R1/F-12).

## Consequências

- (+) "Com 0 reportado, é indistinguível sem fluxo de não instrumentado"
  (f6 §3) deixa de existir: todo 0 tem razão.
- (+) Ledger de passagem e escada E0–E5 tornam-se propriedades do contrato.
- (−) Migração: todo consumidor legado precisa ser reescrito contra getters
  tipados — orquestrado no inventário (KEEP/MERGE/EXTENSION/ARCHIVE).
- (−) Verbosidade inicial — mitigada por macros/derive no `contracts`.
