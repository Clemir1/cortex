Status: EXTENSÃO — fora do core (ADR-0006)

# experimental_ecologies/ — ecologias especializadas

No core existe **apenas um motor ecológico genérico**
(`crates/governance/ecology`): population, fitness, competition,
selection, birth, decay, extinction — um único mecanismo, sem cópias
por domínio.

Especializações ecológicas do legado vivem aqui:

- `symbol ecology` — competição/seleção de símbolos e conceitos.
- `binding ecology` — ecologia de binding semântico/simbólico.
- `causal ecology` — ecologia de hipóteses causais.
- `ecologies_hub` — coordenação entre múltiplas ecologias.
- `ecology_observer` — observação/telemetria específica de ecologias.

## Fronteira com o core

- O motor genérico de `governance/ecology` é o único ponto de
  hospedagem: especializações configuram o motor (fitness, regras de
  seleção via políticas Lua) — não reimplementam
  competição/seleção/extinção.
- `ecologies_hub` e `ecology_observer` só fazem sentido com mais de um
  motor instanciado; enquanto o core tem um, permanecem extensão.

## Promoção

Cinco itens cumulativos do ADR-0006 + ADR próprio + testes de
arquitetura verdes. Pergunta-guia do item "nenhuma duplicação": a
especialização é **configuração** do motor genérico (então pertence ao
core como dado/política) ou **mecanismo novo** (então precisa provar
que não duplica o motor)?
