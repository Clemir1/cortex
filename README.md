# Triad_AEE / Cortex — núcleo mínimo L1–L5

Organismo cognitivo em **Rust + Lua**: mecanismo e estado canônico vivem em
Rust; políticas ajustáveis vivem em Lua. Este repositório é a reescrita do
núcleo cortical mínimo — menos componentes, mais circuito fechado.

> Base documental normativa: [`doc/`](doc/README.md) — comece por
> `doc/architecture/overview.md`. Não existe pasta `docs/` separada: `doc/`
> é a única base documental do projeto.

## O que o organismo é

Um organismo cognitivo com núcleo cortical mínimo, organizado por
**domínio → módulo → implementação**, que persegue uma única propriedade —
a cadeia-mestre:

    L1 → L2 → L3 → L4 → decisão → ação → outcome → learning
        → feedback → reorganização → novo comportamento

Quando esse circuito estiver fechado, recorrente, adaptativo, persistente e
validado em múltiplas runs, aí se aumenta complexidade. Até lá, todo sistema
que não ajuda a fechá-lo deve provar por que permanece no núcleo.

## As cinco camadas funcionais

| Camada | Responsabilidade única | Objetivo-mestre | Fora de escopo |
|:--|:--|:--|:--|
| **L1 Substrato** | fisiologia computacional: clusters, energia, atividade, lifecycle, plasticidade, grafo local, HOTM/Reservoir | estável, barato, previsível, plasticamente disponível | conceitos, identidade, WM, linguagem, leis |
| **L2 Tecidos** | organização mesoscópica: membership, affinity, bridges, coerência local, especialização, adaptação | ponte bidirecional L1↔L2↔L3 | decisão, representação |
| **L3 Cognição local** | 4 domínios: representação, memória local, predição local, seleção local | representações reutilizáveis e diversas, sem crescimento semântico ilimitado | decisão global |
| **L4 Cognição global** | Attention Integration, Global Workspace, World Model, Causal Model, Memory Integration, Decision, Action/Outcome | integrar e decidir (F2 workspace→decisão é o gargalo histórico) | microgestão, memória completa, linguagem |
| **L5 Metacognição** | 3 governadores (Resource, Meta, Development) + Self Model/Identity | deixar de controlar demais: observar, orquestrar, arbitrar, corrigir | microgerenciar cada step |

## Transversais

Atravessam camadas, sem ser "órgãos":

- **learning** — prediction_error, credit, update, validation, forgetting;
  cadeia com terminação obrigatória; nunca em standby.
- **cybernetics** — protocolos O1–O5 **sobre** componentes existentes; O5 é lento.
- **governance** — laws (hard em Rust + soft em Lua), federation, ecology,
  arbitration.
- **development** — embryogenesis, corticalization, maintenance, regeneration;
  morfogênese em STANDBY.
- **runtime** — scheduler, executor, event_bus, lifecycle, TypedContext, tracing.
- **platform** — gpu, persistence (com lineage), telemetry, storage, math.

## Fora do core (extensões)

`extensions/` abriga o que não é necessário para fechar a cadeia-mestre:
language stack, dream/inner speech, full_brain (cerebelo, tronco encefálico,
gânglios da base, paleocórtex — as 12 regiões), ecologias experimentais,
evolução contínua. Nada é apagado: pesquisa fica em `extensions/` ou
`legacy/research/`. Promoção ao core exige **necessidade demonstrada +
efeito reproduzível + consumidor real + benefício multi-seed + sem
duplicação**.

## Contrato Rust ↔ Lua

    Snapshot → Proposal → Validation → Laws → Actuation

Em detalhe:

    Rust PolicySnapshot → Lua analisa → PolicyProposal
        → Rust valida range → LawEnforcer valida invariantes → Governor arbitra
        → Rust aplica → Telemetry observa efeito → Learning avalia

Lua: políticas, heurísticas, thresholds, modos, experimentos. Nunca: estado
canônico, persistência, buffers GPU, threads/locks.

## Árvore resumida

    Triad_AEE/
    ├── Cargo.toml            # workspace raiz (crates/* nascem um por rodada)
    ├── rust-toolchain.toml   # stable + rustfmt + clippy
    ├── crates/               # foundation, contracts, runtime, l1..l5,
    │                         # learning, cybernetics, governance,
    │                         # development, platform, triad
    ├── lua/                  # policies, governance, development, experiments
    ├── config/               # default, research, crisis, corticalization
    ├── schemas/              # events, state, telemetry, persistence, version
    ├── tests/                # architecture/ + cognitive_closure/
    ├── benches/
    ├── extensions/           # language, dream, full_brain, ...
    ├── doc/                  # base documental normativa
    └── var/                  # estado mutável de execução (runs, logs, tmp)

Árvore completa: `doc/architecture/estrutura_diretorios.md` §1.

## Convenções

- **Evidência E0–E5**: DECLARED → INITIALIZED → EXECUTED → PRODUCTIVE →
  CONSUMED → EFFECT_VALIDATED — toda afirmação de estado cita fonte e nível
  de evidência; documento não promove veredito.
- **Ausência ≠ zero**: todo valor carrega status (`VALUE`, `NO_DATA`,
  `STALE`, `INVALID`, `PENDING`, `DISABLED`, `ERROR`, `FALLBACK`); fallback
  nunca é publicado como valor medido.

## Pré-requisitos

- **Rust stable** (≥ 1.78) — `rust-toolchain.toml` fixa o canal e os
  componentes `rustfmt` e `clippy`.
- **Lua** — opcional: embutida via `mlua` (lua54), apenas para políticas; o
  núcleo roda sem nenhum script Lua.
- **GPU** — opcional: `platform` expõe backends wgpu/opencl/cpu; CPU é o
  fallback sempre disponível.

## Comandos

    cargo build                 # compila o workspace
    cargo test                  # testes unitários + de arquitetura
    cargo run -p triad          # executa o organismo (65 ticks, L1→L5)
    cargo clippy --workspace    # lints (CI usa -D warnings)
    cargo fmt --all             # formatação

## Auditoria e grafo

- [`AUDITORIA.md`](AUDITORIA.md) — mapa das cadeias instrumentadas
  (L1/L3/L4/transversais), comandos do `graphifyy` e a arquitetura
  modular, config centralizada e orientação a eventos.
- Ferramenta de auditoria: `graphifyy` (venv na raiz, não versionada) —
  `python -m graphify update .` reconstrói o grafo em `graphify-out/`.
- Coordenação entre sessões de implementação:
  `CHECKLIST_IMPLEMENTACAO.txt` (protocolo em `AGENTS.md`).

## Documentação

Toda a documentação normativa vive em [`doc/`](doc/README.md):

1. `doc/architecture/overview.md` — leitura inicial da arquitetura-alvo;
2. `doc/architecture/estrutura_diretorios.md` — árvore normativa;
3. `doc/adr/` — decisões irreversíveis (mudança exige ADR novo);
4. `doc/migration/` — plano de reescrita e marcos;
5. `doc/audit/` + `doc/analysis/` — estado e auditoria do legado.
