Status: EXTENSÃO — fora do core (ADR-0006)

# advanced_symbolics/ — symbolics avançadas

Manipulação simbólica de alto nível não necessária para fechar o
circuito L1→…→learning: composição simbólica profunda, raciocínio sobre
estruturas simbólicas formais e usos de símbolos além do registro
conceitual básico.

## Fronteira com o core

- O core cobre o mínimo simbólico em `l3_local/semantic`: conceitos,
  emergência semântica, coerência e grounding — o suficiente para L3
  alimentar o workspace com conteúdo representacional.
- Tudo que excede esse registro mínimo vive aqui, sem custo para o
  runtime adulto.

## Vizinhança conceitual (evitar duplicação)

- `extensions/full_brain/association` — binding e projeção simbólica
  como divisão funcional futura.
- `extensions/experimental_ecologies` — symbol ecology.
- Antes de qualquer promoção, o item "nenhuma duplicação funcional"
  do ADR-0006 deve endereçar explicitamente essa sobreposição.

## Promoção

Cinco itens cumulativos do ADR-0006 + ADR próprio + testes de
arquitetura verdes.
