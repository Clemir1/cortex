# AUDITORIA — graphifyy + tracing do Triad_AEE

Ferramenta de análise do projeto: `graphifyy` instalado na `.venv` Python da raiz.
Grafo semântico do código em `graphify-out/` (reconstruído a qualquer momento).

## Setup (já feito na raiz)

```powershell
python -m venv .venv
& .venv\Scripts\pip.exe install graphifyy   # 0.9.71 + tree-sitter + networkx
```

## Ativar e reconstruir o grafo

```powershell
& .venv\Scripts\graphify.exe update D:\cortex   # re-extrai o código (sem LLM)
```

Saída: `graphify-out/graph.json` (grafo), `graph.html` (visual interativo),
`GRAPH_REPORT.md` (relatório de comunidades).

## Comandos de auditoria (grafo estático)

| Comando | Uso no Triad |
|---|---|
| `graphify god-nodes --top 12` | Hubs arquiteturais. Hoje: ClusterBio (39 arestas), Qualified (23), TypedContext (22). |
| `graphify affected "ClusterBio" --depth 2` | Impacto de mudança: quem depende do nó (graph.rs, reservoir.rs, state_matrix.rs, runner.rs). |
| `graphify path "ClusterBio" "L1Ledger" --undirected` | Cadeia entre dois nós. Hoje: ClusterBio ← L1Runner → L1Ledger (2 saltos). |
| `graphify explain "L1Runner"` | Explicação do nó e vizinhos em linguagem comum. |
| `graphify diagnose multigraph` | Integridade do grafo (arestas quebradas, colapso de arestas paralelas). |
| `graphify query "quem usa Qualified?"` | Travessia BFS/DFS para perguntas (orçamento de tokens). |
| `graphify watch D:\cortex` | Reconstrói o grafo ao salvar arquivos (deixar rodando numa janela). |

Sem o filtro de ruído, prefixe com `& .venv\Scripts\graphify.exe`.
Exclusões automáticas: `.venv/`, `target/`, `var/` (padrão do tool + `.gitignore`).

## Cadeias L1 instrumentadas (runtime, tracing Rust)

Cada cadeia do substrato emite trilha de auditoria:

| Cadeia | Arquivo | Nível | Evento |
|---|---|---|---|
| Passo físico | runner.rs | debug | `l1.runner passo concluído` (+ `eventos vitais` quando há mortes/divisões/fusões) |
| Contrato temporal | contract.rs | debug | `l1.ledger transição de fase` (4 fases, com versão do ledger) |
| Ciclo de vida | cluster.rs | debug | nascimento, divisão, morte, dormência, conserto, fusão |
| Energia | energy.rs | warn/debug | `emergência energética ativada`; ajuste de orçamento fora da banda |
| Sobrevivência | survival.rs | debug/warn | decisões de emergência coletiva e histerese |
| Reservoir | reservoir.rs | trace/debug | readouts e mudanças de plasticidade |
| Medição | metrics.rs | trace | snapshots w100 |
| Matriz de estados | state_matrix.rs | trace | bump de versão da matriz |
| Grafo local | graph.rs | trace | reconstruções, adições, atualizações incrementais |
| Ponte runtime | module.rs | debug | `l1.substrate tick publicado` (resumo público do tick) |

## Cadeias L3 instrumentadas (runtime, tracing Rust)

| Cadeia | Arquivo | Nível | Evento |
|---|---|---|---|
| Semântica | semantic.rs | trace/debug | binds, reforços, podas acima do teto de conceitos |
| Memória local | memory.rs | debug/trace | episódios gravados, recall vazio (ausência ≠ zero), consumo |
| Atenção | attention.rs | trace | focos que cruzam o limiar, snapshots |
| Predição | prediction.rs | trace/debug | predições calculadas; ausência quando não há válida |
| Metas | goals.rs | trace/debug | push aceito, pilha cheia, pop |
| Ponte runtime | module.rs | debug | `l3.local tick publicado` (focos fotografados) |

## Cadeias L4 instrumentadas (runtime, tracing Rust)

| Cadeia | Arquivo | Nível | Evento |
|---|---|---|---|
| Workspace | workspace.rs | trace | entrada descartada por excesso (>128) |
| Broadcast | integration.rs | debug | `l4.workspace broadcast venceu` (conteúdo + saliência) |
| Decisão | decision.rs | warn/debug | rejeições (sem identity_context, sem opções); opção vencedora |
| Ação | action.rs | debug | ação executada (com decision_id) |
| Modelo do mundo | world_model.rs | trace | crenças observadas (chave + versão) |
| Memória global | memory.rs | debug/trace | gravações, reconsolidações, esquecimento, ausência |
| Causal | causal.rs | trace/debug | melhor efeito; ausência |
| Modo degradado | degraded.rs | warn/debug | entrada (motivo) e saída |

Níveis: `warn` = emergências; `debug` = trilha de auditoria por passo;
`trace` = eventos de alta frequência (ligar só sob investigação).

Para VER a trilha: `crates/triad/src/main.rs` chama `rt::tracing_init("debug")`.
Com `trace` ativo, o volume é alto (por cluster por passo).

## Regras da casa respeitadas

- Ausência ≠ zero vale para o grafo: nós sem arestas não viram lixo, ficam fora.
- `graphify-out/` é regenerável; `memory/` e `reflections/` são versionados
  (ver `.gitignore`). Auditoria é evidência: nada é apagado sem trilha.
- O grafo é LEITURA: auditoria nunca escreve estado do organismo.

## Cadeias transversais instrumentadas (runtime, tracing Rust)

### learning (triad-learning)

| Cadeia | Arquivo | Nível | Evento |
|---|---|---|---|
| Envelope de aprendizagem | envelope.rs | debug | fechamento SÓ com efeito futuro verificado; ausência classificada |
| Crédito | credit.rs | debug | taxa de crédito com denominador; sem tentativas = ausência, não zero |
| Política | policy.rs | warn/trace | crise reduz eta — aprendizagem NUNCA suspensa; ajustes normais |
| Validador de efeitos | effect_validator.rs | warn/debug | ciclo fechado com efeito no ttl; expiração SEM efeito reverte (lei 6) |
| Esquecimento | forgetting.rs | debug/trace | reforços, decaimento, poda de traços fracos |

### cybernetics (triad-cybernetics)

| Cadeia | Arquivo | Nível | Evento |
|---|---|---|---|
| O1 Homeostase | homeostat.rs | debug/warn | correções fora da banda; estados críticos |
| O2 Estabilidade | stability.rs | warn/debug | oscilação detectada; retorno ao normal |
| O3 Governador | o3_governor.rs | warn/debug | Shed (energia crítica), Throttle, orçamentos reduzidos |
| O4 Metacognição | o4_metrics.rs | debug/trace | auditorias de taxa/envelope; ausências |
| O5 Horizonte | o5_horizon.rs | warn/trace | amostras; tendência de queda no horizonte |
| Genome | genome.rs | debug/trace | mutações determinísticas; avaliação FORA do loop (lei 7) |

### governance (triad-governance)

| Cadeia | Arquivo | Nível | Evento |
|---|---|---|---|
| Leis duras | hard_law.rs | warn/trace | veredito Deny = violação registrada; Allow liberado |
| Leis suaves | soft_law.rs | debug | lei sem ADR rejeitada; registro de leis |
| Arbitragem | arbitration.rs | debug | empate ⇒ SEM mudança (lei); proposta vencedora |
| Federação | federation.rs | debug/trace | assentamentos; ciclo federativo completo |
| Ecologia | ecology.rs | debug/warn | saúde por nicho com denominador; nicho em extinção |

### development (triad-development)

| Cadeia | Arquivo | Nível | Evento |
|---|---|---|---|
| Embriogênese | embryogenesis.rs | debug | gênese: plano inicial gerado |
| Corticalização | corticalization.rs | debug/warn | migração concluída; estagnação e capacidade no limite |
| Manutenção | maintenance.rs | debug/warn | amostra reparada; adiamento por população grande |
| Regeneração | regeneration.rs | debug/warn | danos reportados e curados; fila cheia descarta |
| Morfogênese | morphogenesis.rs | debug | acordar por razão; voltar ao standby; planos enfileirados |
| Ponte runtime | module.rs | debug | `development tick publicado` (fila, descartes, reparos) |

## Arquitetura modular e orientada a eventos

### Modularidade (sem monolitos)

- Um crate por camada (L1–L5) + transversais + platform: cada um compila,
  testa e evolui isolado. Nenhum crate conhece a implementação dos outros —
  só os contratos de `triad-contracts` e os tipos de `triad-foundation`.
- Fronteiras são explicitas: pontes `module.rs`/`integration.rs` vestem as
  camadas como `CognitiveModule` do runtime. Correção num crate não vaza.

### Configuração centralizada (`config/`)

- TODA a política do organismo vive em `config/*.toml` (fonte única).
  `config/default.toml` é a base; perfis (`crisis`, `research`,
  `corticalization`) são DELTAS — chaves não citadas herdam (merge profundo).
- Porta única: `triad_platform::PlatformConfig` (crates/platform/src/config.rs).
  - `PlatformConfig::load_default()` — base canônica (erro explícito se ausente).
  - `PlatformConfig::load_profile("crisis")` — base + delta.
  - `cfg.get_section::<T>("l1.survival")` — seção tipada; cada crate define
    sua própria struct `Deserialize` (o crate é dono do schema; a platform só
    fornece a mecânica). Ausência de seção = erro explícito, nunca default
    fantasma. Heranças ficam registradas em `notes()` (auditoria da config).
- Testes de contrato: `cargo test -p triad-platform` (5 testes verificam o
  TOML real, herança de perfil e erros explícitos).
- Piloto entregue `[l1.survival]`: `triad_l1_substrate::SurvivalConfig`
  (serde, default A/A espelhando as constantes de `l1/config.rs`) +
  construtores de injeção `SurvivalState::with_config` e
  `CollectiveEmergency::with_config`. Injeção muda a histerese de verdade
  (testado em `injecao_de_config_muda_histerese`). Seção TOML com 6 chaves:
  `emergency_energy`, `collective_fraction`, `dormancy_min_sleep`,
  `dormancy_min_awake`, `dormancy_cooldown`, `repair_wave_guard`.
- Padrão de injeção no app: carregar a config UMA vez no boot e injetar nos
  construtores (extensão às demais seções: sessão 7, checklist 11.2–11.6).

### Orientação a eventos

- Barramento: `triad_runtime::EventBus` (bounded; publish não bloqueia o
  passo — estouro de budget é explícito no `StepReport`).
- Envelope: `triad_contracts::EventEnvelope` — `event_id`, `parent_event_id`
  (trilha causal), `entity_id/version`, `event_type` (10 categorias),
  `priority` (Low→Critical, ordena o scheduler), `deadline`, `dirty_mask`.
- Fluxo por tick: módulo publica eventos → scheduler aplica budget → bus →
  `try_receive` drena no fim. Cada camada também expõe visões qualificadas
  via chaves de contexto (`crates/platform/src/keys.rs`) — leitura é sempre
  `Qualified<T>` (ausência ≠ zero).
- Taxonomia de eventos: `EventType::{Cognitive, Memory, Decision, Learning,
  Tissue, Governance, Development, Federation, Law, Ecology}` — o tipo guia
  roteamento e auditoria.

## Legado Python (referência) — classificação para o neocortex consolidado

Fonte: `C:\Pictures\TB\tb\LAB\Triad_AEE` (sistema legado em Python; grafo
próprio em `graphify-out/`: 28.988 nós, 42.399 arestas, 1.727 comunidades).
Regra: a árvore nova tem ZERO arquivos Python — o legado é lido como
referência de mecânica, nunca portado literalmente.

### Essencial (referência de mecânica)
- `core/chladni_frequencies.py` — motor Chladni: energia→frequência,
  padrões de onda estacionária, bandas cognitivas, ressonância entre
  padrões, hashes com evidência. Entregue no crate `triad-chladni`
  (seção 13 do checklist; cadeia instrumentada abaixo).
- `core/attention_system.py` (~linhas 90-165) — bônus de ressonância na
  energia de atenção (peso 0.15) com contadores por passo.
- `core/development.py` (~linhas 720-745) — estágio de desenvolvimento →
  banda cognitiva → frequência aplicada ao cluster.
- Já portados (não reimportar): `cluster.py` (ClusterBio no L1),
  `reservoir_readout.py` (HOTM/reservoir no L1), `config.py` (constantes
  conferidas em `l1/config.rs`), tecidos, governança, federação.

### Legado/histórico (descartado — não portar)
- `core/system.py` (monolito de 1,47 MB), `gpu_engine_unified.py`,
  `numba_jit.py`, engines Wgpu/Vulkan (GPU fica no platform; CPU é o
  fallback sempre disponível), `runtime_observability.py` (substituído
  por tracing + EventBus), `thalamic_network.py`, `semantic_emergence.py`,
  `reporting.py`, `cross_run_persistence.py` (persistence com lineage no
  platform).
- `analysis/*` (harness counterfactual/fatorial com braço sham) e
  `scripts/*` (exploração sacred geometry) — método experimental
  histórico; harness só volta quando houver consumidor real.

## Cadeia Chladni (neocortex consolidado) — crates/chladni + L1 + development

Cadeia completa da harmonia Cluster/HOTM/Chladni (seção 13 do checklist):

- **`triad-chladni`** (biblioteca pura, zero Python): `bands` (4 bandas
  cognitivas em Hz: stability 80–250, memory 250–700, creativity
  700–1500, meta 1500–4000), `table` (tabelas PIRT circular/quadrada,
  20+20 modos, `nearest` por menor delta), `pattern` (síntese
  trigonométrica escolhida pelo nome do modo, features por quadrante,
  similaridade cosseno com clamp, hashes FNV-1a de conteúdo e
  descritor versionado), `system` (motor `ChladniFrequencySystem`:
  energia→frequência com inversão de banda, caches FIFO/LRU,
  estatísticas), `observation` (tipo neutro do sinal publicado).
- **Config central**: seção `[chladni]` em `config/default.toml` (grade,
  tetos de cache, histórico, versões de descritor/evidência, peso de
  atenção 0.15); injetada via `ClusterModule::with_chladni_config`.
- **Sinal do dono do estado**: `l1_substrate::chladni_signal` — o L1
  (dono do estado 97D e da energia) amostra 4 clusters vivos por tick,
  normaliza L2, converte energia→frequência→padrão, compara features e
  publica `ch::Observation` na chave de contexto `l1.substrate.chladni`
  (`Qualified<f32>`). ADITIVO: não muda o passo físico, os eventos nem
  os testes A/A do substrato.
- **Consumidor**: `development` lê o sinal com `get_qualified` + trilha
  `trace`. Uso em POLÍTICA (bônus de atenção com `attention_weight`)
  só via ADR, com necessidade demonstrada e consumidor real.
- **Leis da casa (difere do legado)**: entrada inválida =
  `TriadError::Invalid` explícito (o legado usava 0.0 silencioso);
  taxas de cache sempre com denominador (`Option<tf::Rate>`); trilha
  de auditoria pt-BR (warn = inválido, debug = geração/evicção,
  trace = cache hit).
- **Verificação**: 31 testes no crate, 46 no L1 (3 novos do sinal),
  workspace 166 verdes, house_laws 5/5, app 65 ticks "sem violar as
  leis da casa" com trilha Chladni visível por tick.

## MCP

`graphify-mcp.exe` existe na `.venv` para integrar o grafo a assistentes.
Configuração fica a critério do usuário (não faz parte do scaffold).
