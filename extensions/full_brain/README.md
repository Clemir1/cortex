Status: EXTENSÃO — fora do core (ADR-0006)

# full_brain/ — as 12 divisões funcionais como implementações futuras

As 12 divisões funcionais da fonte 2 (§5) — brainstem, hypothalamus,
thalamus, cerebellum, basal_ganglia, limbic, hippocampal, sensory,
association, executive, action, metacognition — vivem aqui como
**mapa conceitual de implementações futuras**, não como crates.

> **NOTA:** as 12 divisões são uma **abstração computacional funcional,
> não anatomia literal** (fonte 2 §5): não existe divisão científica
> universal em "12 partes do cérebro". Os nomes são metáforas de domínio,
> nunca afirmações biológicas.

## Relação com o core

- As divisões **não são crates do core** nesta versão (decisão pendente
  registrada em `doc/architecture/estrutura_diretorios.md` §6).
- A dimensão `region` vive no **ModuleDescriptor** do core, ao lado de
  `layer`, `domain`, `entity_kind` e `execution_pipeline` — dimensões
  independentes, nunca inferidas umas das outras (fonte 2 §52).
- L1–L5 e as regiões **coexistem**: thalamus, por exemplo, tem substrato
  em L1, circuito tecidual em L2, routing local em L3, integração em L4
  e governor em L5.
- Enquanto não promovidas, a função de cada divisão é absorvida pelos
  crates mínimos do core — ver o README de cada subdiretório.

## Subdiretórios

    brainstem/   hypothalamus/  thalamus/      cerebellum/
    basal_ganglia/  limbic/     hippocampal/   sensory/
    association/  executive/    action/        metacognition/

## Promoção

Cada divisão é promovida **individualmente**, pelos cinco itens
cumulativos do ADR-0006 + ADR próprio + testes de arquitetura verdes.
Governança (leis, federação, ecologia) não é 13ª região: fica no core,
em `governance/` (fonte 2 §6).
