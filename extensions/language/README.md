Status: EXTENSÃO — fora do core (ADR-0006)

# language/ — stack linguística

A stack linguística completa do legado, retirada do caminho crítico por
custo sem efeito demonstrado (f6: meta_pipeline 96–113 s de CPU em 80
steps; f4 §20: linguagem é capacidade tardia, não fundacional).

Componentes preservados aqui:

- `linguistic_decoder`
- `language_predictor`
- `linguistic_feedback`
- `inner_speech`
- `emergent_language`
- `generative_decoder`
- `generative_pipeline`
- `linguistic_prediction`

## Fronteira com o core (ADR-0006, item 6)

- **L3 pode formar proto-representações** linguísticas: o core não é
  proibido de representar padrões linguísticos como conceitos comuns.
- **Interpretação contextual, inner speech e decisão linguística
  dependem de L4 consolidado** — até lá, permanecem extensão.
- Nenhum componente desta stack participa do circuito L1→…→learning
  nesta versão do core.

## Hook de hospedagem

`l3_local/semantic` expõe o ponto de extensão para linguagem: as
proto-representações de L3 são a única superfície pela qual esta
extensão lê (e eventualmente realimenta) o core. Nenhum componente
desta stack é importado pelo core.

## Promoção

Aplicam-se os cinco itens cumulativos do ADR-0006 (necessidade
demonstrada + efeito reproduzível + consumidor real no core + benefício
multi-seed + nenhuma duplicação), ADR próprio e testes de arquitetura
verdes.
