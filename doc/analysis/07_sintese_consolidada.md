# Síntese consolidada — as seis fontes cruzadas

> Cruzamento das fontes 1–6 (ver `README.md` §1). Função: (a) fixar as
> **convergências** que viram regra; (b) expor as **divergências** com
> resolução proposta; (c) listar as **decisões a ratificar** via ADR;
> (d) consolidar o veredicto por camada do legado.
> Fontes citadas como f1 (NUCLEO_MINIMO), f2 (Renomear), f3 (L1),
> f4 (L1AL5), f5 (L2), f6 (L4L5).

## 1. Convergências (viram regra da nova estrutura)

1. **Organização por domínio**, não por tecnologia (f2 §1) e não por fases
   históricas ("FASE 1…FASE 94", f2 §56).
2. **L1–L5 = camadas funcionais**; O1–O5 = eixo transversal **matricial**
   (f1 §2, f4 §41, f6 §1) — nunca O1=L1 etc.
3. **Rust = mecanismo/estado canônico; Lua = política** (f2 §2–3, §58; f1
   usa `lua/` para políticas de energia/atenção/learning/crise e soft laws).
4. **Contratos tipados + eventos versionados substituem `_ctx`** (f2 §51;
   f3 P0; f5 P11; f6 R1). Todo valor: value/status/source/step/version/
   confidence/provenance.
5. **Ausência ≠ zero** (`VALUE/NO_DATA/STALE/INVALID/FALLBACK`) e **E0–E5
   sem promoção automática** (f3, f5, f6; f1 §46 adota a mesma escala).
6. **Learning transversal com `validation` como elo terminal** — a cadeia
   só fecha com `verified_future_effect` (f1 §28, f4 §33, f6 F-2).
7. **F2 (workspace→decisão) é o gargalo histórico** e piorou (22,9% → 6,7%;
   f1 §19, f4 §16, f6) — L4 é o alvo principal da reescrita.
8. **Feedback bidirecional obrigatório:** L3→L2 (AdaptationRequest) e a
   cascata descendente L5→L4→L3→L2→L1 (f1 §6/§55, f4 §32, f5 §6, f6 F-1 —
   no legado: L5toL4=0, learned=0 nas 6 fronteiras).
9. **Decisão e ação como domínios explícitos e separados** (f1 §19–20,
   f4 §17, f6 §3; no legado, decisão com identidade real já existe e é a
   melhor prova do sistema).
10. **Governança enxuta:** 3 governadores (Resource/Meta/Development), leis
    poucas protegendo invariantes, federação/ecologia como mecanismos
    genéricos (f1 §23–40, f2 §26; f6 mostra o oposto no legado: 26 módulos
    L5-de-fato, Graphify com governança ilhada).
11. **Morfogênese/embriogênese = regime de desenvolvimento, STANDBY** (f1 §4;
    f2 §27/§53 "processo temporal, não parte permanente do cérebro").
12. **Prova por gêmeas A/B, ablações por camada, vereditos pré-registrados**
    (f3 §9, f4 §42–48, f5 §12/Run 768, f6 R5/R8) — sem calibração
    pós-observação.
13. **L5 incremental:** acumuladores por camada; controle custa menos que a
    economia que gera (f4 §24, f6 F-6 — no legado 20,7 s × GW 1,26 s).
14. **Extensões fora do core**, promoção por critério demonstrado (f1 §50–52).
15. **Scheduler não é prova de atividade** (f3 P1, f6 §1) — should_run/run_
    count ≠ efeito.

## 2. Divergências e resolução proposta

| # | Divergência | Posições | Resolução proposta | ADR |
|:--|:--|:--|:--|:--|
| D1 | **12 regiões cerebrais no core?** | f2 §13: crates-irmãos de primeira classe (com cortical/subcortical em `brain/`); f1 §0/§50: cerebelo/tronco/gânglios/paleocortex FORA da primeira reescrita | Core = L1–L5 (f1); as 12 regiões viram **dimensão `region` do ModuleDescriptor** (mapa conceitual, f2 §52) + `extensions/full_brain/`; promoção caso a caso | ADR-0005, ADR-0006 |
| D2 | **Elo L3→L2 default** | f1 §6: prioridade estrutural (feedback à fronteira sem ser admitido); f5: elo completo porém **inerte por design** (P4 em aberto); f6: gargalo causativo é F2, não L2 | Não são exclusivos: o elo vira **contrato first-class com default ON** na nova estrutura (AdaptationRequest tipado), e L4 segue sendo a prioridade cognitiva. Ratificação exigida | ADR-0005 + contratos_camadas |
| D3 | **Linguagem/sleep/dream** | f1 §11/§45: fora do core; f2 §28: modes incluem sleep/dream; f6: nocturnal funciona só em janela longa (Run 1515) | Modes de **crise/recuperação são infraestrutura** (entram); dream/linguagem/inner speech = `extensions/` | ADR-0006 |
| D4 | **Taxonomias paralelas** | f6 §1: scheduler 5 níveis × ecology 6 níveis × 26 módulos "L3 por default" | Uma só taxonomia no descriptor com `source=REGISTRY` (f6 R7) | ADR-0005 |
| D5 | **L2 "desaparecida"** | f4 §3: L2 ausente do resumo (prioridade estrutural); f5: L2 existe, funciona como organização de estado, mas é periferia no grafo e inerte como moduladora | Ambos corretos em camadas diferentes: identidade estrutural (contrato próprio, lifecycle, TopologyController) **e** ativação do elo; sem contradição | contratos_camadas |

## 3. Veredicto consolidado do legado (por camada)

| Camada | Estado (fontes) | Evidência-chave |
|:--|:--|:--|
| L1 | **OPERATIONAL** (f3) | instrumentação validada em smoke + adulta (runs 741–752); 4 fases versionadas; ledger; O1-efeito `NOT_VALID_YET` (772/773); O3 runtime demonstrado (776/777) |
| L2 | **PRESENT / PARTIALLY_FUNCTIONAL** (f5) | E5-local fisiologia→sobrevivência; gêmeas 738/739 e 768 (E5=2 braço A); elo L3→L2 admitido=0 em produção corrente |
| L3 | **ATIVA com regressões** (f4) | 93,6% do scheduling, 1.059 conceitos, 74/85% reuso; MemCap 0,0506→0,0389; recurrence 0,408→0,189→0,307; depth=2 |
| L4 | **EXECUTÁVEL, GARGALO** (f4, f6) | F1 63,6% × F2 22,9%→6,7% × F3 88,9%; GW suspensa 2/3 da run adulta; WM sem uso de histórico; identidade desequilibrada (Self 1,0 × Narrative 0,093) |
| L5 | **QUASE INTEIRA MUDA EM JANELA CURTA** (f4, f6) | 13/18 suspensos na crise; 6 sem contadores; custo 96–113 s CPU/80 steps; cascata descendente inexistente (L5toL4=0); longitudinal_gate existe e rejeita, enforcement não |
| Ciclo | **NÃO FECHADO** (f4, f6) | learned=0 nas 6 fronteiras; 413 interrupções em learning; E4=64/E5=1 (768 braço A: 2) |

**Conclusão das fontes, unânime:** reconstruir L1–L5 com ownership, contratos e
feedback **antes** de ampliar população, linguagem ou módulos (f4 §1; f1 tese;
f5/f6 fornecem a evidência que justifica a ordem).

## 4. Decisões a ratificar (checklist para os ADRs)

- [ ] D1 — núcleo mínimo L1–L5; regiões como extensão (ADR-0005/0006).
- [ ] D2 — elo L3→L2 com default ON na nova estrutura (contrato
      AdaptationRequest; herda a lição f5 de que "completo porém inerte" não
      fecha ciclo).
- [ ] D3 — modes de crise no core; dream/linguagem como extensões (ADR-0006).
- [ ] D4 — taxonomia única no ModuleDescriptor; display agrega pelo mapa
      declarado (f6 R2a), nunca por default silencioso (ADR-0005).
- [ ] D5 — L4 nunca desligada: modo degradado obrigatório; crise altera
      política (overview §7; teste de aceite no plano_marcos).
- [ ] D6 — `verified_future_effect` como elo terminal de learning
      (LearningEnvelope; f6 R4).
- [ ] D7 — observação nunca muta estado observado (f5 achado 3: bridges na
      fase de record) → lei/invariante.
- [ ] D8 — custo do controle < economia gerada como critério de aceite de
      qualquer governador (f6 F-6/PERF-01).

## 5. Riscos de migração capturados das fontes

1. Portar `.py`→`.rs` um-a-um apenas transporta a arquitetura doente (f2 §57).
2. Módulos que leem os mesmos dados/controles o mesmo actuator devem ser
   fundidos na migração, não replicados (f1 §48).
3. Sem a escada E0–E5 instrumentada desde o primeiro crate, a nova estrutura
   reproduzirá "efeitos provados fora da obs" (f5 §5; f6 §3).
4. Sem vereditos pré-registrados, a calibração retroativa corrrompe a prova
   (f3 §17: "não autoriza calibrar a política retroativamente").
5. Extensões trazidas cedo multiplicam a superfície antes do circuito fechar
   (f1 §45; f6 mostra L5 experimental custando 96–113 s sem efeito medido).
