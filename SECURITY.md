# SECURITY — Triad_AEE / Cortex (núcleo mínimo L1–L5)

Segurança do organismo cognitivo: como reportar, superfícies críticas e
invariantes que o código deve manter sob qualquer circunstância.

## Como reportar

- **Não** abra issue pública para vulnerabilidades.
- Reporte diretamente aos mantenedores do projeto (canal privado),
  incluindo: reprodução mínima, impacto esperado e camada afetada
  (L1–L5, runtime, platform, `lua/`).
- Damos preferência a discutir e corrigir antes de qualquer divulgação;
  crédito ao autor do relato quando ele desejar.

## Superfícies críticas

1. **FFI Lua (`mlua`, lua54)** — scripts de política executam dentro do
   processo Rust. Todo input que cruza a fronteira é validado; o host
   Lua é registrado com visão limitada (`PolicySnapshot`), sem acesso a
   arquivos, rede, persistência ou bibliotecas nativas.
2. **GPU (wgpu/OpenCL)** — kernels e buffers existem apenas em
   `crates/platform`; nenhuma outra camada conhece a GPU.
3. **Persistência** — checkpoints, journal e lineage vivem em `var/`;
   corrupção, troca ou restauração sem lineage compromete a memória
   cross-run do organismo.

## Invariantes de segurança

- **Lua nunca toca buffers GPU nem estado canônico** — Lua apenas lê
  `PolicySnapshot` e produz `PolicyProposal`; validação de range,
  `LawEnforcer` e aplicação são sempre Rust (ADR-0002, ADR-0003).
- **Checkpoint exige lineage completo**: `parent_run_id`, `checkpoint_hash`,
  `state_hash`, `schema_version`. Checkpoint sem lineage não é restaurável;
  `latest` nunca é tratado como verdade.
- **Cognição nunca conhece SQL** — acesso a storage somente via `platform`.
- **Ausência ≠ zero** — `NO_DATA`/`STALE`/`INVALID`/`FALLBACK` nunca são
  convertidos em valor numérico para satisfazer um contrato.
- **Leis antes da actuação** — toda `PolicyProposal` passa por validação de
  range e leis antes de ser aplicada; violação bloqueia a aplicação e gera
  telemetria, nunca erro silencioso.

## Escopo

O núcleo roda offline por padrão: qualquer componente com acesso a rede,
arquivos fora de `var/` ou execução arbitrária de código exige ADR próprio
antes de entrar no core.
