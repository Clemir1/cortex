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

## Série 17 — escala, paralelismo determinístico e A/A por construção (sessão 6)

Evidência completa da diretriz de escala do dono (boot 1.200, teto 30K,
alta-concorrência interna), com todos os números medidos em release no
hardware do dono. Cada decisão de performance foi medida — inclusive as
que REVERTERAM desenhos mais "paralelos".

### Cadeia de diagnóstico (17.6) — como o gargalo foi caçado

1. Telemetria permanente: `StepReport.module_latency` (perfil por camada
   por passo; o app imprime quando o passo excede 50 ms).
2. Perfil por FASE do runner (instrumentação temporária removida): física
   5,37 s vs resto < 20 ms — dentro dela, `graph.update_positions` = 4,94 s.
3. Dois custos no grafo: `remove_stale` varrendo TODAS as células por movido
   (O(moved × células)) + affected coletando ~540 candidatos por movido
   quando só ~12 são vizinhos reais.
4. INVALIDAÇÃO do baseline da 17.1: `n = min(pop, max_population)` — o
   baseline "30K" rodava com 1024 clusters REAIS; o tick @30K real custava
   5,4 s em TODA configuração. Os thresholds 10/20/50 ms foram escritos
   contra esse baseline inválido.

### Correções (bit-idênticas, provadas por propriedade == rebuild e A/A)

- Índice reverso `where_is` no SpatialHash (remoção O(célula)).
- Física PARALELA por due em 2 estágios sem clone: snapshot puro paralelo
  dos vizinhos (buffer plano) + física in-place `par_iter_mut`; RNG derivado
  de (seed, id, step, tag) por propósito — determinismo POR CONSTRUÇÃO: o
  resultado não depende da ordem nem do nível de paralelismo.
- Grafo: recompute puro em rayon (movidos + rebuild) com commit serial;
  edges seriais por movido (melhor dos 3 desenhos medidos).
- Survival fase 4 par (zip mut-mut por índice); compactação `retain`
  in-place por índice rastreado (zero clones de 30K clusters).
- O1/Lei-1: `regulate_step` 1×/passo — contadores da homeostase passam a
  ter denominador PASSO (antes inflados por chamada/população).
- Matriz: cópia de linhas e ids em rayon (`par_chunks_mut`), SOMA serial na
  ordem canônica (bit-exato), `SmallRng` no L1/L2 (~20× por draw; zero
  testes congelados quebrados).

### Provas A/A

- `tests/aa_fisica_paralela.rs`: bits idênticos com pools de 1, 4 e 8
  threads + run-vs-run bit-exato.
- Testes do grafo 5/5 (incremental == rebuild total; add_cluster == rebuild).
- Twin L4 (16.11) verde: duas runs completas do organismo bit-idênticas.

### Números finais @30K (release)

| Métrica | Antes da série | Depois | Ganho |
|---|---|---|---|
| Passo par (banda madura, ~30K devidos) | 5,4 s | 97-184 ms | ~40× |
| Passo ímpar | 28 ms | 13-20 ms | ~1,7× |
| App E2E 65 passos | 204,7 s | 7,4 s | 28× |
| Média L1 por tick | ~3,15 s | ~68,5 ms | 79× |

### Decisões medidas (inclusive reversões)

| Desenho | Medido @30K | Veredito |
|---|---|---|
| Edges seriais (binário ordenado) | 40-59 ms | MELHOR — permanece |
| Edges por buckets j%16 + HashMap | 61-69 ms | pior — revertido |
| Recompute dos afetados par | 66-78 ms | pior — revertido |

### 17.7 — GPU: NO-GO por dados

- f32 na CPU: zero benefício medido (bench 17.3).
- @1.200 (carga operacional): tick ~1-4 ms — GPU desnecessária.
- @30K: física já paralela CPU com A/A por construção; resto do tick
  (arestas, fases O(n)) não é workload GPU.
- Transferência PCIe 23 MB/tick em f64 + reduções GPU sem ordem canônica
  ameaçam o A/A bit-exato permanente por ~25% em UMA fase.
- CPU permanece canônica; reserva arquitetural wgpu (feature off) no
  triad-compute (sessão 7).

### 17.6 — fechamento com re-baseline aprovado pelo dono

Decisão registrada em pergunta direta (round 13 da sessão 6): o threshold
de 50 ms/tick da 17.1 foi escrito antes de a 17.6 descobrir que o
"baseline 30K" media, de fato, apenas 1.024 clusters reais (flag
`min(max_population)` no runner) — número inválido como referência de
30K. Re-baseline @30K real aprovado: **68,5 ms/tick de média** (73× vs os
5,4 s originais da série), picos previsíveis de 125 ms (híbrido do grafo
por fração movida), app 65 passos em 6,9 s E2E, A/A bit-exato por
construção entre pools de 1/4/8 threads, sem violação de leis. A SEÇÃO 17
está integralmente fechada: 17.1-17.8 [x].

## Série 18 — fechamento consolidado do neocórtex (sessão 6)

### Diagnóstico (18.1, Graphify)
God-nodes: L4Module(56), ModuleDescriptor(55), ClusterBio(54), L3Module(52), L1Runner(38), L5Module(36) — **L1-L5 todas no top-6**: a cadeia cognitiva é densa no grafo real. Diagnose: 3861 nós, 7052 arestas, zero quebradas. Lacunas: TraceEngine ausente; governança sem populations vivas.

### Legado (18.2, 3 subagentes READ-ONLY, com arquivo:linha)
- **Cadeias/cascatas**: 11 elos fecham SÓ com efeito observável; CASCADE_MAP declarativo com proveniência origin/parent; CascadeTracker com denominador.
- **Federação**: inter-organismos NÃO existia (CNP interno, transporte stub); o ouro é o PADRÃO DE VALIDAÇÃO: applied_value+observed_effect+recibo+outcome t+1/t+5+active/sham.
- **Ecologia/instrumentação**: soberana por tecido (nichos, custo 70%, GenDiv, renovação obrigatória); fingerprint canônico por subfase OBSERVACIONAL acha o primeiro (step,fase) divergente; budget por camada com throttle FIXO 1.0 em validação (nunca acoplar wall-clock à cognição).

### 18.3 — TraceEngine + cascatas (commits bfd62e7/ffbed34)
IDs derivados de (seed, passo, tag, seq) — determinísticos; CASCADE_MAP com as 3 cascatas reais (emergência→survival, mortes→compactação, divisões→matriz); linhagem E(t).parent==E(t-1) com órfão = erro; elo fecha só com efeito observável (ausência ≠ zero). A/A: trilha bit-exata entre runs.

### 18.7 — hash e _ms em TODAS as camadas (commits eb0e1c5/da257ac)
L1-L5 expõem state_hash observacional + latency_ms por módulo (aditivo, sem quebrar API da sessão 7); cascatas reconstruíveis do journal; A/A por camada bit-exato.

### Estado na verificação (18.8 parcial)
Escopo sessão 6: l1 60, l2 38, l3 24, l4 36, l5 23, runtime 10, observability 7, house_laws 5/5, app "sem violar as leis". Workspace completo: aguarda WIP da sessão 7 (governance 18.4-18.6, em execução ativa — E0277/E0499 transitórios do voo dela).

### Série 18 — remate (sessão 6, segunda passada do goal)
18.4 FEDERAÇÃO [x]: CNP tipado com outcome t+1/t+5 (chain_hash estável, baseline pós-aplicação, MATCH/DRIFT, benefit_validado só com h1 E h5 — Lei 5), FederationTransport InProcess + remote feature-off (stub recusa sem mentir), ACTIVE/SHAM A/A (gêmeos sham bit-idênticos e intactos; active difere onde deve). 18.5 ECOLOGIA [x]: 2 adapters reais (nichos + recursos federativos, denominadores sempre), GenDiv com injeção entrópica determinística, renovação obrigatória, custo de nascimento da reserva da mãe. 18.6 ARBITRAGEM [x]: prioridade com trilha tipada, ordem canônica, A/A, ausência≠zero. 18.7 [x]: hash real L1-L5 com A/A 4/4, spans no journal, twin no system.log, 79× retificado. Débitos explícitos herdados: budget 25/35/25/15 throttle 1.0 (programa maior, observacional), hash fino L2 por tecido (TissueState fora do crate), Lua policy de federação (Rust-canônico preferido). Verificação final: workspace COMPLETO verde, governance 26/26, house_laws 5/5, app release sem violar as leis. Commits 813e69f..04a627c/f03961b.

### Série 20 — auditoria profunda de instrumentação (sessão 6)
Graphify 0.9.71 atualizado; 4088 nós, 0 quebradas; FederationEngine no top-10 (35). Varredura L1-L5+transversais: 14 lacunas (3 bloqueia-A/A, 7 degrada, 4 cosmético) + 3 débitos de padrão, todas com arquivo:linha no checklist (SEÇÃO 20, critérios antes de medir). FECHADAS: as 3 bloqueia-A/A (L3 threshold NO_DATA nunca 0.0; L5 limbic com Option tipado + limbic_no_data_ticks no hash; 9/9 módulos com state_hash real — A/A cobre o organismo inteiro) + a degrada nº 5 (L1 summary_w100 janela vazia = NO_DATA). Verificação final: workspace completo verde, house_laws 5/5, app release sem violar as leis. Commits: c7d64bf, 39485a6, 572110c, 59e9c36, e3fbb6e. ABERTAS (designs prontos): 6 degrada (20.3-20.5), 4 cosmético, budget 25/35/25/15 throttle 1.0, hash fino L2, Lua ADR — continuidade mapeada.

### Série 20 — remate completo (segunda passada da sessão 6)
Fechado: bloqueia-A/A 100% (Lei 2 em L3/L5, hash 9/9 módulos, janelas NO_DATA), degrada-auditoria quase toda (20.3 exceto território sessão 7; snapshot/Option/documentações), harnesses de validação no system.log (8 testes-chave com veredito e denominador), cosméticos principais, e os 3 débitos de padrão: budget 25/35/25/15 throttle 1.0 observacional no StepReport (A/A prova não-alteração), hash fino L2 por tecido validado com A/A, ADR da policy Lua de federação com fronteira viva (wire pós-ratificação do dono). Débitos explícitos remanescentes (pequenos): logs em 2 test-regions (scheduler/persistence), outcome_ids no StepReport, comentários de caches, 20.3b/d território sessão 7. Verificação final: workspace completo verde, house_laws 5/5, app sem violar as leis, A/A intacto.

### Série 20 — zeragem total (remate final, diretriz do dono: zero débitos técnicos)
Após o fechamento principal, zerados os restantes: 20.3b learning (NO-OP sobre traço ausente, Lei 2), 20.3d contracts (NO_DATA documentado), 20.4 com critério completo (todo teste de validação registra veredito com denominador no var/system.log), 20.5b outcome_ids no StepReport (accessor aditivo, ausência≠zero), 20.5d caches declarados. Wire CNP⇄PolicyHost ratificado pelo dono e implementado: a policy Lua propõe, Rust valida e aplica; feature federation-policy desligada por padrão (bit-idêntico), 31/31 verdes ligada; sham nunca consulta a policy — A/A preservado. Commits: 067aaac, 3c0839a, fede511. Sem pendências de implementação ou instrumentação.

### Série 20 — remate definitivo do wire (fa15d88)
Wire CNP⇄PolicyHost completo: policy Lua determinística proposta no ponto need→proposal, Rust valida (whitelist + Lei 3 tipada) e apenas aperta o cap de perna; proveniência no recibo; sham nunca consulta a policy (A/A 18.4 preservado); gêmeos com chain_hash bit-exato por passo com a policy LIGADA; feature-off bit-idêntico. Governance 36/36 com feature, 31/31 sem; house_laws 5/5. 20.3b re-verificada (Qualified no strength(), crédito sessão 7). Zero débitos técnicos na série 20.

### Remate geral das pendências (diretriz da dona: zerar todas)
17.11 fechada com validação formal dos 6 componentes (LawEngine, CNP como protocolo, Ecology, Arbitration, grafo O3 crédito sessão 7 — 13/13). 18.7: validação formal da orquestradora (L4/L5 A/A por tick, relatório 19-3). 18.4: observação antiga resolvida (policy Lua + ADR fechados na série 20). 17.6: re-baseline ratificado pela dona — threshold 74 ms (média real, 73x; 50 ms era baseline falso; aresta serial comprovada). Pendente: 17.13 (fork dedicado em andamento — escala horizontal real).

### Remate geral — zero pendências (diretriz da dona)
Todas as caixas do checklist fechadas: 17.6 (re-baseline ratificado, 74 ms real), 17.11 (6 componentes validados formalmente, grafo O3 crédito sessão 7), 17.13 (escala horizontal real: RemoteTransport entre 2 organismos, cross-run bit-exato, genome/ecologia em janela longa FIFO-16 com Lei 1/Lei 2, renovação por episódio — commit 6f2e85f), 18.4 (policy Lua + ADR, série 20), 18.7 (hash A/A L4/L5 validado, relatório 19-3), séries 18/20 completas. Wire CNP⇄PolicyHost e transporte remoto: features desligadas por padrão, verdes quando ligadas, A/A preservado em todos os níveis.

### 21.2 — Reestruturação do system.log (análise científica sem duplicações)
Diagnóstico: 57 execuções acumuladas por append num único arquivo (métricas [L1]-[L5] e validação duplicadas 57x). Solução no SystemJournal: rotação por execução (histórico arquivado em var/logs/system-<ts>-<runid>.*, run corrente sozinho em var/system.log), cabeçalho/rodapé [RUN id] por arquivo e índice científico var/runs_index.jsonl com 1 linha JSON por run (run_id, timestamps, resumo). Prova com 2 execuções: histórico preservado, log corrente com 1 run, índice com 2 linhas, app sem violar as leis nas duas. Harnesses de teste seguem em append (sem rotação). Teste novo no journal (observability 8/8).
