# migrations

Mapas e políticas da migração Python→Rust do Triad_AEE/Cortex.
Política central: nada é apagado.
O legado é referência de comportamento até o marco 3.
Todo mapa old→new vive aqui, por domínio: state, database, config, schema.
legacy_python referencia a fonte legada, sempre read-only.
Cada item migrado registra origem, destino e status.
Nenhum código novo depende do legado.
Ao fim da migração, o legado é arquivado fora do runtime.
Mudança de comportamento exige registro aqui antes do código.
