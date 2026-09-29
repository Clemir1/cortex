# config/ — Configuração centralizada do Triad_AEE

**Diretriz do dono:** todas as configurações vivem AQUI. Nenhum módulo
carrega política hard-coded; os módulos IMPORTAM suas seções via
`triad_platform::config::PlatformConfig`.

## Como carregar

```rust
// No boot do organismo (app `triad`):
let cfg = triad_platform::PlatformConfig::load_default()?;   // config/default.toml
let cfg = triad_platform::PlatformConfig::load_profile("crisis")?; // default + delta
let l2: triad_l2_tissue::L2Config = cfg.get_section("l2")?;
let l3p: triad_l3_local::L3Policy = cfg.get_section("l3.adaptation")?;
```

## Arquivos

| Arquivo | Papel |
|---|---|
| `default.toml` | Base canônica — TODAS as chaves de política do organismo |
| `crisis.toml` | Delta p/ regime de crise (herda default onde não cita) |
| `research.toml` | Delta p/ experimentos (telemetria pesada etc.) |
| `corticalization.toml` | Delta p/ corticalização guiada |
| `modes/`, `hardware/`, `experiments/` | Deltas específicos (ver seções internas) |

**Perfis são DELTAS**: merge profundo de tabelas; chaves não citadas
herdam o default. Merge é feito por `PlatformConfig::merged_with`.

## Contrato de leitura

1. **Arquivo base ausente ⇒ erro de boot** — o organismo não inventa
   configuração (lei da evidência: fallback silencioso é violação).
2. **Seção ausente ⇒ default herdado COM razão** registrada em
   `cfg.notes()` (nada silencioso).
3. **Cada crate é dono da sua struct**: `l2_tissue::L2Config`,
   `l3_local::L3Policy` etc. são `Deserialize` com defaults congelados;
   a platform não conhece as camadas (inversão limpa).
4. **Chave ausente numa seção presente ⇒ default do campo** (serde
   default) — os defaults são os valores históricos documentados nos
   crates.

## Política × mecânica (divisão documentada)

- **Política (AQUI no TOML):** o que rege o ORGANISMO — cadências,
  limiares de crise, bandas de histerese, orçamentos, alvos, teto de
  eventos, seeds de perfil. Se muda comportamento observável entre
  perfis, é política.
- **Mecânica (consts nos crates):** física da implementação —
  dimensão do estado 97D, partições do vetor, janelas tau, limiares de
  disparo, tamanho de histórico de ledger. Não é política do organismo.

## Seções e consumidores (estado atual)

| Seção | Consumidor | Status |
|---|---|---|
| `[organism]` | app (seed, nome) | INJETADO |
| `[runtime]` | app (`max_events_per_tick` no StepBudget) | INJETADO |
| `[l2.tissues]` `[l2.affinity]` `[l2.adaptation]` | `l2_tissue::L2Config` → formação/bridges/gate | INJETADO |
| `[l3.adaptation]` | `l3_local::L3Policy` → propostas L3→L2 | INJETADO |
| `[l3.attention]` `[l3.prediction]` | conferem com os defaults dos crates | PENDENTE (injeção futura) |
| `[l1.energy]` `[l1.morphogenesis]` | conferem parcialmente com consts L1 | PENDENTE para a sessão 6 (mecânica do substrato); nota: `max_divisions_per_step` TOML=1 vs const L1=4 — divergência registrada |
| `[l4.*]` `[l5.*]` `[learning]` `[cybernetics]` `[governance]` `[development]` `[telemetry]` `[lua]` `[crisis]` | donos futuros | PENDENTE |

## Auditoria

`cfg.source()` diz de onde veio; `cfg.notes()` lista heranças e deltas.
O app imprime ambos no boot — a origem da configuração é observável.
