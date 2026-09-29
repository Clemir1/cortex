Status: EXTENSÃO — fora do core (ADR-0006)

# extensions/ — sistemas fora do caminho crítico

Este diretório abriga os sistemas **não-essenciais ao circuito mínimo**
do núcleo: L1 → L2 → L3 → L4 → decisão → ação → outcome → learning →
feedback → reorganização → novo comportamento. O core persegue um único
circuito (ADR-0006); tudo que não fecha esse circuito — mas cuja
pesquisa deve permanecer endereçável — vive aqui ou em `legacy/research/`.

Nada aqui é crate do workspace. Este é um esqueleto de documentação:
cada subdiretório descreve domínio, fronteira de hospedagem e o que
deverá ser demonstrado antes de qualquer promoção. Código nasce apenas
quando houver hipótese experimental formal — nunca para preencher a
estrutura.

## Estrutura inicial (ADR-0006, item 2)

    extensions/
    ├── language/                # stack linguística completa
    ├── dream/                   # sleep/dream e consolidação noturna
    ├── full_brain/              # as 12 divisões funcionais como mapa futuro
    │   ├── brainstem/   hypothalamus/  thalamus/     cerebellum/
    │   ├── basal_ganglia/  limbic/    hippocampal/  sensory/
    │   └── association/  executive/    action/       metacognition/
    ├── experimental_ecologies/  # ecologias especializadas
    ├── experimental_evolution/ # genoma cognitivo, structural noise,
    │                            # entropic injection contínuas
    └── advanced_symbolics/      # symbolics não críticas ao circuito

## Regras

1. **Direção única:** extensão → core, nunca o contrário (fonte 1 §52).
   O core não conhece extensões; extensões conhecem os hooks do core.
2. **Hospedagem por hooks estáveis:** os contratos de hospedagem são
   definidos junto aos crates donos (ex.: `l3_local/semantic` expõe o
   ponto de extensão para linguagem). Sem hook estável, não há
   promoção sem refatoração.
3. **Nada é apagado:** sistemas saem do caminho crítico para
   `extensions/` ou `legacy/research/` — nunca para o lixo.
4. **Custo experimental não é pago pelo runtime adulto:** o core não
   executa código de extensão em regime de cognição adulta.

## Critério de promoção extensão → core

Todos os **cinco itens, cumulativos** (ADR-0006, item 3; fonte 1 §52):

1. **Necessidade demonstrada** — o core sente falta em cenário real.
2. **Efeito reproduzível** — benefício medível e repetível.
3. **Consumidor real no core** — existe código do core que usa o
   sistema (não "vai usar um dia").
4. **Benefício multi-seed** — o efeito aparece em sementes diversas,
   não apenas na semente que motivou a pesquisa.
5. **Nenhuma duplicação funcional** — o core não ganha duas versões
   da mesma capacidade.

Promoção **exige ADR próprio** e testes de arquitetura verdes
(`tests/architecture/`) após a mudança. A ausência de qualquer item
mantém o sistema aqui — sem prazo, sem pressa.

## Referências

- `doc/adr/ADR-0006-extensoes-fora-do-core.md` (decisão normativa)
- `doc/architecture/estrutura_diretorios.md` §3 (árvore de extensões)
- `NUCLEO_MINIMO.txt.txt` §§51–52 (extensões e critério de promoção)
- `Renomear.txt.txt` §5 (as 12 divisões funcionais)
