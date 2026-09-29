# migrations/config

Mapa old→new das configs: YAML/JSON do legado para o TOML de config/.
Cobre perfis, modos e configuração de hardware.
Cada chave legada mapeia para uma chave TOML registrada em tabela.
Chave sem equivalente fica documentada; nada é descartado em silêncio.
Valores padrão do novo formato ficam explícitos no mapa.
