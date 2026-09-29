# config/ — Configuração centralizada do Triad_AEE

**Diretriz do dono:** todas as configurações vivem AQUI. Nenhum módulo
carrega política hard-coded; os módulos IMPORTAM suas seções via
`triad_platform::config::PlatformConfig`.

## Como carregar

```rust
// No boot do organismo (app `triad`):
let cfg = triad_platform::PlatformConfig::load_default()?;   // config/default.toml
let cfg = triad_platform::PlatformConfig::load_profile("crisis")?; // default + delta
let l1: triad_l1_substrate::config::L1Config = cfg.get_section("l1")?;
let l2: triad_l2_tissue::L2Config = cfg.get_section("l2")?;
let l3: triad_l3_local::L3Config = cfg.get_section("l3")?;
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
3. **Cada crate é dono da sua struct**: `L1Config`, `L2Config`,
   `L3Config` etc. são `Deserialize` com defaults congelados; a
   platform não conhece as camadas (inversão limpa).
4. **Chave ausente numa seção presente ⇒ default do campo** (serde
   default) — os defaults são os valores históricos documentados nos
   crates.

## Política × mecânica

- **Política (AQUI no TOML):** o que rege o ORGANISMO — cadências,
  limiares, bandas de histerese, orçamentos, alvos, tetos, seeds de
  perfil. Se muda comportamento observável entre perfis, é política.
- **Mecânica (consts nos crates):** física da implementação — dimensão
  do estado 97D, partições do vetor, janelas tau, limiares de disparo,
  tamanho de histórico de ledger, custo metabólico por cluster.

## Status de TODAS as seções (zero pendências escondidas)

### Núcleo real (L1–L3 + runtime) — INJETADO pelo app

| Seção | Struct/consumidor | Notas |
|---|---|---|
| `[organism]` | app (seed, nome, estado inicial) | seed 42 injetada em L1/L2 |
| `[runtime]` | app (`max_events_per_tick` no StepBudget) | `tick_ms`/`scheduler`/`lua_window_ms`/`step_budget_ms` declarados p/ o runtime de tempo real (aguarda scheduler com relógio; o atual é por ticks lógicos) |
| `[l1.energy]` `initial_level`/`target_level` | `l1_substrate::config::EnergyCfg` | nascimento dos clusters + setpoint O1 |
| `[l1.energy]` `decay_per_step`/`recharge_per_step`/`pressure_floor`/`critical_floor` | **NÃO APLICÁVEL (com razão)** | o modelo de piscina global não existe no substrato real: O1 fecha por intake POR CLUSTER (energy.rs); floors de crise pertencem à máquina de crise transversal (abaixo) |
| `[l1.homeostasis]` `band`/`adaptation_gain` | `l1_substrate::config::HomeostasisCfg` | injetados no EnergyBudget; `enabled` = LEI (sempre true; desligar homeostase é violação); `check_interval_steps` NÃO APLICÁVEL (O1 regula a cada step por desenho) |
| `[l1.morphogenesis]` | `l1_substrate::config::MorphogenesisCfg` | `division_threshold` = ENERGIA do cluster que autoriza divisão (semântica do código, nota no TOML); `fusion_threshold`; `max_divisions_per_step`; `max_population` (nova chave, teto A3). `state`/`min_cluster_size`/`max_cluster_size` = declarados p/ o DevelopmentGovernor (L5 real, abaixo) |
| `[l1.survival]` | consts L1 congeladas (valores = comportamento A/A atual) | injável quando a sessão 6 quiser; hoje 1:1 com o código |
| `[l1.reservoir]` `[l1.plasticity]` | **AGUARDANDO CAMADA REAL** | o HOTM/reservoir real do L1 tem mecânica própria (consts); as chaves descrevem o modelo de reservoir clássico — mapear sem reescrever o HOTM seria teatral |
| `[l2.tissues]` `[l2.affinity]` `[l2.adaptation]` | `l2_tissue::config::L2Config` | formação, re-vinculação, dissolução, bridges, gate E3→E4 — tudo injetado |
| `[l3.attention]` | `l3_local::attention::AttentionCfg` | limiar, capacidade, ganho injetados |
| `[l3.prediction]` | `l3_local::prediction::PredictionCfg` | horizonte e confiança mínima injetados; `error_tolerance` declarada p/ o avaliador de acerto (consumidor futuro: comparar \|observado−previsto\|) |
| `[l3.semantic]` `[l3.memory]` | **AGUARDANDO CAMADA REAL** | semantic/memory da sessão 6 são estruturas mínimas sem esses parâmetros; as chaves descrevem o alvo documentado (teto de vocabulário etc.) — injetar em estrutura sem o mecanismo seria teatral |
| `[l3.adaptation]` | `l3_local::L3Policy` | cadência, pisos, banda, salto das propostas L3→L2 — injetado |
| `[l4.workspace]` | `l4_global::config::WorkspaceCfg` | capacidade (7 clássico), rodadas, intervalo de broadcast, perdas tipadas — injetado no `L4Module` real (P1) |
| `[l4.world_model]` | `l4_global::config::WorldModelCfg` | intervalo de atualização injetado; `simulation_max_depth` AGUARDANDO mecanismo de rollout (declarada, não teatral) |
| `[l4.causal]` | `l4_global::config::CausalCfg` | rastreamento, poda por intervalo, `confirm_tolerance` — injetado (hipóteses hits/total com denominador) |
| `[l4.memory_integration]` | `l4_global::config::MemoryIntegrationCfg` | AGUARDANDO consumidor real no `memory.rs` (declaração honesta) |
| `[l4.decision]` | `l4_global::config::DecisionCfg` | commit_threshold (commit/defer), prazo de proposta — injetado |
| `[l4.action]` | `l4_global::config::ActionCfg` | `outcome_timeout_steps` — prazo do outcome do ciclo (Lei 5) — injetado |
| `[l4.degradation]` | `l4_global::config::DegradationCfg` | modo degradado OBRIGATÓRIO consumido como invariante (`allow_suspension=false`); pisos MINIMAL/DEGRADED declarados |

### Camadas de ponte restantes e transversais — AGUARDANDO CAMADA REAL

`[l5.*]`, `[learning]`, `[cybernetics]`, `[governance]`,
`[development]`, `[telemetry]`, `[lua]`, `[crisis]` estão declaradas no
default.toml como CONTRATO DO ALVO (o documento é a especificação do
organismo completo). Os módulos atuais dessas camadas são pontes mínimas
— injetar config em esqueleto sem mecanismo seria teatral. **Cada uma
ganha injeção no momento em que o mecanismo real existir; a chave nunca
é apagada.** Decisão de conflito: sessão 6 (dona dessas camadas).

## Auditoria

`cfg.source()` diz de onde veio; `cfg.notes()` lista heranças e deltas.
O app imprime ambos no boot — a origem da configuração é observável.
