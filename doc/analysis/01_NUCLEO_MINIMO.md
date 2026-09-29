# Análise — NUCLEO_MINIMO.txt.txt (fonte 1)

> **Fonte:** `D:\cortex\NUCLEO_MINIMO.txt.txt` · 2.172 linhas · 75 seções +
> conclusão, lidas integralmente em duas passadas.
> **Papel:** define o **núcleo mínimo** do Triad_AEE capaz de sustentar um
> neocórtex consolidado. É o documento mais restritivo da base: tudo que não
> fecha o circuito cognitivo sai do caminho crítico.
> **Tipo:** análise cirúrgica (prescrição arquitetural), não auditoria de código.

## 1. Tese central

Reduzir o Triad_AEE **sem empobrecê-lo**. O objetivo não é manter todos os
sistemas desenvolvidos nem apagar tudo que ainda não demonstrou efeito, e sim
descobrir o menor conjunto de mecanismos capaz de sustentar:

    substrato → organização → representação → integração → previsão
    → decisão → ação → consequência → aprendizagem → reorganização → continuidade

Tudo que não participa dessa cadeia, não a protege ou não permite sua
adaptação sai do caminho crítico (vira extensão). Estruturas de "cérebro
completo" (cerebelo, hipotálamo, gânglios da base, tronco) permanecem
possibilidades futuras, não obrigações do núcleo (§0 intro; §50).

## 2. Princípios fundamentais

1. **Não criar um módulo para cada conceito** (§1). Cascata, recursão,
   ressonância, campo cognitivo etc. são propriedades do sistema —
   rastreáveis pelo runtime/EventBus, não "órgãos" independentes.
2. **Cinco camadas funcionais** (§2): L1 Substrato, L2 Tecidual, L3 Cognição
   local, L4 Cognição global, L5 Metacognição/governança + Transversal
   (infraestrutura). As Ordens Cibernéticas O1–O5 atravessam as camadas;
   **não são** O1=L1, O2=L2 etc.
3. **Teste de admissão ao core** (§47) — cinco perguntas: qual estado único
   possui? qual input único consome? qual output único produz? quem consome
   esse output? qual função desaparece se ele for removido? Se a quinta
   resposta é "nenhuma", o módulo é redundante.
4. **Regra de fusão** (§48): dois módulos que leem quase os mesmos dados,
   controlam o mesmo actuator ou otimizam a mesma variável devem ser fundidos
   — destino de grande parte da governança atual.
5. **Promoção extensão→core** (§52): necessidade demonstrada + efeito
   reproduzível + consumidor real + benefício multi-seed + nenhuma duplicação.

## 3. Núcleo por camada

| Camada | Mínimo prescrito | Explicitamente fora |
|:--|:--|:--|
| L1 (§3) | ClusterState/Bio, energia, atividade, estado 97D, lifecycle, plasticidade local, grafo local, HOTM/Reservoir, Hebbian, homeostase, divisão/fusão/regeneração sob demanda | linguagem, identidade, World Model, metacognição, ecologia semântica, federação, leis |
| L2 (§6–7) | TissueRegistry/State, membership, affinity, bridges, coerência local, especialização, recurso local, adaptação; **feedback L3→L2 como prioridade estrutural** (há evidência de feedback chegando à fronteira sem ser admitido) | dezenas de classes rígidas de tecido — especialização emerge como dado, não arquitetura duplicada |
| L3 (§8–10) | 4 domínios: representação (semantic/concept/binding/coherence/grounding/emergence), memória local, predição local, seleção local; symbolic projection dentro do domínio representação | 6 controladores independentes para semântica; cascatas de estágio como sistemas separados |
| L4 (§12) | Attention Integration, Global Workspace, World Model, Causal Model, Memory Integration, Decision, Action/Outcome — **ativos ou em modo degradado; nunca desligados por pressão energética normal** | Workspace fazendo tudo (§13); 5 centros concorrentes de integração (§14–16: router/attention/competition/workspace/broadcast como subsistemas do workspace) |
| L5 (§23–27) | Apenas **3 controladores globais**: ResourceGovernor, MetaController, DevelopmentGovernor + Self Model/Identity separado da governança | Auto Adaptive, Architecture Governor/Cortex, Meta Regulator, Cognitive Economy, Thalamus Governor, Causal Graph Cortex, Self Engineering etc. como órgãos independentes |

Transversais: **learning** (§28–29) — essencial, nunca em standby, com cadeia
prediction→outcome→error→credit→update→future_behavior→**validation** (sem a
última seta não há evidência forte de aprendizagem); forgetting como política
dentro de learning; **cybernetics O1–O5** como capacidades/protocolos sobre
componentes existentes (§30–33; O5 lento — por episódio/janela/entre runs;
genome fora do loop principal); **governance** (§34–40: federação como
protocolo do ResourceGovernor com teste de existência — sem Effect é
telemetry aggregation; leis poucas, hard em Rust + soft em Lua, protegendo
invariantes e não microcomportamento; ecologia como um motor genérico +
adapters, só com população/variação/competição/recursos/seleção/extinção);
**crise** (§44) como máquina de estado transversal NORMAL→PRESSURE→CRISIS→
STABILIZATION→RECOVERY→REINTEGRATION que altera políticas, não duplica
sistemas; **evidência E0–E5** essencial e simples (§46): declared,
initialized, executed, produced, consumed, effect_validated.

## 4. Decisões estruturais-chave

- Morfogênese deixa de ser sistema central → `development/regeneration`,
  estado normal **STANDBY**; acorda por dano, falta comprovada de capacidade,
  fragmentação, sobrecarga persistente (§4).
- Algoritmos matemáticos (SVD, JL, RandLA, L2 sampling, kernels GPU,
  compressão, spatial hashing) → `platform/math|compute`: sustentam a
  cognição, não são subsistemas mentais (§5).
- Linguagem não entra no core → `extensions/language/`; primeiro provar
  representação→decisão→ação→aprendizagem (§11).
- Decisão explícita `l4/decision/` (proposal/competition/selection/commit):
  cadeia WorkspaceContent→DecisionProposal→DecisionSelected→
  DecisionCommitted→ActionRequest; **F2 é o grande gargalo histórico** (§19).
- Ação separada de decisão `action/{request,execution,outcome}` (§20) — cria a
  fronteira cognição→atuação→consequência; sem ela, aprendizagem causal fica
  ambígua.
- Memória consolidada `l4/memory/` (working/episodic/semantic/retrieval/
  reconsolidation/consolidation/provenance); HOTM trace fica em L1; allocortex
  vira parte de memory, não subsistema (§21–22).
- Cascata nasce do EventBus (trace_id, parent_event_id, source, target, step,
  payload_version → depth/breadth/latency/dead_end/effect) (§41); recursões
  verdadeiras são HOTM, Workspace, World Model, Learning, Meta, Cross-run —
  cada loop com iteration/gain/error/improvement/cost/**termination_reason**
  (§42–43).
- Sleep/Dream fora da primeira reescrita; AWAKE obrigatório (§45).

## 5. Cadeias canônicas (§56–62)

1. **Cognitiva:** Input→L1State→TissueState→Representation→WorkspaceCandidate→
   WorkspaceSelection→WorldPrediction→DecisionProposal→DecisionCommit→Action→
   Outcome→PredictionError→CreditAssignment→LearningUpdate→FutureBehavior.
   Qualquer módulo que não diga onde entra nessa cadeia precisa justificar a
   existência.
2. **Memória:** Experience→Encode→Store→Retrieve→**Consume**→Decision/
   PredictionEffect→Reconsolidate (sem Consume, `MemoryRetrieved` não é
   memória útil).
3. **Metacognição:** ObserveSystem→DetectProblem→GenerateProposal→
   ValidateProposal→Apply→ObserveEffect→Keep/Revert.
4. **Desenvolvimento:** CapacityProblem→DevelopmentGovernor→
   RegenerationProposal→LawValidation→Morphogenesis→StructuralEffect→
   CognitiveEffect→StopGrowth (embriogênese deixa de ser permanente).
5. **Federação:** Need→Proposal→Agreement→ResourceTransfer→**Effect**→
   Settlement (sem Effect, trade não é sucesso).
6. **Lei:** Invariant→Observation→Violation→Enforcement→**Postcondition**→
   EffectValidated (sem Postcondition, enforcement não está comprovado).
7. **Ecológica:** Variation→Competition→Selection→Survival/Extinction→
   Retention→FutureEffect (sem extinção + efeito futuro, não é
   evolução/ecologia funcional).

## 6. Estrutura e dimensionamento (§49, §73–74)

Árvore minimalista normativa (detalhada em `architecture/
estrutura_diretorios.md`): `config/`, `crates/{foundation, contracts, runtime,
l1_substrate, l2_tissue, l3_local, l4_global, l5_meta, learning, cybernetics,
governance, development, platform, triad}`, `lua/{policies, governance,
development, experiments}`, `schemas/`, `tests/`, `benches/`, `docs/`, `var/`.

Core ≈ **15–20 componentes runtime** (§73): cluster substrate, energy
homeostasis, reservoir/HOTM, tissue organization, tissue bridge/adaptation,
semantic representation, cognitive memory, attention/salience, Global
Workspace, World Model, causal model, decision, action/outcome, learning,
self model/identity, resource governor, meta controller, development
governor, laws/arbitration, persistence/telemetry. A meta não é reduzir por
estética (§74): se a evidência pedir 40 componentes, usam-se 40 — cada um
com responsabilidade única, estado definido, contrato definido e efeito
demonstrável. Modularidade não significa fragmentação.

## 7. Fluxos e migração (§53–55, §75–80)

- Ascendente: ClusterState→TissueState→Representation→WorkspaceContent→
  Decision. Descendente: Policy→attention/decision constraints→salience/
  learning targets→AdaptationRequest→plasticity/structural adjustment.
  Bidirecionalidade obrigatória; sem imports circulares: L5 publica
  PolicyProposal via contrato; L3→L2 usa AdaptationRequest.
- Migração: não apagar o Python inicialmente; inventário
  old_component→responsibility→state_owner→input→output→actuator→
  new_component; classificar **KEEP / MERGE / EXTENSION / ARCHIVE**; só
  então reescrever. Quatro marcos: (1) foundation+contracts+runtime+L1+L2+
  telemetry — provar mesma evolução estrutural com mesmo estado inicial;
  (2) L3 semantics/memory/attention — provar representação→memória→reuso;
  (3) L4 Workspace/WM/Causal/Decision/Action — **o marco mais importante**;
  (4) learning+Self Model+MetaController+ResourceGovernor — fechar
  decision→outcome→learning→future behavior. O5, linguagem e sistemas
  experimentais somente depois.

## 8. Veredicto da análise

Documento coerente e normativo: prescreve núcleo, regras de admissão, cadeias
canônicas e ordem de migração. Diverge parcialmente da fonte 2 sobre o lugar
das 12 regiões cerebrais no core (ver `02_Renomear.md` §8 e
`07_sintese_consolidada.md`, pendente). Lições diretas para a reescrita: as
regras §47–48 (admissão/fusão) devem virar **testes de arquitetura** desde o
primeiro crate; as cadeias §56–62 devem virar eventos tipados em
`crates/contracts`.
