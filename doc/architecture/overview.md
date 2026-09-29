# Arquitetura-alvo — núcleo mínimo L1–L5 (Cortex / Triad_AEE v2)

> Síntese consolidada das fontes 1, 2 e 4 (NUCLEO_MINIMO, Renomear, L1AL5).
> Documento de referência da **nova estrutura**. Estado do legado: ver
> `analysis/03–06` e `audit/` (pendente).

## 1. O que o organismo é

Um **organismo cognitivo** com núcleo cortical mínimo, organizado por
**domínio → módulo → implementação**, em Rust (mecanismo/estado canônico) +
Lua (política ajustável). O núcleo persegue uma única propriedade:

    L1 → L2 → L3 → L4 → decisão → ação → outcome → learning
    → feedback → reorganização → novo comportamento

Quando esse circuito estiver fechado, recorrente, adaptativo, persistente e
validado em múltiplas runs, aí se aumenta complexidade. Até lá, todo sistema
que não ajuda a fechá-lo deve provar por que permanece no núcleo.

## 2. As cinco camadas funcionais

| Camada | Responsabilidade única | Objetivo-mestre | Fora de escopo |
|:--|:--|:--|:--|
| L1 Substrato | fisiologia computacional: clusters, energia, atividade, lifecycle, plasticidade, grafo local, HOTM/Reservoir | estável, barato, previsível, plasticamente disponível | conceitos, identidade, WM, linguagem, leis |
| L2 Tecidos | organização mesoscópica: membership, affinity, bridges, coerência local, especialização, recurso local, adaptação | **ponte bidirecional L1↔L2↔L3** | decisão, representação |
| L3 Cognição local | 4 domínios: representação, memória local, predição local, seleção local | representações reutilizáveis e diversas, sem crescimento semântico ilimitado | decisão global |
| L4 Cognição global | Attention Integration, Global Workspace, World Model, Causal Model, Memory Integration, Decision, Action/Outcome | **integrar e decidir** — alvo principal; F2 workspace→decisão é o gargalo histórico | microgestão, memória completa, linguagem |
| L5 Metacognição | 3 governadores (Resource, Meta, Development) + Self Model/Identity | **deixar de controlar demais**: observar, orquestrar, arbitrar, corrigir | microgerenciar cada step |

Regras estruturais:
- Componentes L4 nunca desaparecem sob pressão energética normal; **modo
  degradado obrigatório** (fonte 1 §12; crise altera política, não desliga).
- L5 publica **PolicyProposal** via contrato — nunca importa L4 para alterar
  objetos; L3→L2 usa **AdaptationRequest**; zero imports circulares (§53).
- Camadas ≠ Ordens Cibernéticas: O1–O5 atravessam L1–L5 (fonte 4 §41).

## 3. Transversais (atravessam camadas, sem ser "órgãos")

- **learning/**: prediction_error, credit, update, validation, transfer,
  forgetting. Cadeia com terminação: prediction→outcome→error→credit→
  update→future_behavior→**validation**. Essencial; nunca em standby.
- **cybernetics/**: O1–O5 como capacidades/protocolos sobre componentes
  existentes (O1 sobre EnergyHomeostasis; O3 sobre ResourceGovernor/
  DevelopmentGovernor; O4 sobre SelfModel/MetaController; O5 avalia políticas/
  genomas). O5 é **lento** (por episódio/janela longa/entre runs); genome fora
  do loop principal.
- **governance/**: laws (poucas; hard em Rust + soft em Lua; protegem
  invariantes, não microcomportamento — ex.: não tratar STALE como FRESH, não
  converter NO_DATA em zero, DREAM não vira memória factual, learning só
  recebe E5 após consequência observada); federation (protocolo de negociação
  do ResourceGovernor; sem Effect não é trade); ecology (um motor genérico +
  adapters; só com população/variação/competição/seleção/extinção);
  arbitration.
- **development/**: embryogenesis, corticalization, maintenance,
  regeneration. Morfogênese em **STANDBY**; acorda por dano, falta de
  capacidade, fragmentação ou sobrecarga persistente.
- **runtime/**: scheduler, executor, event_bus, lifecycle, TypedContext,
  tracing. Cascata = rastreio do EventBus (trace_id, parent_event_id,
  source, target, step, payload_version) — não sistema próprio. Recursões
  legítimas (HOTM, Workspace, WM, Learning, Meta, Cross-run) exigem
  terminação declarada.
- **platform/**: gpu, persistence (lineage: parent_run_id, checkpoint_hash,
  state_hash, schema_version — mata o `latest`), telemetry (E0–E5 por módulo:
  declared, initialized, executed, produced, consumed, effect_validated),
  storage, math (SVD/JL/RandLA/kernels sustentam a cognição; não são
  subsistemas mentais).

## 4. Dimensionamento e regras de admissão do núcleo

≈ **15–20 componentes runtime** (fonte 1 §73): cluster substrate, energy
homeostasis, reservoir/HOTM, tissue organization, tissue bridge/adaptation,
semantic representation, cognitive memory, attention/salience, Global
Workspace, World Model, causal model, decision, action/outcome, learning,
self model/identity, resource governor, meta controller, development
governor, laws/arbitration, persistence/telemetry.

Gate de admissão (todo módulo novo ou promovido):
1. Cinco perguntas (estado único? input único? output único? quem consome?
   o que desaparece se remover?) — resposta "nenhuma" ⇒ redundante.
2. Fusão obrigatória quando dois módulos leem os mesmos dados / controlam o
   mesmo actuator / otimizam a mesma variável.
3. Promoção extensão→core: necessidade demonstrada + efeito reproduzível +
   consumidor real + benefício multi-seed + sem duplicação funcional.

**Fora do core (extensões, sem apagar pesquisa):** language stack, dream/
inner speech/generative, full-brain (cerebelo, tronco, gânglios da base,
paleocortex), ecologias experimentais, genome contínuo, structural noise/
entropic injection contínuos → `extensions/` ou `legacy/research/`.

## 5. Fronteiras e fluxos

    Ascendente:  L1 ClusterState → L2 TissueState → L3 Representation
                 → L4 WorkspaceContent → Decision
    Descendente: L5 Policy → L4 attention/decision constraints
                 → L3 salience/learning targets → L2 AdaptationRequest
                 → L1 plasticity/structural adjustment

Bidirecionalidade obrigatória (fonte 1 §55); feedback L3→L2 é prioridade
estrutural — sem ele o sistema aprende cognitivamente sem reorganizar o
substrato que o sustenta (fonte 1 §6; no legado o elo existe e está inerte,
ver `analysis/05`).

Dependência unidirecional sem ciclos: foundation → contracts → runtime →
L1 → L2 → L3 → L4 → L5; learning/cybernetics/governance/development
atravessam por contratos; platform entra por interface. Proibições:
L5→L4 import direto; semantic→database; Lua→GPU; Lua→estado canônico;
fallback publicado como valor medido.

## 6. Contrato Rust ↔ Lua (resumo)

    Rust PolicySnapshot → Lua analisa → PolicyProposal
    → Rust valida range → LawEnforcer valida invariantes → Governor arbitra
    → Rust aplica → Telemetry observa efeito → Learning avalia

Lua: políticas/heurísticas/thresholds/modos/experimentos. Nunca: estado
canônico, persistência, buffers GPU, ownership, threads/locks. Detalhes em
`architecture/rust_lua_boundary.md` (pendente).

## 7. Estados de operação

Máquina de crise transversal: NORMAL→PRESSURE→CRISIS→STABILIZATION→
RECOVERY→REINTEGRATION — altera políticas, não duplica sistemas. Durante
crise: morphogenesis↓, division↓, telemetry pesada↓, persistência pesada↓,
O5↓, linguagem OFF, experimentos OFF. Preservar: L1 homeostase, L2 tecidos,
L3 memória/semântica, L4 Workspace MINIMAL, World Model DEGRADED, decision,
outcome, learning. Sleep/Dream: extensões futuras; AWAKE é obrigatório na
primeira versão.

## 8. O que muda em relação ao legado (resumo executivo)

| Problema comprovado no legado (fonte) | Resposta estrutural |
|:--|:--|
| L2 invisível no resumo da hierarquia; elo L3→L2 inerte por flag OFF (4, 5) | L2 camada explícita com contrato próprio; AdaptationRequest first-class |
| F2 workspace→decisão 22,9% e piorando (~8,9% nas runs recentes) (4, 6) | `l4/decision/` explícito (proposal→competition→selection→commit) + perdas tipadas por fronteira |
| learned=0 em todas as 6 fronteiras; 413 interrupções em learning (4, 6) | LearningEnvelope + validation como elo terminal obrigatório |
| L4 suspensa no regime adulto por energia <0.3 (6) | modo degradado obrigatório; crise altera política, nunca desliga |
| 26 módulos L5-de-fato contados como L3; 3 taxonomias paralelas (6) | taxonomia única no descriptor (region/layer/domain/entity_kind/pipeline) |
| dezenas de reguladores sobrepostos (1) | 3 governadores + leis + arbitragem |
| `_ctx` heterogêneo; fallback silencioso como zero (3, 5) | TypedContext versionado; ausência ≠ zero em fundação |
| `latest` como única verdade de persistência (2) | checkpoint com lineage/hash/versão de schema |

Estado detalhado do legado: `audit/estado_atual_evidencias.md` (pendente).
