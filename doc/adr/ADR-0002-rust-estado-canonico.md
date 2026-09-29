# ADR-0002 — Rust é o estado canônico

- **Status:** PROPOSTO (ratificação pendente)
- **Fontes:** f2 §2, §49, §58; f3 §6 (P0 autoridades múltiplas); f6 §1.

## Contexto

No legado Python, o mesmo estado existe simultaneamente em clusters, matriz
e `_ctx`, sem versão monotônica comum (f3 P0): um consumidor podia ler matriz
pós-sincronização, `mean_state` pré-aplicação e clusters pós-ação no mesmo
step. `None` ambíguo virou vetor zero via fallback silencioso. Telemetria de
scheduler era confundida com atividade. A auditoria L1 só conseguiu
disciplinar isso com o contrato `l1_state_contract` (state_version/phase/
hash) — depois de instrumentar.

## Decisão

Todo **estado semanticamente inambíguo** vive em Rust, com owner único,
versionamento monotônico e hashes por passagem: ClusterBio, tecidos, grafos,
HOTM/Reservoir, memória estrutural, scheduler, event bus, máquinas de estado,
World Model/Workspace estruturais, causalidade, persistência, concorrência,
GPU, telemetria, contratos, validação, FFI, segurança, invariantes (f2 §2).
Rust é o **único escritor**: nenhuma política, script ou observador muta
estado sem passar por aplicação Rust validada (ciclo
`architecture/rust_lua_boundary.md` §2).

## Consequências

- (+) Múltiplas autoridades de estado tornam-se impossíveis por construção;
  o que no legado exigiu dois lotes de instrumentação (f3 §12–13) vira
  propriedade da fundação (`foundation/status`, `contracts/state/versioned`).
- (+) Determinismo auditável (hash de ordem de clusters, state_hash por fase).
- (−) Políticas deixam de poder mutar diretamente — todo ajuste vira ciclo
  snapshot→proposal→validação (custo de latência aceito em troca de prova).
- (−) Fronteira FFI precisa ser explícita e testada (fuzz de propostas).
