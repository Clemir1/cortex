# ADR-0007 — Recursão Chladni na cognição (bônus de atenção + memória por ressonância)

- **Status:** ACEITO (diretriz do dono: "cadeia completa funcional e
  aprimorada e recursão, assim como no sistema legado"; checklist 16.4)
- **Fontes:** legado `attention_system.py` §90–165 (bônus peso 0.15),
  `development.py` §720–745 (banda por estágio), `chladni_frequencies.py`
  (`_compare_patterns`); checklist 13.2–13.5 (motor entregue) e 16.4;
  ANALISE_WGPU_WGSL.txt.txt §4.8 (diretriz de escala do dono).

## Contexto

O motor Chladni (`triad-chladni`, seção 13) publica a cada tick o sinal
`l1.substrate.chladni` — ressonância média qualificada de 4 clusters
vivos. Até aqui o sinal era ADITIVO: nenhuma camada o consumia para
pensar. A diretriz do dono pede a cadeia COMPLETA com recursão: a
harmonia do substrato deve realimentar a cognição (atenção L3, L4,
memória) como no legado. Achado técnico bloqueador corrigido no
caminho: o scheduler cria UM CONTEXTO POR MÓDULO (scheduler.rs), então
consumo por `ctx.get_qualified` cross-módulo nunca vê a chave — o
consumer original do development (13.4) estava morto. Débito corrigido
neste ADR: sinal trafega por PONTE READ-ONLY (padrão L1→L2→L3→L4).

## Decisão

1. **Promoção concedida** pelos critérios da casa (ADR-0006 §3),
   cumulativos: necessidade demonstrada (diretriz explícita do dono),
   efeito reproduzível (testes A/A abaixo), consumidor real (atenção L3
   → focos → `read_for_l4` → workspace/decisão L4).
2. **Bônus de atenção L3:** a saliência de cada foco ganha
   `+ ressonância × peso`, com peso em
   `[l3.attention].chladni_bonus_weight` (default 0.15, espelho do
   legado e de `[chladni].attention_weight`). Peso 0 = recursão
   desligada, comportamento BIT-IDÊNTICO ao anterior (política
   desligável). Sinal ausente/não-VALUE = sem bônus, contado em
   telemetria com denominador (`chladni_stats()`), nunca zero fantasma.
3. **Transporte por ponte read-only:** `ClusterModule::last_chladni()`
   (observação do passo corrente — o L1 ticka antes do L3); o L3 liga a
   ponte via `L3Module::with_chladni_source(Arc<ClusterModule>)`. A
   observação nunca muta o dono. O development consumidor idem
   (`DevelopmentModule::with_chladni_source`) — correção do débito 13.4.
4. **Efeito visível ao consumidor real:** o `read_for_l4` soma o mesmo
   bônus com o mesmo peso — sem ponte, byte-idêntico ao anterior (A/A
   das seções 14/15 da sessão 7 preservado); com ponte, a harmonia
   entra nos focos que o workspace/decisão L4 consomem.
5. **Memória por ressonância:** episódios podem carregar
   `ResonanceSignature` (banda + ressonância + amostra do tick da
   gravação; construída SÓ de sinal VALUE) e `similar_by_resonance`
   ranqueia por distância de banda × delta de ressonância — herdeiro
   do `_compare_patterns` do legado. Episódio sem assinatura não
   participa (ausência ≠ zero). Consumidores: reforço L4→L3 (16.2) e
   estágios de development (16.6).
6. **Leis preservadas:** ausência ≠ zero em toda a cadeia; taxas com
   denominador; determinismo bit-exato por seed; CPU canônica intacta
   (o passo físico do L1 não muda).

## Verificação

- Testes novos: bônus reproduzível e determinístico (duas runs com
  recursão = bit-idênticas; contra sem recursão, pesos DIFEREM e são
  maiores); peso 0 = idêntico ao sem-fonte e zero bônus contado;
  similaridade prefere mesma banda/ressonância próxima, determinística,
  sem assinatura não participa.
- Workspace 190 testes verdes; house_laws 5/5; app 65 ticks com bônus
  em 65/65 e "Núcleo encerrado sem violar as leis da casa".

## Consequências

- A harmonia do substrato agora realimenta o pensamento (atenção →
  workspace/decisão) de forma controlada, medida e desligável.
- Escala futura (boot 1.200, teto 30K — seção 17): o bônus é O(1) por
  foco; a amostragem do sinal continua 4/tick (custo constante).
- Banda por estágio no development (16.6) e uso do peso de atenção em
  políticas Lua seguem o MESMO peso desta config — uma única política
  central, sem duplicação.
