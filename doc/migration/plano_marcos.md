# Plano de reescrita — marcos e critérios de aceite

> Deriva de f1 §75–80 (migração e marcos), f4 §42–48 (fases e ablações),
> f6 R5/R8 (baterias experimentais), f3 §8–9 (critérios de aceite das
> auditorias). Complementa `inventario_migracao.md` (classificação).

## 1. Princípios

1. Não apagar o Python no início (f1 §75): o legado é a referência de
   comportamento até o marco 3.
2. Nada é reescrito antes do inventário campo a campo
   (`old→responsibility→owner→input→output→actuator→new`).
3. Vereditos pré-registrados; sem calibração retroativa (f3 §17).
4. Gêmeas A/B para mudança comportamental (protocolo t+1/t+5, sham pareado,
   `origin_state_hash` igual); A/A bit-idêntica para mudança de display
   (padrão das runs 772/773 e 768).
5. Custo do controle < economia gerada como aceite de qualquer governador
   (síntese D8; f6 F-6).

## 2. Fase 0 — fundação (antes de qualquer camada)

- Contratos, IDs, ownership, versionamento, observabilidade (f4 §42):
  `foundation` (ids/status/valor), `contracts` (eventos das 7 cadeias,
  TypedContext), descriptor de módulo, E0–E5 por módulo, ledger de passagem.
- Preencher o inventário com owners/inputs/outputs/actuators reais.
- Suíte `tests/architecture/` verde desde o primeiro commit (impede a
  regressão estrutural — f2 §43).
- Fechar as ratificações D1–D8 da síntese (ADRs 0001–0006 assinados).

## 3. Marcos

### Marco 1 — fundação + runtime + L1 + L2 + telemetry (f1 §76)

Portar: foundation, contracts, runtime (scheduler/executor/event_bus/
lifecycle/TypedContext), L1 (cluster/energy/lifecycle/plasticity/graph/
reservoir), L2 (tissue/registry/affinity/bridge/adaptation/**feedback ON**),
telemetry.
**Prova de aceite:** mesmo estado inicial → mesma evolução estrutural dentro
dos limites numéricos esperados. Critérios de conclusão L1 (f4 §3: homeostase
fechada, Hebbian real ou BYPASSED, histerese) e L2 (f4 §8: tecidos com
lifecycle, TopologyController único convergindo, bridges fora do record,
elo L3→L2 admitindo com efeito observado). Ablação L1 e L1+L2 (f4 §43–44).

### Marco 2 — L3 (f1 §77)

Portar: semantic (representation/concept/binding/coherence/grounding/
emergence), memory local (WorkingMemory/TemporalTrace/SemanticLTM/Episodic
separados), attention, prediction.
**Prova:** representação→memória→reuso (74/85% de reuso do legado como
baseline); recorrência estável e MemCap sem queda (regressões do legado:
0,408→0,189→0,307 e 0,0506→0,0389 não se reproduzem); DAG de conceitos com
depth orgânico >2. Ablação L1–L3 (f4 §45).

### Marco 3 — L4 (o mais importante, f1 §78)

Portar: integration (workspace + subsistemas), world_model (3 camadas com
**uso contextual de histórico**), causal, memory_integration, decision
(explicito), action/outcome.
**Prova:** F2 (workspace→decisão) > melhor valor histórico do legado
(22,9%) com perdas tipadas por fronteira; decisões com identidade completa
e outcome; L4 **viva em regime adulto** (modo degradado, nunca SHUTDOWN —
no legado F2 caiu a 6,7% com GW suspensa 2/3 da run). Ablação L1–L4 (f4 §46).

### Marco 4 — learning + L5 (f1 §79)

Portar: learning (com `validation` terminal + LearningEnvelope), self_model/
identity, meta_controller, resource_governor, development_governor, leis/
arbitragem, federação.
**Prova:** fechar **decision→outcome→learning→future_behavior** com
`verified_future_effect` > 0 (no legado: learned=0 nas 6 fronteiras, 413
interrupções); cascata descendente com pelo menos uma intervenção L5→L4 com
`observed_effect` e keep/revert (f6 R3); custo do meta_controller O(1) por
chamada via acumuladores (f6 R6).

### Depois do marco 4 (f1 §80)

O5 (lento), linguagem, dream, full-brain, ecologias experimentais —
exclusivamente via promoção ADR-0006.

## 4. Baterias de validação (transversais aos marcos)

- **Ablação cumulativa** (f4 §48; f6 R5): L1, L1+L2, L1–L3, L1–L4, L1–L5 —
  mesma seed, mesmos estímulos; medir Causal Yield, RCM, learning closure,
  memory retention, generalization, adaptation, runtime, RAM, VRAM.
- **Teste final de L5 (f6 R8 = f4 §47):** par L5-ON × L5-OFF, janela adulta
  ≥60 steps — "se L5 aumenta custo sem melhorar aprendizagem/adaptação,
  NÃO está funcional". Veredito pré-registrado.
- **Run adulta de estabilidade:** 200–300 steps (f6 F-11) — o legado nunca
  validou além de 80; janelas curtas escondiam módulos com intervalo ×1.5.
- **Matriz E5 por componente** (f6 F-11): meta de E4=64/E5=1 → cada
  componente do core com efeito identificado no tempo.

## 5. Gates de não-retorno (bloqueiam avançar de marco)

1. Qualquer violação de `tests/architecture/`.
2. Fallback publicado como VALUE; STALE tratado como valor (lei-contrato).
3. Componente sem as cinco perguntas respondidas no PR.
4. Extensão promotida sem os 5 critérios cumulativos (ADR-0006).
5. Veredito calibrado após a observação (corrupção de prova).
6. Módulo L4 SUSPENDED sem razão explícita (modo degradado obrigatório).
