# ADR-0005 — Núcleo mínimo L1–L5

- **Status:** PROPOSTO (ratificação pendente — decide a divergência D1/D4)
- **Fontes:** f1 §2/§21/§49/§50/§73–74; f2 §5/§13/§52/§54; f4 §50; f5 §4;
  f6 §1 (evidência da fragmentação); síntese §2 D1–D5.

## Contexto

Duas propostas concorrentes de estrutura para a reescrita:
(a) **f2**: 12 divisões funcionais inspiradas no cérebro como crates-irmãos
de primeira classe (`brain/regions/…`, com cortical/subcortical);
(b) **f1**: núcleo mínimo de 5 camadas funcionais (L1–L5), com estruturas de
"cérebro completo" (cerebelo, tronco, gânglios da base, paleocortex)
**fora** da primeira reescrita. Enquanto isso, o legado prova o custo da
fragmentação: 3 taxonomias paralelas, 26 módulos L5-de-fato contados como
L3 por default, L2 invisível no resumo, learned=0 nas 6 fronteiras e
reguladores sobrepostos lendo os mesmos dados (f6 §1; f1 §48).

## Decisão

1. **O core é o núcleo mínimo L1–L5** (f1 §49): cinco camadas funcionais +
   transversais (learning, cybernetics O1–O5, governance, development,
   runtime, platform), ~15–20 componentes runtime (f1 §73).
2. **As 12 divisões de f2 não são crates do core**: viram a dimensão
   `region` do ModuleDescriptor (mapa conceitual matricial com `layer`,
   f2 §52) e implementações concretas em `extensions/full_brain/`.
   Exceção: nenhuma — promoção passa pelo critério do ADR-0006.
3. **Taxonomia única** no descriptor com `source=REGISTRY`; display agrega
   pelo mapa declarado (f6 R2a); "L5* INFERRED" e default silencioso são
   abolidos.
4. **L4 nunca desligada**: modo degradado obrigatório; crise altera
   política, não existência (f1 §12/§44; síntese D5).
5. **Elo L3→L2 first-class com default ON** (AdaptationRequest tipado;
   síntese D2) — o legado provou que "estruturalmente completo porém
   inerte" não fecha ciclo (f5 §6: admitido=0 em produção corrente).
6. Modularidade ≠ fragmentação (f1 §74): se a evidência pedir 40
   componentes, usam-se 40 — cada um com responsabilidade única, estado,
   contrato e efeito demonstrável (gate das cinco perguntas + fusão).

## Alternativas consideradas

- **12 regiões como core (f2 §13):** rejeitada para a primeira reescrita —
  multiplica a superfície arquitetural antes de fechar
  decision→outcome→learning (f1 §45); mantida como mapa conceitual + extensão.
- **Núcleo mínimo sem as regiões no descriptor:** rejeitada — perderia a
  dimensão organizacional que permite evoluir "córtex → cérebro completo"
  sem reestruturar (f2 §54).
- **Manter o legado Python e instrumentar apenas:** rejeitada — o
  gargalo é estrutural (F2 22,9%→6,7%, learned=0, cascata inexistente);
  instrumentação já deu o que tinha a dar (f3: L1 OPERATIONAL; f6: resto
  não fecha).

## Consequências

- (+) Uma só verdade de classificação; resumos honestos por camada.
- (+) Foco total da reescrita no gargalo comprovado (L4/decisão +
  learning), sem pagar antecipadamente estruturas de cérebro completo.
- (+) Caminho de crescimento preservado (regiões entram por promoção).
- (−) Proponentes de estruturas subcorticais precisam esperar o critério
  de promoção — registrado explicitamente no ADR-0006.
- (−) O descriptor precisa existir desde o marco 1 (custo inicial).
