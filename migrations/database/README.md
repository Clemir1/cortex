# migrations/database

Mapa old→new dos esquemas de persistência do legado.
Destino: nova persistência versionada sob var/runs.
Cada tabela/coleção legada vira um artefato do esquema versionado.
Cada run guarda seu snapshot próprio; nada é sobrescrito.
Exportação confere dados item a item antes de marcar migrado.
