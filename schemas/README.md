# schemas/ — Contratos canônicos de dados

Schemas JSON (draft 2020-12) que definem o **contrato canônico de dados que
atravessa fronteiras** (L1–L5). Nenhuma camada acessa estruturas internas de
outra: a comunicação entre camadas acontece somente por eventos e mensagens
tipadas válidas contra estes schemas (contratos_camadas §1).

## Mapa

- `common/` — `event_envelope` (envelope fechado) e `typed_value`
  (lei: ausência ≠ zero; value somente com status=VALUE).
- `events/` — cadeias de `decision`, `learning`, `tissue` e `governance`.
- `state/` — `cluster_state_ref` (L1→L2) e `checkpoint` (lineage obrigatória).
- `telemetry/` — `metric` (razão tipada: toda taxa com denominador) e
  `evidence` (escada E0–E5, sem promoção automática).
- `version/` — `schema_version` (integer monotônico ≥ 1).

## Espelhamento e validação

- `crates/contracts/` espelha estes schemas em Rust: tipos e validação são
  derivados daqui — o schema é a fonte da verdade.
- Lua valida proposals contra `common/typed_value.schema.json` antes de
  publicar valores tipados.
- Versionamento: qualquer mudança de contrato avança a versão em
  `version/schema_version.schema.json`; a versão é monotônica, nunca regride.
- `$id` é relativo à raiz do repositório e sempre começa com `schemas/`.
