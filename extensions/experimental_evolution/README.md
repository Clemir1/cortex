Status: EXTENSÃO — fora do core (ADR-0006)

# experimental_evolution/ — evolução estrutural contínua

Sistemas evolutivos do legado que executavam continuamente no caminho
crítico, hoje suspensos como extensões:

- **Genoma cognitivo** — parâmetros estruturais herdados/mutados
  executando a cada step.
- **Structural noise** — perturbação contínua da estrutura de
  clusters/tecidos.
- **Entropic injection** — injeção contínua de entropia/variância no
  substrato.

Evidência de custo (f6): 13/18 módulos L5 suspensos na crise; custo de
CPU contínuo sem efeito medido em multi-seed.

## Fronteira com o core

- No core, **O5** (`crates/cybernetics/o5_evolution`) avalia políticas
  e genomas **lentamente, fora do loop principal**: evolução é
  observação + proposta validável, nunca mutação direta durante a
  cognição.
- `development/` (embryogenesis, corticalization, maintenance,
  regeneration) cobre mudança estrutural planejada; esta extensão cobre
  a variação contínua de pesquisa, em ritmo experimental.

## Promoção

Cinco itens cumulativos do ADR-0006 + ADR próprio + testes de
arquitetura verdes. O caminho natural passa pelo O5: demonstrar em
bateria multi-seed que a variação contínua produz benefício líquido
sobre a avaliação lenta já existente no core.
