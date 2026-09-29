Status: EXTENSÃO — fora do core (ADR-0006)

# action — saída e agência

Papel funcional (fonte 2 §5, divisão 11): action plans, actuators,
execution, outcomes, efference copy, external interaction e agency
tracking. Princípio: **decision ≠ execution** (fonte 2 §5).

## Absorvido pelo core por enquanto

- `l4_global/action`: request, execution e outcome — a cadeia canônica
  DecisionProposal → SelectedAction → ActionExecution → Outcome →
  LearningSignal já fecha no core (fonte 2 §24).
- `learning`: outcome converte-se em sinal de aprendizagem.

## Restaria exclusivo à extensão

- Efference copy e agency tracking como mecanismos próprios.
- Atuadores externos e planos de ação compostos de múltiplos passos.

Promoção: cinco itens cumulativos do ADR-0006 + ADR próprio.
