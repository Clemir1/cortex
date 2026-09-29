Status: EXTENSÃO — fora do core (ADR-0006)

# cerebellum — coordenação temporal/preditiva

Papel funcional (fonte 2 §5, divisão 04): timing, prediction-error
coordination, trajectory correction, sequence coordination e precisão
temporal.

## Absorvido pelo core por enquanto

- `l3_local/prediction`: predição local como um dos quatro domínios L3.
- `learning/prediction_error`: o circuito de erro de predição já fecha
  no core (prediction_error → credit → update).

## Restaria exclusivo à extensão

- Precisão temporal fina como módulo dedicado.
- Correção de trajetória e coordenação de sequências complexas,
  independentes do pipeline de decisão.

Promoção: cinco itens cumulativos do ADR-0006 + ADR próprio.
