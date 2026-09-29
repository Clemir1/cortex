Status: EXTENSÃO — fora do core (ADR-0006)

# brainstem — sobrevivência operacional

Papel funcional (fonte 2 §5, divisão 01): arousal, lifecycle,
wake/sleep, emergency state, basic survival e viabilidade de runtime.

## Absorvido pelo core por enquanto

- `runtime/lifecycle` (activation/suspension/recovery): viabilidade de
  execução é infraestrutura, não região cognitiva.
- Modos de crise/recuperação **ficam no core** (ADR-0006 item 5):
  sobrevivência não é extensão.

## Restaria exclusivo à extensão

- Arousal como sinal cognitivo contínuo do organismo.
- Transições wake/sleep como estado fisiológico (hoje apenas modo de
  política; o sistema em si vive em `extensions/dream`).
- Reflexos de sobrevivência de baixo nível, sem passagem por L4.

Promoção: cinco itens cumulativos do ADR-0006 + ADR próprio.
