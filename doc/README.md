# doc/ — Base documental do Cortex (Triad_AEE · núcleo mínimo)

Esta pasta consolida a **análise, investigação e auditoria** dos seis
documentos-fonte do Triad_AEE e estabelece a documentação de referência da
nova estrutura (reescrita em Rust + Lua, núcleo mínimo L1–L5).

Regra da casa (herdada das auditorias): nenhum documento aqui substitui o
código ou uma run; toda afirmação de estado cita a fonte e o nível de
evidência E0–E5. Documento não promove veredito.

## 1. Fontes analisadas

| # | Arquivo | Tam. | Conteúdo | Papel |
|:--|:--|--:|:--|:--|
| 1 | `D:\cortex\NUCLEO_MINIMO.txt.txt` | 2.172 linhas | Análise cirúrgica do núcleo mínimo (75 seções + conclusão) | Define **o que** entra no core |
| 2 | `D:\cortex\Renomear.txt.txt` | 2.016 linhas | Reestruturação completa para Rust + Lua (58 seções + conclusão) | Define **como organizar** o projeto |
| 3 | `C:\Pictures\TB\tb\LAB\Triad_AEE\L1.md` | 663 linhas | Auditoria da camada L1 | Estado atual do substrato |
| 4 | `C:\Pictures\TB\tb\LAB\Triad_AEE\L1AL5.txt` | 1.104 linhas | Análise temporal da reconstrução L1–L5 (50 seções) | Diagnóstico + arquitetura-alvo |
| 5 | `C:\Pictures\TB\tb\LAB\Triad_AEE\L2.md` | 446 linhas | Auditoria da camada L2 | Estado atual dos tecidos |
| 6 | `C:\Pictures\TB\tb\LAB\Triad_AEE\L4L5.md` | 316 linhas | Auditoria funcional L4 × L5 | Estado atual da cognição global/meta |

Leitura: 100% das linhas dos seis arquivos foram lidas nesta sessão
(as fontes 1 e 2 em duas passadas por truncamento).

## 2. Estrutura e estado desta pasta

    doc/
    ├── README.md                        # este índice
    ├── analysis/                        # análise por documento-fonte
    │   ├── 01_NUCLEO_MINIMO.md         # ESCRITO (parte 1)
    │   ├── 02_Renomear.md              # ESCRITO (parte 1)
    │   ├── 03_auditoria_L1.md          # ESCRITO (parte 2)
    │   ├── 04_analise_L1AL5.md         # ESCRITO (parte 2)
    │   ├── 05_auditoria_L2.md          # ESCRITO (parte 3)
    │   ├── 06_auditoria_L4L5.md        # ESCRITO (parte 3)
    │   └── 07_sintese_consolidada.md   # ESCRITO (parte 4)
    ├── architecture/                    # arquitetura-alvo da nova estrutura
    │   ├── overview.md                 # ESCRITO (parte 1)
    │   ├── estrutura_diretorios.md     # ESCRITO (parte 1)
    │   ├── contratos_camadas.md        # ESCRITO (parte 4)
    │   ├── rust_lua_boundary.md        # ESCRITO (parte 5)
    │   └── dependency_rules.md         # ESCRITO (parte 5)
    ├── adr/                             # decisões arquiteturais
    │   ├── ADR-0001-dominio-primeiro.md            # ESCRITO (parte 5)
    │   ├── ADR-0002-rust-estado-canonico.md        # ESCRITO (parte 5)
    │   ├── ADR-0003-lua-so-politicas.md            # ESCRITO (parte 5)
    │   ├── ADR-0004-contexto-tipado-versionado.md # ESCRITO (parte 5)
    │   ├── ADR-0005-nucleo-minimo-L1-L5.md         # ESCRITO (parte 6)
    │   └── ADR-0006-extensoes-fora-do-core.md     # ESCRITO (parte 6)
    ├── migration/                       # plano de reescrita
    │   ├── inventario_migracao.md      # ESCRITO (parte 6)
    │   └── plano_marcos.md             # ESCRITO (parte 7)
    └── audit/                           # estado atual do sistema legado
        ├── estado_atual_evidencias.md  # ESCRITO (parte 7)
        └── pendencias_riscos.md        # ESCRITO (parte 7)

Produção em partes pequenas: 2–5 arquivos por rodada, sem exceder o orçamento
de tokens da rodada.

## 3. Convenções epistêmicas (obrigatórias em todos os arquivos)

Escala de evidência E0–E5 — sem promoção automática:

    E0 DECLARED         componente/contrato declarado          ≠ existência funcional
    E1 INITIALIZED      objeto/dependências inicializados      ≠ execução produtiva
    E2 EXECUTED         caminho chamado                        ≠ saída válida/consumo
    E3 PRODUCTIVE       estado/saída nova e válida             ≠ integração L2/L3
    E4 CONSUMED         saída lida por consumidor identificado ≠ efeito downstream
    E5 EFFECT_VALIDATED efeito posterior validado no tempo    ≠ maturidade global

Ausência ≠ zero — todo valor carrega status:

    VALUE      medido e válido
    NO_DATA    sem provider ou sem amostra elegível
    STALE      fora da janela temporal
    INVALID    rejeitado por contrato
    FALLBACK   estrutural; nunca publicável como valor medido

Regras duras: decisão de scheduler (`should_run`/`run_count`) nunca é prova de
efeito; cobertura estrutural ≠ efeito causal; pipeline de execução ≠ camada
cognitiva; E5 exige efeito identificado no tempo (t+1/t+5 com braço sham
pareado quando houver mudança comportamental).

## 4. Mapa doc/ ↔ nova estrutura de código

| Documento | Serve a |
|:--|:--|
| `architecture/overview.md` | leitura inicial; decisão geral da nova arquitetura |
| `architecture/estrutura_diretorios.md` | árvore normativa do repositório (fonte 1 §49) |
| `architecture/contratos_camadas.md` | `crates/contracts/` + fronteiras L1→L5 |
| `architecture/rust_lua_boundary.md` | `crates/bindings/lua/` + raiz `lua/` |
| `architecture/dependency_rules.md` | regras de import + `tests/architecture/` |
| `adr/` | decisões irreversíveis (fonte 2 §45) |
| `migration/` | plano da fonte 1 §75–80 + fonte 2 §57 |
| `audit/` | espelho documental do legado Python (`core/*.py`) |
| `analysis/` | meta-documentação: como cada fonte foi lida e o que dela deriva |

## 5. Como usar

1. Comece por `architecture/overview.md` (o que a nova estrutura é).
2. `analysis/01–02` registram as decisões que a originaram (fontes 1–2).
3. `analysis/03–06` + `audit/` registram o estado real do legado (fontes 3–6).
4. ADRs fixam decisões irreversíveis; mudança de ADR exige ADR novo.
5. `migration/` define marcos e critérios de aceite por marco.
