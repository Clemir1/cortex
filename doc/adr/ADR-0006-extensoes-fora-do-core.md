# ADR-0006 — Extensões fora do core

- **Status:** PROPOSTO (ratificação pendente — decide a divergência D3)
- **Fontes:** f1 §11/§45/§50–52; f2 §48/§54; f4 §20 (linguagem tardia);
  f6 §3–4 (custo de L5 experimental sem efeito medido).

## Contexto

O legado carrega no caminho crítico sistemas experimentais caros e sem
efeito demonstrado: stack de linguagem (linguistic_decoder, language_
predictor, linguistic_feedback, inner_speech, emergent_language,
generative_decoder, generative_pipeline, linguistic_prediction), dream/
consolidação noturna, ecologias especializadas (symbol/binding/causal
ecology, ecologies_hub, ecology_observer), genoma cognitivo executando
continuamente, structural noise e entropic injection contínuas. Evidência
de custo (f6): meta_pipeline 96–113 s CPU em 80 steps, cognitive_economy
20,7 s contra GW 1,26 s; 13/18 módulos L5 suspensos na crise; nocturnal
NOT_EXECUTED em 5/5 runs curtas. Evidência de foco (f1 §45): antes de fechar
decision→outcome→learning, aumentar a superfície arquitetural atrasa o
objetivo principal.

## Decisão

1. **Nada é apagado:** sistemas não-essenciais saem do caminho crítico para
   `extensions/` ou `legacy/research/` (f1 §50).
2. **Extensões iniciais** (f1 §51):
   `extensions/{language, dream, full_brain, experimental_ecologies,
   experimental_evolution, advanced_symbolics}`.
3. **Critério de promoção extensão→core** (f1 §52) — todos os cinco,
   cumulativos: necessidade demonstrada + efeito reproduzível +
   consumidor real no core + benefício multi-seed + nenhuma duplicação
   funcional. Promoção exige ADR próprio + testes de arquitetura verdes.
4. **Direção única:** extension → core, nunca o contrário (f1 §52).
5. Modos de crise/recuperação **ficam no core** (infraestrutura de
   sobrevivência); sleep/dream ficam como extensão (f1 §45: AWAKE é
   obrigatório na primeira versão; síntese D3).
6. Linguagem: L3 pode formar proto-representações; interpretação
   contextual/inner speech/decisão linguística dependem de L4 consolidado
   (f4 §20) — extensão até lá.
7. `legacy/python/` read-only temporário; nenhum código novo depende dele;
   arquivado fora do runtime ao fim da migração (f2 §48).

## Consequências

- (+) O core persegue um único circuito (f1 conclusão): L1→L2→L3→L4→
  decisão→ação→outcome→learning→feedback→reorganização→novo comportamento.
- (+) Custo experimental deixa de ser pago pelo runtime em regime adulto.
- (+) Pesquisa preservada e endereçável (não "jogada fora").
- (−) Extensões precisam de contratos de hospedagem (hooks estáveis) para
  poderem ser promovidas sem refatoração — os hooks são definidos junto aos
  crates donos (ex.: `l3_local/semantic` expõe ponto de extensão para
  linguagem).
- (−) O critério de promoção exige bateria multi-seed — custo de prova
  deliberado (é o mesmo gate que a casa já usa para O2–O5).
