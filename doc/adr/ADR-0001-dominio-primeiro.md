# ADR-0001 — Domínio primeiro

- **Status:** PROPOSTO (ratificação pendente)
- **Decisores:** donos do projeto Triad_AEE
- **Fontes:** f2 §1–4; f1 §49; f2 §56.

## Contexto

O legado organiza-se por histórico ("FASE 1…FASE 94") e a proposta inicial de
reescrita corria o risco de organizar-se por tecnologia (`rust/`, `lua/`,
`gpu/`, `utils/`, `random_files/`). Isso define a arquitetura pelo mecanismo
de implementação e reproduz o acoplamento atual: `system.py` orquestrador
central (grau 308 no Graphify), responsabilidades sem endereço arquitetural.

## Decisão

Organizar o novo Triad_AEE por **domínio e responsabilidade**:
`domínio → módulo → implementação`. Rust e Lua são mecanismos de
implementação e nunca definem a arquitetura. A árvore de crates é a do
núcleo mínimo (f1 §49; ver `architecture/estrutura_diretorios.md`):
foundation, contracts, runtime, l1_substrate … l5_meta, learning,
cybernetics, governance, development, platform, triad. Cada conceito tem
endereço arquitetural claro; não existe um diretório `core/` gigante
(f2 §4: `core/misc.rs`, `core/manager.rs` são o anti-padrão).

## Alternativas consideradas

- Portar `.py`→`.rs` um-a-um: rejeitada — transporta a arquitetura doente
  (f2 §57).
- Organizar por camadas de tecnologia (rust/lua/gpu): rejeitada — acoplamento
  pelo mecanismo, não pela função.
- Organizar pelas 12 regiões cerebrais: parcialmente aceita **como dimensão
  do descriptor e extensão `full_brain`** (ver ADR-0005/0006).

## Consequências

- (+) Responsabilidade única com endereço único; fusão/abolição de módulos
  vira operação local.
- (+) Testes de arquitetura passam a ser possíveis por camada.
- (−) Custo de decisão inicial: todo componente legado precisa ser
  classificado antes de ser reescrito (inventário em `migration/`).
- (−) Crates a mais no workspace — mitigado pelo agregador enxuto por domínio.
