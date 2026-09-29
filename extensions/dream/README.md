Status: EXTENSÃO — fora do core (ADR-0006)

# dream/ — sleep/dream e consolidação noturna

Sono, sonho e consolidação noturna: reorganização em segundo plano,
replay de episódios e integração de memória fora do ciclo ativo.

## Por que extensão

- **AWAKE é obrigatório na primeira versão do core** (fonte 1 §45): o
  núcleo mínimo precisa fechar o circuito L1→…→learning sem depender
  de processamento noturno.
- Evidência de custo (f6): consolidação noturna `NOT_EXECUTED` em 5/5
  runs curtas — o preço arquitetural era pago sem efeito observado.
- Modos de crise/recuperação **ficam no core** (são infraestrutura de
  sobrevivência); sleep/dream **ficam como extensão** (síntese D3,
  ADR-0006 item 5).

## Fronteira com o core

- O core nunca depende de sonho para consolidar ou corrigir estado:
  `l3_local/memory` e `l4_global/memory_integration`
  (consolidação/reconsolidação) operam em AWAKE.
- A extensão, quando ativa, atua via hooks de hospedagem estáveis:
  consome snapshots de memória e produz **propostas de reorganização
  validáveis** — nunca escrita direta no estado do core.

## Promoção

Cinco itens cumulativos do ADR-0006 + ADR próprio + testes de
arquitetura verdes — provando, em bateria multi-seed, efeito
reproduzível de consolidação que apareça no comportamento do
organismo em AWAKE.
