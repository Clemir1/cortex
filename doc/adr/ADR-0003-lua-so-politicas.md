# ADR-0003 — Lua apenas para políticas

- **Status:** PROPOSTO (ratificação pendente)
- **Fontes:** f2 §3, §35–36, §58; f1 §29, §36; f6 §0 (gates de auditoria).

## Contexto

O valor de Lua no Triad é a **iteração rápida de políticas** sem recompilar
e sem quebrar invariantes: heurísticas, gates, thresholds, modos cognitivos,
políticas de crise/federação/ecologia, experimentos. O risco é o oposto: um
script tornar-se dono de estado, persistência, causalidade, buffers GPU ou
threads —ambíguo, não determinístico e não auditável. A auditoria L4L5
operou com gate rígido ("zero edição de core por script") e provou que a
disciplina é viável e produtiva.

## Decisão

Lua **existe somente para políticas e experimentos**, sempre pelo ciclo
validado: `PolicySnapshot → análise → PolicyProposal → validação de range →
LawEnforcer → Governor → aplicação Rust → observação → learning`. Lua nunca
possui: memória canônica, clusters, tissue graph, persistência, estado
causal, buffers GPU, ownership, threads/locks (f2 §3). Escrever direto no
estado (`cluster.energy = 0.4`) é impossível por construção. Detalhes:
`architecture/rust_lua_boundary.md`.

## Consequências

- (+) Mudança de política não exige rebuild; experimentos ficam declarativos
  (`lua/experiments/`).
- (+) Toda mudança comportamental atravessa Lei/governança — rastreável.
- (+) Teste de arquitetura `no_direct_actuation_from_lua` torna a regra
  contínua (f2 §43).
- (−) Custo do ciclo para micro-ajustes (latência de uma janela do
  scheduler) — aceito: ajuste fino pertence aos mecanismos Rust.
- (−) Schema de Proposal fechado limita espontaneidade dos scripts — por
  design: extensão de schema exige ADR.
