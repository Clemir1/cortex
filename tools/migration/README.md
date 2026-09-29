# tools/migration

Utilitários de apoio à migração Python→Rust do Triad_AEE/Cortex.
Ferramentas aqui são scripts/standalone; o runtime nunca as importa.
Diffs de comportamento: comparam saídas do legado com as do novo núcleo.
Extração de parâmetros: leem valores direto do código Python legado.
Inventário old→new: tabela gerada com status por item migrado.
Resultados alimentam os mapas mantidos em migrations/.
Sem efeito no build do workspace: tudo roda sob demanda.
