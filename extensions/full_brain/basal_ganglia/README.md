Status: EXTENSÃO — fora do core (ADR-0006)

# basal_ganglia — seleção de ação

Papel funcional (fonte 2 §5, divisão 05): proposal competition, action
gating, inhibition, selection e política recompensa/ação.

## Absorvido pelo core por enquanto

- `l4_global/decision`: proposal, competition, selection e commit já
  formam o subsistema canônico de decisão do core.
- `learning/credit`: recompensa observada em outcome converte-se em
  crédito de aprendizagem.

## Restaria exclusivo à extensão

- Inibição lateral entre ações concorrentes como mecanismo próprio.
- Política recompensa/ação como módulo de gating dedicado (fonte 2 §18:
  Workspace → proposals → competition → selected action).

Promoção: cinco itens cumulativos do ADR-0006 + ADR próprio.
